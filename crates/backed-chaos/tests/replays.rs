//! The three 2026 incident replays, on a real local NEAR sandbox.
//!
//! Run (Linux/macOS/WSL): `scripts/build-wasm.sh && cargo test -p backed-chaos --features sandbox replay_`
//!
//! Each replay runs twice: against `naive-vault` (must break: owed > held and
//! the attacker profits) and against `backed-vault` (must hold: no shortfall at
//! any step, and the attacker gains nothing).
#![cfg(feature = "sandbox")]

use backed_chaos::replays::{self, Scenario};
use backed_chaos::sandbox::{render, replay, VaultKind};

fn check(sc: Scenario) {
    let naive = replay(&sc, VaultKind::Naive).expect("naive replay");
    println!("{}", render(&sc, &naive));
    assert!(!naive.report.findings.is_empty(), "{}: naive vault should end up owing more than it holds", sc.name);
    assert!(naive.attacker_gain > 0, "{}: the attack should pay against the naive vault", sc.name);

    let backed = replay(&sc, VaultKind::Backed).expect("backed replay");
    println!("{}", render(&sc, &backed));
    assert!(backed.report.findings.is_empty(), "{}: backed vault broke the invariant: {:?}", sc.name, backed.report.findings);
    assert!(backed.attacker_gain <= 0, "{}: attacker gained {} against the backed vault", sc.name, backed.attacker_gain);
}

#[test]
fn replay_intents_log_limit() {
    check(replays::intents_log_limit());
}

#[test]
fn replay_sweat_public_resolver() {
    check(replays::sweat_public_resolver());
}

#[test]
fn replay_rhea_claimed_credit() {
    check(replays::rhea_claimed_credit());
}
