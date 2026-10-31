//! `backed mcp`: a Model Context Protocol erver on stdin/stdout, so AI agents
//! can ask "does this protocol hold what it owes?" before moving funds.
//!
//! Read-only: it never signs or sends a transaction. Messages are JSON-RPC 2.0,
//! one per line. Nothing but protocol messages is written to stdout.

use std::io::{BufRead, Write};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::rpc::Rpc;
use crate::{adapter_for, read, Reading};

const TESTNET_RPCS: &[&str] = &["https://test.rpc.fastnear.com", "https://rpc.testnet.near.org"];
const SNAPSHOT_URL: &str = "https://backed-near.vercel.app/intents-snapshot.json";
const DEFAULT_CHECKER_TESTNET: &str = "simplekilt7490.testnet";

pub fn serve(mainnet: Rpc) -> Result<()> {
    let testnet = Rpc::new(TESTNET_RPCS.iter().map(|s| s.to_string()).collect());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                reply(&mut out, json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}}))?;
                continue;
            }
        };
        // Notifications (no id) need no answer.
        let Some(id) = msg.get("id").cloned() else { continue };
        let method = msg["method"].as_str().unwrap_or_default();
        let resp = match method {
            "initialize" => Ok(json!({
                "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "backed", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Backed checks whether a NEAR protocol holds the tokens its own books say it owes, reading both at the same block. Call check_backing before depositing into a NEAR protocol; never deposit into one that is UNDER-BACKED without investigating."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => Ok(call(&mainnet, &testnet, &msg["params"])),
            other => Err(json!({"code": -32601, "message": format!("method not found: {other}")})),
        };
        let body = match resp {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
        };
        reply(&mut out, body)?;
    }
    Ok(())
}

fn reply(out: &mut impl Write, v: Value) -> Result<()> {
    writeln!(out, "{v}")?;
    out.flush()?;
    Ok(())
}

fn tools() -> Value {
    json!([
        {
            "name": "check_backing",
            "description": "Read what a NEAR protocol owes (its own accounting view) and what it holds (each token's ft_balance_of) at the same block, so the answer is exact. Use before depositing into a protocol. Works for intents.near, contract.main.burrow.near (Rhea lending) and any contract implementing backed_solvency.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "Protocol account, e.g. intents.near"},
                    "tokens": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 20, "description": "NEP-141 token contracts, e.g. usdt.tether-token.near"},
                    "adapter": {"type": "string", "enum": ["nep245", "burrow", "backed"], "description": "How to read 'owed'. Guessed for intents.near and contract.main.burrow.near; defaults to backed (backed_solvency)."},
                    "block": {"type": "integer", "description": "Past block height (needs an archival RPC via --rpc). Default: latest final block."}
                },
                "required": ["target", "tokens"]
            }
        },
        {
            "name": "intents_backing_summary",
            "description": "The latest daily scan of every NEP-141 token in NEAR Intents (intents.near): how many are backed, which are under-backed and by how much. Cheap: one HTTP request, no RPC calls.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "checker_status",
            "description": "What the on-chain Backed checker contract last recorded for a (protocol, token) pair: Healthy, Suspected or Confirmed breach, with owed, held and block.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": {"type": "string"},
                    "token": {"type": "string"},
                    "checker": {"type": "string", "description": "Checker contract account. Default: the testnet deployment."},
                    "network": {"type": "string", "enum": ["testnet", "mainnet"], "description": "Default: testnet"}
                },
                "required": ["target", "token"]
            }
        }
    ])
}

fn call(mainnet: &Rpc, testnet: &Rpc, params: &Value) -> Value {
    let args = &params["arguments"];
    let result = match params["name"].as_str().unwrap_or_default() {
        "check_backing" => check_backing(mainnet, args),
        "intents_backing_summary" => intents_summary(),
        "checker_status" => checker_status(mainnet, testnet, args),
        other => Err(anyhow!("unknown tool {other}")),
    };
    match result {
        Ok((text, data)) => json!({"content": [{"type": "text", "text": text}], "structuredContent": data, "isError": false}),
        Err(e) => json!({"content": [{"type": "text", "text": format!("error: {e}")}], "isError": true}),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("missing argument: {key}"))
}

fn check_backing(rpc: &Rpc, args: &Value) -> Result<(String, Value)> {
    let target = str_arg(args, "target")?;
    let tokens: Vec<String> = args["tokens"]
        .as_array()
        .ok_or_else(|| anyhow!("missing argument: tokens"))?
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .take(20)
        .collect();
    if tokens.is_empty() {
        return Err(anyhow!("tokens is empty"));
    }
    let kind = adapter_for(target, args["adapter"].as_str())?;
    let block = match args["block"].as_u64() {
        Some(b) => b,
        None => rpc.final_block()?,
    };
    let rows: Vec<Reading> = tokens.iter().map(|t| read(rpc, target, &kind, t, block)).collect();
    let under: Vec<&Reading> = rows.iter().filter(|r| r.status == "UNDER-BACKED").collect();
    let errors = rows.iter().filter(|r| r.status == "error").count();
    let names: Vec<&str> = under.iter().map(|r| r.token.as_str()).collect();
    let verdict = if !under.is_empty() {
        format!("UNDER-BACKED for {}: the protocol owes more of these tokens than it holds. Do not deposit them without investigating.", names.join(", "))
    } else if errors > 0 {
        "UNKNOWN for some tokens: a view call failed. Treat those as unverified.".to_string()
    } else {
        "BACKED: for every token checked, the protocol holds at least what it owes.".to_string()
    };
    let mut text = format!("{target} at block {block}: {verdict}\n");
    for r in &rows {
        text.push_str(&format!(
            "- {}: {} (owed {}, held {}{})\n",
            r.token,
            r.status,
            r.owed.as_deref().unwrap_or("?"),
            r.held.as_deref().unwrap_or("?"),
            r.shortfall.as_ref().map(|s| format!(", short {s}")).or(r.error.as_ref().map(|e| format!(", error: {e}"))).unwrap_or_default()
        ));
    }
    Ok((text, json!({"target": target, "block": block, "verdict": verdict, "readings": rows})))
}

fn intents_summary() -> Result<(String, Value)> {
    let snap: Value = reqwest::blocking::get(SNAPSHOT_URL)?.error_for_status()?.json()?;
    let under: Vec<Value> = snap["readings"]
        .as_array()
        .map(|a| a.iter().filter(|r| r["status"] == "UNDER-BACKED").cloned().collect())
        .unwrap_or_default();
    let mut text = format!(
        "intents.near at block {}: {} of {} NEP-141 tokens backed, {} under-backed, {} unreadable.\n",
        snap["block"], snap["backed"], snap["checked"], snap["under_backed"], snap["errors"]
    );
    for r in &under {
        text.push_str(&format!("- {} ({}): owed {}, held {}\n", r["token"].as_str().unwrap_or("?"), r["symbol"].as_str().unwrap_or("?"), r["owed"].as_str().unwrap_or("?"), r["held"].as_str().unwrap_or("?")));
    }
    text.push_str("Causes of the current shortfalls: https://github.com/rohan911438/Backed/blob/main/docs/FINDINGS.md");
    Ok((text, json!({"block": snap["block"], "checked": snap["checked"], "backed": snap["backed"], "under_backed": under})))
}

fn checker_status(mainnet: &Rpc, testnet: &Rpc, args: &Value) -> Result<(String, Value)> {
    let target = str_arg(args, "target")?;
    let token = str_arg(args, "token")?;
    let on_mainnet = args["network"].as_str() == Some("mainnet");
    let checker = match args["checker"].as_str() {
        Some(c) => c.to_string(),
        None if on_mainnet => return Err(anyhow!("pass `checker`: no default mainnet checker yet")),
        None => DEFAULT_CHECKER_TESTNET.to_string(),
    };
    let rpc = if on_mainnet { mainnet } else { testnet };
    let block = rpc.final_block()?;
    let status = rpc.view(&checker, "get_status", &json!({"target": target, "token": token}), block)?;
    let text = if status.is_null() {
        format!("{checker} has no attestation for ({target}, {token}) yet.")
    } else {
        format!(
            "{checker}: ({target}, {token}) is {} as of block {} (owed {}, held {}).",
            status["state"], status["last_block"], status["owed"], status["held"]
        )
    };
    Ok((text, json!({"checker": checker, "status": status})))
}
