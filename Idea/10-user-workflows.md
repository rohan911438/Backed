# 10 — User Workflows

Who touches Backed, in what order, and what they see. Each flow notes what exists today (✅) and what's still being built (🔨).

## 1. Contract developer (main user)

**Goal:** ship a NEAR contract that holds tokens without the async ledger bugs.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | `cargo add backed` | Library with `Ledger`, `Delivered`, `Claimed`, `LogBudget` | ✅ crate builds, 16 tests |
| 2 | In `ft_on_transfer`: `Delivered::from_token_transfer(&token, amount)`, then `ledger.credit(...)` | Unknown tokens are returned automatically; real ones credited | ✅ (`backed-vault` example) |
| 3 | Withdraw: `ledger.debit_for_withdraw(...)` → `ft_transfer` → private `resolve_withdraw` → `ledger.settle_withdraw(...)` | Failed transfers re-credit; `owed ≤ held` checked on every change | ✅ |
| 4 | Follow-up deposits: `credit_with_followup` → notify → private resolve → `finalize_followup` | Refunds capped; the resolve can't panic | ✅ |
| 5 | Expose `backed_solvency(token)` and a public `reconcile(token)` | Anyone can check the books against the token's real balance | ✅ |
| 6 | Run `backed chaos` (or `cargo test -p backed-chaos --features sandbox replay_`) in CI | Step-by-step report: injection, owed vs held, shortfall, attacker gain | ✅ built-in replays; 🔨 plugging in your own contract (add a `VaultKind`) |
| 7 | Deploy, then `register_self` on the checker | Contract shows up on the web page | ✅ checker live on testnet; 🔨 mainnet |

**Where they get stuck, and the answer:**
- "My callback isn't marked private in tests." `#[private]` is only enforced in the compiled wasm; `backed-chaos` F5 tests it in the sandbox.
- "Reconcile says breach." Tokens left without the ledger knowing: a fee or burn token, or a bug. Withdrawals of that token pause until a healthy reconcile.

## 2. Protocol that won't change code (NEAR Intents, Rhea) and the NEAR team

**Goal:** know continuously whether they're backed, at zero effort.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | Nothing to install | Checked from their own public views (`mt_supply`, `get_asset`) | ✅ |
| 2 | Open the web page → Live | Token-by-token owed vs held at one block; issues sorted first | ✅ |
| 3 | Press **Verify** on any row | Fresh same-block check from the browser at the current block | ✅ |
| 4 | Look at an under-backed token | Coverage %, shortfall, block; note that burns or fees can explain small tokens | ✅ (3 found today) |
| 5 | Want it on-chain? Anyone runs `attest(target, token)` | `Suspected` → `Confirmed` after ≥30 blocks; event log | ✅ testnet (real Burrow attestations); 🔨 mainnet |
| 6 | Adopt the standard later | Implement `backed_solvency` → no custom adapter needed | ✅ spec, 🔨 NEP text |

## 3. AI agent, wallet or user about to deposit

**Goal:** a yes/no signal before putting money in.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | `backed check <protocol> --token <token>` | `backed` / `UNDER-BACKED` with owed and held at one block | ✅ |
| 2 | Or `--json` for machines | Structured result for an agent's tool call | ✅ |
| 3 | Or read `get_status(target, token)` on the checker | On-chain state, usable by other contracts | ✅ testnet; 🔨 mainnet |
| 4 | Rule | Never deposit into a `Confirmed` breach | ✅ documented |

## 4. AI coding agent (Claude Code, Codex)

| Step | Action | Status |
|---|---|---|
| 1 | Reads `skill/SKILL.md` when writing a NEAR contract that holds tokens | ✅ |
| 2 | Uses `backed` building blocks; never credits caller-supplied numbers | ✅ library ready |
| 3 | Runs `backed chaos` before finishing | ✅ |

## 5. Hackathon judge (human or AI)

| Step | Action | What they see |
|---|---|---|
| 1 | Open the README | Problem, three incidents with sources, what's built |
| 2 | `cargo test -p backed -p backed-checker-logic -p backed-checker -p backed-chaos -p naive-vault -p backed-vault` | 36 passing tests, incl. an async timing simulation with zero false alarms and Rhea math on a real mainnet fixture |
| 3 | `backed scan-intents` | Live: 370 tokens, 362 backed, 3 under-backed |
| 4 | Open the web page | Live sphere of tokens, explorer, Verify buttons |
| 5 | ✅ `cargo test -p backed-chaos --features sandbox replay_` in the sandbox | Naive vault loses money, Backed vault doesn't |

## Web page map
1. **Hero:** headline, live badge (`intents.near · N of M tokens backed`), token sphere (one dot per token, colored by status), stats.
2. **Problem:** three incident cards with loss, cause and the Backed fix.
3. **How it works:** Prevent / Prove / Verify.
4. **Live:** explorer with tabs (NEAR Intents from the latest snapshot, Rhea lending live), filters, search, Verify per row.
5. **Who uses it:** the three flows above + code and CLI samples (real output).
6. **Footer:** links, data note.
