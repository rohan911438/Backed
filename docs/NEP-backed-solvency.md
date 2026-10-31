---
NEP: TBD
Title: Solvency view for token-holding contracts
Authors: Backed (https://github.com/rohan911438/Backed)
Status: Draft
DiscussionsTo: TBD
Type: Standards Track
Category: Contract
Version: 0.1.0
Created: 2026-10-25
Requires: 141
---

## Summary

A single view method, `backed_solvency(token)`, through which any contract that holds NEP-141 tokens on behalf of users reports how much of a token it **owes** according to its own books. Anyone can compare that number with the token contract's `ft_balance_of(contract)` at the same block and see whether the contract holds what it owes.

## Motivation

On NEAR, a contract's internal ledger and the tokens it actually holds are updated by different receipts. When a callback fails, is skipped or trusts a number it shouldn't, the two drift apart, and nothing rolls back. In 2026 this drift caused three incidents:

- **NEAR Intents (2026-10-01, $3.8M):** the refund log in `mt_resolve_deposit` grew past the log size limit and the callback failed. The token refunded the deposits, but the balances stayed recorded ([regression test](https://github.com/near/intents/commit/0c1a530c8da337008e28baa9987a816249d707e6)).
- **Rhea Finance (2026-04-16, $18.4M):** margin trading validated positions against a claimed swap output and accepted a delivered amount about 4 million times smaller ([Rekt](https://rekt.news/rhea-finance-rekt)).
- **SWEAT/HOT (April 2026):** a public `ft_resolve_transfer` let anyone claim refunds nobody was owed ([Fireblocks](https://www.fireblocks.com/blog/near-zero-day-sweat-hot-token-exploit)).

After the Intents incident, NEAR's engineers compared `mt_supply` against `intents.near`'s real balance on every token with a hand-written script ([`list_nep245_tokens.sh`](https://github.com/near/intents/blob/0c1a530c8da337008e28baa9987a816249d707e6/scripts/list_nep245_tokens.sh)). Today every protocol exposes its books differently (`mt_supply`, `get_asset`, custom views), so each one needs its own adapter. A standard view lets wallets, agents, monitors and other contracts check any compliant protocol the same way, before depositing and continuously afterwards.

## Specification

### Method

```ts
// View. MUST NOT panic for a token the contract has never held: return null.
function backed_solvency(token: AccountId): Solvency | null;

interface Solvency {
  // Total the contract owes users in `token`, in the token's base units,
  // as recorded by its own books at this block.
  owed: string;          // U128 as a decimal string

  // The contract's own running estimate of what it holds. Informational;
  // checkers MUST use the token's ft_balance_of instead.
  shadow_held: string;   // U128 as a decimal string

  // The contract's own reconcile result.
  status: "Healthy" | { Breach: { since_block: number; shortfall: string } };
}
```

### Rules

1. `owed` MUST include every balance a user could withdraw. A withdrawal MUST be removed from `owed` when it is debited, **before** the outgoing transfer is sent, and added back only if that transfer fails. While a transfer is in flight the tokens are still held, so `owed` never exceeds `held` because of it, and a correct contract never shows a false shortfall.
2. `owed` MUST NOT include amounts that were only *claimed* (returned by another contract, passed in arguments) and not yet *delivered* to this contract by the token.
3. The method MUST be a pure view: no state changes, bounded gas (under 10 Tgas for any `token`).
4. A contract is **backed** for `token` at block `b` when `ft_balance_of(contract)` at `b` is at least `owed` at `b`.
5. A contract that implements this view MAY register with any checker that supports it, so no protocol-specific adapter is needed.

### Reading it correctly

`owed` and `ft_balance_of` MUST be read **at the same block height** (`block_id` in the RPC `query` call). Readings from different blocks can be skewed by transfers in flight and produce false alarms. On-chain checkers that can't pin a block SHOULD treat one shortfall as *suspected* and confirm it only with a second reading some blocks later. The Backed checker waits 30 blocks.

### Events

When a contract's own reconcile finds a shortfall, it SHOULD emit:

```
EVENT_JSON:{"standard":"backed_solvency","version":"0.1.0","event":"breach","data":[{"token":"<token>","owed":"<u128>","held":"<u128>","block":<u64>}]}
```

## Reference implementation

- Library: [`crates/backed`](../crates/backed): `Ledger::solvency(token)` returns this struct; `debit_for_withdraw` and `settle_withdraw` follow rule 1.
- Example contract: [`contracts/examples/backed-vault`](../contracts/examples/backed-vault/src/lib.rs).
- Checker: [`contracts/checker`](../contracts/checker), adapter `BackedStandard`, plus `register_self()`.

## Security considerations

- The view is self-reported. A malicious or buggy contract can under-report `owed` and so hide a shortfall. Checkers SHOULD cross-check large protocols with their protocol-specific views (as the Backed checker does for Intents and Rhea), and treat `backed_solvency` as a lower bound on what is owed. The standard makes honest protocols cheap to verify; it doesn't make dishonest ones honest.
- Tokens whose balances change without a transfer (rebasing, burns from holders, fee-on-transfer) can produce shortfalls that aren't exploits. Readers SHOULD report these as findings to investigate.
- The view MUST NOT be used as a price or collateral oracle.

## Alternatives

- **Protocol-specific adapters only.** Works today (the Backed checker reads Intents' `mt_supply` and Rhea's `get_asset`), but each new protocol needs code in every checker.
- **Proof-of-reserves off chain.** Needs a trusted reporter. Reading the token contract at a block needs none.

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).
