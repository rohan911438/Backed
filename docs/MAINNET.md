# Going to mainnet

Everything below runs in WSL (or Linux/macOS) from the repo root. Nothing here asks for a seed phrase: the key is read from `~/.near-credentials/mainnet/<account>.json`.

## Cost

| Item | NEAR |
|---|---|
| Checker code storage (161,003 bytes at 1 NEAR / 100 KB) | ~1.61 |
| Headroom: state, two adapters, gas | ~0.30 |
| **Minimum** | **~1.91** |
| Recommended balance (buffer for attestations) | 2.5 |

Each attestation costs about 0.001 NEAR of gas. The 0.005 NEAR deposit it attaches is refunded except for the storage it adds (about 0.003 NEAR the first time a protocol/token pair is recorded, nothing after).

## Steps

```bash
export CARGO_TARGET_DIR=$HOME/backed-target
LIVE="cargo run -q -p backed-chaos --features sandbox --example live --"
ACCOUNT=luckycity5453.near

# 1. Build the contract
bash scripts/build-wasm.sh

# 2. Free check: wasm size, cost, balance. Stops if the account can't afford it.
$LIVE mainnet $ACCOUNT preflight

# 3. Deploy, init (owner = $ACCOUNT), set the Intents and Rhea adapters
$LIVE mainnet $ACCOUNT deploy

# 4. First real attestations
$LIVE mainnet $ACCOUNT attest intents.near usdt.tether-token.near
$LIVE mainnet $ACCOUNT attest intents.near wrap.near
$LIVE mainnet $ACCOUNT attest contract.main.burrow.near wrap.near

# 5. Read back (free)
$LIVE mainnet $ACCOUNT status intents.near usdt.tether-token.near
```

Then point the web app at mainnet and redeploy it (from Windows PowerShell, where Vercel is logged in):

```powershell
cd web
$env:VITE_NETWORK="mainnet"; $env:VITE_CHECKER_ACCOUNT="luckycity5453.near"; npm run build
npx vercel deploy --prod
```

`VITE_NETWORK` must also be set in the Vercel project's environment variables if Vercel builds the app itself.

## Later

- **Upgrades:** `$LIVE mainnet $ACCOUNT upgrade` replaces the code and keeps all recorded attestations. Run the tests and the sandbox replays first.
- **Ownership:** call `set_owner` (with 1 yoctoNEAR) to hand adapter management to a multisig or DAO.
- **More protocols:** the owner calls `set_adapter(target, kind)`, or the protocol implements [`backed_solvency`](NEP-backed-solvency.md) and calls `register_self` itself.

## What was checked before mainnet

- Every `attest` pays for its own storage and gets the rest refunded, so repeated calls can't drain the checker's balance. Tested on testnet: [`Gc4SYcQr…`](https://testnet.nearblocks.io/txns/Gc4SYcQrXbpp3WSYp5mnjrxKVoX1Gzic38iFW6wf63Kf) refunded the full 0.005 NEAR.
- A breach is logged only when a pair changes state (Healthy → Suspected → Confirmed), so repeated calls can't flood the breach log. Unit test: `repeated_attests_log_a_breach_once_per_state_change`.
- View results are read with a size cap, so a hostile target can't make the callback run out of gas.
- `set_adapter` and `set_owner` are owner-only and need 1 yoctoNEAR (a full-access key signature).
