//! The three 2026 NEAR incidents as scenarios. Each runs against
//! `naive-vault` (expected: findings) and `backed-vault` (expected: none).

use crate::Injection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// User deposits through the token's `ft_transfer_call`.
    Deposit { amount: u128, msg: String },
    /// User withdraws.
    Withdraw { amount: u128 },
    /// Attacker chains a failing call into the vault's withdraw resolver.
    CallResolverDirectly { amount: u128 },
    /// User swaps through a venue.
    Swap { amount_in: u128, claimed_min_out: u128 },
    /// Anyone calls `reconcile(token)` (Backed contracts only).
    Reconcile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub actor: &'static str,
    pub action: Action,
    pub injection: Option<Injection>,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: &'static str,
    pub incident: &'static str,
    pub steps: Vec<Step>,
}

/// NEAR Intents, Oct 2026: a deposit with a notify call; the receiver asks for
/// an absurd refund so the resolve fails; the token refunds the whole deposit
/// but the vault keeps the credit; the attacker withdraws it.
pub fn intents_log_limit() -> Scenario {
    Scenario {
        name: "replay_intents_log_limit",
        incident: "NEAR Intents, 2026-10-01, $3.8M",
        steps: vec![
            Step { actor: "lp", action: Action::Deposit { amount: 1_000, msg: String::new() }, injection: None, note: "honest liquidity in the vault" },
            Step {
                actor: "attacker",
                action: Action::Deposit { amount: 100, msg: "notify:evil-receiver".into() },
                injection: Some(Injection::AbsurdRefund),
                note: "receiver requests a refund far above the deposit; naive resolve fails",
            },
            Step { actor: "attacker", action: Action::Withdraw { amount: 100 }, injection: None, note: "withdraw the credit the token already refunded" },
        ],
    }
}

/// SWEAT/HOT: a public resolver lets an attacker re-credit a withdrawal.
pub fn sweat_public_resolver() -> Scenario {
    Scenario {
        name: "replay_sweat_public_resolver",
        incident: "SWEAT/HOT zero-day (Fireblocks)",
        steps: vec![
            Step { actor: "lp", action: Action::Deposit { amount: 1_000, msg: String::new() }, injection: None, note: "honest liquidity" },
            Step { actor: "attacker", action: Action::Deposit { amount: 100, msg: String::new() }, injection: None, note: "attacker deposits" },
            Step { actor: "attacker", action: Action::Withdraw { amount: 100 }, injection: None, note: "real withdrawal succeeds" },
            Step {
                actor: "attacker",
                action: Action::CallResolverDirectly { amount: 100 },
                injection: Some(Injection::DirectCallbackCall),
                note: "chain a failing call into resolve_withdraw: naive re-credits",
            },
            Step { actor: "attacker", action: Action::Withdraw { amount: 100 }, injection: None, note: "withdraw again" },
        ],
    }
}

/// Rhea, Apr 2026: the vault credits the route's claimed minimum, while the
/// (lying or fake) venue delivers almost nothing.
pub fn rhea_claimed_credit() -> Scenario {
    Scenario {
        name: "replay_rhea_claimed_credit",
        incident: "Rhea Finance, 2026-04-16, $18.4M",
        steps: vec![
            Step { actor: "lp", action: Action::Deposit { amount: 1_000, msg: String::new() }, injection: None, note: "honest liquidity (token_out side)" },
            Step { actor: "attacker", action: Action::Deposit { amount: 100, msg: String::new() }, injection: None, note: "attacker deposits token_in" },
            Step {
                actor: "attacker",
                action: Action::Swap { amount_in: 100, claimed_min_out: 900 },
                injection: Some(Injection::LyingVenue),
                note: "venue delivers 1 unit; naive credits the claimed 900",
            },
            Step { actor: "attacker", action: Action::Withdraw { amount: 900 }, injection: None, note: "drain honest liquidity" },
            Step { actor: "anyone", action: Action::Reconcile, injection: None, note: "Backed: books vs reality" },
        ],
    }
}

pub fn all() -> Vec<Scenario> {
    vec![intents_log_limit(), sweat_public_resolver(), rhea_claimed_credit()]
}
