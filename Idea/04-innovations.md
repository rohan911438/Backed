# 04 — What's New in the Design

Seven ideas we made, each tied to a real NEAR failure.

### 1. Two-tier solvency: an instant check and a truth check
- **Tier 1 (synchronous):** a shadow ledger (`shadow_held`) updated only by verified flows, with `owed ≤ shadow_held` asserted after every state change in constant time. It catches **logic bugs** (Rhea-style over-crediting) before state is saved.
- **Tier 2 (asynchronous):** `reconcile()` compares the shadow ledger with the token contract's real balance. It catches **silent outflows** the contract never learned about. That's exactly the Intents failure: tokens refunded after a failed resolve, invisible to the contract's own logic.
- Most solvency tools do only one tier, usually off-chain monitoring.

### 2. Race-free reconcile with monotone counters
Async reads normally create false alarms: a transfer in flight looks like missing money. Backed orders every flow so timing errors only go in the safe direction (debit before sending, credit after receiving), and records `out_started` at the start. The check `real + (out_started_now − O0) ≥ S0` **cannot fire from in-flight transfers**, only from tokens leaving without the ledger knowing. That gives a deterministic on-chain alarm with no persistence window for Backed contracts.

### 3. "Delivered" vs "Claimed" amounts in the type system
Rhea credited a number the caller's route claimed. In Backed, a balance can only grow from a `Delivered` value, which can only be built inside verified receivers and private callbacks. There's no conversion from `Claimed`, so **the Rhea bug becomes a compile error**, not a review item.

### 4. Infallible callbacks
On NEAR, a failed receiver callback means "refund everything" to the token contract, while the receiver may already have credited the user. Backed treats the resolve as code that **must not fail** and bounds every resource it touches: refund caps, log bytes measured on capped values under the 16,384-byte limit, reserved gas, prepaid storage, no panics. The Intents incident was a resolve failing on a protocol limit; this makes that class impossible.

### 5. Chaos testing for NEAR's own failure modes
General fuzzers mutate inputs. `backed-chaos` injects **NEAR-specific runtime failures** (callback out-of-gas, log-limit overflow, absurd refund values, direct callback calls, lying venues, fee-on-transfer and burning tokens) and checks the backing invariant **after every receipt**. Each of the three 2026 incidents maps to an injection.

### 6. Two ways to read: exact off-chain, trustless on-chain
- **Off-chain:** RPC reads at one `block_id` give an exact, race-free snapshot for free. That's how we scanned all 370 NEP-141 tokens in Intents.
- **On-chain:** cross-contract reads are trustless but land in different blocks, so the checker uses a two-attestation persistence rule (`Suspected` → `Confirmed` after ≥ N blocks).
- **Lazy attestation joins them.** The free exact scanner watches; the on-chain checker is called only when there's something to prove. You get continuous coverage with on-chain proof, at near-zero cost.

### 7. A solvency standard any contract can adopt
`backed_solvency(token) → { owed, shadow_held, status }` (draft NEP). Any protocol that exposes it is checkable by anyone with no custom adapter, so "fully backed" becomes a property you can query across the ecosystem, like `ft_balance_of`.

## Not claimed as new
- Comparing liabilities to holdings (proof of reserves) is an old idea. What's new is applying it to NEAR's async internal ledgers, with prevention, race-free reconcile and runtime failure injection.
- Refund caps and private resolvers are known NEAR best practice. What's new is packaging them as default building blocks that a chaos suite verifies, instead of a checklist teams forget.
