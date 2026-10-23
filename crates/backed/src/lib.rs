//! # backed
//!
//! Accounting building blocks for NEAR contracts that hold tokens.
//! One rule: **a contract never owes more than it holds.**
//!
//! NEAR calls between contracts are asynchronous and nothing rolls back on its
//! own. Three 2026 incidents (NEAR Intents, Rhea Finance, SWEAT/HOT) came from
//! a contract's ledger drifting away from the tokens it really held. This crate
//! makes those patterns hard or impossible to write:
//!
//! - [`Delivered`] vs [`Claimed`]: balances only grow from verified amounts.
//! - [`Ledger`]: per-token books with an `owed <= held` check on every change.
//! - [`refund`]: capped refunds and a log budget, so resolve callbacks can't fail.
//! - [`Ledger::begin_reconcile`] / [`Ledger::finish_reconcile`]: compare the
//!   books with the token contract's real balance, without false alarms from
//!   transfers that are still in flight.

pub mod amount;
pub mod ledger;
pub mod refund;

pub use amount::{Claimed, Delivered};
pub use ledger::{
    BookStatus, Ledger, PendingIn, PendingOut, ReconcileOutcome, ReconcileTicket, Solvency,
};
pub use refund::{cap_refund, LogBudget, LogBudgetExceeded, MAX_LOGS, TOTAL_LOG_LENGTH_LIMIT};

/// Panic message used when the core invariant would be broken.
pub const ERR_OWED_EXCEEDS_HELD: &str = "BACKED: owed would exceed held";
