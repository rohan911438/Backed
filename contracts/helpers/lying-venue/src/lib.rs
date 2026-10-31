//! TEST HELPER. A swap venue that lies (Rhea Finance pattern, Apr 2026).
//!
//! `swap` sends only `deliver` units of `token_out` back to the vault (with
//! `ft_transfer_call`, msg `swap_out:<owner>`), but its return value reports
//! `report` units. A vault that trusts the reported or claimed amount owes more
//! than it holds.

use near_sdk::json_types::U128;
use near_sdk::{env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct LyingVenue {
    deliver: u128,
    report: u128,
}

#[near]
impl LyingVenue {
    #[init]
    pub fn new(deliver: U128, report: U128) -> Self {
        Self { deliver: deliver.0, report: report.0 }
    }

    /// Called by a vault. Token_in stays with the vault in this model; only the
    /// output side matters for the replay.
    pub fn swap(&mut self, token_in: AccountId, token_out: AccountId, amount_in: U128, owner: AccountId) -> U128 {
        let _ = (token_in, amount_in);
        let vault = env::predecessor_account_id();
        // Detached promise: delivery happens, but the reply below doesn't wait for it.
        let _ = Promise::new(token_out).function_call(
            "ft_transfer_call".to_string(),
            format!(
                "{{\"receiver_id\":\"{vault}\",\"amount\":\"{}\",\"msg\":\"swap_out:{owner}\"}}",
                self.deliver
            )
            .into_bytes(),
            NearToken::from_yoctonear(1),
            Gas::from_tgas(60),
        );
        U128(self.report)
    }
}
