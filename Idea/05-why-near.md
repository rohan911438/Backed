# 05 — Why NEAR

## 1. The problem only exists here (in this form)
- On Ethereum, a transaction is atomic: if any step fails, everything reverts. On NEAR, each cross-contract step is a separate receipt and **nothing reverts automatically**. Every protocol hand-writes compensation logic.
- NEAR protocols keep **internal ledgers** to work around async (Intents' NEP-245 multi-token ledger, Rhea's deposits and shares). Internal ledgers are exactly what can drift from real holdings.
- Result: all three 2026 incidents (Intents $3.8M, Rhea $18.4M, SWEAT/HOT zero-day) are this family. **A tool built for Ethereum can't catch them; a tool built for NEAR can.**

## 2. NEAR makes the check trustless
- Any contract can read any other contract's views and any token's `ft_balance_of` through cross-contract calls. The checker computes owed vs held **on-chain**, with no reporter, signature committee or oracle.
- RPC reads at a fixed `block_id` give exact off-chain snapshots for free.
- Proof-of-reserve products elsewhere need trusted reporters because reserves live off-chain. On NEAR, both sides of the equation are on-chain.

## 3. NEAR's biggest product needs it now
- NEAR Intents moves 4B+ per month in volume (Illia's figure) and holds ~16.36M wNEAR, ~4.29M USDC and more (our scan, Oct 23).
- It was exploited 3 weeks ago through exactly this bug class. NEAR's engineers then wrote a manual script for this exact check.
- Illia: "At this scale, we have to hold ourselves to a higher security standard." Backed is that standard, packaged.

## 4. It fits NEAR's direction
- **AI agents:** NEAR's strategy is to be the place agents transact. Agents need a machine-readable "is this protocol safe to deposit into?" signal, and AI coding agents need safe defaults when writing NEAR contracts.
- **Formal verification:** NEAR announced formal verification for its release process. Backed complements it: verification proves code matches a spec; Backed enforces the money invariant at runtime and tests NEAR's runtime failure modes.

## 5. Built with NEAR's own tools
- near-sdk 5.x (Intents uses 5.29.0), NEP-141/145/245/297 standards, near-sandbox for chaos tests, near-api (Rust) for the CLI, GitHub Pages plus public RPC for the web page.
- Real adapters verified on mainnet: `intents.near` (`mt_supply`, `mt_tokens`), `contract.main.burrow.near` (`get_asset`), `aggregate-oracle.near` (`list_prices`).
