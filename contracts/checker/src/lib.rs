//! Backed checker: anyone can record, on-chain, whether a NEAR protocol holds
//! the tokens it owes. `owed` comes from the protocol's own accounting views,
//! `held` from the token contract's `ft_balance_of`. No trusted reporter.
//!
//! On-chain reads land in different blocks, so one reading can be off by
//! transfers in flight. A breach is `Suspected` first, and `Confirmed` only
//! when a second attestation at least `min_gap_blocks` later still shows it.

pub mod logic;

use logic::{classify, parse_owed, parse_u128, Params, PairState};
use near_sdk::json_types::U128;
use near_sdk::store::{LookupMap, Vector};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

pub use logic::AdapterKind;

const GAS_VIEW: Gas = Gas::from_tgas(10);
const GAS_CALLBACK: Gas = Gas::from_tgas(15);
/// Paid by whoever creates a new (target, token) record; covers its storage.
const NEW_PAIR_DEPOSIT: NearToken = NearToken::from_millinear(5);
const MAX_BREACH_LOG: u32 = 1_000;
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

    /// A protocol registers itself as implementing `backed_solvency(token)`.
    #[payable]
    pub fn register_self(&mut self) {
        require!(env::attached_deposit() >= NEW_PAIR_DEPOSIT, "attach 0.005 NEAR for storage");
        let me = env::predecessor_account_id();
        self.adapters.insert(me, AdapterKind::BackedStandard);
    }

    /// Reads owed and held, then records the result. Anyone can call.
    #[payable]
    pub fn attest(&mut self, target: AccountId, token: AccountId) -> Promise {
        let kind = self.adapters.get(&target).cloned().expect("no adapter for target");
        let key = (target.clone(), token.clone());
        if !self.pairs.contains_key(&key) {
            require!(env::attached_deposit() >= NEW_PAIR_DEPOSIT, "new pair: attach 0.005 NEAR for storage");
        }
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
                .on_attest(target, token, kind),
        )
    }

    #[private]
    pub fn on_attest(&mut self, target: AccountId, token: AccountId, kind: AdapterKind) -> Option<PairStatus> {
        let owed = read_result(0, MAX_OWED_RESULT).and_then(|b| parse_owed(&kind, &b));
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
        if !matches!(state, PairState::Healthy) && self.breaches.len() < MAX_BREACH_LOG {
            let confirmed = matches!(state, PairState::Confirmed { .. });
            let newly = !matches!((prev, &state), (Some(PairState::Confirmed { .. }), PairState::Confirmed { .. }));
            if newly {
                self.breaches.push(BreachRecord { target, token, block, owed: U128(owed), held: U128(held), confirmed });
            }
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
    #[should_panic(expected = "new pair")]
    fn new_pair_needs_storage_deposit() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("owner.near")).attached_deposit(NearToken::from_yoctonear(1)).build());
        let mut c = Checker::new(acct("owner.near"));
        c.set_adapter(acct("intents.near"), AdapterKind::Nep245Ledger);
        testing_env!(ctx.predecessor_account_id(acct("anyone.near")).attached_deposit(NearToken::from_yoctonear(0)).build());
        let _ = c.attest(acct("intents.near"), acct("usdt.tether-token.near"));
    }
}
