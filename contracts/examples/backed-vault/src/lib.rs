//! The naive vault's features, built on `backed`. Same methods, none of the bugs:
//!
//! 1. Notify deposits: refunds are capped and finalizing never panics.
//! 2. Withdraw resolve is `#[private]`.
//! 3. Swaps credit only what the venue's own result says, and `reconcile`
//!    catches a venue that reports more than it sent.
//! 4. `ft_on_transfer` only accepts registered tokens.

use backed::{Claimed, Delivered, Ledger, PendingIn, PendingOut, ReconcileTicket, Solvency};
use near_sdk::json_types::U128;
use near_sdk::{env, is_promise_success, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue};

const GAS: Gas = Gas::from_tgas(20);
/// Gas the resolve callbacks need; checked at entry so they can't run out.
const GAS_RESOLVE: Gas = Gas::from_tgas(15);

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct BackedVault {
    owner: AccountId,
    ledger: Ledger,
}

#[near]
impl BackedVault {
    #[init]
    pub fn new(owner: AccountId, tokens: Vec<AccountId>) -> Self {
        let mut ledger = Ledger::new(b"L");
        for t in tokens {
            ledger.register_token(t);
        }
        Self { owner, ledger }
    }

    pub fn balance_of(&self, token: AccountId, owner: AccountId) -> U128 {
        U128(self.ledger.balance_of(&token, &owner))
    }

    /// Standard view (draft NEP): lets the Backed checker read this contract.
    pub fn backed_solvency(&self, token: AccountId) -> Option<Solvency> {
        self.ledger.solvency(&token)
    }

    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        let token = env::predecessor_account_id();
        if !self.ledger.is_registered(&token) {
            return PromiseOrValue::Value(amount); // unknown token: return everything
        }
        let Some(delivered) = Delivered::from_token_transfer(&token, amount.0) else {
            return PromiseOrValue::Value(amount);
        };
        if let Some(receiver) = msg.strip_prefix("notify:") {
            let Ok(receiver) = receiver.parse::<AccountId>() else {
                return PromiseOrValue::Value(amount);
            };
            require!(env::prepaid_gas() >= GAS.saturating_add(GAS_RESOLVE).saturating_add(GAS), "not enough gas for resolve");
            let pending = self.ledger.credit_with_followup(&token, &sender_id, delivered);
            return PromiseOrValue::Promise(
                Promise::new(receiver)
                    .function_call(
                        "on_deposit_notify".to_string(),
                        format!("{{\"sender_id\":\"{sender_id}\",\"amount\":\"{}\"}}", amount.0).into_bytes(),
                        NearToken::from_yoctonear(0),
                        GAS,
                    )
                    .then(Self::ext(env::current_account_id()).with_static_gas(GAS_RESOLVE).resolve_deposit(pending)),
            );
        }
        self.ledger.credit(&token, &sender_id, delivered);
        PromiseOrValue::Value(U128(0))
    }

    /// Never panics: the refund is capped by the deposit and the balance.
    #[private]
    pub fn resolve_deposit(&mut self, pending: PendingIn) -> U128 {
        let requested = env::promise_result_checked(0, 64)
            .ok()
            .and_then(|b| near_sdk::serde_json::from_slice::<U128>(&b).ok())
            .map(|x| x.0)
            .unwrap_or(0);
        U128(self.ledger.finalize_followup(&pending, requested))
    }

    pub fn withdraw(&mut self, token: AccountId, amount: U128) -> Promise {
        let owner = env::predecessor_account_id();
        let pending = self.ledger.debit_for_withdraw(&token, &owner, Claimed(amount.0));
        Promise::new(token)
            .function_call(
                "ft_transfer".to_string(),
                format!("{{\"receiver_id\":\"{owner}\",\"amount\":\"{}\"}}", amount.0).into_bytes(),
                NearToken::from_yoctonear(1),
                GAS,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_RESOLVE).resolve_withdraw(pending))
    }

    #[private]
    pub fn resolve_withdraw(&mut self, pending: PendingOut) {
        self.ledger.settle_withdraw(&pending, is_promise_success());
    }

    pub fn swap(&mut self, venue: AccountId, token_in: AccountId, token_out: AccountId, amount_in: U128) -> Promise {
        require!(self.ledger.is_registered(&token_out), "unknown token_out");
        let owner = env::predecessor_account_id();
        // Treated like a withdrawal to the venue: debited before it leaves.
        let pending = self.ledger.debit_for_withdraw(&token_in, &owner, Claimed(amount_in.0));
        Promise::new(venue)
            .function_call(
                "swap".to_string(),
                format!("{{\"token_in\":\"{token_in}\",\"token_out\":\"{token_out}\",\"amount_in\":\"{}\"}}", amount_in.0).into_bytes(),
                NearToken::from_yoctonear(0),
                GAS,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_RESOLVE).on_swap(pending, token_out))
    }

    /// Credits only the amount the venue's own result reports. There is no
    /// way to credit a caller-supplied minimum: it isn't a `Delivered`.
    #[private]
    pub fn on_swap(&mut self, pending: PendingOut, token_out: AccountId) {
        match Delivered::from_callback_u128(0) {
            Some(out) => {
                self.ledger.settle_withdraw(&pending, true);
                self.ledger.credit(&token_out, &pending.owner, out);
            }
            None => self.ledger.settle_withdraw(&pending, false),
        }
    }

    /// Compares the books with the token's real balance. Anyone can call.
    pub fn reconcile(&mut self, token: AccountId) -> Promise {
        let ticket = self.ledger.begin_reconcile(&token);
        Promise::new(token)
            .function_call(
                "ft_balance_of".to_string(),
                format!("{{\"account_id\":\"{}\"}}", env::current_account_id()).into_bytes(),
                NearToken::from_yoctonear(0),
                GAS,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_RESOLVE).on_reconcile(ticket))
    }

    #[private]
    pub fn on_reconcile(&mut self, ticket: ReconcileTicket) -> bool {
        let Some(real) = env::promise_result_checked(0, 64)
            .ok()
            .and_then(|b| near_sdk::serde_json::from_slice::<U128>(&b).ok())
        else {
            return false;
        };
        matches!(self.ledger.finish_reconcile(&ticket, real.0), backed::ReconcileOutcome::Healthy)
    }

    pub fn owner(&self) -> AccountId {
        self.owner.clone()
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

    fn vault() -> (BackedVault, VMContextBuilder) {
        let mut ctx = VMContextBuilder::new();
        ctx.current_account_id(acct("vault.near"));
        testing_env!(ctx.build());
        (BackedVault::new(acct("owner.near"), vec![acct("usdc.near")]), ctx)
    }

    #[test]
    fn fake_deposit_is_returned() {
        let (mut v, mut ctx) = vault();
        testing_env!(ctx.predecessor_account_id(acct("attacker.near")).build());
        match v.ft_on_transfer(acct("attacker.near"), U128(1_000), String::new()) {
            PromiseOrValue::Value(unused) => assert_eq!(unused.0, 1_000),
            _ => panic!("expected value"),
        }
        assert_eq!(v.balance_of(acct("attacker.near"), acct("attacker.near")).0, 0);
    }

    #[test]
    fn real_deposit_is_credited_and_solvent() {
        let (mut v, mut ctx) = vault();
        testing_env!(ctx.predecessor_account_id(acct("usdc.near")).build());
        let _ = v.ft_on_transfer(acct("alice.near"), U128(500), String::new());
        assert_eq!(v.balance_of(acct("usdc.near"), acct("alice.near")).0, 500);
        let s = v.backed_solvency(acct("usdc.near")).unwrap();
        assert_eq!((s.owed.0, s.shadow_held.0), (500, 500));
    }

    // `#[private]` is enforced by the wrapper near-sdk generates for the compiled
    // contract, not by the Rust method, so a unit test calling the method
    // directly can't check it. It is covered in the sandbox by backed-chaos
    // injection F5 (DirectCallbackCall), see replay_sweat_public_resolver.
}
