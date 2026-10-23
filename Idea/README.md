# Backed — Verified Settlement for NEAR

> **NEAR lost $22M+ in 2026 to one bug family that barely exists on Ethereum: a contract's ledger drifts from the tokens it actually holds. Backed makes that bug impossible to ship, proves it with the real exploits replayed, and lets anyone check on-chain, live, that NEAR's protocols hold what they owe.**

Hackathon: NEAR HACKATHON (build Nov 1–14, 2026). Solo, Rust. Main tracks: `security`, `defi`. Add-ons: `open`, `agents`.

## The folder

| File | What's inside |
|---|---|
| [01-problem-and-evidence.md](01-problem-and-evidence.md) | The bug family, the 3 incidents of 2026 with root causes, our live mainnet scans |
| [02-product.md](02-product.md) | What Backed is, who uses it and how, what it guarantees and what it doesn't |
| [03-architecture.md](03-architecture.md) | Full technical design: library, chaos tester, on-chain checker, scanner, CLI, web app, agent skill |
| [04-innovations.md](04-innovations.md) | What's new in the design |
| [05-why-near.md](05-why-near.md) | Why this only makes sense on NEAR |
| [06-competitors.md](06-competitors.md) | Big and small competitors, and how we differ |
| [07-deliverables-and-plan.md](07-deliverables-and-plan.md) | SDK / CLI / web app decision, deliverables, 14-day plan, budget, submission checklist |
| [08-design-audit.md](08-design-audit.md) | Audit of this design: findings, fixes, open questions |
| [09-demo-and-pitch.md](09-demo-and-pitch.md) | 2-minute demo script, pitch lines, judge Q&A |

## In 30 seconds

**Problem.** On NEAR, calls between contracts are async. When a callback fails or is skipped, a contract's internal ledger keeps numbers that no real tokens back.
- NEAR Intents lost $3.8M on 2026-10-01 this way: a callback failed on NEAR's log-size limit after a deposit was recorded.
- Rhea lost $18.4M: it credited value it never received.
- SWEAT/HOT had a zero-day: a refund callback anyone could call.

**Product. Four parts, one rule: never owe more than you hold.**
1. **`backed` (Rust library)**: accounting building blocks that make these bugs impossible to write.
2. **`backed-chaos` (test harness)**: fails every async step on purpose. It replays the three 2026 incidents: naive code loses money, Backed code survives.
3. **Backing checker (mainnet contract)**: anyone can prove on-chain whether a protocol holds what it owes. No trusted reporter.
4. **CLI + live web page + AI-coding-agent skill**: check any protocol in one command, see NEAR Intents' backing token by token, and have Claude Code / Codex write NEAR contracts with Backed by default.

**Proof it's real (verified 2026-10-23 on mainnet):**
- intents.near: 342 of 370 NEAR-native tokens checked at one block height are fully backed. **2 small memecoins are under-backed** (unexplained shortfall). Our check finds real mismatches today.
- Rhea lending: every major token is fully backed.
- NEAR's own engineers wrote this exact check as a manual script after the October exploit (`scripts/list_nep245_tokens.sh` in near/intents). Backed makes it continuous, on-chain and preventive.
