//! Backed checker: anyone can record, on-chain, whether a NEAR protocol holds
//! the tokens it owes. `owed` comes from the protocol's own accounting views,
//! `held` from the token contract's `ft_balance_of`. No trusted reporter.
//!
//! On-chain reads land in different blocks, so one reading can be off by
//! transfers in flight. A breach is `Suspected` first, and `Confirmed` only
//! when a second attestation at least `min_gap_blocks` later still shows it.

pub use backed_checker_logic as logic;

use logic::{classify, parse_owed, parse_u128, Params, PairState};
use near_sdk::json_types::U128;
use near_sdk::store::{LookupMap, Vector};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

pub use logic::AdapterKind;

const GAS_VIEW: Gas = Gas::from_tgas(10);
const GAS_CALLBACK: Gas = Gas::from_tgas(15);
/// Attached to every `attest`. The callback keeps exactly the storage the call
/// used and refunds the rest, so callers, not the checker, pay for what they add.
const ATTEST_DEPOSIT: NearToken = NearToken::from_millinear(5);
/// Burrow `get_asset` JSON is ~2KB; Intents `mt_supply` is a short string.
const MAX_OWED_RESULT: usize = 8_192;
const MAX_U128_RESULT: usize = 64;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairStatus {
    pub state: PairState,
    pub last_block: u64,
    pub owed: U128,
    pub held: U128,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreachRecord {
    pub target: AccountId,
    pub token: AccountId,
    pub block: u64,
    pub owed: U128,
    pub held: U128,
    pub confirmed: bool,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Checker {
    owner: AccountId,
    params: Params,
    adapters: LookupMap<AccountId, AdapterKind>,
    pairs: LookupMap<(AccountId, AccountId), PairStatus>,
    breaches: Vector<BreachRecord>,
}

#[near]
impl Checker {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            owner,
            params: Params::default(),
            adapters: LookupMap::new(b"a"),
            pairs: LookupMap::new(b"p"),
            breaches: Vector::new(b"b"),
        }
    }

    /// Owner adds an adapter for a protocol that doesn't implement the Backed standard.
    #[payable]
    pub fn set_adapter(&mut self, target: AccountId, kind: AdapterKind) {
        self.assert_owner();
        self.adapters.insert(target, kind);
    }

    /// Hand ownership to another account (e.g. a multisig or DAO after launch).
    #[payable]
    pub fn set_owner(&mut self, owner: AccountId) {
        self.assert_owner();
        self.owner = owner;
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    /// A protocol registers itself as implementing `backed_solvency(token)`.
    #[payable]
    pub fn register_self(&mut self) {
        require!(env::attached_deposit() >= ATTEST_DEPOSIT, "attach 0.005 NEAR for storage");
        let me = env::predecessor_account_id();
        self.adapters.insert(me, AdapterKind::BackedStandard);
    }

    /// Reads owed and held, then records the result. Anyone can call.
    /// Attach 0.005 NEAR; whatever the new storage doesn't use is refunded.
    #[payable]
    pub fn attest(&mut self, target: AccountId, token: AccountId) -> Promise {
        let kind = self.adapters.get(&target).cloned().expect("no adapter for target");
        let deposit = env::attached_deposit();
        require!(deposit >= ATTEST_DEPOSIT, "attach 0.005 NEAR for storage (unused part is refunded)");
        let payer = env::predecessor_account_id();
        let owed_call = kind.owed_call(&target, &token, GAS_VIEW);
        let held_call = Promise::new(token.clone()).function_call(
            "ft_balance_of".to_string(),
            format!("{{\"account_id\":\"{}\"}}", target).into_bytes(),
            NearToken::from_yoctonear(0),
            GAS_VIEW,
        );
        owed_call.and(held_call).then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_CALLBACK)
                .on_attest(target, token, kind, payer, U128(deposit.as_yoctonear())),
        )
    }

    #[private]
    pub fn on_attest(
        &mut self,
        target: AccountId,
        token: AccountId,
        kind: AdapterKind,
        payer: AccountId,
        deposit: U128,
    ) -> Option<PairStatus> {
        let storage_before = env::storage_usage();
        let status = self.record(target, token, &kind);
        // Keep what the new storage costs, refund the rest. Never panics.
        let used = env::storage_usage().saturating_sub(storage_before) as u128;
        let cost = env::storage_byte_cost().as_yoctonear().saturating_mul(used);
        let refund = deposit.0.saturating_sub(cost);
        if refund > 0 {
            Promise::new(payer).transfer(NearToken::from_yoctonear(refund)).detach();
        }
        status
    }

    fn record(&mut self, target: AccountId, token: AccountId, kind: &AdapterKind) -> Option<PairStatus> {
        let owed = read_result(0, MAX_OWED_RESULT).and_then(|b| parse_owed(kind, &b));
        let held = read_result(1, MAX_U128_RESULT).and_then(|b| parse_u128(&b));
        let (Some(owed), Some(held)) = (owed, held) else {
            env::log_str("backed: attestation skipped, a view call failed or returned bad data");
            return None;
        };
        let key = (target.clone(), token.clone());
        let block = env::block_height();
        let prev = self.pairs.get(&key).map(|p| p.state.clone());
        let state = classify(prev.as_ref(), owed, held, block, &self.params);
        let status = PairStatus { state: state.clone(), last_block: block, owed: U128(owed), held: U128(held) };
        self.pairs.insert(key, status.clone());

        let event = match &state {
            PairState::Healthy => "attestation",
            PairState::Suspected { .. } => "breach_suspected",
            PairState::Confirmed { .. } => "breach_confirmed",
        };
        env::log_str(&format!(
            "EVENT_JSON:{{\"standard\":\"backed_checker\",\"version\":\"0.1.0\",\"event\":\"{event}\",\"data\":[{{\"target\":\"{target}\",\"token\":\"{token}\",\"owed\":\"{owed}\",\"held\":\"{held}\",\"block\":{block}}}]}}"
        ));
        // Log a breach only when the state changes (Healthy -> Suspected,
        // Suspected -> Confirmed), so repeated attests can't flood the log.
        let was = prev.as_ref().map(std::mem::discriminant);
        if !matches!(state, PairState::Healthy) && was != Some(std::mem::discriminant(&state)) {
            let confirmed = matches!(state, PairState::Confirmed { .. });
            self.breaches.push(BreachRecord { target, token, block, owed: U128(owed), held: U128(held), confirmed });
        }
        Some(status)
    }

    pub fn get_status(&self, target: AccountId, token: AccountId) -> Option<PairStatus> {
        self.pairs.get(&(target, token)).cloned()
    }

    pub fn get_adapter(&self, target: AccountId) -> Option<AdapterKind> {
        self.adapters.get(&target).cloned()
    }

    pub fn list_breaches(&self, from_index: u32, limit: u32) -> Vec<BreachRecord> {
        self.breaches.iter().skip(from_index as usize).take(limit.min(100) as usize).cloned().collect()
    }

    pub fn get_params(&self) -> Params {
        self.params.clone()
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner, "owner only");
        require!(env::attached_deposit() >= NearToken::from_yoctonear(1), "attach 1 yoctoNEAR");
    }
}

/// Bounded read, so a target returning a huge result can't make the
/// callback run out of gas (griefing the checker).
fn read_result(i: u64, max_len: usize) -> Option<Vec<u8>> {
    env::promise_result_checked(i, max_len).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn acct(s: &str) -> AccountId {
        s.parse().unwrap()
    }

    #[test]
    fn owner_sets_adapter_and_others_cannot() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).attached_deposit(NearToken::from_yoctonear(1)).build());
        let mut c = Checker::new(acct("owner.near"));
        c.set_adapter(acct("intents.near"), AdapterKind::Nep245Ledger);
        assert_eq!(c.get_adapter(acct("intents.near")), Some(AdapterKind::Nep245Ledger));
    }

    #[test]
    #[should_panic(expected = "owner only")]
    fn stranger_cannot_set_adapter() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).build());
        let mut c = Checker::new(acct("owner.near"));
        testing_env!(ctx.predecessor_account_id(acct("x.near")).attached_deposit(NearToken::from_yoctonear(1)).build());
        c.set_adapter(acct("intents.near"), AdapterKind::Nep245Ledger);
    }

    #[test]
    #[should_panic(expected = "attach 0.005 NEAR")]
    fn attest_needs_storage_deposit() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).attached_deposit(NearToken::from_yoctonear(1)).build());
        let mut c = Checker::new(acct("owner.near"));
        c.set_adapter(acct("intents.near"), AdapterKind::Nep245Ledger);
        testing_env!(ctx.predecessor_account_id(acct("anyone.near")).attached_deposit(NearToken::from_yoctonear(0)).build());
        let _ = c.attest(acct("intents.near"), acct("usdt.tether-token.near"));
    }

    /// Runs `on_attest` as the checker's own callback with the given view results.
    fn callback(c: &mut Checker, owed: &str, held: &str, block: u64) -> Option<PairStatus> {
        use near_sdk::{test_vm_config, PromiseResult, RuntimeFeesConfig};
        let ctx = VMContextBuilder::new()
            .current_account_id(acct("checker.near"))
            .predecessor_account_id(acct("checker.near"))
            .block_height(block)
            .build();
        testing_env!(
            ctx,
            test_vm_config(),
            RuntimeFeesConfig::test(),
            Default::default(),
            vec![
                PromiseResult::Successful(owed.as_bytes().to_vec()),
                PromiseResult::Successful(held.as_bytes().to_vec()),
            ]
        );
        c.on_attest(
            acct("evil.near"),
            acct("token.near"),
            AdapterKind::Nep245Ledger,
            acct("caller.near"),
            U128(ATTEST_DEPOSIT.as_yoctonear()),
        )
    }

    #[test]
    fn repeated_attests_log_a_breach_once_per_state_change() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).build());
        let mut c = Checker::new(acct("owner.near"));
        // A target that always reports owing far more than it holds.
        for b in 0..20 {
            callback(&mut c, "\"1000\"", "\"1\"", 100 + b);
        }
        let log = c.list_breaches(0, 100);
        assert_eq!(log.len(), 1, "20 suspected readings in the gap window add one record");
        assert!(!log[0].confirmed);
        // After the gap, the state changes to Confirmed: exactly one more record.
        for b in 0..20 {
            callback(&mut c, "\"1000\"", "\"1\"", 200 + b);
        }
        let log = c.list_breaches(0, 100);
        assert_eq!(log.len(), 2);
        assert!(log[1].confirmed);
    }

    #[test]
    fn owner_can_hand_over_ownership() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).attached_deposit(NearToken::from_yoctonear(1)).build());
        let mut c = Checker::new(acct("owner.near"));
        c.set_owner(acct("dao.near"));
        assert_eq!(c.get_owner(), acct("dao.near"));
    }
}
