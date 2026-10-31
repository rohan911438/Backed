//! Minimal NEAR JSON-RPC client: view calls pinned to one block height.

use anyhow::{anyhow, bail, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;

pub struct Rpc {
    urls: Vec<String>,
    http: reqwest::blocking::Client,
}

impl Rpc {
    pub fn new(urls: Vec<String>) -> Self {
        let http = reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).build().expect("http client");
        Self { urls, http }
    }

    fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let mut last = anyhow!("no RPC url");
        for attempt in 0..8u32 {
            let url = &self.urls[attempt as usize % self.urls.len()];
            match self.http.post(url).json(&body).send() {
                Ok(r) if r.status().as_u16() == 429 || r.status().is_server_error() => {
                    last = anyhow!("{url}: HTTP {}", r.status());
                }
                Ok(r) => {
                    let v: Value = r.json()?;
                    if let Some(e) = v.get("error") {
                        let s = e.to_string();
                        if s.contains("limit") || s.contains("Too many") || s.contains("discontinued") {
                            last = anyhow!("{url}: unavailable ({s})");
                        } else {
                            bail!("RPC error: {s}");
                        }
                    } else {
                        return Ok(v["result"].clone());
                    }
                }
                Err(e) => last = anyhow!("{url}: {e}"),
            }
            std::thread::sleep(Duration::from_millis(300 * (attempt as u64 + 1)));
        }
        Err(last)
    }

    pub fn final_block(&self) -> Result<u64> {
        let r = self.call("block", json!({"finality": "final"}))?;
        r["header"]["height"].as_u64().ok_or_else(|| anyhow!("no block height"))
    }

    /// View call at `block`. Returns the raw result bytes.
    pub fn view_raw(&self, account: &str, method: &str, args: &Value, block: u64) -> Result<Vec<u8>> {
        let args_b64 = base64::engine::general_purpose::STANDARD.encode(args.to_string());
        let r = self.call(
            "query",
            json!({"request_type": "call_function", "block_id": block, "account_id": account,
                   "method_name": method, "args_base64": args_b64}),
        )?;
        if let Some(e) = r.get("error") {
            bail!("{account}.{method}: {}", e.as_str().unwrap_or("view failed"));
        }
        let bytes = r["result"].as_array().ok_or_else(|| anyhow!("{account}.{method}: no result"))?;
        Ok(bytes.iter().map(|b| b.as_u64().unwrap_or(0) as u8).collect())
    }

    pub fn view(&self, account: &str, method: &str, args: &Value, block: u64) -> Result<Value> {
        Ok(serde_json::from_slice(&self.view_raw(account, method, args, block)?)?)
    }
}
