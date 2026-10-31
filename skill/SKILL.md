---
name: backed
description: Use when writing, reviewing or changing a NEAR smart contract (Rust, near-sdk) that holds or moves fungible tokens (ft_on_transfer, ft_transfer_call, withdraw, swap, lending, vaults, bridges, intents), or before depositing into a NEAR protocol. Prevents the async "ledger owes more than it holds" bugs behind the 2026 NEAR Intents, Rhea Finance and SWEAT/HOT losses.
---

# Backed: NEAR contracts that never owe more than they hold

NEAR cross-contract calls are asynchronous and nothing rolls back by itself.
A contract's books (what it **owes**) can drift away from what the token
contract says it **holds**. Three 2026 incidents came from that drift:

| Incident | Bug | Rule that prevents it |
|---|---|---|
| NEAR Intents, 2026-10-01, $3.8M | Resolve callback panicked (uncapped refund, log limit), token refunded the deposit, credit stayed | R3: resolves never panic |
| Rhea Finance, 2026-04-16, $18.4M | Validated positions against the swap route's *claimed* output, then accepted far less delivered | R1: credit and check only delivered amounts |
| SWEAT/HOT, Apr 2026 (Fireblocks) | `ft_resolve_transfer` lacked `#[private]`: anyone could call it for refunds nobody was owed | R2: callbacks are `#[private]` |

## The rules

**R1. Credit only what was delivered.** A balance may only grow from an
amount the token contract vouched for (`ft_on_transfer`, with the caller
checked against a token allowlist) or from the result of a call your contract
made itself. Never from function arguments, `msg` fields, route minimums or a
venue's return value.

**R2. Every callback is `#[private]`.** Any method that reads
`env::promise_result*` or `is_promise_success()` must be `#[private]`.

**R3. Resolve callbacks never panic.** Cap refunds (`min(requested,
deposited, balance)`), bound promise-result reads (`promise_result_checked(i,
max_len)`), keep logs under 16,384 bytes and 100 entries, and check at entry
that enough gas is attached for the callback.

**R4. Debit before tokens leave, credit after they arrive.** Then any timing
gap only makes real holdings look higher than the books, never lower.

**R5. Reject unknown tokens.** In `ft_on_transfer`, if the caller isn't a
registered token, return the full amount as unused.

**R6. Expose `backed_solvency(token)`** so anyone, including the on-chain
checker, can verify `owed <= held`.

## Use the `backed` crate instead of hand-writing this

```toml
[dependencies]
backed = { git = "https://github.com/rohan911438/Backed" }
```

```rust
use backed::{Claimed, Delivered, Ledger, PendingIn, PendingOut};

// ft_on_transfer
let Some(delivered) = Delivered::from_token_transfer(&token, amount.0) else {
    return PromiseOrValue::Value(amount); // not a registered token: refund all
};
self.ledger.credit(&token, &sender_id, delivered);

// deposit with a follow-up call (notify, forward)
let pending: PendingIn = self.ledger.credit_with_followup(&token, &sender_id, delivered);
// ...then in the #[private] resolve:
let unused = self.ledger.finalize_followup(&pending, requested_refund); // capped, never panics

// withdraw: debit first, settle in a #[private] callback
let pending: PendingOut = self.ledger.debit_for_withdraw(&token, &owner, Claimed(amount.0));
// ...callback: self.ledger.settle_withdraw(&pending, is_promise_success());

// view for checkers
pub fn backed_solvency(&self, token: AccountId) -> Option<backed::Solvency> { self.ledger.solvency(&token) }
```

`Claimed` (anything a caller says) has no conversion into `Delivered`, so the
compiler rejects R1 violations. A complete example: `contracts/examples/backed-vault`.
For swaps, have the venue send output with `ft_transfer_call` and credit it in
`ft_on_transfer`; ignore what the venue reports.

## Review checklist (run on every change)

- [ ] Every `+=` / insert on a user balance traces back to a `Delivered` (R1)
- [ ] `grep -n "promise_result\|is_promise_success"`: each hit is in a `#[private]` method (R2)
- [ ] No `-`, `unwrap`, `expect` or unbounded loop/log in a resolve callback (R3)
- [ ] Balance is debited before the outgoing `ft_transfer` promise is created (R4)
- [ ] `ft_on_transfer` checks the predecessor against an allowlist (R5)

## Prove it: chaos replays

Linux, macOS or WSL:

```bash
scripts/build-wasm.sh
cargo test -p backed-chaos --features sandbox replay_   # or: backed chaos
```

This deploys real contracts into a local NEAR sandbox and runs the three
incident attacks. Each step checks `owed <= held`. To test your own contract,
add it as a `VaultKind` in `crates/backed-chaos/src/sandbox.rs` with the same
`ft_on_transfer` / `withdraw` / `balance_of` interface.

## Before depositing into a NEAR protocol

```bash
backed check intents.near --token usdt.tether-token.near
backed check contract.main.burrow.near --token wrap.near
backed scan-intents
```

If the `backed` MCP server is connected (`backed mcp`), call its
`check_backing` tool instead of the shell command.

`owed` and `held` are read at the same block height, so the result is exact.
`UNDER-BACKED` means the protocol owes more of that token than it holds. Report
it as a finding to the protocol, and don't deposit that token.
