# 06 — Competitors and How We Differ

**Short version:** monitoring tools *alert*, auditors *review once*, and proof-of-reserve products *trust reporters*. None of them prevent NEAR's async ledger bugs in code, test NEAR's runtime failure modes, or verify NEAR protocols' backing trustlessly on-chain. Backed does all three, and only for NEAR, where the problem lives.

## Big players

| Competitor | What it does | Covers NEAR? | Why Backed is different |
|---|---|---|---|
| **NEAR's own response** (SHIELD AI monitor; Intents "deposits and withdrawals guardian", commit `153cab96`; planned formal verification) | SHIELD spots abnormal activity and pauses Intents. The guardian deduplicates bridge deposit/withdrawal ids (role-gated). | Intents only, internal | SHIELD is private and reactive, and protects one product. The guardian is replay protection, not a backing check. Backed is open, preventive (library), reusable by every NEAR protocol, and publicly verifiable. **Complement, not competitor:** it hands NEAR a standard to adopt. |
| **Chainlink Proof of Reserve** | Reports reserves for wrapped and stable assets; protocols can halt minting on low reserves | Mostly EVM | Relies on reporters for off-chain reserves. Backed reads both sides on-chain trustlessly and targets internal ledgers, not token reserves. |
| **Forta Firewall** | Pre-execution screening with an AI risk score; risky transactions get no attestation and revert | EVM rollups and protocols | Screens transactions by risk score. Backed enforces a hard money invariant and prevents the bug in code. NEAR's async model needs post-receipt reconciliation, which pre-screening a single transaction can't see. |
| **Phylax Credible Layer** | Solidity "assertions" checked by the sequencer during block building (incl. balance invariants) | EVM L2s with a sidecar (e.g. Linea) | Needs control of the sequencer; NEAR has no such hook for apps. Backed enforces invariants inside contracts and on-chain via the checker. |
| **CertiK** (audits, Skynet monitoring) | One-time audits and alerting | Yes | Audits are point-in-time (Rhea's margin code was added 2 years after its audit). Skynet alerts after the fact. Backed is continuous, preventive and open. |
| **TRM Labs** | Tracing and risk alerts for illicit flows | Yes | Follows stolen money after an attack. Backed stops the accounting bug that creates it. |
| **BlockSec Rustle** | Static analyzer for NEAR Rust contracts (e.g. reentrancy detector) | Yes | Finds suspicious code patterns statically. Backed provides safe building blocks plus **runtime** failure injection (log limits, out-of-gas, lying venues) that static analysis can't simulate. Usable together. |
| **near-contract-standards / contract-tools** | Reference token implementations (resolver, storage) | Yes | Cover the token side (e.g. a correct resolver). Backed covers the **protocol ledger side** (receivers, provisional credit, reconciliation, solvency standard), where Intents and Rhea failed. |

## Small players and prior art

| Project | What it is | Difference |
|---|---|---|
| **ProofSentinel** (Chainlink hackathon 2026) | Monitors reserve ratios; triggers pauses via a scheduled Chainlink workflow | EVM, reporter-based, monitoring only. No prevention, no chaos testing, no NEAR. |
| **ERC-7893** (draft) | Interface for DeFi protocols to publish solvency data via oracles | Ethereum draft relying on oracle feeds. Backed's `backed_solvency` standard is computed from on-chain state, and Backed contracts enforce it. |
| **Risk Harbor** (2021) | Parametric cover with automatic payouts on on-chain triggers | Insurance, not prevention. Shows trustless triggers are valued; Backed's checker could feed such products later. |
| **AI auditors** (many in 2026) | LLMs review code and find bugs | Point-in-time and probabilistic. Backed makes a bug class impossible by construction and tests it deterministically. |
| **OutLayer** | Policy wallets for AI agents (TEE + MPC) | Different problem (agent key safety). Not a competitor. |

## What competitors would need to copy us
1. Deep knowledge of NEAR's receipt model and its failure modes (log limits, gas, resolver semantics).
2. A NEAR-native library plus chaos harness plus trustless checker together.
3. Replays of all three real 2026 NEAR incidents.

For an EVM-first vendor, that's a NEAR-specific rebuild. For other hackathon teams, it's unlikely in 2 weeks unless they already know this incident history.
