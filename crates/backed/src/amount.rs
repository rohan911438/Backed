//! Verified vs unverified amounts.
//!
//! Rhea Finance lost $18.4M (April 2026) by crediting the amount a swap route
//! *claimed* instead of what was delivered. Here, a balance can only grow from a
//! [`Delivered`] value, and there is no way to turn a [`Claimed`] into one.

use near_sdk::{env, AccountId, PromiseResult};

/// Any amount a caller says: function arguments, `msg` fields, route minimums.
/// It can be used for checks and limits, never to credit a balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Claimed(pub u128);

/// An amount the contract has verified. It can only be built by:
/// - [`Delivered::from_token_transfer`]: inside `ft_on_transfer`, when the caller
///   is a token contract the contract trusts (the token vouches for the amount);
/// - [`Delivered::from_callback_u128`]: inside a private callback, from the
///   result of a call the contract made itself.
#[derive(Debug, PartialEq, Eq)]
pub struct Delivered {
    amount: u128,
}

impl Delivered {
    /// Amount the token contract reports transferring to this contract.
    ///
    /// Returns `None` when the caller isn't `expected_token`. A contract
    /// receiving an unexpected token should return the full amount as unused.
    pub fn from_token_transfer(expected_token: &AccountId, amount: u128) -> Option<Self> {
        if &env::predecessor_account_id() == expected_token {
            Some(Self { amount })
        } else {
            None
        }
    }

    /// Reads a `U128` (JSON string) result of promise `index` inside a private
    /// callback. Returns `None` if this isn't a self-callback, the promise
    /// failed, or the value can't be parsed.
    pub fn from_callback_u128(index: u64) -> Option<Self> {
        if env::predecessor_account_id() != env::current_account_id() {
            return None;
        }
        match env::promise_result(index) {
            PromiseResult::Successful(bytes) => parse_u128_json(&bytes).map(|amount| Self { amount }),
            PromiseResult::Failed => None,
        }
    }

    /// Splits off `part` (e.g. a capped refund). Returns `None` if `part` is
    /// larger than this amount.
    pub fn split(self, part: u128) -> Option<(Delivered, Delivered)> {
        let rest = self.amount.checked_sub(part)?;
        Some((Self { amount: part }, Self { amount: rest }))
    }

    pub fn amount(&self) -> u128 {
        self.amount
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn for_tests(amount: u128) -> Self {
        Self { amount }
    }
}

/// Parses `"123"` or `123` as u128.
pub(crate) fn parse_u128_json(bytes: &[u8]) -> Option<u128> {
    let s = std::str::from_utf8(bytes).ok()?.trim();
    let s = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(s);
    s.parse().ok()
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
    fn token_transfer_only_from_expected_token() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx.predecessor_account_id(acct("usdc.near")).build());
        assert_eq!(Delivered::from_token_transfer(&acct("usdc.near"), 5).unwrap().amount(), 5);

        // A fake "token" (or any account) calling ft_on_transfer gets nothing.
        testing_env!(ctx.predecessor_account_id(acct("attacker.near")).build());
        assert!(Delivered::from_token_transfer(&acct("usdc.near"), 5).is_none());
    }

    #[test]
    fn callback_value_only_in_self_callback() {
        let mut ctx = VMContextBuilder::new();
        testing_env!(ctx
            .current_account_id(acct("vault.near"))
            .predecessor_account_id(acct("attacker.near"))
            .build());
        assert!(Delivered::from_callback_u128(0).is_none());
    }

    #[test]
    fn parse_json_u128() {
        assert_eq!(parse_u128_json(b"\"42\""), Some(42));
        assert_eq!(parse_u128_json(b"42"), Some(42));
        assert_eq!(parse_u128_json(b"\"-1\""), None);
        assert_eq!(parse_u128_json(b"garbage"), None);
    }

    #[test]
    fn split_never_creates_value() {
        let d = Delivered::for_tests(10);
        assert!(Delivered::for_tests(10).split(11).is_none());
        let (a, b) = d.split(3).unwrap();
        assert_eq!((a.amount(), b.amount()), (3, 7));
    }
}
