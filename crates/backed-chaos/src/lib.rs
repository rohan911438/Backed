//! # backed-chaos
//!
//! Chaos testing for NEAR contracts that hold tokens. It injects NEAR-specific
//! runtime failures and checks the backing invariant (`owed <= held`) after
//! every receipt.
//!
//! This crate has three layers:
//! - [`Injection`]: the failure catalog, each mapped to a real incident;
//! - [`check_backing`]: the invariant oracle over one snapshot;
//! - [`replays`]: the three 2026 incident scenarios as step lists.
//!
//! A [`Chain`] implementation runs the steps. The NEAR sandbox runner
//! (near-sandbox, Linux/WSL) implements it; see `Idea/03-architecture.md`.

pub mod replays;

/// Failure injections. Each one models how a NEAR async step can go wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Injection {
    /// F1: attach too little gas so the callback runs out.
    CallbackOutOfGas,
    /// F2: make the resolve emit logs past NEAR's 16,384-byte limit.
    LogLimitOverflow,
    /// F3: receiver panics.
    ReceiverPanics,
    /// F4: receiver asks for an absurd refund (u128::MAX, more than deposited).
    AbsurdRefund,
    /// F5: call a callback / resolver directly from outside.
    DirectCallbackCall,
    /// F6: token misbehaves: wrong amounts, fees on transfer, burns, re-entry.
    MaliciousToken,
    /// F7: venue reports more output than it delivers.
    LyingVenue,
    /// F8: storage runs out during a callback.
    StorageExhaustion,
}

impl Injection {
    pub const ALL: [Injection; 8] = [
        Injection::CallbackOutOfGas,
        Injection::LogLimitOverflow,
        Injection::ReceiverPanics,
        Injection::AbsurdRefund,
        Injection::DirectCallbackCall,
        Injection::MaliciousToken,
        Injection::LyingVenue,
        Injection::StorageExhaustion,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Injection::CallbackOutOfGas => "F1",
            Injection::LogLimitOverflow => "F2",
            Injection::ReceiverPanics => "F3",
            Injection::AbsurdRefund => "F4",
            Injection::DirectCallbackCall => "F5",
            Injection::MaliciousToken => "F6",
            Injection::LyingVenue => "F7",
            Injection::StorageExhaustion => "F8",
        }
    }

    /// The real incident this injection reproduces, if any.
    pub fn incident(self) -> Option<&'static str> {
        match self {
            Injection::LogLimitOverflow | Injection::AbsurdRefund => Some("NEAR Intents, 2026-10-01, $3.8M"),
            Injection::DirectCallbackCall => Some("SWEAT/HOT zero-day (Fireblocks)"),
            Injection::LyingVenue => Some("Rhea Finance, 2026-04-16, $18.4M"),
            Injection::MaliciousToken => Some("Under-backed memecoins in intents.near (Oct 2026 scan)"),
            _ => None,
        }
    }
}

/// One reading of a contract's books against reality, at a single block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub block: u64,
    pub token: String,
    /// What the contract owes (its own accounting).
    pub owed: u128,
    /// What the token contract says the contract holds (`ft_balance_of`).
    pub held: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub step: usize,
    pub injection: Option<Injection>,
    pub snapshot: Snapshot,
    pub shortfall: u128,
}

/// The invariant oracle: `owed <= held`. Returns the shortfall if broken.
pub fn check_backing(s: &Snapshot) -> Option<u128> {
    (s.owed > s.held).then(|| s.owed - s.held)
}

/// What a runner must provide. The sandbox runner implements this.
pub trait Chain {
    type Error: std::fmt::Debug;
    /// Executes one step and returns snapshots taken after each receipt.
    fn run_step(&mut self, step: &replays::Step) -> Result<Vec<Snapshot>, Self::Error>;
}

/// Runs a scenario and collects every point where the invariant broke.
pub fn run<C: Chain>(chain: &mut C, scenario: &replays::Scenario) -> Result<Vec<Finding>, C::Error> {
    let mut findings = Vec::new();
    for (i, step) in scenario.steps.iter().enumerate() {
        for snap in chain.run_step(step)? {
            if let Some(shortfall) = check_backing(&snap) {
                findings.push(Finding { step: i, injection: step.injection, snapshot: snap, shortfall });
            }
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_injection_has_a_unique_id() {
        let mut ids: Vec<_> = Injection::ALL.iter().map(|i| i.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8);
    }

    #[test]
    fn oracle_flags_only_shortfalls() {
        let s = |owed, held| Snapshot { block: 1, token: "t".into(), owed, held };
        assert_eq!(check_backing(&s(10, 10)), None);
        assert_eq!(check_backing(&s(10, 12)), None);
        assert_eq!(check_backing(&s(10, 7)), Some(3));
    }

    /// A toy chain that replays a fixed list of snapshots, to test `run`.
    struct Scripted(Vec<Vec<Snapshot>>);
    impl Chain for Scripted {
        type Error = ();
        fn run_step(&mut self, _: &replays::Step) -> Result<Vec<Snapshot>, ()> {
            Ok(if self.0.is_empty() { vec![] } else { self.0.remove(0) })
        }
    }

    #[test]
    fn run_collects_findings_with_step_and_injection() {
        let sc = replays::intents_log_limit();
        let snap = |owed, held| Snapshot { block: 1, token: "tok".into(), owed, held };
        let mut chain = Scripted(vec![vec![snap(0, 0)], vec![snap(100, 100)], vec![snap(100, 0)]]);
        let f = run(&mut chain, &sc).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].shortfall, 100);
        assert_eq!(f[0].step, 2);
    }

    #[test]
    fn each_replay_maps_to_a_real_incident() {
        for sc in replays::all() {
            let tagged = sc.steps.iter().filter_map(|s| s.injection).filter_map(|i| i.incident()).count();
            assert!(tagged > 0, "{} has no incident-tagged step", sc.name);
        }
    }
}
