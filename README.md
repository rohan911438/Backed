# Backed

**Verified settlement for NEAR: a contract should never owe more than it holds.**

Backed is a Rust library, a chaos-testing harness and an on-chain checker. Together they prevent, test for and detect one NEAR-specific bug family: a contract's internal ledger drifting away from the tokens it actually holds.

> **Status: working end to end on testnet.** Built: the `backed` library, the sandbox chaos runner with three incident replays, example vaults, the checker contract (**live on testnet** at [`simplekilt7490.testnet`](https://testnet.nearblocks.io/address/simplekilt7490.testnet), recording real attestations against Rhea/Burrow), the `backed` CLI, the web page and the agent skill. Next: mainnet deployment. Design: [`Idea/`](Idea/README.md).

---

## The problem

On NEAR, calls between contracts are asynchronous. Each step is a separate receipt, and **nothing rolls back automatically** when a later step fails. Protocols keep internal ledgers and hand-write compensation logic, and when that logic misses a case, the ledger records tokens the contract doesn't have.

This one bug family caused three NEAR incidents in 2026:

| Incident | Loss | What went wrong |
|---|---|---|
| NEAR Intents (2026-10-01) | $3.8M | A refund log grew past NEAR's 16,384-byte log limit, so the `mt_resolve_deposit` callback failed. Tokens were returned but balances stayed recorded. |
| Rhea Finance (2026-04-16) | $18.4M | Margin trading credited the amount a swap route *claimed*, not what was delivered. |
| SWEAT / HOT tokens (Fireblocks zero-day) | 8.5M SWEAT from one victim | A refund callback anyone could call paid out refunds that weren't owed. |

On Ethereum a transaction is all-or-nothing, so this family barely exists there. Details and sources: [`Idea/01-problem-and-evidence.md`](Idea/01-problem-and-evidence.md).

## What Backed does

| Part | Form | Job |
|---|---|---|
| `backed` | Rust crate for near-sdk contracts | **Prevent.** Balances can only grow from verified amounts; refund callbacks are private, capped and can't fail; `owed ≤ held` is checked on every state change. |
| `backed-chaos` | Rust test harness (NEAR sandbox) | **Prove.** Fails every async step on purpose (out of gas, log-limit overflow, absurd refunds, lying venues, malicious tokens) and checks the books after every step. Replays all three 2026 incidents on a local NEAR sandbox. |
| Backing checker | Contract on NEAR (live on testnet; mainnet next) | **Verify.** Anyone can record on-chain whether a protocol holds what it owes, read straight from the protocol's and token's own views. No trusted reporter. |
| `backed` CLI, web page, agent skill | Rust binary, static page, `SKILL.md` | **Use.** Check any protocol in one command, see live backing, and have AI coding agents write NEAR contracts with Backed by default. |

Architecture: [`Idea/03-architecture.md`](Idea/03-architecture.md)

## Evidence the check works (NEAR mainnet, 2026-10-23)

Backing = tokens the protocol holds (`ft_balance_of`) vs what it owes (its own views), read at the same block height.

- **NEAR Intents** (`intents.near`, block 221,256,203): of 370 NEP-141 tokens, **362 are fully backed**, **3 are under-backed** (`ninu-4.nearlytrade.near`, `batman-4.nearlytrade.near`, and the test token `nvdastest.optimal-bone.near`) and 5 can't be read (broken token contracts). The shortfall is consistent with token burns or transfer fees that the ledger doesn't see. It's reported as a finding, not an exploit.
- **Rhea lending** (`contract.main.burrow.near`): every major token is fully backed.

After the October exploit, NEAR's engineers ran this kind of check by hand (`scripts/list_nep245_tokens.sh` in [near/intents](https://github.com/near/intents)). Backed makes it continuous, on-chain and preventive.

## Try it

Rust builds need Linux, macOS or WSL (near-sdk tests don't build with the Windows GNU toolchain).

| Do | See |
|---|---|
| `cargo test -p backed -p backed-checker -p backed-chaos -p naive-vault -p backed-vault` | 34 passing tests, incl. an async timing simulation (200 runs, zero false alarms) and Rhea lending math on a real mainnet fixture |
| `cargo run -p backed-cli -- check intents.near --token usdt.tether-token.near` | Owed vs held for that token, read at one block height |
| `cargo run -p backed-cli -- scan-intents` | Every NEP-141 token in NEAR Intents: backed, under-backed, unreadable |
| `cd web && npm install && npm run dev` | The web page: token sphere, live explorer, a Verify button per row |
| `cd contracts/checker && cargo near build non-reproducible-wasm` | The checker contract wasm (156KB) |
| `scripts/build-wasm.sh && cargo test -p backed-chaos --features sandbox replay_ -- --nocapture` | Three incident replays on a real local NEAR sandbox: the naive vault ends up owing more than it holds and the attacker profits; the Backed vault blocks every attack (zero shortfall at every step) |
| `cargo run -p backed-cli --features chaos -- chaos` | The same replays as a step-by-step report (`--scenario rhea`, `--vault backed`, `--json`) |

## Live on testnet

The checker contract runs on NEAR testnet at `simplekilt7490.testnet`, with the Burrow (Rhea lending) adapter for `contract.1638481328.burrow.testnet`. Real attestations, 2026-10-24:

| Token | Owed | Held | State | Tx |
|---|---|---|---|---|
| `wrap.testnet` | 1,252.78 NEAR | 20,115.29 NEAR | Healthy | [`GPi36PvK…`](https://testnet.nearblocks.io/txns/GPi36PvKK5sw95BDVa6f2tLoXXWRjRCXyEEgZbm2wHuG) |
| `usdc.fakes.testnet` | 103,906.634556 | 103,906.635640 | Healthy | [`5MGdqCBQ…`](https://testnet.nearblocks.io/txns/5MGdqCBQ296W7dyFogHrXKM6CFGDQLFBjxBoXRsseycE) |

Reproduce: `cargo run -p backed-chaos --features sandbox --example live -- testnet <your-account> attest contract.1638481328.burrow.testnet wrap.testnet` (key read from `~/.near-credentials/testnet/`).

## Repository layout

```
crates/backed/          library: Ledger, Delivered/Claimed, capped refunds, LogBudget, reconcile
crates/backed-chaos/    failure catalog F1–F8, invariant oracle, three incident replays, NEAR sandbox runner, live deploy tool (examples/live.rs)
contracts/checker/      backing checker contract (Intents, Rhea and Backed-standard adapters)
contracts/examples/     naive-vault (the three 2026 bug patterns, on purpose) and backed-vault
contracts/helpers/      test-token, evil-receiver (attacker), lying-venue: used by the replays only
cli/                    backed CLI: check, scan-intents, attest, chaos
web/                    static web page (Vite + React), reads NEAR RPC from the browser
.github/workflows/      CI, and Pages deploy with a fresh scan every 30 minutes
Idea/                   design documents, incl. user workflows
scripts/build-wasm.sh   builds every contract with cargo-near
skill/SKILL.md          instructions for AI coding agents (Claude Code, Codex, ...)
```

## What Backed does not do

- It doesn't make a contract secure in general: oracle manipulation, bad liquidation math, governance attacks and key theft are out of scope.
- It holds no one's funds and can't pause other protocols. A protocol has to choose to read the checker.
- A lending protocol's bad debt only shows up as a backing gap once it's written off. The library prevents that bug at the source; the checker is the backstop.

## Hackathon

- **Event:** NEAR HACKATHON (rules in [`HACKATHON.md`](HACKATHON.md))
- **Tracks:** AI Security & Defense, Unstoppable DeFi, Open Innovation
- **NEAR features used:** async receipts and callbacks, NEP-141 / 145 / 245 / 297, cross-contract views, near-sdk 5.x, near-sandbox

## Design documents

| | |
|---|---|
| [Problem and evidence](Idea/01-problem-and-evidence.md) | [Product](Idea/02-product.md) |
| [Architecture](Idea/03-architecture.md) | [Innovations](Idea/04-innovations.md) |
| [Why NEAR](Idea/05-why-near.md) | [Competitors](Idea/06-competitors.md) |
| [Deliverables and plan](Idea/07-deliverables-and-plan.md) | [Design audit](Idea/08-design-audit.md) |
| [Demo and pitch](Idea/09-demo-and-pitch.md) | [User workflows](Idea/10-user-workflows.md) |
