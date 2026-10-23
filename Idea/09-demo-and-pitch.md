# 09 — Demo and Pitch

## Pitch lines
- **Headline:** "NEAR lost $22M+ in 2026 to one bug family that barely exists on Ethereum. Backed makes it impossible to ship."
- **Mechanism:** "On NEAR, nothing rolls back. Backed makes sure a contract never owes more than it holds."
- **Proof:** "We replayed all three 2026 incidents. Naive code loses money. Backed code doesn't."
- **Live:** "Right now, anyone can verify on-chain that NEAR Intents holds what it owes, token by token."

## 2-minute demo script

| Time | Show | Say |
|---|---|---|
| 0:00–0:15 | Three headlines: Intents $3.8M, Rhea $18.4M, SWEAT/HOT zero-day | "Three NEAR incidents this year. Same bug: the ledger said one thing, the tokens said another." |
| 0:15–0:35 | NEAR's own regression test comment (log limit → resolve failed) | "On NEAR, a callback can fail on a log-size limit, and nothing rolls back. Ethereum doesn't have this problem." |
| 0:35–1:05 | Terminal: `cargo test replay_` → naive ❌ owed > held; Backed ✅ | "Same attack, three times. Naive contracts end up owing money they don't have. Backed contracts can't." |
| 1:05–1:25 | Terminal: `backed chaos naive.wasm` finding F2/F4 by itself | "The chaos tester fails every async step on purpose and checks the books after each one." |
| 1:25–1:45 | Web page: Intents live backing, 362 tokens ✅, 3 under-backed ⚠️ | "Live on mainnet: NEAR Intents is fully backed for its major tokens. Our scan even found three small tokens it owes more of than it holds." |
| 1:45–2:00 | Explorer: a real `attest` transaction on the checker contract | "And anyone can prove it on-chain, no trusted reporter. Backed: never owe more than you hold." |

## Judge Q&A

**"Isn't this just proof of reserves?"**
Proof of reserves trusts reporters and only watches. Backed prevents the bug in code, proves it with runtime failure injection, and verifies on-chain without reporters. Watching is only the last layer.

**"Why would protocols adopt it?"**
It costs them nothing to be checked (Intents and Rhea are covered today without changes). For new code, it replaces accounting they'd otherwise hand-write and get wrong; the chaos suite runs in CI. AI coding agents use it by default through the skill.

**"NEAR already fixed the Intents bug."**
They fixed one instance (removed deposit notifications, added gas). The family remains: any protocol with an internal ledger and callbacks. NEAR's own post-incident script is our core check; we made it continuous, on-chain and preventive.

**"Does it stop every hack?"**
No. It stops the "owe more than you hold" family and makes everything else that drains backing visible fast. Oracle manipulation, governance and key theft are out of scope.

**"Where are the AI agents?"**
Three places: AI attackers make continuous checks necessary (the 2026 attacks were set up over days); AI coding agents get safe-by-default patterns through the skill; AI agents and wallets read the checker before depositing.

**"Did you find something real?"**
Yes. Three small tokens in Intents (two memecoins and a test token) are under-backed at block 221,256,203. It's likely token mechanics (burns or transfer fees) that Intents' ledger doesn't see: the same class, small scale. We report it as a finding, not an exploit.

## README structure (for AI judges)
1. What it is (2 lines). 2. The problem with sources. 3. Try it in 3 commands: `cargo test replay_`, `backed check intents.near`, open the web page. 4. Architecture diagram. 5. What it guarantees and what it doesn't. 6. Mainnet addresses. 7. Tracks and NEAR features used.
