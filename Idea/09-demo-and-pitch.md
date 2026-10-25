# 09 — Demo and Pitch

Pitch order: **data first, then the bug, then the fix.** The judge should see a real number about NEAR before hearing about a library.

## Pitch lines
- **Hook:** "Does NEAR Intents actually hold every token it says you own? We checked all 370, at one block. 362 yes, 3 no."
- **Why it matters:** "That gap, books saying more than the tokens, is the bug behind $22M+ of 2026 NEAR exploits: Intents, Rhea, SWEAT/HOT. Each one is sourced."
- **Outside support:** "After the Intents exploit, NEAR's own engineers wrote this check by hand as a script. We made it a product: daily, on-chain, for any protocol."
- **Fix:** "And a Rust library that stops contracts from writing the bug, proven by replaying all three attacks: naive code loses money, Backed code doesn't."

## 2-minute demo script

| Time | Show | Say |
|---|---|---|
| 0:00–0:20 | backed-near.vercel.app home: dot field, "362 of 370 tokens backed" | "Does NEAR Intents hold every token it owes? Each dot is a token, read from mainnet at one block. Green is backed, amber isn't." |
| 0:20–0:40 | App → Live backing → filter **issues** → press **Verify** on `ninu-4.nearlytrade.near`, then open docs/FINDINGS.md | "Three are under-backed. Verify re-reads both numbers from mainnet right now. And we traced why: this memecoin takes a 3% sell tax, tells Intents it was refunded, and Intents credits a refund that never came back. One transaction, 1,133 tokens from nothing." |
| 0:40–1:00 | Home → Problem section, incident cards with source links | "This gap caused $22M+ of losses this year. Intents' callback failed on a log limit and the books kept a balance the tokens had refunded. NEAR's own regression test says so; it's linked." |
| 1:00–1:10 | NEAR's `list_nep245_tokens.sh` on GitHub | "After that, NEAR's engineers wrote this check by hand. Backed runs it every day and records it on-chain." |
| 1:10–1:40 | App → Replays → Intents: naive vs Backed side by side | "We replay each attack on a real local NEAR sandbox. The naive vault ends up owing 100 more than it holds. The Backed vault refuses the withdrawal." |
| 1:40–1:55 | App → Checker → **Sign & record** with a wallet, then the explorer tx | "Anyone can record the result on-chain from their wallet. The checker reads both contracts itself, so nobody has to trust the reporter." |
| 1:55–2:00 | Repo README | "Open source, MIT. Backed: never owe more than you hold." |

Record at 1080p, browser zoom 110%, dark theme. Keep the terminal out unless there's time: the web app shows everything.

## Draft X post (you post it)

> Does NEAR Intents actually hold every token it owes?
>
> We built Backed at #NEARHackathon to check: 370 tokens, owed vs held at one block, on mainnet. 362 backed, 3 not.
>
> That gap is the bug behind $22M+ of 2026 NEAR exploits. Backed also ships the Rust library that prevents it, proven by replaying all 3 attacks.
>
> Try it: https://backed-near.vercel.app
> Code: https://github.com/rohan911438/Backed
>
> [video]

## Draft "What we built" (submission field, 20–2000 chars)

> Backed checks whether NEAR protocols hold the tokens their own books say they owe, and prevents the bug that makes them drift apart.
>
> On NEAR, contract calls are asynchronous and nothing rolls back. When a callback fails or trusts a claimed amount, a contract's ledger can record tokens it doesn't hold. That caused NEAR Intents' $3.8M exploit (Oct 2026), Rhea Finance's $18.4M (Apr 2026) and the SWEAT/HOT zero-day; sources are linked in the README.
>
> What we built: (1) a live check of every NEP-141 token in intents.near, owed (mt_supply) vs held (ft_balance_of) at the same block. 362 of 370 are backed and 3 are under-backed, re-verifiable from the browser. (2) A checker contract that records these attestations on-chain with no trusted reporter, deployed on testnet with the same code ready for mainnet. (3) The `backed` Rust library for near-sdk contracts: balances grow only from delivered amounts, callbacks are private and can't panic, and owed ≤ held is asserted on every change. (4) A chaos harness that replays all three attacks on a local NEAR sandbox: the naive vault loses money, the Backed vault doesn't. (5) A CLI, a daily GitHub scan with history, a draft NEP for a standard `backed_solvency` view, and a skill so AI coding agents write NEAR contracts with these rules by default.
>
> NEAR's own engineers ran this check by hand after the Intents exploit. Backed makes it continuous and preventive.

## Judge Q&A

**"Isn't this just proof of reserves?"**
Proof of reserves trusts reporters and only watches. Backed reads both numbers from the chain itself, at one block, prevents the bug in code, and proves the prevention with replays. Watching is only the last layer.

**"Is NEAR broken?"**
No. These are bugs in individual apps. NEAR's async design is deliberate, and its docs warn about callbacks. Each team fixed its own contract after the fact. Backed is the missing tooling: safe patterns by default, plus continuous checking.

**"Why would protocols adopt it?"**
Being checked costs them nothing: Intents and Rhea are covered today without changes. For new code, the library replaces accounting they'd otherwise hand-write, and the replays run in CI. AI coding agents use it by default through the skill.

**"NEAR already fixed the Intents bug."**
They fixed one instance (removed deposit notifications, added callback gas). The family remains: any protocol with an internal ledger and callbacks. NEAR's own post-incident script is our core check.

**"Does it stop every hack?"**
No. It stops the "owe more than you hold" family and makes anything else that drains backing visible. Oracle manipulation, governance attacks and key theft are out of scope.

**"Why testnet?"**
The checker contract is on testnet; mainnet protocols are read live. The same code deploys to mainnet with one command after a free cost preflight (docs/MAINNET.md).

**"Did you find something real?"**
Yes, and we traced it to the transaction. Two memecoins charge a 1–3% sell tax and then tell Intents the taxed part was "refunded". Intents credits that refund although nothing came back, so every taxed sell through Intents grows the gap by exactly the tax (tx `hmKEgaX2…`: 37,793.33 NINU sold, 1,133.80 credited from nothing). The amounts are small, but the mechanism is Rhea's: a ledger crediting what it was told, not what arrived. Details: docs/FINDINGS.md.
