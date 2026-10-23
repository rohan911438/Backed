# 08 — Design Audit

Method: attack the design, fix it, verify against NEAR code or mainnet. Severity: **C** critical · **H** high · **M** medium · **L** low.

## Round 1 — Does the core idea hold?

| # | Sev | Finding | Fix (now in the design) | Verified by |
|---|---|---|---|---|
| 1.1 | **C** | An in-contract invariant alone **can't catch the Intents bug**: the contract never learns the tokens were refunded, so its own numbers still look fine. | **Two tiers:** constant-time shadow invariant (logic bugs) + `reconcile()` against the real token balance (silent outflows). | near/intents regression test `mt_deposit_resolve_gas.rs`: the failure happens in a callback the ledger never hears from. |
| 1.2 | **H** | On-chain reconcile reads land in other blocks, so in-flight transfers would cause false alarms. | Order flows (debit before send, credit after receive) + monotone `out_started` counter: check `real + Δout ≥ S0`. Can't fire on in-flight transfers. | Reasoned case by case (deposit, withdraw, failed withdraw); to be covered by unit tests on day 5. |
| 1.3 | **H** | The checker reads **external** protocols (Intents, Rhea) that don't have our counters, so they're exposed to the same race. | Two-attestation persistence rule (`Suspected` → `Confirmed` ≥ 30 blocks apart) + tolerance/dust floor. Exact detection is done off-chain at one `block_id`. | Live scan of 370 Intents tokens at one block: consistent results. |
| 1.4 | **H** | Polling on-chain would burn the budget (~0.003 NEAR per attest × many pairs × every few minutes). | **Lazy attestation:** free off-chain scanner watches; on-chain `attest` only on mismatch (+ optional daily heartbeat). | Cost estimate; measure real gas day 1. |
| 1.5 | **H** | Log budgeting can be done wrong: Intents *had* a log pre-check, but on values that weren't capped yet. | Fixed order in the resolve: **cap refunds → compute log size on capped values → emit**. | `crates/near/nep245/src/checked.rs` (`check_refund`, Aug 2026) existed before the exploit. |
| 1.6 | **M** | Fee-on-transfer, burning or rebasing tokens make real holdings drop without a bug in the protocol. Our scan found 2 under-backed memecoins in Intents. | Report **token behavior** with each breach; the library can refuse such tokens or measure the delivered amount through reconcile. Don't call every breach an exploit. | Live: `ninu-4` / `batman-4.nearlytrade.near` shortfalls; supply below initial (burns). |

## Round 2 — Attack the product

| # | Sev | Finding | Fix | Verified by |
|---|---|---|---|---|
| 2.1 | **H** | "Isn't this just a dashboard / scanner?" | Core is **prevention (library) + proof (chaos replays)**; the checker and page are the public layer. Lead every pitch with the replays. | — |
| 2.2 | **H** | Adoption: protocols must change code to use the library. | Value on day one **without adoption**: the checker covers Intents and Rhea today. The library targets new contracts, hackathon teams, and AI coding agents via the skill. | Live adapters verified. |
| 2.3 | **M** | Rhea-class losses only show in owed-vs-held when bad debt is written off. | Be precise: the library prevents it (`Delivered`-only credit); the checker sees it during the attack, after the first liquidation. | Burrow adapter math: borrowing moves owed and held equally; write-offs don't. |
| 2.4 | **M** | Could someone grief the checker into a false `Confirmed`? | No trusted input: values come from the target's and token's own views. Faking a breach means really changing the protocol's state. Persistence rule handles noise. | Design review. |
| 2.5 | **M** | Could a protocol fake health? It controls its own views. | For `BackedStandard`, `held` always comes from the **token contract**, never the protocol; only `owed` is self-reported. A protocol could under-report owed, which we flag as a known limit. Known adapters (Intents `mt_supply`, Burrow `get_asset`) are the protocols' real accounting views. | Adapter design. |
| 2.6 | **M** | Checker admin abuse. | No power over anyone's funds; owner can only add adapter configs; keys removed after judging. | — |
| 2.7 | **L** | Library gas overhead. | Constant-time counters per mutation; one extra private callback only where a cross-contract call already exists. | — |

## Round 3 — Build risk

| # | Sev | Finding | Fix |
|---|---|---|---|
| 3.1 | **H** | Solo scope: library + harness + contract + CLI + web + skill. | Strict order with a cut list (see 07). The replays and Intents adapter are must-haves; the rest is trimmable. |
| 3.2 | **M** | near-sandbox doesn't run natively on Windows. | WSL Ubuntu is installed on this machine; Rust 1.95 + wasm32 target present. |
| 3.3 | **M** | Checker contract size vs $5 budget. | Minimal dependencies, opt-level "z", wasm-opt; realistic total $6–10. |
| 3.4 | **L** | Replays use minimal reproductions, not the production contracts. | Say so plainly. Stretch: run chaos against the real `near/intents` v0.4.4 (public source). |

**Status: no open critical findings.** Remaining items are measurements for day 1:
1. Real gas cost of `attest` (cross-contract views + storage).
2. Checker wasm size after optimization.
3. Exact log-size behavior of near-sdk events in the sandbox (to tune the log budget).

## Facts relied on (all verified 2026-10-23)
- near/intents: `TOTAL_LOG_LENGTH_LIMIT = 16384`; resolver caps `refund.min(amount)` and `refund.min(receiver_balance)`; near-sdk 5.29.0; commits `0c1a530c`, `16d04f6e`, `153cab96`.
- Burrowland `margin_trading.rs:111` sums `min_amount_out` across steps.
- Mainnet views: `intents.near` `mt_tokens`, `mt_supply`; `contract.main.burrow.near` `get_asset`; `aggregate-oracle.near` `list_prices`; RPC `block_id` snapshots work.
