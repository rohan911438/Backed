# Backed — design documents

> **Never owe more than you hold.**
>
> Backed is proof of backing for NEAR protocols. It reads what a protocol owes and what it holds at the same block, so anyone can check that every token is there. It also ships a Rust library that stops contracts from writing the bug that breaks this.

NEAR HACKATHON (Nov 1–14, 2026) · solo · Rust · tracks: **Open Innovation** (`open`), **Unstoppable DeFi** (`defi`) · live app: <https://backed-near.vercel.app>

## Status at submission

| Part | Status |
|---|---|
| `backed` library + six rules | ✅ shipped, 36 unit tests across the workspace |
| Three 2026 attacks replayed on a NEAR sandbox | ✅ 3 of 3 break the naive vault, 3 of 3 blocked by the Backed vault, in CI |
| Live backing for Intents, Rhea lending, Ref (floor) | ✅ reads mainnet; latest Intents scan 380 of 387 backed, 2 under-backed, 5 unreadable (2026-11-09) |
| Root cause of every under-backed token | ✅ traced to transactions ([findings](../docs/FINDINGS.md)) |
| Checker contract | ✅ live on **testnet**; same wasm ready for mainnet ([checklist](../docs/MAINNET.md)) |
| Daily scan, history and alerts | ✅ GitHub Actions, every day since 2026-10-25 |
| MCP server | ✅ hosted, and listed in the official MCP Registry |
| CLI release | ✅ v0.1.1 for Linux, macOS, Windows |
| Draft NEP `backed_solvency` | ✅ [`docs/NEP-backed-solvency.md`](../docs/NEP-backed-solvency.md) |

## The folder

| File | What's inside |
|---|---|
| [01-problem-and-evidence.md](01-problem-and-evidence.md) | The bug family, the 3 incidents of 2026 with root causes and sources, our live mainnet scans |
| [02-product.md](02-product.md) | What Backed is, who uses it and how, what it guarantees and what it doesn't, common questions |
| [03-architecture.md](03-architecture.md) | Technical design: library, chaos harness, checker, scanner, CLI, web app, MCP, agent skill |
| [04-innovations.md](04-innovations.md) | What's new in the design |
| [05-why-near.md](05-why-near.md) | Why this only makes sense on NEAR |
| [06-competitors.md](06-competitors.md) | Big and small competitors, and how we differ |
| [07-deliverables-and-plan.md](07-deliverables-and-plan.md) | What was delivered against the plan, scope decisions, budget |
| [08-design-audit.md](08-design-audit.md) | Audit of the design: findings, fixes, how each was verified |
| [10-user-workflows.md](10-user-workflows.md) | Step by step for developers, protocols, agents, AI coding agents and judges |
| [11-go-to-market.md](11-go-to-market.md) | Target users, how each one starts, distribution, sustainability, first-month targets |

## In 30 seconds

**Problem.** On NEAR, calls between contracts are async and nothing rolls back on its own. When a callback fails or trusts a reported number, a contract's ledger keeps balances no real tokens back.
- NEAR Intents lost $3.8M on 2026-10-01: a resolve callback failed on NEAR's log-size limit after a deposit was recorded.
- Rhea Finance lost $18.4M on 2026-04-16: margin trading accepted far less than the route claimed.
- SWEAT/HOT had a zero-day: a refund callback anyone could call.

**Product: prevent, prove, verify, use.**
1. **`backed` (Rust library):** balances grow only from `Delivered` amounts, callbacks are private and can't panic, `owed ≤ held` is asserted on every change.
2. **`backed-chaos` (test harness):** replays the three 2026 attacks on a NEAR sandbox. Naive code loses money; Backed code doesn't.
3. **Checker contract:** anyone records on-chain whether a protocol holds what it owes. No trusted reporter.
4. **CLI, web app, daily scan, MCP server, agent skill:** check any supported protocol in one command, in the browser, or from an AI agent before a deposit.

**Proof it's real.**
- Every day Backed checks every NEP-141 token in `intents.near` at one block. On 2026-11-09: **380 of 387 backed, 2 under-backed, 5 unreadable.**
- We traced the gaps to transactions. Two memecoins take a sell tax and report it as "refunded"; Intents credits a refund that never arrived (tx `hmKEgaX2…`, exactly 3%).
- NEAR's own engineers wrote this check by hand after the October exploit (`scripts/list_nep245_tokens.sh` in near/intents). Backed makes it continuous, on-chain and preventive.

**Next 4 weeks** (Nov 15 – Dec 12): disclosure to Intents and the NEP discussion, then mainnet checker with on-chain daily attestations, then protocol self-registration and multisig ownership, then a wallet pre-deposit check and chaos replays against any wasm. Details in the main [README roadmap](../README.md#roadmap-next-4-weeks).
