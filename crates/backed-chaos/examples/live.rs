//! Deploys the checker to a live network and records real attestations.
//!
//! ```text
//! cargo run -p backed-chaos --features sandbox --example live -- \
//!     <testnet|mainnet> <account-id> <deploy|attest|status> [target token]
//! ```
//!
//! The signing key is read from `~/.near-credentials/<network>/<account-id>.json`
//! (the near-cli layout), never from the command line.
//!
//! - `deploy`: uploads `backed_checker.wasm` to `<account-id>`, calls
//!   `new(owner = account-id)`, and sets the Burrow (and on mainnet, Intents) adapters.
//! - `attest <target> <token>`: one on-chain attestation; prints the result.
//! - `status <target> <token>`: reads the stored status (view, free).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use near_workspaces::types::{Gas, NearToken};
use near_workspaces::{Account, AccountId, Contract, Network, Worker};
use serde_json::{json, Value};

const TESTNET_RPC: &str = "https://test.rpc.fastnear.com";
const MAINNET_RPC: &str = "https://free.rpc.fastnear.com";

fn adapters(network: &str) -> Vec<(&'static str, &'static str)> {
    match network {
        "mainnet" => vec![("intents.near", "Nep245Ledger"), ("contract.main.burrow.near", "BurrowLending")],
        _ => vec![("contract.1638481328.burrow.testnet", "BurrowLending")],
    }
}

fn credentials(network: &str, account: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let p = PathBuf::from(home).join(".near-credentials").join(network).join(format!("{account}.json"));
    if !p.exists() {
        bail!("no key file at {}", p.display());
    }
    Ok(p)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [network, account, cmd, rest @ ..] = args.as_slice() else {
        bail!("usage: live <testnet|mainnet> <account-id> <deploy|attest|status> [target token]");
    };
    match network.as_str() {
        "testnet" => run(near_workspaces::testnet().rpc_addr(TESTNET_RPC).await?, network, account, cmd, rest).await,
        "mainnet" => run(near_workspaces::mainnet().rpc_addr(MAINNET_RPC).await?, network, account, cmd, rest).await,
        other => bail!("unknown network {other}"),
    }
}

async fn run<N: Network + 'static>(worker: Worker<N>, network: &str, account: &str, cmd: &str, rest: &[String]) -> Result<()> {
    let signer = Account::from_file(credentials(network, account)?, &worker)?;
    let checker: AccountId = account.parse()?;
    match (cmd, rest) {
        ("deploy", []) => {
            let wasm = std::fs::read(backed_chaos::sandbox::wasm_path("backed_checker")?)?;
            println!("deploying {} bytes to {checker} on {network}...", wasm.len());
            let contract: Contract = signer.deploy(&wasm).await?.into_result()?;
            println!("  deployed, tx ok");
            let init = contract.call("new").args_json(json!({ "owner": checker })).transact().await?;
            report("new", &init)?;
            for (target, kind) in adapters(network) {
                let res = signer
                    .call(&checker, "set_adapter")
                    .args_json(json!({ "target": target, "kind": kind }))
                    .deposit(NearToken::from_yoctonear(1))
                    .transact()
                    .await?;
                report(&format!("set_adapter {target} = {kind}"), &res)?;
            }
        }
        ("attest", [target, token]) => {
            let res = signer
                .call(&checker, "attest")
                .args_json(json!({ "target": target, "token": token }))
                .deposit(NearToken::from_millinear(5))
                .gas(Gas::from_tgas(100))
                .transact()
                .await?;
            report("attest", &res)?;
            for log in res.logs() {
                println!("  log: {log}");
            }
            let status: Option<Value> = res.json().unwrap_or(None);
            println!("  result: {}", status.map(|s| s.to_string()).unwrap_or_else(|| "null".into()));
        }
        ("status", [target, token]) => {
            let v: Value = worker
                .view(&checker, "get_status")
                .args_json(json!({ "target": target, "token": token }))
                .await?
                .json()?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        _ => bail!("usage: live <testnet|mainnet> <account-id> <deploy|attest|status> [target token]"),
    }
    Ok(())
}

fn report(what: &str, res: &near_workspaces::result::ExecutionFinalResult) -> Result<()> {
    let hash = res.outcome().transaction_hash;
    if let Some(f) = res.receipt_failures().first() {
        bail!("{what} failed (tx {hash}): {:?}", (*f).clone().into_result().err());
    }
    println!("  {what}: ok (tx {hash})");
    Ok(())
}
