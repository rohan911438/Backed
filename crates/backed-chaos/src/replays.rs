//! The three 2026 NEAR incidents as scenarios. Each runs against
//! `naive-vault` (expected: findings) and `backed-vault` (expected: none).

use crate::Injection;

/// Which test token a step uses. The Rhea replay swaps A for B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tok {
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// User deposits through the token's `ft_transfer_call`.
    Deposit { token: Tok, amount: u128, msg: String },
    /// User withdraws.
    Withdraw { token: Tok, amount: u128 },
    /// Attacker chains a failing call into the vault's withdraw resolver.
    CallResolverDirectly { token: Tok, amount: u128 },
    /// User swaps A for B through the lying venue.
    Swap { amount_in: u128, claimed_min_out: u128 },
    /// Anyone calls `reconcile(token)` (Backed contracts only).
    Reconcile { token: Tok },
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

fn step(actor: &'static str, action: Action, injection: Option<Injection>, note: &'static str) -> Step {
    Step { actor, action, injection, note }
}

/// NEAR Intents, Oct 2026: a deposit with a notify call; the receiver asks for
/// an absurd refund so the resolve fails; the token refunds the whole deposit
/// but the vault keeps the credit; the attacker withdraws it.
pub fn intents_log_limit() -> Scenario {
    Scenario {
        name: "replay_intents_log_limit",
        incident: "NEAR Intents, 2026-10-01, $3.8M",
        steps: vec![
            step("lp", Action::Deposit { token: Tok::A, amount: 1_000, msg: String::new() }, None, "honest liquidity in the vault"),
            step(
                "attacker",
                Action::Deposit { token: Tok::A, amount: 100, msg: "notify:evil".into() },
                Some(Injection::AbsurdRefund),
                "notify receiver asks for an absurd refund (u128::MAX)",
            ),
            step("attacker", Action::Withdraw { token: Tok::A, amount: 100 }, None, "withdraw the credit the token already refunded"),
        ],
    }
}

/// SWEAT/HOT: a public resolver lets an attacker re-credit a withdrawal.
pub fn sweat_public_resolver() -> Scenario {
    Scenario {
        name: "replay_sweat_public_resolver",
        incident: "SWEAT/HOT zero-day (Fireblocks)",
        steps: vec![
            step("lp", Action::Deposit { token: Tok::A, amount: 1_000, msg: String::new() }, None, "honest liquidity"),
            step("attacker", Action::Deposit { token: Tok::A, amount: 100, msg: String::new() }, None, "attacker deposits"),
            step("attacker", Action::Withdraw { token: Tok::A, amount: 100 }, None, "real withdrawal succeeds"),
            step(
                "attacker",
                Action::CallResolverDirectly { token: Tok::A, amount: 100 },
                Some(Injection::DirectCallbackCall),
                "chain a failing call into the vault's resolve_withdraw",
            ),
            step("attacker", Action::Withdraw { token: Tok::A, amount: 100 }, None, "withdraw again"),
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
            step("lp", Action::Deposit { token: Tok::B, amount: 1_000, msg: String::new() }, None, "honest liquidity (token_out side)"),
            step("attacker", Action::Deposit { token: Tok::A, amount: 100, msg: String::new() }, None, "attacker deposits token_in"),
            step(
                "attacker",
                Action::Swap { amount_in: 100, claimed_min_out: 900 },
                Some(Injection::LyingVenue),
                "swap: venue delivers 1 unit, reports 900, route claims 900",
            ),
            step("attacker", Action::Withdraw { token: Tok::B, amount: 900 }, None, "drain honest liquidity"),
            step("anyone", Action::Reconcile { token: Tok::B }, None, "reconcile: books vs real balance"),
        ],
    }
}

pub fn all() -> Vec<Scenario> {
    vec![intents_log_limit(), sweat_public_resolver(), rhea_claimed_credit()]
}
