//! DELIBERATELY VULNERABLE. Used by `backed-chaos` replays. Never deploy with real funds.
//!
//! A small token vault that contains, on purpose, the three bug patterns
//! behind NEAR's 2026 incidents:
//!
//! 1. **Intents pattern (Oct 2026):** a deposit with a follow-up "notify" call.
//!    The resolve subtracts an uncapped refund and can panic. When it does,
//!    the token contract refunds the whole deposit, but the credit stays.
//! 2. **SWEAT/HOT pattern:** `resolve_withdraw` is public. Anyone can chain a
//!    failing call into it and get re-credited for a withdrawal that happened.
//! 3. **Rhea pattern (Apr 2026):** `swap` credits the minimum the caller's
//!    route *claims*, not what the venue delivered.
//!
//! Plus: `ft_on_transfer` accepts any caller as if it were the token.

use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, is_promise_success, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue};

const GAS: Gas = Gas::from_tgas(20);

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct NaiveVault {
    balances: LookupMap<(AccountId, AccountId), u128>,
}

#[near]
impl NaiveVault {
    #[init]
    pub fn new() -> Self {
        Self { balances: LookupMap::new(b"b") }
    }

    pub fn balance_of(&self, token: AccountId, owner: AccountId) -> U128 {
        U128(self.get(&token, &owner))
    }

    /// BUG: no check that the caller is a token this vault expects.
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        let token = env::predecessor_account_id();
        self.add(&token, &sender_id, amount.0);
        if let Some(receiver) = msg.strip_prefix("notify:") {
            let receiver: AccountId = receiver.parse().expect("bad receiver");
            return PromiseOrValue::Promise(
                Promise::new(receiver)
                    .function_call(
                        "on_deposit_notify".to_string(),
                        format!("{{\"sender_id\":\"{sender_id}\",\"amount\":\"{}\"}}", amount.0).into_bytes(),
                        NearToken::from_yoctonear(0),
                        GAS,
                    )
                    .then(Self::ext(env::current_account_id()).with_static_gas(GAS).resolve_deposit(token, sender_id)),
            );
        }
        PromiseOrValue::Value(U128(0))
    }

    /// BUG (Intents pattern): refund isn't capped, so this can panic. A failed
    /// resolve makes the token refund everything while the credit stays.
    #[private]
    pub fn resolve_deposit(&mut self, token: AccountId, sender_id: AccountId) -> U128 {
        #[allow(deprecated)]
        let requested: u128 = match env::promise_result(0) {
            near_sdk::PromiseResult::Successful(b) => {
                near_sdk::serde_json::from_slice::<U128>(&b).map(|x| x.0).unwrap_or(0)
            }
            near_sdk::PromiseResult::Failed => 0,
        };
        let bal = self.get(&token, &sender_id);
        self.set(&token, &sender_id, bal - requested); // panics if requested > balance
        U128(requested)
    }

    pub fn withdraw(&mut self, token: AccountId, amount: U128) -> Promise {
        let owner = env::predecessor_account_id();
        let bal = self.get(&token, &owner);
        assert!(bal >= amount.0, "insufficient balance");
        self.set(&token, &owner, bal - amount.0);
        Promise::new(token.clone())
            .function_call(
                "ft_transfer".to_string(),
                format!("{{\"receiver_id\":\"{owner}\",\"amount\":\"{}\"}}", amount.0).into_bytes(),
                NearToken::from_yoctonear(1),
                GAS,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS).resolve_withdraw(token, owner, amount))
    }

    /// BUG (SWEAT/HOT pattern): missing #[private]. Chaining a failing call
    /// into this re-credits a withdrawal that never failed.
    pub fn resolve_withdraw(&mut self, token: AccountId, owner: AccountId, amount: U128) {
        if !is_promise_success() {
            self.add(&token, &owner, amount.0);
        }
    }

    /// Swaps `amount_in` of `token_in` on `venue`; output arrives as `token_out`.
    pub fn swap(
        &mut self,
        venue: AccountId,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: U128,
        claimed_min_out: U128,
    ) -> Promise {
        let owner = env::predecessor_account_id();
        let bal = self.get(&token_in, &owner);
        assert!(bal >= amount_in.0, "insufficient balance");
        self.set(&token_in, &owner, bal - amount_in.0);
        Promise::new(venue)
            .function_call(
                "swap".to_string(),
                format!("{{\"token_in\":\"{token_in}\",\"token_out\":\"{token_out}\",\"amount_in\":\"{}\"}}", amount_in.0).into_bytes(),
                NearToken::from_yoctonear(0),
                GAS,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS).on_swap(token_out, owner, claimed_min_out))
    }

    /// BUG (Rhea pattern): credits what the route claimed, not what arrived.
    #[private]
    pub fn on_swap(&mut self, token_out: AccountId, owner: AccountId, claimed_min_out: U128) {
        if is_promise_success() {
            self.add(&token_out, &owner, claimed_min_out.0);
        }
    }

    fn get(&self, token: &AccountId, owner: &AccountId) -> u128 {
        self.balances.get(&(token.clone(), owner.clone())).copied().unwrap_or(0)
    }
    fn set(&mut self, token: &AccountId, owner: &AccountId, v: u128) {
        self.balances.insert((token.clone(), owner.clone()), v);
    }
    fn add(&mut self, token: &AccountId, owner: &AccountId, v: u128) {
        let b = self.get(token, owner);
        self.set(token, owner, b + v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn acct(s: &str) -> AccountId {
        s.parse().unwrap()
    }

    /// Demonstrates the fake-deposit bug: any account can mint itself a balance.
    #[test]
    fn anyone_can_fake_a_deposit() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("attacker.near")).build());
        let mut v = NaiveVault::new();
        let _ = v.ft_on_transfer(acct("attacker.near"), U128(1_000), String::new());
        assert_eq!(v.balance_of(acct("attacker.near"), acct("attacker.near")).0, 1_000);
    }
}
