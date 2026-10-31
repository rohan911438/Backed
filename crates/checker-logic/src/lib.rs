//! Pure logic for the Backed checker contract: adapters, parsing and breach classification.
//! Kept free of chain calls so it can be tested against real mainnet data.

use near_sdk::{near, AccountId, Gas, NearToken, Promise};

/// How to read what a protocol owes for a token.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterKind {
    /// NEAR Intents style multi-token ledger: `mt_supply("nep141:<token>")`.
    Nep245Ledger,
    /// Rhea (Burrow) lending: `get_asset(token)`;
    /// owed = (supplied + reserved + prot_fee - borrowed) / 10^extra_decimals.
    BurrowLending,
    /// Contract implementing `backed_solvency(token) -> { owed, .. }`.
    BackedStandard,
}

impl AdapterKind {
    pub fn owed_call(&self, target: &AccountId, token: &AccountId, gas: Gas) -> Promise {
        let (method, args) = match self {
            AdapterKind::Nep245Ledger => ("mt_supply", format!("{{\"token_id\":\"nep141:{token}\"}}")),
            AdapterKind::BurrowLending => ("get_asset", format!("{{\"token_id\":\"{token}\"}}")),
            AdapterKind::BackedStandard => ("backed_solvency", format!("{{\"token\":\"{token}\"}}")),
        };
        Promise::new(target.clone()).function_call(method.to_string(), args.into_bytes(), NearToken::from_yoctonear(0), gas)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    /// Shortfalls up to this share of `owed` are treated as noise (basis points).
    pub tolerance_bps: u32,
    /// Shortfalls up to this absolute amount (base units) are treated as noise.
    pub dust: u128,
    /// A second breach reading this many blocks after the first confirms it.
    pub min_gap_blocks: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self { tolerance_bps: 10, dust: 0, min_gap_blocks: 30 }
    }
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairState {
    Healthy,
    Suspected { first_block: u64 },
    Confirmed { first_block: u64 },
}

pub fn is_breach(owed: u128, held: u128, p: &Params) -> bool {
    if held >= owed {
        return false;
    }
    let shortfall = owed - held;
    let tolerance = owed / 10_000 * p.tolerance_bps as u128;
    shortfall > tolerance.max(p.dust)
}

pub fn classify(prev: Option<&PairState>, owed: u128, held: u128, block: u64, p: &Params) -> PairState {
    if !is_breach(owed, held, p) {
        return PairState::Healthy;
    }
    match prev {
        Some(PairState::Suspected { first_block }) if block.saturating_sub(*first_block) >= p.min_gap_blocks => {
            PairState::Confirmed { first_block: *first_block }
        }
        Some(PairState::Suspected { first_block }) => PairState::Suspected { first_block: *first_block },
        Some(PairState::Confirmed { first_block }) => PairState::Confirmed { first_block: *first_block },
        _ => PairState::Suspected { first_block: block },
    }
}

// A tiny JSON reader for the few fields we need. A general JSON parser added
// ~50KB to the wasm, which costs ~0.5 NEAR in storage.

/// Reads a number written as `"123"` or `123` at the start of `s`.
fn leading_u128(s: &str) -> Option<u128> {
    let s = s.trim_start();
    let s = s.strip_prefix('"').unwrap_or(s);
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    s[..end].parse().ok()
}

/// Finds each key in order (`"key"` followed by `:`), then reads the number
/// after the last one. Nested paths work because each search starts after the
/// previous key, e.g. `["supplied", "balance"]`.
fn u128_after_keys(json: &str, keys: &[&str]) -> Option<u128> {
    let mut rest = json;
    for key in keys {
        let pat = format!("\"{key}\"");
        loop {
            let i = rest.find(&pat)?;
            let after = rest[i + pat.len()..].trim_start();
            if let Some(v) = after.strip_prefix(':') {
                rest = v;
                break;
            }
            rest = &rest[i + pat.len()..];
        }
    }
    leading_u128(rest)
}

/// `"123"` or `123` → 123.
pub fn parse_u128(bytes: &[u8]) -> Option<u128> {
    let s = core::str::from_utf8(bytes).ok()?.trim();
    let n = leading_u128(s)?;
    // the whole value must be the number (optionally quoted)
    let digits = s.trim_matches('"');
    (digits.len() == n.to_string().len()).then_some(n)
}

pub fn burrow_owed(bytes: &[u8]) -> Option<u128> {
    let s = core::str::from_utf8(bytes).ok()?;
    let supplied = u128_after_keys(s, &["supplied", "balance"])?;
    let borrowed = u128_after_keys(s, &["borrowed", "balance"])?;
    let reserved = u128_after_keys(s, &["reserved"])?;
    let prot_fee = u128_after_keys(s, &["prot_fee"]).unwrap_or(0);
    let extra = u128_after_keys(s, &["config", "extra_decimals"])? as u32;
    let inner = supplied.checked_add(reserved)?.checked_add(prot_fee)?.checked_sub(borrowed)?;
    Some(inner / 10u128.checked_pow(extra)?)
}

pub fn parse_owed(kind: &AdapterKind, bytes: &[u8]) -> Option<u128> {
    match kind {
        AdapterKind::Nep245Ledger => parse_u128(bytes),
        AdapterKind::BurrowLending => burrow_owed(bytes),
        AdapterKind::BackedStandard => u128_after_keys(core::str::from_utf8(bytes).ok()?, &["owed"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::serde_json::{self, Value};

    const BURROW_WRAP: &[u8] = include_bytes!("fixture_burrow_wrap_near.json");

    #[test]
    fn burrow_owed_matches_live_mainnet_math() {
        // Fixture: real `get_asset("wrap.near")` response, 2026-10-23.
        let owed = burrow_owed(BURROW_WRAP).expect("parse");
        let v: Value = serde_json::from_slice(BURROW_WRAP).unwrap();
        let s: u128 = v["supplied"]["balance"].as_str().unwrap().parse().unwrap();
        let r: u128 = v["reserved"].as_str().unwrap().parse().unwrap();
        let f: u128 = v["prot_fee"].as_str().unwrap().parse().unwrap();
        let b: u128 = v["borrowed"]["balance"].as_str().unwrap().parse().unwrap();
        assert_eq!(owed, s + r + f - b);
        // ~2.8M wNEAR owed (24 decimals), in line with the live scan.
        let whole = owed / 10u128.pow(24);
        assert!((2_000_000..4_000_000).contains(&whole), "owed {whole} wNEAR");
    }

    #[test]
    fn parses_intents_mt_supply_string() {
        assert_eq!(parse_u128(b"\"513080149399\""), Some(513_080_149_399));
        assert_eq!(parse_u128(b"null"), None);
    }

    #[test]
    fn parses_backed_standard_view() {
        let b = br#"{"owed":"100","shadow_held":"100","status":"Healthy"}"#;
        assert_eq!(parse_owed(&AdapterKind::BackedStandard, b), Some(100));
    }

    #[test]
    fn breach_needs_shortfall_beyond_tolerance() {
        let p = Params { tolerance_bps: 10, dust: 0, min_gap_blocks: 30 };
        assert!(!is_breach(1_000_000, 1_000_000, &p));
        assert!(!is_breach(1_000_000, 999_000, &p)); // 0.1%: noise
        assert!(is_breach(1_000_000, 998_999, &p));
        // Live example: intents USDT owed 513,080.149399, held 513,100.579125 → healthy.
        assert!(!is_breach(513_080_149_399, 513_100_579_125, &p));
    }

    #[test]
    fn suspected_then_confirmed_after_gap() {
        let p = Params::default();
        let s1 = classify(None, 100, 50, 1_000, &p);
        assert_eq!(s1, PairState::Suspected { first_block: 1_000 });
        let s2 = classify(Some(&s1), 100, 50, 1_010, &p);
        assert_eq!(s2, PairState::Suspected { first_block: 1_000 }, "too soon to confirm");
        let s3 = classify(Some(&s2), 100, 50, 1_030, &p);
        assert_eq!(s3, PairState::Confirmed { first_block: 1_000 });
        assert_eq!(classify(Some(&s3), 100, 100, 1_040, &p), PairState::Healthy);
    }

    #[test]
    fn in_flight_noise_clears_before_confirming() {
        let p = Params::default();
        let s1 = classify(None, 100, 50, 1_000, &p);
        let s2 = classify(Some(&s1), 100, 100, 1_005, &p); // transfer landed
        assert_eq!(s2, PairState::Healthy);
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;

    #[test]
    fn reader_handles_nesting_and_formats() {
        let j = r#"{"supplied":{"shares":"9","balance":"100"},"borrowed":{"shares":"1","balance":"30"},"reserved":"5","reserve_ratio":2500,"prot_fee":"1","config":{"extra_decimals":1}}"#;
        assert_eq!(burrow_owed(j.as_bytes()), Some((100 + 5 + 1 - 30) / 10));
        assert_eq!(parse_u128(b"\"12x\""), None);
        assert_eq!(parse_u128(b" 77 "), Some(77));
    }

    #[test]
    fn reader_rejects_missing_fields() {
        assert_eq!(burrow_owed(br#"{"supplied":{"balance":"1"}}"#), None);
        assert_eq!(parse_owed(&AdapterKind::BackedStandard, br#"{"shadow_held":"1"}"#), None);
    }
}
