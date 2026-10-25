# Findings: why 3 tokens in NEAR Intents are under-backed

Backed's scan of `intents.near` finds 3 of 370 NEP-141 tokens where Intents' ledger (`mt_supply`) is higher than the tokens it holds (`ft_balance_of`). We traced each shortfall to the exact transaction that created it. All three are the bug family Backed exists for: **the ledger credited an amount a token *reported*, not an amount that *arrived*.**

Investigated 2026-10-25 using archival NEAR RPC. Amounts are in each token's base units.

| Token | Shortfall now | Cause | First seen |
|---|---|---|---|
| `ninu-4.nearlytrade.near` (NINU) | 1,192.71 NINU | 3% sell tax, refunded by Intents as "unused" | by block 218,583,333 |
| `batman-4.nearlytrade.near` (BATMAN) | 255.67 BATMAN | 1% sell tax, same mechanism | block 220,025,605 |
| `nvdastest.optimal-bone.near` (tNVDAs) | 0.1 tNVDAs | token reported a deposit that never moved tokens | block 186,091,853 |

## 1. NINU and BATMAN: a transfer tax that Intents refunds

Both tokens run the same contract (code hash `1uGuBEpx3dFRDrr2wNzm5Vcb5sF3jWY3AKQ3Gopd6we`). `get_tax` shows a tax on transfers to or from the Ref pool `dclv2.ref-labs.near`: NINU 2% buy / 3% sell, BATMAN 1% / 1%.

**Transaction [`hmKEgaX2…`](https://nearblocks.io/txns/hmKEgaX2gX1pinNzAPisYny8Xk1WFg2Fz1KxMGcUSYv) (block 220,825,682), a user selling NINU through Intents:**

1. Intents withdraws **37,793.33 NINU** for the user and calls `ft_transfer_call` to the Ref pool (`mt_burn`, memo `withdraw`).
2. The NINU contract moves **36,659.53** to the pool and **1,133.80** to itself, memo `"tax"`. Both come out of Intents' balance.
3. NINU's `ft_resolve_transfer` returns **36,659.53** as the amount used. Under NEP-141, that tells the sender the remaining 1,133.80 was refunded. It wasn't: the tax kept it.
4. Intents' `ft_resolve_withdraw` trusts that number and credits the user **1,133.80 NINU** (`mt_mint`, memo `refund`).
5. Result: Intents' ledger +1,133.80, Intents' holdings +0. The shortfall jumps by **exactly 3% of the withdrawal**: `37,793,333,685,746,210,000,000 × 3% = 1,133,800,010,572,386,300,000`.

BATMAN, transaction [`C5gEpbqL…`](https://nearblocks.io/txns/C5gEpbqLRmFCFjhW5C5sgBiyZRXzPjsbhYsTMJduF6tV) (block 220,025,605): a 2,117 BATMAN sell, a 1% tax of 21.17, and the shortfall rises from 0 to exactly 21.17 BATMAN.

**Effect:** whoever sells these tokens through Intents gets the sell tax credited back, and Intents' other holders of the token cover it. Each sell adds to the gap. The amounts are small, but the gap only grows.

**Whose bug?** The token breaks NEP-141: its resolve reports a refund it doesn't make. Intents takes the reported number as fact. Backed's rule R1, *credit only what you can verify was delivered*, would have stopped it: a ledger that credits the measured balance change (or compares against `ft_balance_of`) can't be told about a refund that never happened.

## 2. tNVDAs: a deposit that never arrived

Transaction [`1QPqfjj6…`](https://nearblocks.io/txns/1QPqfjj6JY2MB34MopbYRahMgm74LpknNq1kuX32Jcx) (block 186,091,853): the token contract `nvdastest.optimal-bone.near`, signed by its owner, calls `intents.near.ft_on_transfer` saying **0.1 tNVDAs** arrived for `ariete.near`. Intents credits it; its `ft_balance_of` doesn't change. The gap has been exactly 0.1 tNVDAs (10,000,000 base units) ever since.

`ft_on_transfer` is only a token's *claim* that tokens arrived. Intents accepts any token, so a token that claims a delivery without moving anything creates an unbacked balance. This one looks like a test token, and the amount is tiny.

## Reproduce

Each change is visible by reading both numbers one block apart:

```bash
backed --rpc https://archival-rpc.mainnet.near.org --block 220825681 check intents.near --token ninu-4.nearlytrade.near
backed --rpc https://archival-rpc.mainnet.near.org --block 220825682 check intents.near --token ninu-4.nearlytrade.near
```

To find the cause of a change at block `B`: `EXPERIMENTAL_changes` for `intents.near` at `B` gives the receipt; `EXPERIMENTAL_receipt` shows the call (`ft_resolve_withdraw` / `ft_on_transfer`).

## What it means

- None of this is a large loss, and NINU/BATMAN are launchpad memecoins. But it's the same mechanism as Rhea's $18.4M: **a callback trusted a number someone else reported.** It happens on mainnet today, in NEAR's largest intents protocol, through ordinary trading.
- It shows why the check has to be continuous: these gaps grow with every taxed sell and would be invisible without comparing books to holdings.
- Fixes on the Intents side: check `ft_balance_of` before and after a withdrawal with `ft_transfer_call` (or reject fee-on-transfer tokens), and don't credit `ft_on_transfer` amounts that can't be verified.

We'll share this with the NEAR Intents team before the repository goes public.
