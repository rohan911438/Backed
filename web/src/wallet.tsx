// NEAR wallet connection (wallet-selector core + our own picker, no extra React copy).

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { ModuleState, WalletSelector } from "@near-wallet-selector/core";
import { CHECKER, WALLET_NETWORK } from "./config";

type CallResult = { hash?: string; redirected?: boolean };

interface WalletApi {
  ready: boolean;
  accountId: string | null;
  network: typeof WALLET_NETWORK;
  /** Opens the wallet picker. After sign-in, `then` runs (e.g. go to the app). */
  connect: (then?: () => void) => void;
  disconnect: () => Promise<void>;
  /** Calls a method on the checker contract, signed by the connected wallet. */
  callChecker: (method: string, args: object, depositYocto: bigint, gasTgas: number) => Promise<CallResult>;
}

const Ctx = createContext<WalletApi | null>(null);

export function useWallet(): WalletApi {
  const v = useContext(Ctx);
  if (!v) throw new Error("useWallet outside WalletProvider");
  return v;
}

function appUrl(path: string) {
  return `${window.location.origin}${window.location.pathname}${path}`;
}

export function WalletProvider({ children }: { children: ReactNode }) {
  const [selector, setSelector] = useState<WalletSelector | null>(null);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [modal, setModal] = useState(false);
  const afterConnect = useRef<(() => void) | undefined>(undefined);

  useEffect(() => {
    let sub: { unsubscribe(): void } | undefined;
    // Loaded after first paint: the wallet libraries are most of the bundle.
    import("buffer")
      .then(({ Buffer }) => {
        const g = globalThis as any;
        g.Buffer ??= Buffer;
        // Some wallet SDKs read Node's `process` at import time.
        g.process ??= { env: {}, browser: true, version: "", versions: {}, nextTick: (f: () => void) => setTimeout(f, 0) };
      })
      .then(() => Promise.all([
      import("@near-wallet-selector/core"),
      import("@near-wallet-selector/my-near-wallet"),
      import("@near-wallet-selector/meteor-wallet"),
    ]))
      .then(([core, mnw, meteor]) =>
        core.setupWalletSelector({ network: WALLET_NETWORK, modules: [mnw.setupMyNearWallet(), meteor.setupMeteorWallet()] }),
      )
      .then((s) => {
        setSelector(s);
        const pick = () => s.store.getState().accounts.find((a) => a.active)?.accountId ?? null;
        setAccountId(pick());
        sub = s.store.observable.subscribe(() => setAccountId(pick()));
      })
      .catch((e) => console.error("wallet selector failed to start", e));
    return () => sub?.unsubscribe();
  }, []);

  const connect = useCallback((then?: () => void) => {
    afterConnect.current = then;
    setModal(true);
  }, []);

  const disconnect = useCallback(async () => {
    if (!selector) return;
    try {
      const w = await selector.wallet();
      await w.signOut();
    } catch {
      /* already signed out */
    }
  }, [selector]);

  const callChecker = useCallback<WalletApi["callChecker"]>(
    async (method, args, depositYocto, gasTgas) => {
      if (!selector) throw new Error("Wallet is still loading");
      const w = await selector.wallet();
      const { actionCreators } = await import("@near-wallet-selector/core");
      const params = {
        receiverId: CHECKER.account,
        actions: [actionCreators.functionCall(method, args, BigInt(gasTgas) * 10n ** 12n, depositYocto)],
        callbackUrl: appUrl("#/app/checker"),
      };
      const out = await w.signAndSendTransaction(params);
      if (!out) return { redirected: true };
      return { hash: out.transaction_outcome?.id ?? (out as any).transaction?.hash };
    },
    [selector],
  );

  const api = useMemo<WalletApi>(
    () => ({ ready: !!selector, accountId, network: WALLET_NETWORK, connect, disconnect, callChecker }),
    [selector, accountId, connect, disconnect, callChecker],
  );

  return (
    <Ctx.Provider value={api}>
      {children}
      {modal && (
        <WalletModal
          selector={selector}
          onClose={() => setModal(false)}
          onConnected={() => {
            setModal(false);
            afterConnect.current?.();
          }}
        />
      )}
    </Ctx.Provider>
  );
}

function WalletModal({
  selector,
  onClose,
  onConnected,
}: {
  selector: WalletSelector | null;
  onClose: () => void;
  onConnected: () => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const modules: ModuleState[] = selector?.store.getState().modules ?? [];

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  async function pick(m: ModuleState) {
    if (!selector) return;
    setBusy(m.id);
    setError(null);
    try {
      const w = await selector.wallet(m.id);
      if (w.type === "browser") {
        // Redirects to the wallet; it brings the user back to the app.
        await w.signIn({ contractId: CHECKER.account, methodNames: [], successUrl: appUrl("#/app"), failureUrl: appUrl("#/") });
      } else if (w.type === "injected" || w.type === "instant-link") {
        await w.signIn({ contractId: CHECKER.account, methodNames: [] });
      } else {
        throw new Error(`${m.metadata.name} isn't supported here yet`);
      }
      onConnected();
    } catch (e: any) {
      setError(String(e?.message ?? e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal card" role="dialog" aria-modal="true" aria-labelledby="wallet-title" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <p className="eyebrow">NEAR {WALLET_NETWORK}</p>
            <h3 id="wallet-title">Connect a wallet</h3>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>
        <p className="muted modal-note">
          {WALLET_NETWORK === "testnet" ? (
            <>
              The checker contract runs on <b>testnet</b>. Use a testnet account (free NEAR from the faucet).
            </>
          ) : (
            <>
              The checker contract runs on <b>mainnet</b>. Each attestation costs a little gas plus a 0.005 NEAR storage
              deposit, and the unused part is refunded.
            </>
          )}{" "}
          Reading data never needs a wallet; signing an attestation does.
        </p>
        {!selector && <p className="muted">Loading wallets…</p>}
        <div className="wallet-list">
          {modules.map((m) => (
            <button key={m.id} className="wallet-opt" onClick={() => pick(m)} disabled={!!busy || !m.metadata.available}>
              {m.metadata.iconUrl && <img src={m.metadata.iconUrl} alt="" width={28} height={28} />}
              <span>
                <b>{m.metadata.name}</b>
                <small className="muted">
                  {!m.metadata.available ? "Not available in this browser" : m.type === "browser" ? "Web wallet" : "Extension or app"}
                </small>
              </span>
              <span className="wallet-go">{busy === m.id ? "Opening…" : "→"}</span>
            </button>
          ))}
        </div>
        {error && <p className="warn modal-error">{error}</p>}
        <p className="muted modal-foot">
          No {WALLET_NETWORK} account?{" "}
          <a href={WALLET_NETWORK === "testnet" ? "https://testnet.mynearwallet.com" : "https://app.mynearwallet.com"} target="_blank" rel="noreferrer">
            Create one at MyNearWallet
          </a>
          .
        </p>
      </div>
    </div>
  );
}
