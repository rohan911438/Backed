# 02 — Product

## One sentence
**Backed makes "owe more than you hold" bugs impossible to ship on NEAR, and lets anyone prove on-chain that a protocol holds what it owes.**

## The four parts

| Part | Form | For whom | Job |
|---|---|---|---|
| **`backed`** | Rust library (crate) | NEAR contract developers | Prevent: accounting building blocks where the three 2026 bugs can't compile or can't execute |
| **`backed-chaos`** | Rust test harness (crate + CLI command) | Developers, auditors, CI | Prove: fail every async step on purpose and check the safety rule after every step |
| **Backing checker** | Mainnet contract | Everyone: protocols, wallets, agents, users | Verify: trustless on-chain record of "owed vs held" for any protocol |
| **Tooling** | `backed` CLI, live web page, AI-coding-agent skill | Developers, judges, AI agents, the NEAR team | Use: one-command checks, a public live view, safe-by-default code from AI |

## Who uses it, step by step

### A. A NEAR DeFi developer (main user)
1. `cargo add backed`
2. Replace hand-written deposit/withdraw/refund code with Backed's building blocks (`Ledger`, safe receiver, safe withdraw, verified credit).
3. Add one test: `backed_chaos::run(contract_wasm)`. It fails every callback, overflows logs, plays malicious tokens, and reports any moment where owed > held.
4. Ship. The contract exposes a standard `backed_solvency(token)` view and can self-register with the checker.
5. Optional: the contract calls `reconcile(token)` itself and pauses withdrawals of a token if real holdings ever fall below its ledger.

### B. An existing protocol that won't change code (Intents, Rhea)
Nothing to install. The checker reads their public views (`mt_supply`, `get_asset`) and token balances. They get continuous, public backing verification for free, plus a ready-made standard to adopt later.

### C. An AI agent, wallet or user about to deposit
- `backed check <protocol>` (CLI), or read `get_status` on the checker contract, or open the web page.
- Answer: "fully backed as of block N", or "breach suspected/confirmed for token X".
- Agents can make this a rule: *never deposit into a protocol with a confirmed breach.*

### D. An AI coding agent (Claude Code, Codex)
The Backed skill file tells it: when writing a NEAR contract that holds tokens, use Backed's building blocks, never credit from caller-supplied numbers, keep callbacks private and bounded, run `backed-chaos` before finishing.

### E. The NEAR Intents team
A public, continuous version of the script they wrote by hand after the exploit (`list_nep245_tokens.sh`). Day one it shows 2 under-backed memecoins they may want to look at.

## What Backed guarantees
For a contract built with `backed`:
1. **No unverified credit.** Balances only increase from (a) a token contract's real transfer notification, or (b) a value the contract itself verified in a private callback. Caller-supplied numbers can't become balances (enforced by types).
2. **Callbacks that can't silently fail.** Refunds are capped (≤ deposited, ≤ balance); event logs stay inside NEAR's log limit; gas for callbacks is reserved and checked at entry; callbacks are private.
3. **Never owe more than held.** Checked on every state change in constant time, and reconciled against the real token balance on demand, with no false alarms from in-flight transfers (see 04-innovations).

For any protocol, through the checker:
4. **Public, trustless backing status** that anyone can verify, with no reporter, key or committee involved.

## What Backed does not do
- It doesn't make a contract "secure" in general: oracle manipulation, bad liquidation math, governance attacks and key theft are out of scope.
- It doesn't hold anyone's funds and has no power to pause other protocols. A protocol must choose to read the checker.
- Some losses only appear in owed-vs-held when they're written off. A lending protocol's bad debt shows once it's liquidated or written off, so the checker sees a Rhea-style attack during the attack, not before. The library prevents that bug at the source; the checker is the backstop.
- The checker covers token kinds and protocols it has adapters for (v1: NEP-141 tokens in Intents and Rhea lending, plus any contract that implements the Backed standard).

## Why people will care
- **Protocols:** stop the bug family that cost NEAR $22M+ this year, and show users a live "fully backed" badge.
- **NEAR ecosystem:** a shared safety standard instead of each team relearning async accounting after a hack. It complements NEAR's planned formal verification with runtime guarantees.
- **Users and agents:** a simple, trustless signal before depositing.
