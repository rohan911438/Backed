# 11 — Go-to-Market and Target Users

**Tagline:** *Never owe more than you hold.*
**One line:** Backed is proof of backing for NEAR protocols: it reads what a protocol owes and what it holds at the same block, and ships the Rust library that keeps the two equal.

## Why it can be used today, without a mainnet checker

The checker contract runs on testnet by choice for this submission. That doesn't limit who can use Backed, because the checker is only the optional on-chain **record** of a reading. The reading itself always comes from live mainnet:

| Product surface | Network it reads | Needs the checker? |
|---|---|---|
| Web app: Live backing and **Verify** | Mainnet RPC, at one block | No |
| CLI: `check`, `scan-intents`, `scan-ref` | Mainnet RPC (any past block with `--block`) | No |
| MCP server: `check_backing`, `intents_backing_summary` | Mainnet RPC + daily snapshot | No |
| Daily scan + alerts | Mainnet RPC | No |
| Library + chaos replays | Local NEAR sandbox | No |
| On-chain attestation (`attest`) | Testnet today; same wasm for mainnet | Yes |

Five of the six surfaces are in use on mainnet data now. The sixth moves to mainnet in week 2 of the roadmap.

## Target users

### 1. Beachhead: NEAR DeFi protocols with an internal ledger

**Who:** NEAR Intents, Rhea lending, Ref Finance, and every new vault, lending market or DEX on NEAR that holds user tokens in its own books.

**Pain:** the three 2026 incidents ($22M+) were all this bug. After the Intents exploit, NEAR's engineers wrote the check by hand (`list_nep245_tokens.sh`). Protocols have no public, continuous way to show users that the books are backed.

**Evidence of the need:**
- NEAR wrote the check themselves after a $3.8M loss.
- Backed found live gaps in Intents and traced them to transactions ([findings](../docs/FINDINGS.md)).
- Most large NEAR protocols expose no total of what they owe, so no outsider can verify them today. The `backed_solvency` view fixes that.

**What they get, by effort:**

| Effort | What they get |
|---|---|
| None | Checked today from their existing views (Intents `mt_supply`, Rhea `get_asset`) in the web app, CLI, MCP and daily scan |
| One view method | `backed_solvency(token)` makes them checkable by anyone with no custom adapter, and listed by the checker via `register_self` |
| New code | The `backed` library for deposits, withdrawals and callbacks, plus `backed chaos` in CI |

**First conversation:** responsible disclosure of the tax-refund finding to NEAR Intents (week 1). It is concrete and useful to them, and it shows the product working on their own contract.

### 2. AI agents and agent wallets that move funds

**Who:** agents that deposit, swap or lend on a user's behalf (Claude, Cursor, wallet agents, NEAR AI agents).

**Pain:** an agent has no machine-readable signal that a deposit target is solvent.

**What they use:** the hosted MCP server, already listed in the official MCP Registry as `io.github.rohan911438/backed`. One tool call returns *BACKED* or *UNDER-BACKED for X, don't deposit X without investigating*, with the block height and both numbers. It is read-only and never signs.

**Onboarding:** one line, `claude mcp add --transport http backed https://backed-near.vercel.app/api/mcp`, or a JSON config for any other client.

### 3. NEAR contract developers and AI coding agents

**Who:** teams writing token-holding contracts, hackathon teams, and the AI coding agents that write a growing share of that code.

**Pain:** async compensation logic is hand-written per team, and AI agents repeat the unsafe patterns unless the safe one is the default.

**What they use:** the `backed` crate (`Delivered`/`Claimed`, capped refunds, log budget, reconcile), `skill/SKILL.md` for Claude Code, Codex and similar agents, and the replays as a CI gate (`backed chaos` exits non-zero if a Backed vault breaks).

### 4. Wallets and deposit front-ends

**Who:** NEAR wallets and protocol front-ends that show a "Deposit" button.

**Pain:** users deposit into contracts they can't inspect.

**What they use:** the MCP endpoint or `backed check --json` as a pre-deposit check. Week 4 of the roadmap ships a working example a wallet can copy.

### 5. Auditors and security teams

**Who:** firms and independent reviewers auditing NEAR contracts.

**Pain:** audits are point-in-time, and NEAR runtime failures (log limit, out-of-gas callbacks, lying venues, direct callback calls) are hard to reproduce by hand.

**What they use:** the `backed-chaos` failure catalog (F1–F8) and replays. Week 4 of the roadmap lets them run the replays against any contract's wasm.

## How each user starts

| You are | Do this | You see |
|---|---|---|
| A user | Open <https://backed-near.vercel.app> → **Live backing** → **Verify** | Owed vs held for every Intents token, re-read from mainnet at one block |
| An agent builder | Add the MCP server, ask "Is it safe to deposit NINU into NEAR Intents?" | *UNDER-BACKED, don't deposit* with numbers and block |
| A protocol team | `backed check <contract> --token <token> --json` in a cron, or fork the repo and keep `monitor.yml` | Daily result; GitHub issue, Discord or Telegram alert on a new gap |
| A contract developer | Add `backed` as a git dependency and follow `contracts/examples/backed-vault` | Balances that only grow from delivered tokens |
| An auditor | `backed chaos --scenario intents` | Owed and held after every step of the attack |

## Distribution channels

| Channel | Status |
|---|---|
| Official MCP Registry (`io.github.rohan911438/backed`) | Live since 2026-10-25 |
| GitHub releases (CLI for Linux, macOS, Windows + checker wasm) | Live, v0.1.1 |
| Public web app on Vercel | Live |
| Agent skill (`skill/SKILL.md`) in AI coding tools | Live in the repo |
| Responsible disclosure to NEAR Intents | Week 1 |
| `backed_solvency` NEP discussion with protocol teams | Week 1 |
| First external protocol reporting through `backed_solvency` | Week 3 |
| Wallet pre-deposit example | Week 4 |

## Cost and sustainability

- **Running cost is close to zero:** the web app is static, the daily scan runs on free GitHub Actions, the MCP endpoint is one serverless function, and the checker pays for its own storage per attestation. A mainnet checker deploy costs about 2 NEAR once.
- **Model:** the library, checker, CLI and MCP server stay free and open source (MIT or Apache-2.0). That is what lets it become a standard. Options to fund ongoing work:
  - ecosystem grants for the NEP and the checker as public infrastructure;
  - paid integration for protocols that want a custom adapter, alert routing or a reviewed `backed_solvency` implementation;
  - audit support: running the chaos replays against a client's contract.

## What we'll measure in the first month

These are targets, not results:

| Signal | Target by Dec 12, 2026 |
|---|---|
| Intents acknowledges the tax-refund report | Yes |
| Checker live on mainnet with Intents and Rhea attestations | Yes |
| External contracts reporting through `backed_solvency` | 1 |
| Daily scan running without a missed day | 100% |
| Wallet or front-end pre-deposit example | 1 working example |

## Risks to adoption, and the answer

| Risk | Answer |
|---|---|
| Protocols won't change code | They don't have to: Intents and Rhea are checked today from existing views. The library targets new code. |
| "It's just a dashboard" | The dashboard is the last layer. The library prevents the bug and the replays prove it. |
| A protocol under-reports what it owes | `held` always comes from the token contract, never from the protocol. Known adapters use the protocols' own accounting views. |
| Small-token gaps from burns or fees look like exploits | Every finding is labeled with its cause; small-token gaps are treated as findings to investigate, not exploits. |
