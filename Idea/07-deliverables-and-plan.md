# 07 — Deliverables, SDK/CLI/Web Decision, Plan, Budget

## Do we need an SDK, a CLI, or just a web app?

| Piece | Build it? | Why |
|---|---|---|
| **Rust library (`backed` crate)** | ✅ **Yes, core** | It *is* the SDK. Prevention only happens inside contracts, and a web app can't do that. |
| **Chaos harness (`backed-chaos`)** | ✅ **Yes, core** | It proves the claims: the three incident replays are the strongest demo and what AI judges can run. |
| **Checker contract (mainnet)** | ✅ **Yes** | Required for "on mainnet" and for trustless, on-chain verification others can rely on. Small; fits the budget. |
| **CLI (`backed`)** | ✅ **Yes, thin** | One binary for judges (`backed check intents.near`), CI (`backed chaos`), the keeper cron and AI agents (a CLI is the simplest agent tool). Reuses the Rust adapter code. |
| **Web page** | ✅ **Yes, static** | The public face for the demo: live Intents backing token by token, Rhea, breach history. No backend, so it costs nothing. |
| **AI-coding-agent skill (SKILL.md)** | ✅ **Yes, tiny** | Gets Backed used by default by Claude Code / Codex. Half a day of work. |
| JS/TS SDK | ❌ Not in v1 | The web page reads RPC directly; no outside JS users yet. |
| MCP server | ❌ Not in v1 | CLI + skill already give agents everything; can be added later in a day. |
| Backend server / database | ❌ No | Static page + free cron + on-chain records are enough. Nothing to host or pay for. |

**Answer:** a web app alone would look like "just a dashboard", which is the scanning trap. The library + chaos harness are what make it infrastructure. The web page and CLI are how people see and use it.

## Deliverables (what the judges get)
1. **Public GitHub repo** with: `crates/backed`, `crates/backed-chaos`, `contracts/checker`, `contracts/examples/{naive,backed}-*`, `cli/`, `web/`, `skill/SKILL.md`.
2. **Three replay tests**: `cargo test replay_` shows naive contracts losing money and Backed contracts surviving, for Intents, Rhea and SWEAT/HOT patterns.
3. **Checker contract on mainnet** with Intents and Rhea adapters, plus real attestations.
4. **Live web page** (GitHub Pages).
5. **CLI release** (prebuilt binary + `cargo install`).
6. **README written for AI judges**: what, why, and "do X → see Y" steps.
7. **Demo video + X post** saying it was built at NEAR HACKATHON.
8. **Draft NEP text** for `backed_solvency`.

## 14-day plan (Nov 1–14, 2026)

| Day | Work | Done when |
|---|---|---|
| 1 | Repo, WSL sandbox, mainnet checks: `attest` gas estimate, checker size estimate | Sandbox test runs; budget confirmed |
| 2–3 | `Ledger`, `Delivered`/`Claimed`, safe withdraw, constant-time invariant | Unit tests green |
| 4 | Safe receiver: registry check, provisional credit, infallible resolve (caps → log budget → gas) | F2/F4 cases pass |
| 5 | Race-free `reconcile` + breach mode + `backed_solvency` view | Reconcile tests with in-flight transfers show no false alarms |
| 6–7 | `backed-chaos`: sandbox runner, invariant oracle, injections F1–F8, helper contracts | Harness flags the naive contracts |
| 8 | Three replays: naive vs Backed | `cargo test replay_` tells the story |
| 9–10 | Checker contract: adapters, Suspected/Confirmed, storage deposits; deploy to mainnet | Real attestations for Intents and Rhea on mainnet |
| 11 | CLI: `check`, `scan`, `attest`, `chaos`; GitHub Actions keeper | `backed scan intents.near` matches our research numbers |
| 12 | Web page (static) + SKILL.md | Live page online |
| 13 | README for AI judges, NEP draft, polish, stretch (run chaos against near/intents v0.4.4) | Fresh-clone instructions work end to end |
| 14 | Demo video, X post (you publish), make repo public, submit (after you confirm the payload) | Submission returns 201 |

**Cut list if behind (in order):** stretch goal → NEP text → F8 injection → web history view → Rhea adapter (keep Intents).

## Budget
| Item | Cost |
|---|---|
| Checker contract deploy (**measured: 156KB** with cargo-near 0.22, at 1 NEAR / 100KB of storage) | ~1.56 NEAR |
| Mainnet attestations during the demo (~0.003 NEAR each, estimated) | <0.1 NEAR |
| Everything else (sandbox, GitHub Pages, Actions cron, RPC) | free |
| **Total** | **~1.7 NEAR ≈ $8–9** at ~$4.9/NEAR |

Keeping the checker contract small (no heavy dependencies, opt-level "z", wasm-opt) is what keeps this near $5.

## Submission checklist (from HACKATHON.md)
- [ ] Repo public before 2026-11-14 23:59 UTC
- [ ] X post by you: built at NEAR HACKATHON, with demo video or screenshots
- [ ] Tracks: `security`, `defi`, `open` (+ `agents` if the skill and agent check are strong)
- [ ] Mainnet NEAR account for payout (you create it)
- [ ] "What we built" text (20–2000 chars)
- [ ] Payload printed and confirmed by you before sending; one attempt only
