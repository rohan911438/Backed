# 03 — Architecture

## Overview

```
                         ┌───────────────────────────── Developers ─────────────────────────────┐
                         │  backed (crate)  ──used by──▶  their contract  ◀──tested by── backed-chaos │
                         └───────────────────────────────────┬──────────────────────────────────┘
                                                             │ exposes backed_solvency(token)
                                                             ▼
 ┌────────────────┐  same-block snapshot   ┌────────────────────────────────────────────┐
 │ backed CLI /   │ ─────(free reads)────▶ │ NEAR mainnet: intents.near, Rhea lending,   │
 │ scanner (cron) │                        │ Backed-standard contracts, token contracts  │
 └───────┬────────┘                        └───────────────▲────────────────────────────┘
         │ only when a mismatch is seen                    │ cross-contract views
         ▼                                                 │
 ┌────────────────────────────────────────────────────────┴───┐      ┌─────────────────────┐
 │ Backing checker contract (testnet)                          │◀─────│ web page (static)   │
 │ attest(target, token) → Attestation, Suspected/Confirmed    │      │ agents, wallets      │
 └─────────────────────────────────────────────────────────────┘      └─────────────────────┘
```

Three layers:
1. **Prevent**: `backed` crate inside contracts.
2. **Prove**: `backed-chaos` in tests and CI.
3. **Verify**: scanner (exact, off-chain, free) + checker contract (trustless, on-chain, actionable).

---

## 1. `backed` crate (Rust, for near-sdk 5.x contracts)

### 1.1 Core: `Ledger`
Per token it keeps:
- `balances: LookupMap<AccountId, u128>`: per-user balances
- `owed_total: u128`: Σ balances + protocol-owned amounts (fees)
- `shadow_held: u128`: tokens the contract **knows** it holds, changed only by verified inbound and outbound flows
- `out_started: u128`, `in_credited: u128`: monotone counters (used by race-free reconcile, §1.5)
- `status: Healthy | Breach { since_block }`

**Invariant, checked after every mutation (constant time):** `owed_total ≤ shadow_held`. A violation panics the transaction before any state is saved.

### 1.2 Verified amounts at the type level
```rust
pub struct Claimed(u128);    // from args or msg — anything a caller says
pub struct Delivered(u128);  // only constructible inside Backed's private callbacks / receivers
```
- `Ledger::credit(account, Delivered)`: the only way to increase a balance.
- `Delivered` can be produced only by:
  - `on_ft_transfer(token, sender, amount)`, where predecessor == a registered token contract (the token itself vouches for the amount);
  - `Delivered::from_callback_result(...)` inside a `#[private]` callback, parsing the return value of an allowlisted venue call (e.g. a DEX `swap` that returns the amount out), or a measured balance delta.
- There is no `From<Claimed> for Delivered`. **Rhea's pattern (crediting the route's claimed minimum) does not compile.**

### 1.3 Safe receiver (`ft_on_transfer` / `mt_on_transfer`)
- Rejects tokens not on the contract's registry (returns the full amount as unused). This fixes the "fake deposit" pattern from NEAR's ft_transfer_call security docs.
- If the contract wants follow-up work (forwarding, notifying another contract), Backed uses **provisional credit**: the amount is credited as `provisional`, and the follow-up resolve callback finalizes `kept = deposited − refund`.
- **Infallible resolve.** NEAR's token standard treats a failed receiver callback as "refund everything", so the resolve must never fail. Backed bounds every resource the resolve uses:
  - **Refund cap:** `refund = min(requested, deposited, current_balance)`. That's Intents' own post-fix rule.
  - **Log budget:** events are aggregated or chunked so total log bytes stay under NEAR's `TOTAL_LOG_LENGTH_LIMIT` (16,384 bytes), with the size computed before emitting. This was the Intents trigger. **Order matters:** cap the refund first, then measure the log of the capped values. Intents already had a size pre-check (`check_refund` in `crates/near/nep245/src/checked.rs`, since Aug 2026), but it ran on values that weren't yet capped, and refunds with more digits than the deposit broke it.
  - **Gas budget:** callback gas reserved as a constant; entry methods assert `prepaid_gas ≥ work + reserved`.
  - **No panics:** checked arithmetic with a defined fallback (refund in full, don't credit) instead of `unwrap`.
  - **Storage:** paid up front (NEP-145) so the resolve never runs out of storage.

### 1.4 Safe withdraw / outbound transfer
1. Debit the user, `owed_total −= x`, `shadow_held −= x`, `out_started += x`, **before** sending (state before call, per NEAR guidance).
2. `ft_transfer(to, x)` with 1 yocto, then `#[private] on_withdraw(...)` with reserved gas.
3. On failure: re-credit the user and add `x` back to `shadow_held`. On success: nothing more to do.

The same flow serves the token-contract side: `backed` ships a resolver that is `#[private]` and caps refunds by the original amount and the receiver's balance (the SWEAT/HOT fix).

### 1.5 Race-free reconcile against the real balance
`reconcile(token)` (public; anyone can call):
1. Record `S0 = shadow_held`, `O0 = out_started`.
2. Call `token.ft_balance_of(self)` → `#[private] on_reconcile(S0, O0, real)`.
3. Check **`real + (out_started_now − O0) ≥ S0`**.

Why no false alarms:
- Deposits that land between the steps only increase `real`, the safe direction. They're excluded from `S0`.
- Transfers started after step 1 may have left before the balance was read. They're added back through the counter delta.
- Failed transfers return tokens, also the safe direction.
- So the check can only fail if tokens left **without the ledger knowing**: exactly the Intents failure (tokens refunded after a failed resolve), a burning or fee-taking token, or a bug outside Backed.

On failure: `status = Breach`, emit `BackingBreach`, and (if the contract opts in) withdrawals of that token pause until the owner resolves it. Deposits stay allowed.

### 1.6 Standard view (draft NEP)
```rust
fn backed_solvency(&self, token: AccountId) -> Solvency { owed: U128, shadow_held: U128, status: Status }
```
Any contract exposing it can be checked by the checker with no custom adapter.

---

## 2. `backed-chaos` harness

Runs in the NEAR sandbox (near-sandbox on Linux/WSL) against a compiled contract wasm.

**Invariant oracle:** after every receipt, read `owed` (via `backed_solvency` or an adapter) and `ft_balance_of(contract)` at the same block. Any point where owed > held is a finding.

**Failure injections:**
| ID | Injection | Real incident it models |
|---|---|---|
| F1 | Callback out of gas (minimal gas attached) | general; NEAR removed a `poc_deposit_oog` test during the Intents fix |
| F2 | Receiver emits or triggers logs past `TOTAL_LOG_LENGTH_LIMIT` | **NEAR Intents, Oct 2026** |
| F3 | Receiver panics | general |
| F4 | Receiver returns absurd refunds (u128::MAX, > deposited) | **NEAR Intents** (huge refund requests) |
| F5 | Direct external call to callbacks / resolvers | **SWEAT/HOT** |
| F6 | Malicious token: wrong amounts, fee-on-transfer, burns, re-entry into `ft_on_transfer` | live memecoin shortfalls |
| F7 | Venue returns less than claimed | **Rhea, Apr 2026** |
| F8 | Storage exhaustion during callback | general |

**Helper contracts (we write, small):** `evil-token`, `evil-receiver`, `lying-venue`. Plus Burrowland's `mock-ref-exchange` (public domain, Unlicense) for the swap venue.

**Replays (each = naive contract vs Backed contract):**
- `replay_intents_log_limit`: provisional credit and the log-limit resolve failure → naive: unbacked balance; Backed: resolve can't fail.
- `replay_rhea_claimed_credit`: multi-hop route through a fake pool → naive: credited claimed amount; Backed: `Delivered`-only credit.
- `replay_sweat_public_resolver`: direct call to the resolver → naive: unearned refund; Backed: private and capped.

**Stretch:** point `backed-chaos` at the real `near/intents` at its vulnerable release (`defuse v0.4.4`, commit `a2dd1408`) and show F2/F4 find the bug without being told where it is.

---

## 3. Backing checker contract (live on testnet, mainnet-ready)

### Interface
```rust
// anyone; caller pays gas
pub fn attest(&mut self, target: AccountId, token: AccountId) -> Promise;
// a protocol registers itself as Backed-standard (predecessor == target)
pub fn register_self(&mut self, tokens: Vec<AccountId>);
// views
pub fn get_status(&self, target: AccountId, token: AccountId) -> Option<PairStatus>;
pub fn list_breaches(&self, from: u64, limit: u64) -> Vec<BreachRecord>;
```

### Adapters (v1)
| Kind | owed | held |
|---|---|---|
| `Nep245Ledger` (intents.near) | `mt_supply("nep141:<token>")` | `token.ft_balance_of(target)` |
| `BurrowLending` (contract.main.burrow.near) | `get_asset`: (supplied + reserved + prot_fee − borrowed) / 10^extra_decimals | `token.ft_balance_of(target)` |
| `BackedStandard` | `target.backed_solvency(token).owed` | `token.ft_balance_of(target)` |

All verified against live mainnet data on 2026-10-23.

### Breach logic (handles cross-shard timing)
On-chain reads of `owed` and `held` land in different blocks, so one reading can be off by in-flight flows. Rule:
- `ratio < 1 − tolerance` (default 0.1%, or an absolute dust floor) → `Suspected` (stored, event emitted).
- A second `Suspected` attestation **≥ N blocks later** (default 30) → `Confirmed` (event `BackingBreachConfirmed`).
- A healthy attestation clears `Suspected`.

Real breaches persist (Intents' unbacked balances lasted 2 days); in-flight noise doesn't.

### Storage and cost
- One small record per (target, token) plus a bounded breach log. Every attest attaches 0.005 NEAR; the checker keeps only the storage the call adds and refunds the rest. A breach is logged only when a pair changes state.
- **Lazy attestation:** the contract isn't polled. The free off-chain scanner (§4) watches continuously and calls `attest` only when it sees a mismatch, plus an optional daily heartbeat for headline pairs. Keeps running cost near zero.
- Gas is paid by the caller. Deployment is live on testnet (`simplekilt7490.testnet`); mainnet needs about 1.91 NEAR for the 161 KB wasm (`live mainnet <acct> preflight` prints the exact cost).
- No admin power over anyone's funds. The owner can only add adapter configs, and `set_owner` hands that to a multisig or DAO (roadmap week 3).

---

## 4. Scanner + CLI (`backed`, Rust)

| Command | What it does |
|---|---|
| `backed check <target> --token T… [--adapter] [--json] [--block H]` | Exact same-block snapshot of owed vs held; any past block with an archival RPC |
| `backed scan-intents` | Every NEP-141 token in Intents (387 on 2026-11-09), same block |
| `backed scan-ref` | Ref Finance: total pool reserves (floor) vs holdings |
| `backed chaos [--scenario intents\|sweat\|rhea]` | Replays the attacks on a local sandbox; exits non-zero if a Backed vault breaks |
| `backed attest <target> <token> --signer <acct>` | Prints the near-cli command for an on-chain attestation |
| `backed mcp` | Local MCP server (stdio, read-only) |

- **Same-block snapshot:** RPC view calls accept `block_id`, so off-chain reads of owed and held can use one block height. That's exact, with no race (the technique NEAR's own script uses).
- **Daily scan:** a free GitHub Actions job (`monitor.yml`) runs `backed scan-intents` once a day, commits a snapshot and a history row, and opens a GitHub issue (plus optional Discord/Telegram alerts) when a token newly becomes under-backed. Calling `attest` on a mismatch is roadmap week 2.
- Plain JSON-RPC over `reqwest`. The CLI shares adapter and parsing code with the checker contract through `crates/checker-logic`, so the off-chain and on-chain checks can't disagree on the math.
- Prebuilt binaries for Linux, macOS and Windows are published on each `v*` tag (v0.1.1 today).

## 5. Web page (static, no backend)
- Hosted on Vercel (<https://backed-near.vercel.app>); reads NEAR RPC directly from the browser. Wallet-selector (MyNearWallet, Meteor) on testnet for signing `attest`.
- **Live:** Intents token-by-token backing (same-block), Rhea lending, Backed-standard contracts. USD values for major tokens from Rhea's `aggregate-oracle.near` (Pyth-compatible `list_prices`, verified live).
- **Checker:** `Suspected`/`Confirmed` records and the breach log, a free same-block **Preview**, and **Sign & record** from the user's wallet.
- **Replays:** the three 2026 attacks step by step, naive vs Backed, with owed/held bars.
- **Build:** why, principles, roadmap, and how to start (MCP, library, standard).

## 6. AI-coding-agent skill
`SKILL.md` for Claude Code / Codex / other agents:
- When writing a NEAR contract that holds tokens: use `backed` building blocks; never credit from caller numbers; callbacks private, gas-reserved, log-bounded; run `backed chaos`.
- Before depositing into a NEAR protocol: run `backed check`.

## 7. MCP server (for agents that move funds)
- Read-only; never signs. Tools: `check_backing(target, tokens, adapter?, block?)`, `intents_backing_summary()`, `checker_status(target, token, network?, checker?)`.
- **Hosted:** Streamable HTTP at `https://backed-near.vercel.app/api/mcp` (one Vercel function, `api/mcp.ts`). Listed in the official MCP Registry as `io.github.rohan911438/backed`.
- **Local:** `backed mcp` over stdio, for agents that shouldn't depend on our server or need past blocks.
