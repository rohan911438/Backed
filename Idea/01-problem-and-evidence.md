# 01 — Problem and Evidence

## The bug family: "ledger ≠ holdings"

A NEAR DeFi contract keeps an **internal ledger**: who owns how much of each token inside it (Intents' multi-token balances, Rhea's deposits and lending shares). The **real holdings** are the tokens the contract actually owns on each token contract (`ft_balance_of(contract)`).

Safety rule: **total owed ≤ total held**, for every token, at all times.

On NEAR, calls between contracts are **asynchronous**: each step is a separate receipt, possibly on another shard, in a later block. If a step fails, earlier steps are **not** rolled back; developers must write compensation logic by hand (NEAR docs: "the calling function has already completed… failures have to be handled explicitly in the callback"). Every missed or failing compensation path is a way for ledger and holdings to drift apart.

On Ethereum a transaction is all-or-nothing, so this family barely exists there. **It is NEAR's own problem.**

## Three incidents in 2026

### 1. NEAR Intents — $3.8M (2026-10-01)
- **What happened:** deposits arrived with a "notification" to a receiver. The receiver asked for refunds larger than the deposits. The refund event log grew past NEAR's `TOTAL_LOG_LENGTH_LIMIT`, so **the whole `mt_resolve_deposit` callback failed**. The token contract then returned the tokens, but Intents had already recorded the balance, leaving **balances not backed by tokens**. The attacker repeated this with 23 failed deposits from Sep 29 to Oct 1, then withdrew.
- **Source code evidence (github.com/near/intents):** regression test `mt_deposit_resolve_gas.rs`, commit `0c1a530c` "chore: reproduce bug" (Oct 1), comment: *"a receiver with a large balance could request refunds bigger than the deposited amounts… the refund `mt_burn` event grew past `TOTAL_LOG_LENGTH_LIMIT` and the whole `mt_resolve_deposit` callback failed (as happened to the deployed revision)."*
- **Fix:** commit `16d04f6e` "fix(verifier): remove notifications on deposits" (Oct 7), plus "add more gas to callbacks".
- **Detection:** NEAR's internal AI monitor "SHIELD" spotted abnormal activity; service paused; funds recovered within ~36 hours; users compensated.
- **NEAR's own check:** the same commit added `scripts/list_nep245_tokens.sh`, which compares `mt_supply` against intents.near's real balance on each token contract. **That is Backed's core check, done by hand after the fact.**
- Illia Polosukhin: *"At this scale, we have to hold ourselves to a higher security standard."* NEAR announced formal verification for the contract release process.

### 2. Rhea Finance — $18.4M (2026-04-16)
- **What happened:** margin trading summed `min_amount_out` across every swap step (`burrowland/contracts/contract/src/margin_trading.rs:111`, verified in source), and the callback **credited what the route claimed, not what was delivered**. The attacker routed through ~123 fake tokens and ~25 fake pools built over 2 days; positions were credited ~4 million times their real value, then liquidated, draining the reserve.
- **After:** margin trading permanently shut down; lending relaunched with monitoring bots and an audit. Margin debt on mainnet is 0 today (verified).

### 3. SWEAT and HOT tokens — zero-day (found by Fireblocks)
- **What happened:** `ft_resolve_transfer` (the refund callback) lacked `#[private]`, so anyone could call it with fake "nothing was used" results and get refunds they weren't owed. One victim lost 8.5M SWEAT.
- NEAR docs now warn: make the resolver private, cap refunds by the original amount and current balance, and prefer the standard implementations.

### The common thread
| Incident | Ledger said | Reality |
|---|---|---|
| Intents | user has X | tokens were refunded, contract holds less |
| Rhea | position worth Y | protocol received ~0.00002% of Y |
| SWEAT/HOT | refund owed | nothing was owed |

All three: **an async step's real outcome never reached the ledger correctly.**

## Our live mainnet evidence (2026-10-23)

Backing check = `held` (token contract's `ft_balance_of(protocol)`) vs `owed` (the protocol's own liability views), read **at the same block height**.

**NEAR Intents (`intents.near`, v0.4.5), snapshot block 221,249,220**
- 87,327 token ids registered: 370 NEP-141, 339 NEP-245, 86,618 NEP-171 (NFTs).
- NEP-141 checked: **342 fully backed** (44 with a small surplus), **2 under-backed**, 26 not checkable (23 because of a discontinued RPC endpoint, 3 token-side errors).
- Under-backed: `ninu-4.nearlytrade.near` (NEAR INU) short 1.19e21 base units; `batman-4.nearlytrade.near` (BATMAN) short 2.56e20 base units. Both are launchpad memecoins whose total supply shrinks (burns). The shortfall is consistent with token mechanics Intents' ledger doesn't see (burns or transfer fees): **the same "credited ≠ received" class, small scale.** Not claimed as an exploit; flagged for investigation.
- Major tokens: wNEAR owes 16,360,481 / holds 16,360,501; USDC 4,286,910 / 4,291,913; USDT 513,080 / 513,100; ZEC 28,659.99 / 28,660.05; nBTC 141.52 / 141.52.

**Rhea lending (`contract.main.burrow.near`)**: owed = supplied + reserved + protocol fees − borrowed (adjusted for `extra_decimals`).
- wNEAR 1.0020, USDT 1.0112, USDC 1.0114, ETH 1.6223 (untracked surplus), nBTC 1.0004, ZEC 1.0001: all backed.

**Conclusion:** the check works trustlessly against NEAR's largest money pools today, and it already surfaces real, unexplained mismatches.

## Why the AI era makes it urgent
- Anthropic's SCONE-bench research (late 2025): AI agents produced working exploits worth ~$4.6M on contracts released after March 2025; reported scan cost ≈ $1 per contract.
- Both big 2026 NEAR attacks were **prepared over ~2 days** (failed deposits; fake pools) and executed fast. A continuous backing check catches the **setup**, not only the drain.
- AI coding agents now write a growing share of contracts, and they repeat these async patterns unless the safe pattern is the default.

## Sources
- near/intents commits `0c1a530c`, `16d04f6e`, `153cab96` (cloned 2026-10-23)
- crypto.news, "NEAR Intents resumes service after $3.8M exploit"; CoinDesk 2026-10-01; Crypto Times 2026-10-17 (postmortem report)
- Rekt, "Rhea Finance"; QuillAudits Rhea analysis; The Block Rhea post-mortem; AMBCrypto (margin discontinued)
- Fireblocks, "NEAR zero-day: SWEAT/HOT token exploit"; NEAR docs, ft_transfer_call security
- Anthropic SCONE-bench coverage (spaziocrypto, cryptometer)
- NEAR mainnet RPC reads, 2026-10-23
