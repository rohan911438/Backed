//! Making resolve callbacks infallible.
//!
//! On NEAR, if a receiver's resolve callback fails, the token contract refunds
//! everything, even if the receiver already credited the user. NEAR Intents
//! (Oct 2026) lost $3.8M this way: refund requests larger than the deposits
//! made the refund log exceed the 16,384-byte limit and the resolve failed.
//!
//! Rule: cap first, then measure the log of the capped values, then emit.

use near_sdk::env;

/// NEAR's total log size limit per receipt, in bytes.
pub const TOTAL_LOG_LENGTH_LIMIT: usize = 16_384;
/// NEAR's maximum number of log entries per receipt.
pub const MAX_LOGS: usize = 100;

/// Refund = min(requested, deposited, what the owner still has).
pub fn cap_refund(requested: u128, deposited: u128, owner_balance: u128) -> u128 {
    requested.min(deposited).min(owner_balance)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogBudgetExceeded {
    pub needed: usize,
    pub remaining: usize,
}

/// Tracks log bytes and log count for one receipt. Create it at the start of
/// the method, and send every log through it.
#[derive(Debug, Clone)]
pub struct LogBudget {
    bytes_left: usize,
    logs_left: usize,
}

impl Default for LogBudget {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBudget {
    pub fn new() -> Self {
        Self::with_limits(TOTAL_LOG_LENGTH_LIMIT, MAX_LOGS)
    }

    /// Reserve part of the budget, e.g. for logs emitted before this point.
    pub fn with_limits(bytes: usize, logs: usize) -> Self {
        Self { bytes_left: bytes, logs_left: logs }
    }

    pub fn fits(&self, msg: &str) -> bool {
        self.logs_left > 0 && msg.len() <= self.bytes_left
    }

    pub fn remaining_bytes(&self) -> usize {
        self.bytes_left
    }

    /// Emits only if it fits. Never panics.
    pub fn emit(&mut self, msg: &str) -> Result<(), LogBudgetExceeded> {
        if !self.fits(msg) {
            return Err(LogBudgetExceeded { needed: msg.len(), remaining: self.bytes_left });
        }
        env::log_str(msg);
        self.bytes_left -= msg.len();
        self.logs_left -= 1;
        Ok(())
    }

    /// Emits `msg` if it fits, otherwise the shorter `fallback`, otherwise
    /// nothing. Returns whether anything was emitted. Never panics.
    pub fn emit_or(&mut self, msg: &str, fallback: &str) -> bool {
        self.emit(msg).is_ok() || self.emit(fallback).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::testing_env;

    #[test]
    fn cap_is_min_of_three() {
        assert_eq!(cap_refund(10u128.pow(35), 1, 10u128.pow(37)), 1);
        assert_eq!(cap_refund(5, 10, 3), 3);
        assert_eq!(cap_refund(2, 10, 30), 2);
    }

    #[test]
    fn budget_refuses_oversized_logs() {
        testing_env!(VMContextBuilder::new().build());
        let mut b = LogBudget::with_limits(10, 5);
        assert!(b.emit("12345").is_ok());
        assert!(b.emit("123456").is_err());
        assert!(b.emit_or("123456", "ok"));
        assert_eq!(get_logs(), vec!["12345".to_string(), "ok".to_string()]);
    }

    #[test]
    fn budget_respects_log_count() {
        testing_env!(VMContextBuilder::new().build());
        let mut b = LogBudget::with_limits(1000, 2);
        assert!(b.emit("a").is_ok());
        assert!(b.emit("b").is_ok());
        assert!(b.emit("c").is_err());
    }

    /// The Intents bug in miniature: the log size depends on the refund value.
    /// Measured on capped values, it stays tiny; uncapped, it would explode.
    #[test]
    fn capping_first_keeps_logs_small() {
        let deposited = 1u128;
        let requested = 10u128.pow(35);
        let uncapped_log = format!("refund {}", requested).repeat(400);
        let capped_log = format!("refund {}", cap_refund(requested, deposited, u128::MAX)).repeat(400);
        assert!(uncapped_log.len() > TOTAL_LOG_LENGTH_LIMIT);
        assert!(capped_log.len() <= TOTAL_LOG_LENGTH_LIMIT);
    }
}
