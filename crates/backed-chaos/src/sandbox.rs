//! The NEAR sandbox runner: real contracts, real receipts, local node.
//!
//! [`Sandbox::start`] boots a local NEAR sandbox (near-workspaces), deploys two
//! test tokens, the vault under test, the attacker's contract and a lying swap
//! venue, then funds the actors. [`Chain::run_step`] sends each step as a real
//! transaction and reads `owed` (sum of vault balances) vs `held`
//! (`ft_balance_of(vault)`) once all its receipts have finished.
//!
//! Needs the compiled wasm files (`scripts/build-wasm.sh`). Looks in
//! `$BACKED_WASM_DIR`, then `$CARGO_TARGET_DIR/near`, then `<repo>/target/near`.

use std::path::PathBuf;

use near_workspaces::network::Sandbox as SandboxNet;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, AccountId, Contract, Worker};
use serde_json::{json, Value};
use tokio::runtime::Runtime;

use crate::replays::{Action, Step, Tok};
use crate::{Chain, Snapshot, StepOutcome};

/// Which vault the scenario attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultKind {
    /// `naive-vault`: the three 2026 bug patterns, on purpose.
    Naive,
    /// `backed-vault`: the same features built on the `backed` library.
    Backed,
}

impl VaultKind {
    pub fn wasm_name(self) -> &'static str {
        match self {
            VaultKind::Naive => "naive_vault",
            VaultKind::Backed => "backed_vault",
        }
    }
}

/// Starting token balance minted to each actor (and to the venue for B).
pub const START_BALANCE: u128 = 10_000;

pub struct Sandbox {
    rt: Runtime,
    worker: Worker<SandboxNet>,
    kind: VaultKind,
    vault: Contract,
    token_a: Contract,
    token_b: Contract,
    evil: Contract,
    venue: Contract,
    lp: Account,
    attacker: Account,
}

/// Finds a compiled contract: `<dir>/<name>/<name>.wasm` (cargo-near layout).
pub fn wasm_path(name: &str) -> anyhow::Result<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("BACKED_WASM_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Ok(d) = std::env::var("CARGO_TARGET_DIR") {
        dirs.push(PathBuf::from(d).join("near"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/near"));
    for d in &dirs {
        let p = d.join(name).join(format!("{name}.wasm"));
        if p.exists() {
            return Ok(p);
        }
    }
    anyhow::bail!("{name}.wasm not found in {dirs:?}; run scripts/build-wasm.sh first")
}

fn read_wasm(name: &str) -> anyhow::Result<Vec<u8>> {
    Ok(std::fs::read(wasm_path(name)?)?)
}

impl Sandbox {
    /// Boots a fresh sandbox and deploys everything for `kind`.
    pub fn start(kind: VaultKind) -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let (worker, vault, token_a, token_b, evil, venue, lp, attacker) = rt.block_on(async {
            let worker = near_workspaces::sandbox().await?;
            let root = worker.root_account()?;

            let deploy = |name: &'static str, wasm: Vec<u8>| {
                let root = root.clone();
                async move {
                    let acc = root
                        .create_subaccount(name)
                        .initial_balance(NearToken::from_near(30))
                        .transact()
                        .await?
                        .into_result()?;
                    anyhow::Ok(acc.deploy(&wasm).await?.into_result()?)
                }
            };
            let user = |name: &'static str| {
                let root = root.clone();
                async move {
                    anyhow::Ok(
                        root.create_subaccount(name)
                            .initial_balance(NearToken::from_near(20))
                            .transact()
                            .await?
                            .into_result()?,
                    )
                }
            };

            let token_wasm = read_wasm("test_token")?;
            let token_a = deploy("token-a", token_wasm.clone()).await?;
            let token_b = deploy("token-b", token_wasm).await?;
            let vault = deploy("vault", read_wasm(kind.wasm_name())?).await?;
            let evil = deploy("evil", read_wasm("evil_receiver")?).await?;
            let venue = deploy("venue", read_wasm("lying_venue")?).await?;
            let lp = user("lp").await?;
            let attacker = user("attacker").await?;

            for t in [&token_a, &token_b] {
                expect_ok(t.call("new").transact().await?, "token new")?;
            }
            let init = match kind {
                VaultKind::Naive => vault.call("new").transact().await?,
                VaultKind::Backed => {
                    vault
                        .call("new")
                        .args_json(json!({ "owner": root.id(), "tokens": [token_a.id(), token_b.id()] }))
                        .transact()
                        .await?
                }
            };
            expect_ok(init, "vault new")?;
            expect_ok(
                venue.call("new").args_json(json!({ "deliver": "1", "report": "900" })).transact().await?,
                "venue new",
            )?;

            for t in [&token_a, &token_b] {
                for who in [vault.id(), evil.id(), venue.id(), lp.id(), attacker.id()] {
                    expect_ok(t.call("register").args_json(json!({ "account_id": who })).transact().await?, "register")?;
                }
                for who in [lp.id(), attacker.id()] {
                    mint(t, who, START_BALANCE).await?;
                }
            }
            mint(&token_b, venue.id(), START_BALANCE).await?;
            anyhow::Ok((worker, vault, token_a, token_b, evil, venue, lp, attacker))
        })?;
        Ok(Self { rt, worker, kind, vault, token_a, token_b, evil, venue, lp, attacker })
    }

    pub fn kind(&self) -> VaultKind {
        self.kind
    }

    fn token(&self, t: Tok) -> &Contract {
        match t {
            Tok::A => &self.token_a,
            Tok::B => &self.token_b,
        }
    }

    fn actor(&self, name: &str) -> &Account {
        match name {
            "attacker" => &self.attacker,
            _ => &self.lp,
        }
    }

    /// The attacker's total position across both tokens (wallet + vault
    /// balance) minus what they started with. Positive = the attack paid.
    pub fn attacker_gain(&self) -> anyhow::Result<i128> {
        self.rt.block_on(async {
            let mut total: i128 = 0;
            for t in [Tok::A, Tok::B] {
                let token = self.token(t);
                total += ft_balance(token, self.attacker.id()).await? as i128;
                total += vault_balance(&self.vault, token.id(), self.attacker.id()).await? as i128;
            }
            Ok(total - 2 * START_BALANCE as i128)
        })
    }

    async fn snapshot(&self, t: Tok) -> anyhow::Result<Snapshot> {
        let token = self.token(t);
        let mut owed = 0u128;
        for who in [self.lp.id(), self.attacker.id(), self.evil.id(), self.venue.id()] {
            owed += vault_balance(&self.vault, token.id(), who).await?;
        }
        let held = ft_balance(token, self.vault.id()).await?;
        let block = self.worker.view_block().await?.height();
        Ok(Snapshot { block, token: token.id().to_string(), owed, held })
    }

    async fn send(&self, step: &Step) -> anyhow::Result<Option<ExecutionFinalResult>> {
        let who = self.actor(step.actor);
        let res = match &step.action {
            Action::Deposit { token, amount, msg } => {
                let msg = match msg.strip_prefix("notify:") {
                    Some("evil") => format!("notify:{}", self.evil.id()),
                    _ => msg.clone(),
                };
                who.call(self.token(*token).id(), "ft_transfer_call")
                    .args_json(json!({ "receiver_id": self.vault.id(), "amount": amount.to_string(), "msg": msg }))
                    .deposit(NearToken::from_yoctonear(1))
                    .max_gas()
                    .transact()
                    .await?
            }
            Action::Withdraw { token, amount } => {
                who.call(self.vault.id(), "withdraw")
                    .args_json(json!({ "token": self.token(*token).id(), "amount": amount.to_string() }))
                    .max_gas()
                    .transact()
                    .await?
            }
            Action::CallResolverDirectly { token, amount } => {
                let fields = json!({ "token": self.token(*token).id(), "owner": who.id(), "amount": amount.to_string() });
                let args: Value = match self.kind {
                    VaultKind::Naive => fields,
                    VaultKind::Backed => json!({ "pending": fields }),
                };
                who.call(self.evil.id(), "chain_resolve")
                    .args_json(json!({ "vault": self.vault.id(), "method": "resolve_withdraw", "args": args.to_string() }))
                    .max_gas()
                    .transact()
                    .await?
            }
            Action::Swap { amount_in, claimed_min_out } => {
                let mut args = json!({
                    "venue": self.venue.id(),
                    "token_in": self.token_a.id(),
                    "token_out": self.token_b.id(),
                    "amount_in": amount_in.to_string(),
                });
                if self.kind == VaultKind::Naive {
                    args["claimed_min_out"] = json!(claimed_min_out.to_string());
                }
                who.call(self.vault.id(), "swap").args_json(args).max_gas().transact().await?
            }
            Action::Reconcile { token } => {
                if self.kind == VaultKind::Naive {
                    return Ok(None); // the naive vault has no reconcile
                }
                who.call(self.vault.id(), "reconcile")
                    .args_json(json!({ "token": self.token(*token).id() }))
                    .max_gas()
                    .transact()
                    .await?
            }
        };
        Ok(Some(res))
    }
}

impl Chain for Sandbox {
    type Error = anyhow::Error;

    fn run_step(&mut self, step: &Step) -> anyhow::Result<StepOutcome> {
        self.rt.block_on(async {
            let (ok, error) = match self.send(step).await? {
                None => (true, Some("skipped: not supported by this vault".to_string())),
                Some(res) => match first_failure(&res) {
                    None => (true, None),
                    Some(e) => (false, Some(e)),
                },
            };
            let snapshots = vec![self.snapshot(Tok::A).await?, self.snapshot(Tok::B).await?];
            Ok(StepOutcome { ok, error, snapshots })
        })
    }
}

/// The first failed receipt's error, shortened. A transaction can "succeed"
/// while a receipt inside it fails (that is how the Intents bug works), so we
/// look at every receipt.
fn first_failure(res: &ExecutionFinalResult) -> Option<String> {
    let msg = if let Some(f) = res.receipt_failures().first() {
        match (*f).clone().into_result() {
            Err(e) => format!("{e:?}"),
            Ok(_) => "receipt failed".to_string(),
        }
    } else if res.is_failure() {
        "transaction failed".to_string()
    } else {
        return None;
    };
    Some(shorten(&msg))
}

/// Keeps the useful part of a NEAR error ("Smart contract panicked: ...").
fn shorten(msg: &str) -> String {
    let key = "Smart contract panicked: ";
    let s = match msg.find(key) {
        Some(i) => &msg[i + key.len()..],
        None => msg,
    };
    let s: String = s.chars().take(160).collect();
    s.replace("\\n", " ").trim_end_matches(['"', ')', '}', ' ']).to_string()
}

fn expect_ok(res: ExecutionFinalResult, what: &str) -> anyhow::Result<()> {
    match first_failure(&res) {
        None => Ok(()),
        Some(e) => anyhow::bail!("setup step `{what}` failed: {e}"),
    }
}

async fn mint(token: &Contract, who: &AccountId, amount: u128) -> anyhow::Result<()> {
    let res = token.call("mint").args_json(json!({ "account_id": who, "amount": amount.to_string() })).transact().await?;
    expect_ok(res, "mint")
}

async fn ft_balance(token: &Contract, who: &AccountId) -> anyhow::Result<u128> {
    let v: String = token.view("ft_balance_of").args_json(json!({ "account_id": who })).await?.json()?;
    Ok(v.parse()?)
}

async fn vault_balance(vault: &Contract, token: &AccountId, who: &AccountId) -> anyhow::Result<u128> {
    let v: String = vault.view("balance_of").args_json(json!({ "token": token, "owner": who })).await?.json()?;
    Ok(v.parse()?)
}

/// The result of replaying one scenario against one vault.
pub struct ReplayResult {
    pub kind: VaultKind,
    pub report: crate::Report,
    /// See [`Sandbox::attacker_gain`].
    pub attacker_gain: i128,
}

impl ReplayResult {
    /// Broke = owed > held at some step, or the attacker made a profit.
    pub fn broke(&self) -> bool {
        !self.report.findings.is_empty() || self.attacker_gain > 0
    }
}

/// Boots a fresh sandbox, deploys `kind`, and runs `sc` against it.
pub fn replay(sc: &crate::replays::Scenario, kind: VaultKind) -> anyhow::Result<ReplayResult> {
    let mut chain = Sandbox::start(kind)?;
    let report = crate::run(&mut chain, sc)?;
    let attacker_gain = chain.attacker_gain()?;
    Ok(ReplayResult { kind, report, attacker_gain })
}

/// A readable step-by-step report: what each step did, books vs reality.
pub fn render(sc: &crate::replays::Scenario, r: &ReplayResult) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let verdict = if r.broke() { "BROKEN" } else { "SAFE" };
    let _ = writeln!(out, "== {} vs {:?} vault: {verdict}  ({})", sc.name, r.kind, sc.incident);
    for (i, (step, o)) in sc.steps.iter().zip(&r.report.outcomes).enumerate() {
        let status = if o.ok { "ok    " } else { "FAILED" };
        let tag = step.injection.map(|j| format!(" [{}]", j.id())).unwrap_or_default();
        let _ = writeln!(out, "  {i}. {:<8} {status} {}{tag}", step.actor, step.note);
        if let Some(e) = &o.error {
            let _ = writeln!(out, "       -> {e}");
        }
        for s in &o.snapshots {
            if s.owed > 0 || s.held > 0 {
                let mark = if s.owed > s.held { format!("  <-- SHORTFALL {}", s.owed - s.held) } else { String::new() };
                let _ = writeln!(out, "       {:<20} owed {:>5} / held {:>5}{mark}", s.token, s.owed, s.held);
            }
        }
    }
    let _ = writeln!(out, "  attacker gain: {}", r.attacker_gain);
    out
}
