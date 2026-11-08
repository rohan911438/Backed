# 10 — User Workflows

Who touches Backed, in what order, and what they see. ✅ = works today. 🗓 = on the 4-week roadmap (Nov 15 – Dec 12, 2026), with the week number.

## 1. Contract developer (main user)

**Goal:** ship a NEAR contract that holds tokens without the async ledger bugs.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | Add `backed = { git = "https://github.com/rohan911438/Backed" }` | Library with `Ledger`, `Delivered`, `Claimed`, `LogBudget` | ✅ |
| 2 | In `ft_on_transfer`: `Delivered::from_token_transfer(&token, amount)`, then `ledger.credit(...)` | Unknown tokens are returned automatically; real ones credited | ✅ (`backed-vault` example) |
| 3 | Withdraw: `ledger.debit_for_withdraw(...)` → `ft_transfer` → private resolve → `ledger.settle_withdraw(...)` | Failed transfers re-credit; `owed ≤ held` checked on every change | ✅ |
| 4 | Follow-up deposits: `credit_with_followup` → notify → private resolve → `finalize_followup` | Refunds capped; the resolve can't panic | ✅ |
| 5 | Expose `backed_solvency(token)` and a public `reconcile(token)` | Anyone can check the books against the token's real balance | ✅ |
| 6 | Run `backed chaos` in CI | Step-by-step report: injection, owed vs held, shortfall, attacker gain; non-zero exit if a Backed vault breaks | ✅ built-in replays; 🗓 week 4: replays against your own wasm |
| 7 | Deploy, then `register_self` on the checker | Contract readable by the checker with no custom adapter | ✅ testnet (`backed-vault`, Healthy); 🗓 week 2: mainnet |

**Where they get stuck, and the answer:**
- "My callback isn't marked private in tests." `#[private]` is only enforced in the compiled wasm; the SWEAT/HOT replay tests it in the sandbox.
- "Reconcile says breach." Tokens left without the ledger knowing: a fee or burn token, or a bug. Withdrawals of that token pause until a healthy reconcile.

## 2. Protocol that won't change code (NEAR Intents, Rhea) and the NEAR team

**Goal:** know continuously whether they're backed, at zero effort.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | Nothing to install | Checked from their own public views (`mt_supply`, `get_asset`) | ✅ |
| 2 | Open the web app → Live backing | Token-by-token owed vs held at one block; issues sorted first | ✅ |
| 3 | Press **Verify** on any row | Fresh same-block check from the browser | ✅ |
| 4 | Look at an under-backed token | Coverage %, shortfall, block, and the traced cause in `docs/FINDINGS.md` | ✅ (2 today, 3 traced in total) |
| 5 | Get alerted | Daily scan opens a GitHub issue (optional Discord/Telegram) when a token newly becomes under-backed | ✅ |
| 6 | Record it on-chain: anyone runs `attest(target, token)` | `Suspected` → `Confirmed` after ≥30 blocks; event log | ✅ testnet; 🗓 week 2: mainnet, triggered by the daily scan |
| 7 | Adopt the standard | Implement `backed_solvency` → no custom adapter needed | ✅ draft NEP; 🗓 week 1: public discussion; week 3: first external adopter |

## 3. AI agent, wallet or user about to deposit

**Goal:** a yes/no signal before putting money in.

| Step | Action | What they see | Status |
|---|---|---|---|
| 1 | Agent calls MCP `check_backing(target, tokens)` (hosted, nothing to install) | *BACKED*, or *UNDER-BACKED for X: don't deposit X without investigating*, with block and numbers | ✅ |
| 2 | Or `backed check <protocol> --token <token> --json` | Structured result for a script or tool call | ✅ |
| 3 | Or read `get_status(target, token)` on the checker | On-chain state, usable by other contracts | ✅ testnet; 🗓 week 2: mainnet |
| 4 | Wallet shows a warning before "Deposit" | Same check, inside the deposit flow | 🗓 week 4: working example |
| 5 | Rule | Never deposit into an under-backed token or a `Confirmed` breach | ✅ in the MCP verdict and `SKILL.md` |

## 4. AI coding agent (Claude Code, Codex)

| Step | Action | Status |
|---|---|---|
| 1 | Reads `skill/SKILL.md` when writing a NEAR contract that holds tokens | ✅ |
| 2 | Uses `backed` building blocks; never credits caller-supplied numbers | ✅ |
| 3 | Runs `backed chaos` before finishing | ✅ |
| 4 | Runs `backed check` (or the MCP tool) before depositing into any protocol | ✅ |

## 5. Hackathon judge (human or AI)

| Step | Do this | You see |
|---|---|---|
| 1 | Open the README → "Every claim, and how to check it" | Each claim with the exact step to verify it |
| 2 | Open <https://backed-near.vercel.app> → Live backing → filter **issues** → **Verify** | Under-backed Intents tokens, re-read from mainnet in your browser |
| 3 | Open `docs/FINDINGS.md` → tx `hmKEgaX2…` on nearblocks | The sell tax that Intents credits back as a refund, exactly 3% |
| 4 | Web app → Replays | Naive vault loses money; Backed vault doesn't, in all three attacks |
| 5 | `cargo test -p backed -p backed-checker-logic -p backed-checker -p backed-chaos -p naive-vault -p backed-vault` | 36 passing tests, incl. an async timing simulation with zero false alarms and Rhea math on a real mainnet fixture |
| 6 | `backed scan-intents` (release binary) | Today's numbers: 387 tokens, 380 backed, 2 under-backed, 5 unreadable (2026-11-09) |
| 7 | `claude mcp add --transport http backed https://backed-near.vercel.app/api/mcp` | An agent that refuses to deposit into an under-backed token |

## Web app map (<https://backed-near.vercel.app>)

| Page | Route | What it does |
|---|---|---|
| Home | `#/` | The problem, the Intents replay side by side, connect wallet |
| Live backing | `#/app` | Every Intents token, Rhea lending, Ref (floor); filters, search, **Verify** per row |
| Checker | `#/app/checker` | On-chain attestations, breach log, free **Preview**, **Sign & record** |
| Replays | `#/app/replays` | The three attacks step by step, naive vs Backed |
| Build | `#/app/build` | Why, principles, roadmap, get started (MCP, library, standard), the six rules |
