//! Deploys the checker to a live network and records real attestations.
//!
//! ```text
//! cargo run -p backed-chaos --features sandbox --example live -- \
//!     <testnet|mainnet> <account-id> <preflight|deploy|upgrade|vault|attest|status> [target token]
//! ```
//!
//! The signing key is read from `~/.near-credentials/<network>/<account-id>.json`
//! (the near-cli layout), never from the command line.
//!
//! - `deploy`: uploads `backed_checker.wasm` to `<account-id>`, calls
//!   `new(owner = account-id)`, and sets the Burrow (and on mainnet, Intents) adapters.
//! - `preflight`: free. Prints the wasm size, the storage it locks, and whether
//!   the account can afford it. Run this before `deploy` on mainnet.
//! - `upgrade`: uploads a new `backed_checker.wasm` without calling `new`, so
//!   recorded attestations and adapters are kept.
//! - `attest <target> <token>`: one on-chain attestation; prints the result.
//! - `status <target> <token>`: reads the stored status (view, free).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use near_workspaces::types::{Gas, NearToken};
use near_workspaces::{Account, AccountId, Contract, Network, Worker};
use serde_json::{json, Value};

const TESTNET_RPC: &str = "https://test.rpc.fastnear.com";
const MAINNET_RPC: &str = "https://free.rpc.fastnear.com";

fn adapters(network: &str) -> Vec<(&'static str, &'static str)> {
    match network {
        "mainnet" => vec![("intents.near", "Nep245Ledger"), ("contract.main.burrow.near", "BurrowLending")],
        _ => vec![("contract.1638481328.burrow.testnet", "BurrowLending")],
    }
}

fn credentials(network: &str, account: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let p = PathBuf::from(home).join(".near-credentials").join(network).join(format!("{account}.json"));
    if !p.exists() {
        bail!("no key file at {}", p.display());
    }
    Ok(p)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [network, account, cmd, rest @ ..] = args.as_slice() else {
        bail!("usage: live <testnet|mainnet> <account-id> <preflight|deploy|upgrade|vault|attest|status> [target token]");
    };
    match network.as_str() {
        "testnet" => {
            let worker = near_workspaces::testnet().rpc_addr(TESTNET_RPC).await?;
            // `vault` needs a fresh account funded by the testnet faucet helper.
            let dev = if cmd == "vault" { Some(worker.dev_create_account().await?) } else { None };
            run(worker, network, account, cmd, rest, dev).await
        }
        "mainnet" => run(near_workspaces::mainnet().rpc_addr(MAINNET_RPC).await?, network, account, cmd, rest, None).await,
        other => bail!("unknown network {other}"),
    }
}

async fn run<N: Network + 'static>(
    worker: Worker<N>,
    network: &str,
    account: &str,
    cmd: &str,
    rest: &[String],
    dev: Option<Account>,
) -> Result<()> {
    let signer = Account::from_file(credentials(network, account)?, &worker)?;
    let checker: AccountId = account.parse()?;
    match (cmd, rest) {
        ("preflight", []) => {
            let wasm = std::fs::read(backed_chaos::sandbox::wasm_path("backed_checker")?)?;
            let details = worker.view_account(&checker).await?;
            // 1 NEAR per 100 KB of storage (10^19 yocto per byte), plus state and gas headroom.
            let code_cost = NearToken::from_yoctonear(wasm.len() as u128 * 10u128.pow(19));
            let headroom = NearToken::from_millinear(300);
            let need = code_cost.saturating_add(headroom);
            println!("network:        {network}");
            println!("account:        {checker}");
            println!("wasm:           {} bytes", wasm.len());
            println!("storage locked: {} NEAR (for the code)", fmt_near(code_cost));
            println!("plus headroom:  {} NEAR (state, adapters, gas)", fmt_near(headroom));
            // Storage already used by this account is locked and not spendable.
            let locked = NearToken::from_yoctonear(details.storage_usage as u128 * 10u128.pow(19));
            let free = details.balance.saturating_sub(locked);
            println!("balance:        {} NEAR ({} locked by current storage, {} free)", fmt_near(details.balance), fmt_near(locked), fmt_near(free));
            if free >= need {
                println!("OK: enough to deploy.");
            } else {
                bail!("not enough NEAR: need about {} more", fmt_near(need.saturating_sub(free)));
            }
        }
        ("upgrade", []) => {
            let wasm = std::fs::read(backed_chaos::sandbox::wasm_path("backed_checker")?)?;
            println!("upgrading {checker} on {network} to {} bytes (state kept)...", wasm.len());
            signer.deploy(&wasm).await?.into_result()?;
            let owner: Value = worker.view(&checker, "get_owner").await?.json()?;
            println!("  upgraded; owner is still {owner}");
        }
        ("vault", []) if dev.is_some() => {
            // Example Backed vault at vault.<account>, checked through the backed_solvency standard.
            let token: AccountId = "wrap.testnet".parse()?;
            let wasm = std::fs::read(backed_chaos::sandbox::wasm_path("backed_vault")?)?;
            // A fresh dev account funded by the testnet faucet helper (the vault's
            // code needs ~1.6 NEAR of storage). Its key is saved like near-cli's.
            let vault = dev.expect("dev account");
            let dir = credentials(network, account)?.parent().map(|p| p.to_path_buf()).context("credentials dir")?;
            vault.store_credentials(&dir).await?;
            println!("  created {} (key saved in {})", vault.id(), dir.display());
            let c: Contract = vault.deploy(&wasm).await?.into_result()?;
            let init = c.call("new").args_json(json!({ "owner": checker, "tokens": [token] })).transact().await?;
            report("vault new", &init)?;
            for (who, what) in [(vault.id().clone(), "vault"), (checker.clone(), "signer")] {
                let res = signer
                    .call(&token, "storage_deposit")
                    .args_json(json!({ "account_id": who, "registration_only": true }))
                    .deposit(NearToken::from_micronear(1_250))
                    .transact()
                    .await?;
                report(&format!("wrap storage_deposit for {what}"), &res)?;
            }
            let res = signer
                .call(&checker, "set_adapter")
                .args_json(json!({ "target": vault.id(), "kind": "BackedStandard" }))
                .deposit(NearToken::from_yoctonear(1))
                .transact()
                .await?;
            report("set_adapter vault = BackedStandard", &res)?;
            // A real deposit: wrap 0.01 NEAR, send it to the vault, then attest.
            let amount = NearToken::from_millinear(10);
            let res = signer.call(&token, "near_deposit").deposit(amount).transact().await?;
            report("wrap 0.01 NEAR", &res)?;
            let res = signer
                .call(&token, "ft_transfer_call")
                .args_json(json!({ "receiver_id": vault.id(), "amount": amount.as_yoctonear().to_string(), "msg": "" }))
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(100))
                .transact()
                .await?;
            report("deposit 0.01 wNEAR into the vault", &res)?;
            let solvency: Value = worker.view(vault.id(), "backed_solvency").args_json(json!({ "token": token })).await?.json()?;
            println!("  vault backed_solvency: {solvency}");
            let res = signer
                .call(&checker, "attest")
                .args_json(json!({ "target": vault.id(), "token": token }))
                .deposit(NearToken::from_millinear(5))
                .gas(Gas::from_tgas(100))
                .transact()
                .await?;
            report("attest vault", &res)?;
            println!("  result: {}", res.json::<Option<Value>>().ok().flatten().map(|v| v.to_string()).unwrap_or_default());
        }
        ("deploy", []) => {
            let wasm = std::fs::read(backed_chaos::sandbox::wasm_path("backed_checker")?)?;
            println!("deploying {} bytes to {checker} on {network}...", wasm.len());
            let contract: Contract = signer.deploy(&wasm).await?.into_result()?;
            println!("  deployed, tx ok");
            let init = contract.call("new").args_json(json!({ "owner": checker })).transact().await?;
            report("new", &init)?;
            for (target, kind) in adapters(network) {
                let res = signer
                    .call(&checker, "set_adapter")
                    .args_json(json!({ "target": target, "kind": kind }))
                    .deposit(NearToken::from_yoctonear(1))
                    .transact()
                    .await?;
                report(&format!("set_adapter {target} = {kind}"), &res)?;
            }
        }
        ("attest", [target, token]) => {
            let res = signer
                .call(&checker, "attest")
                .args_json(json!({ "target": target, "token": token }))
                .deposit(NearToken::from_millinear(5))
                .gas(Gas::from_tgas(100))
                .transact()
                .await?;
            report("attest", &res)?;
            for log in res.logs() {
                println!("  log: {log}");
            }
            let status: Option<Value> = res.json().unwrap_or(None);
            println!("  result: {}", status.map(|s| s.to_string()).unwrap_or_else(|| "null".into()));
        }
        ("status", [target, token]) => {
            let v: Value = worker
                .view(&checker, "get_status")
                .args_json(json!({ "target": target, "token": token }))
                .await?
                .json()?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        _ => bail!("usage: live <testnet|mainnet> <account-id> <preflight|deploy|upgrade|vault|attest|status> [target token]"),
    }
    Ok(())
}

fn report(what: &str, res: &near_workspaces::result::ExecutionFinalResult) -> Result<()> {
    let hash = res.outcome().transaction_hash;
    if let Some(f) = res.receipt_failures().first() {
        bail!("{what} failed (tx {hash}): {:?}", (*f).clone().into_result().err());
    }
    println!("  {what}: ok (tx {hash})");
    Ok(())
}

fn fmt_near(t: NearToken) -> String {
    format!("{:.4}", t.as_yoctonear() as f64 / 1e24)
}
