# 08 — Design Audit

Method: attack the design, fix it, verify against NEAR code or mainnet. Severity: **C** critical · **H** high · **M** medium · **L** low.

## Round 1 — Does the core idea hold?

| # | Sev | Finding | Fix (now in the design) | Verified by |
|---|---|---|---|---|
| 1.1 | **C** | An in-contract invariant alone **can't catch the Intents bug**: the contract never learns the tokens were refunded, so its own numbers still look fine. | **Two tiers:** constant-time shadow invariant (logic bugs) + `reconcile()` against the real token balance (silent outflows). | near/intents regression test `mt_deposit_resolve_gas.rs`: the failure happens in a callback the ledger never hears from. |
| 1.2 | **H** | On-chain reconcile reads land in other blocks, so in-flight transfers would cause false alarms. | Order flows (debit before send, credit after receive) + monotone `out_started` counter: check `real + Δout ≥ S0`. Can't fire on in-flight transfers. | Unit test: async timing simulation with in-flight deposits and withdrawals, zero false alarms (`crates/backed`). |
| 1.3 | **H** | The checker reads **external** protocols (Intents, Rhea) that don't have our counters, so they're exposed to the same race. | Two-attestation persistence rule (`Suspected` → `Confirmed` ≥ 30 blocks apart) + tolerance/dust floor. Exact detection is done off-chain at one `block_id`. | Live scan of 370 Intents tokens at one block: consistent results. |
| 1.4 | **H** | Polling on-chain would burn the budget (~0.003 NEAR per attest × many pairs × every few minutes). | **Lazy attestation:** free off-chain scanner watches; on-chain `attest` only on mismatch (+ optional daily heartbeat). | Checker live on testnet: each `attest` keeps only the storage it adds and refunds the rest of 0.005 NEAR. |
| 1.5 | **H** | Log budgeting can be done wrong: Intents *had* a log pre-check, but on values that weren't capped yet. | Fixed order in the resolve: **cap refunds → compute log size on capped values → emit**. | `crates/near/nep245/src/checked.rs` (`check_refund`, Aug 2026) existed before the exploit. |
| 1.6 | **M** | Fee-on-transfer, burning or rebasing tokens make real holdings drop without a bug in the protocol. Our scan found 3 under-backed tokens in Intents (2 memecoins, 1 test token). | Report **token behavior** with each breach; the library can refuse such tokens or measure the delivered amount through reconcile. Don't call every breach an exploit. | Live: `ninu-4` / `batman-4.nearlytrade.near` shortfalls; supply below initial (burns). |

## Round 2 — Attack the product

| # | Sev | Finding | Fix | Verified by |
|---|---|---|---|---|
| 2.1 | **H** | "Isn't this just a dashboard / scanner?" | Core is **prevention (library) + proof (chaos replays)**; the checker and page are the public layer. Lead every pitch with the replays. | — |
| 2.2 | **H** | Adoption: protocols must change code to use the library. | Value on day one **without adoption**: the checker covers Intents and Rhea today. The library targets new contracts, hackathon teams, and AI coding agents via the skill. | Live adapters verified. |
| 2.3 | **M** | Rhea-class losses only show in owed-vs-held when bad debt is written off. | Be precise: the library prevents it (`Delivered`-only credit); the checker sees it during the attack, after the first liquidation. | Burrow adapter math: borrowing moves owed and held equally; write-offs don't. |
| 2.4 | **M** | Could someone grief the checker into a false `Confirmed`? | No trusted input: values come from the target's and token's own views. Faking a breach means really changing the protocol's state. Persistence rule handles noise. | Design review. |
| 2.5 | **M** | Could a protocol fake health? It controls its own views. | For `BackedStandard`, `held` always comes from the **token contract**, never the protocol; only `owed` is self-reported. A protocol could under-report owed, which we flag as a known limit. Known adapters (Intents `mt_supply`, Burrow `get_asset`) are the protocols' real accounting views. | Adapter design. |
| 2.6 | **M** | Checker admin abuse. | No power over anyone's funds; owner can only add adapter configs; `set_owner` moves that to a multisig or DAO (roadmap week 3). | `set_owner` / `get_owner` in the contract. |
| 2.7 | **L** | Library gas overhead. | Constant-time counters per mutation; one extra private callback only where a cross-contract call already exists. | — |

## Round 3 — Build risk

| # | Sev | Finding | Fix |
|---|---|---|---|
| 3.1 | **H** | Solo scope: library + harness + contract + CLI + web + skill. | Strict order with a cut list (see 07). The replays and Intents adapter are must-haves; the rest is trimmable. |
| 3.2 | **M** | near-sandbox doesn't run natively on Windows. | Sandbox runs on Linux, macOS or WSL; documented in the README and enforced by a `sandbox` cargo feature, so the default build works everywhere. |
| 3.3 | **M** | Checker contract size vs $5 budget. | Measured 161 KB after the abuse-resistance changes (164 KB before replacing serde_json::Value). Mainnet deploy ≈ 1.91 NEAR; testnet is free. |
| 3.4 | **L** | Replays use minimal reproductions, not the production contracts. | Said plainly in the README. Roadmap week 4: run the replays against any contract's wasm. |

**Status: no open critical findings.** The day-1 measurements are done: the checker wasm is 161 KB, `attest` pays for its own storage (0.005 NEAR attached, the rest refunded), and `LogBudget` keeps resolve logs under 16,384 bytes and 100 entries, with unit tests for oversized logs, log count, and capping before logging (`crates/backed/src/refund.rs`). Remaining work is on the 4-week roadmap (mainnet deploy, multisig owner, replays against any wasm).

## Facts relied on (all verified 2026-10-23)
- near/intents: `TOTAL_LOG_LENGTH_LIMIT = 16384`; resolver caps `refund.min(amount)` and `refund.min(receiver_balance)`; near-sdk 5.29.0; commits `0c1a530c`, `16d04f6e`, `153cab96`.
- Burrowland `margin_trading.rs:111` sums `min_amount_out` across steps.
- Mainnet views: `intents.near` `mt_tokens`, `mt_supply`; `contract.main.burrow.near` `get_asset`; `aggregate-oracle.near` `list_prices`; RPC `block_id` snapshots work.
