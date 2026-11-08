# 02 — Product

**Tagline:** *Never owe more than you hold.*

## One sentence
**Backed is proof of backing for NEAR protocols: anyone can check, from public chain state at one block, that a protocol holds every token its books say it owes, and contract developers get a Rust library that makes the bug behind a gap impossible to ship.**

## The parts

| Part | Form | For whom | Job | Status |
|---|---|---|---|---|
| **`backed`** | Rust library (crate) | NEAR contract developers | **Prevent:** accounting building blocks where the three 2026 bugs can't compile or can't execute | ✅ |
| **`backed-chaos`** | Rust test harness + `backed chaos` CLI command | Developers, auditors, CI | **Prove:** replay each 2026 attack as real sandbox transactions and check owed ≤ held after every step | ✅ |
| **Checker contract** | NEAR contract | Protocols, wallets, agents, users | **Verify:** trustless on-chain record of owed vs held, with no reporter | ✅ testnet; mainnet in roadmap week 2 |
| **Live backing** | Web app, CLI, daily scan | Users, protocols, the NEAR team | **Watch:** every Intents token, Rhea lending and Ref (floor), read from mainnet at one block | ✅ |
| **Agent tooling** | Hosted MCP server, `skill/SKILL.md` | AI agents and AI coding agents | **Use:** check before depositing; write safe NEAR contracts by default | ✅ |

## Who uses it, step by step

### A. A NEAR DeFi developer (main user)
1. Add `backed = { git = "https://github.com/rohan911438/Backed" }`.
2. Replace hand-written deposit, withdraw and refund code with Backed's building blocks (`Ledger`, `Delivered`, `Claimed`, `credit_with_followup`, `debit_for_withdraw`). The example is [`contracts/examples/backed-vault`](../contracts/examples/backed-vault/src/lib.rs).
3. Run `backed chaos` in CI. It replays the three attacks and exits non-zero if a Backed vault ever owes more than it holds. Running the replays against your own wasm is roadmap week 4.
4. Ship. The contract exposes the standard `backed_solvency(token)` view and can call `register_self` on the checker.
5. Optional: call `reconcile(token)` so withdrawals of a token pause if real holdings ever fall below the ledger.

### B. An existing protocol that won't change code (Intents, Rhea)
Nothing to install. Backed reads their public views (`mt_supply`, `get_asset`) and the tokens' balances. They get continuous, public backing verification for free, plus a ready-made standard to adopt later.

### C. An AI agent, wallet or user about to deposit
- Web app → **Verify**, `backed check <protocol> --token <token> [--json]`, or the MCP tool `check_backing`.
- Answer: *BACKED at block N*, or *UNDER-BACKED for X: don't deposit X without investigating*.
- Agents can make this a rule: *never deposit into a token that is under-backed or has a confirmed breach.*

### D. An AI coding agent (Claude Code, Codex)
`skill/SKILL.md` tells it: when writing a NEAR contract that holds tokens, use Backed's building blocks, never credit caller-supplied numbers, keep callbacks private and bounded, and run `backed chaos` before finishing.

### E. The NEAR Intents team
A public, daily version of the script they wrote by hand after the exploit (`list_nep245_tokens.sh`). Today it shows 2 under-backed tokens, each traced to its cause in [`docs/FINDINGS.md`](../docs/FINDINGS.md). The NINU and BATMAN gaps come from Intents trusting a refund amount the token reports; we plan to disclose it to them in roadmap week 1.

Target users in priority order, distribution and sustainability: [11-go-to-market.md](11-go-to-market.md).

## What Backed guarantees
For a contract built with `backed`:
1. **No unverified credit.** Balances only increase from (a) a registered token's real transfer notification, or (b) a value the contract itself verified in a private callback. Caller-supplied numbers can't become balances (enforced by types).
2. **Callbacks that can't silently fail.** Refunds are capped (≤ deposited, ≤ balance), event logs stay inside NEAR's log limit, callbacks are private, and the resolve never panics.
3. **Never owe more than held.** Checked on every state change in constant time, and reconciled against the real token balance on demand, with no false alarms from in-flight transfers (see [04-innovations](04-innovations.md)).

For any protocol, through the checker and the scans:

4. **Public, trustless backing status** that anyone can verify, with no reporter, key or committee involved.

## What Backed does not do
- It doesn't make a contract "secure" in general: oracle manipulation, bad liquidation math, governance attacks and key theft are out of scope.
- It doesn't hold anyone's funds and can't pause other protocols. A protocol has to choose to read the checker.
- Some losses only appear in owed vs held once they're written off. A lending protocol's bad debt shows once it's liquidated or written off, so the checker sees a Rhea-style attack during the attack, not before. The library prevents that bug at the source; the checker is the backstop.
- The checker covers the protocols it has adapters for: NEP-141 tokens in Intents and Rhea lending, plus any contract that implements `backed_solvency`.

## Why people will care
- **Protocols:** stop the bug family that cost NEAR $22M+ this year, and give users public proof of backing they can check themselves.
- **NEAR ecosystem:** one shared safety standard instead of each team relearning async accounting after a hack. It complements NEAR's planned formal verification with runtime guarantees.
- **Users and agents:** a simple, trustless signal before depositing.
