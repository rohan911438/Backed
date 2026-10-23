//! `backed`: check whether NEAR protocols hold what they owe.
//!
//! Every reading takes `owed` and `held` at the same block height, so the
//! result is exact: no in-flight transfer can make it look wrong.

mod rpc;

use anyhow::{anyhow, Result};
use backed_checker::logic::{parse_owed, parse_u128, AdapterKind};
use clap::{Parser, Subcommand};
use rpc::Rpc;
use serde::Serialize;
use serde_json::json;

const DEFAULT_RPCS: &[&str] = &["https://free.rpc.fastnear.com", "https://rpc.mainnet.fastnear.com", "https://near.drpc.org"];

#[derive(Parser)]
#[command(name = "backed", version, about = "Check whether NEAR protocols hold the tokens they owe.")]
struct Cli {
    /// RPC endpoints, tried in turn (comma-separated).
    #[arg(long, global = true, value_delimiter = ',')]
    rpc: Vec<String>,
    /// Print JSON instead of a table.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check one protocol for one or more tokens.
    Check {
        /// Protocol account, e.g. intents.near or contract.main.burrow.near
        target: String,
        /// Token contracts to check.
        #[arg(long = "token", required = true)]
        tokens: Vec<String>,
        /// Adapter: nep245 | burrow | backed (guessed for known targets).
        #[arg(long)]
        adapter: Option<String>,
    },
    /// Check every NEP-141 token held by intents.near.
    ScanIntents {
        /// Only check the first N tokens.
        #[arg(long)]
        limit: Option<usize>,
        /// Parallel requests (keep low on free RPCs).
        #[arg(long, default_value_t = 3)]
        parallel: usize,
        /// Also fetch each token's symbol and decimals (one extra call per token).
        #[arg(long)]
        metadata: bool,
    },
    /// Print the near-cli command that records an attestation on-chain.
    Attest {
        target: String,
        token: String,
        /// Checker contract account.
        #[arg(long, default_value = "backed-checker.near")]
        checker: String,
        /// Your account (pays gas).
        #[arg(long)]
        signer: String,
    },
}

#[derive(Serialize, Clone)]
struct Reading {
    target: String,
    token: String,
    block: u64,
    owed: Option<String>,
    held: Option<String>,
    status: &'static str,
    shortfall: Option<String>,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decimals: Option<u8>,
}

fn add_metadata(rpc: &Rpc, r: &mut Reading) {
    if let Ok(m) = rpc.view(&r.token, "ft_metadata", &json!({}), r.block) {
        r.symbol = m["symbol"].as_str().map(|s| s.chars().take(24).collect());
        r.decimals = m["decimals"].as_u64().and_then(|d| u8::try_from(d).ok());
    }
}

fn adapter_for(target: &str, explicit: Option<&str>) -> Result<AdapterKind> {
    match explicit.unwrap_or(match target {
        "intents.near" => "nep245",
        "contract.main.burrow.near" => "burrow",
        _ => "backed",
    }) {
        "nep245" => Ok(AdapterKind::Nep245Ledger),
        "burrow" => Ok(AdapterKind::BurrowLending),
        "backed" => Ok(AdapterKind::BackedStandard),
        other => Err(anyhow!("unknown adapter {other}")),
    }
}

fn owed_view(kind: &AdapterKind, token: &str) -> (&'static str, serde_json::Value) {
    match kind {
        AdapterKind::Nep245Ledger => ("mt_supply", json!({"token_id": format!("nep141:{token}")})),
        AdapterKind::BurrowLending => ("get_asset", json!({"token_id": token})),
        AdapterKind::BackedStandard => ("backed_solvency", json!({"token": token})),
    }
}

fn read(rpc: &Rpc, target: &str, kind: &AdapterKind, token: &str, block: u64) -> Reading {
    let mut r = Reading {
        target: target.into(),
        token: token.into(),
        block,
        owed: None,
        held: None,
        status: "error",
        shortfall: None,
        error: None,
        symbol: None,
        decimals: None,
    };
    let (method, args) = owed_view(kind, token);
    let owed = rpc
        .view_raw(target, method, &args, block)
        .and_then(|b| parse_owed(kind, &b).ok_or_else(|| anyhow!("could not read owed")));
    let held = rpc
        .view_raw(token, "ft_balance_of", &json!({"account_id": target}), block)
        .and_then(|b| parse_u128(&b).ok_or_else(|| anyhow!("could not read held")));
    match (owed, held) {
        (Ok(o), Ok(h)) => {
            r.owed = Some(o.to_string());
            r.held = Some(h.to_string());
            if h >= o {
                r.status = "backed";
            } else {
                r.status = "UNDER-BACKED";
                r.shortfall = Some((o - h).to_string());
            }
        }
        (Err(e), _) | (_, Err(e)) => r.error = Some(e.to_string()),
    }
    r
}

fn print_table(rows: &[Reading]) {
    println!("{:<46} {:>14} {:>30} {:>30}", "token", "status", "owed", "held");
    for r in rows {
        let tok = if r.token.len() > 45 { format!("{}…", &r.token[..44]) } else { r.token.clone() };
        println!(
            "{:<46} {:>14} {:>30} {:>30}{}",
            tok,
            r.status,
            r.owed.as_deref().unwrap_or("-"),
            r.held.as_deref().unwrap_or("-"),
            r.error.as_ref().map(|e| format!("  ({e})")).unwrap_or_default()
        );
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let urls = if cli.rpc.is_empty() { DEFAULT_RPCS.iter().map(|s| s.to_string()).collect() } else { cli.rpc.clone() };
    let rpc = Rpc::new(urls);

    match cli.cmd {
        Cmd::Check { target, tokens, adapter } => {
            let kind = adapter_for(&target, adapter.as_deref())?;
            let block = rpc.final_block()?;
            let rows: Vec<_> = tokens.iter().map(|t| read(&rpc, &target, &kind, t, block)).collect();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&json!({"block": block, "readings": rows}))?);
            } else {
                println!("{target} at block {block}");
                print_table(&rows);
            }
        }
        Cmd::ScanIntents { limit, parallel, metadata } => {
            let block = rpc.final_block()?;
            let mut tokens = Vec::new();
            let mut from = 0u64;
            loop {
                let page = rpc.view("intents.near", "mt_tokens", &json!({"from_index": from.to_string(), "limit": 1000}), block)?;
                let page = page.as_array().cloned().unwrap_or_default();
                for t in &page {
                    if let Some(id) = t["token_id"].as_str().and_then(|s| s.strip_prefix("nep141:")) {
                        tokens.push(id.to_string());
                    }
                }
                if page.len() < 1000 {
                    break;
                }
                from += page.len() as u64;
            }
            if let Some(n) = limit {
                tokens.truncate(n);
            }
            let kind = AdapterKind::Nep245Ledger;
            let mut rows = Vec::with_capacity(tokens.len());
            for chunk in tokens.chunks(parallel.max(1)) {
                let got: Vec<Reading> = std::thread::scope(|s| {
                    let handles: Vec<_> =
                        chunk.iter().map(|t| s.spawn(|| {
                            let mut r = read(&rpc, "intents.near", &kind, t, block);
                            if metadata {
                                add_metadata(&rpc, &mut r);
                            }
                            r
                        })).collect();
                    handles.into_iter().map(|h| h.join().expect("thread")).collect()
                });
                rows.extend(got);
            }
            let under: Vec<_> = rows.iter().filter(|r| r.status == "UNDER-BACKED").cloned().collect();
            let backed = rows.iter().filter(|r| r.status == "backed").count();
            let errors = rows.iter().filter(|r| r.status == "error").count();
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "target": "intents.near", "block": block, "checked": rows.len(),
                        "backed": backed, "under_backed": under.len(), "errors": errors, "readings": rows
                    }))?
                );
            } else {
                println!("intents.near at block {block}: {} NEP-141 tokens", rows.len());
                println!("  backed: {backed}   under-backed: {}   could not read: {errors}", under.len());
                if !under.is_empty() {
                    println!();
                    print_table(&under);
                }
            }
        }
        Cmd::Attest { target, token, checker, signer } => {
            println!("Records an on-chain attestation (first time per pair: attach 0.005 NEAR for storage):\n");
            println!(
                "near contract call-function as-transaction {checker} attest json-args '{{\"target\":\"{target}\",\"token\":\"{token}\"}}' prepaid-gas '60 Tgas' attached-deposit '0.005 NEAR' sign-as {signer} network-config mainnet sign-with-keychain send"
            );
        }
    }
    Ok(())
}
