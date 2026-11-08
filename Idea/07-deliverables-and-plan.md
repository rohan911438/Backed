# 07 — Deliverables, Scope Decisions, Budget

## What was delivered

| # | Deliverable | Planned | Delivered | Where to check |
|---|---|---|---|---|
| 1 | `backed` Rust library | ✅ | ✅ `Ledger`, `Delivered`/`Claimed`, capped refunds, `LogBudget`, race-free reconcile, `backed_solvency` | `crates/backed` |
| 2 | Three attack replays (Intents, SWEAT/HOT, Rhea) | ✅ | ✅ 3 of 3 break the naive vault, 3 of 3 blocked by the Backed vault, in CI | `backed chaos`, web app → Replays |
| 3 | Checker contract | Mainnet | ✅ **Testnet** (by decision, see below); mainnet-ready, same wasm | `simplekilt7490.testnet`, `docs/MAINNET.md` |
| 4 | Live web page | GitHub Pages | ✅ Vercel, multi-page app with wallet signing | <https://backed-near.vercel.app> |
| 5 | CLI release | ✅ | ✅ v0.1.1, Linux/macOS/Windows | GitHub releases |
| 6 | README for AI judges ("do X → see Y") | ✅ | ✅ including a claims-and-checks table | `README.md` |
| 7 | Draft NEP `backed_solvency` | ✅ | ✅ and used end to end on testnet by `backed-vault` | `docs/NEP-backed-solvency.md` |
| 8 | AI coding agent skill | ✅ | ✅ | `skill/SKILL.md` |
| 9 | Daily scan with history and alerts | Not planned | ✅ added | `.github/workflows/monitor.yml`, `data/backing-history.csv` |
| 10 | Root-cause findings for under-backed tokens | Not planned | ✅ added | `docs/FINDINGS.md` |
| 11 | MCP server | Not in v1 | ✅ added: hosted + local, listed in the official MCP Registry | `api/mcp.ts`, `server.json` |
| 12 | Ref Finance floor check | Not planned | ✅ added | `backed scan-ref`, web app |
| 13 | Demo video + X post | ✅ | The team lead records and posts | — |

## Why the checker stays on testnet for this submission

- Every **reading** in the product (web app, CLI, daily scan, MCP) already comes from live **mainnet** RPC. The checker only **records** a reading on-chain.
- Testnet shows the full on-chain flow: wallet-signed `attest`, cross-contract reads, Suspected → Confirmed, storage refunds, and the `backed_solvency` standard end to end with a real deposit.
- The mainnet deploy is prepared (`live mainnet <acct> preflight` prints the cost; about 1.91 NEAR for the 161 KB wasm) and scheduled for week 2 of the roadmap.

## Scope decisions

| Piece | Built? | Why |
|---|---|---|
| **Rust library** | ✅ core | Prevention only happens inside contracts; a web app can't do it. |
| **Chaos harness** | ✅ core | It proves the claims, and judges (human or AI) can run it. |
| **Checker contract** | ✅ | Trustless on-chain record others can build on. |
| **CLI** | ✅ thin | One binary for judges, CI, the daily job and agents. Shares the checker's logic crate. |
| **Web app** | ✅ static | The public face; reads RPC from the browser, so nothing to host or pay for. |
| **MCP server** | ✅ added | Agents that move funds needed a one-call check; a CLI alone wasn't enough. |
| JS/TS SDK | ❌ | The web app reads RPC directly; no outside JS users yet. |
| Backend / database | ❌ | Static app + free cron + on-chain records are enough. |

A web app alone would be "just a dashboard". The library and replays make it infrastructure; the web app, CLI and MCP are how people use it.

## Budget

| Item | Cost |
|---|---|
| Testnet checker and example vault | Free (testnet NEAR) |
| Web app (Vercel), daily scan (GitHub Actions), MCP endpoint (one Vercel function), RPC | Free |
| Mainnet checker deploy (roadmap week 2) | ~1.91 NEAR once, storage for 161 KB |
| Mainnet attestations | 0.005 NEAR attached per call, unused storage refunded |

## The next 4 weeks

| Week | Work |
|---|---|
| 1 (Nov 15–21) | Responsible disclosure of the tax-refund finding to NEAR Intents; open the `backed_solvency` NEP discussion |
| 2 (Nov 22–28) | Checker on mainnet with Intents and Rhea adapters; daily job records an on-chain attestation on a mismatch |
| 3 (Nov 29–Dec 5) | First external protocol self-registers via `backed_solvency`; checker ownership to a multisig (`set_owner`) |
| 4 (Dec 6–12) | Wallet pre-deposit check example; chaos replays against any contract's wasm |

Go-to-market for the same period: [11-go-to-market.md](11-go-to-market.md).
