import { useCallback, useEffect, useState } from "react";
import { CHECKER, CHECKER_TARGETS, explorerAccount, explorerTx } from "../config";
import { checkToken, coverage, finalBlock, formatUnits, nearBalance, tokenMeta, viewJson } from "../near";
import { takeReturnedTxHashes } from "../router";
import { useWallet } from "../wallet";

type State = "Healthy" | { Suspected: { first_block: number } } | { Confirmed: { first_block: number } } | string | Record<string, unknown>;
type PairStatus = { state: State; last_block: number; owed: string; held: string };
type Breach = { target: string; token: string; block: number; owed: string; held: string; confirmed: boolean };
type Row = { target: string; token: string; symbol?: string; decimals?: number; status: PairStatus | null; error?: string };

const NEW_PAIR_DEPOSIT = 5n * 10n ** 21n; // 0.005 NEAR storage deposit; unused part refunded

function stateName(s: State): "Healthy" | "Suspected" | "Confirmed" | "Unknown" {
  if (s === "Healthy") return "Healthy";
  if (typeof s === "object" && s) {
    if ("Suspected" in s) return "Suspected";
    if ("Confirmed" in s) return "Confirmed";
  }
  return typeof s === "string" && ["Suspected", "Confirmed"].includes(s) ? (s as "Suspected" | "Confirmed") : "Unknown";
}

export default function Checker() {
  const { accountId, connect, callChecker, network } = useWallet();
  const [rows, setRows] = useState<Row[] | null>(null);
  const [breaches, setBreaches] = useState<Breach[] | null>(null);
  const [balance, setBalance] = useState<number | null>(null);
  const [target, setTarget] = useState(CHECKER_TARGETS[0].target);
  const [token, setToken] = useState(CHECKER_TARGETS[0].tokens[0]);
  const [preview, setPreview] = useState<Awaited<ReturnType<typeof checkToken>> | null>(null);
  const [busy, setBusy] = useState<"preview" | "sign" | null>(null);
  const [txs, setTxs] = useState<string[]>(() => takeReturnedTxHashes());
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const block = await finalBlock(CHECKER.network);
    const pairs = CHECKER_TARGETS.flatMap((t) => t.tokens.map((tok) => ({ target: t.target, token: tok })));
    const out = await Promise.all(
      pairs.map(async (p): Promise<Row> => {
        const meta = await tokenMeta(p.token, block, CHECKER.network);
        try {
          const status = await viewJson<PairStatus | null>(CHECKER.network, CHECKER.account, "get_status", p);
          return { ...p, ...meta, status };
        } catch (e: any) {
          return { ...p, ...meta, status: null, error: String(e?.message ?? e) };
        }
      }),
    );
    setRows(out);
    setBreaches(await viewJson<Breach[]>(CHECKER.network, CHECKER.account, "list_breaches", { from_index: 0, limit: 20 }).catch(() => []));
  }, []);

  useEffect(() => {
    void load();
  }, [load, txs]);

  useEffect(() => {
    if (accountId) nearBalance(network, accountId).then(setBalance);
    else setBalance(null);
  }, [accountId, network, txs]);

  const targetCfg = CHECKER_TARGETS.find((t) => t.target === target) ?? CHECKER_TARGETS[0];

  async function runPreview() {
    setBusy("preview");
    setError(null);
    setPreview(await checkToken(target, targetCfg.adapter, token.trim(), undefined, CHECKER.network));
    setBusy(null);
  }

  async function sign() {
    setBusy("sign");
    setError(null);
    try {
      // Always attach 0.005 NEAR: the checker keeps only the storage it uses and refunds the rest.
      const res = await callChecker("attest", { target, token: token.trim() }, NEW_PAIR_DEPOSIT, 100);
      if (res.hash) setTxs((t) => [res.hash!, ...t]);
    } catch (e: any) {
      setError(String(e?.message ?? e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="page">
      <div className="page-head">
        <p className="eyebrow">On-chain checker · {CHECKER.network}</p>
        <h1>
          Record it <em>on-chain.</em>
        </h1>
        <p className="lede">
          The checker contract reads a protocol's own accounting view and the token's <code>ft_balance_of</code>, then stores
          the result. Anyone can call it, and no one reports numbers to it. A shortfall is <b>Suspected</b> first and{" "}
          <b>Confirmed</b> only if it's still there 30+ blocks later.
        </p>
        <p className="mono muted contract-line">
          contract{" "}
          <a href={explorerAccount(CHECKER.network, CHECKER.account)} target="_blank" rel="noreferrer">
            {CHECKER.account} ↗
          </a>
        </p>
      </div>

      <div className="checker-grid">
        <div className="card attest-card">
          <h3>New attestation</h3>
          <label className="field">
            <span>Protocol</span>
            <select value={target} onChange={(e) => setTarget(e.target.value)}>
              {CHECKER_TARGETS.map((t) => (
                <option key={t.target} value={t.target}>
                  {t.label}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Token contract</span>
            <input list="tokens" value={token} onChange={(e) => setToken(e.target.value)} spellCheck={false} />
            <datalist id="tokens">
              {targetCfg.tokens.map((t) => (
                <option key={t} value={t} />
              ))}
            </datalist>
          </label>

          <div className="attest-actions">
            <button className="btn-ghost" onClick={runPreview} disabled={!!busy || !token.trim()}>
              {busy === "preview" ? "Reading…" : "Preview (free)"}
            </button>
            {accountId ? (
              <button className="btn btn-ok" onClick={sign} disabled={!!busy || !token.trim()}>
                {busy === "sign" ? "Waiting for wallet…" : "Sign & record"}
              </button>
            ) : (
              <button className="btn btn-ok" onClick={() => connect()}>
                Connect wallet to sign
              </button>
            )}
          </div>

          {preview && (
            <div className="preview">
              <div>
                <span className="muted">Owed</span>
                <b className="mono">{preview.owed ?? "—"}</b>
              </div>
              <div>
                <span className="muted">Held</span>
                <b className="mono">{preview.held ?? "—"}</b>
              </div>
              <div>
                <span className="muted">Coverage</span>
                <b className="mono">{coverage(preview.owed, preview.held)}</b>
              </div>
              <span className={`pill pill-${preview.status === "backed" ? "ok" : preview.status === "error" ? "muted" : "warn"}`}>
                {preview.status === "backed" ? "Backed" : preview.status === "error" ? preview.error : "Under-backed"}
              </span>
              <p className="muted mono small">same block · {preview.block.toLocaleString("en-US")}</p>
            </div>
          )}

          <p className="muted small">
            Costs gas (about 0.001 NEAR). Each attestation attaches 0.005 NEAR for storage; the unused part is refunded.
            {accountId && balance !== null && (
              <>
                {" "}
                Your balance: <span className="mono">{balance.toFixed(3)} NEAR</span>.
              </>
            )}
          </p>
          {error && <p className="warn small">{error}</p>}
          {txs.length > 0 && (
            <div className="tx-list">
              <p className="eyebrow">Your transactions</p>
              {txs.map((h) => (
                <a key={h} className="mono" href={explorerTx(CHECKER.network, h)} target="_blank" rel="noreferrer">
                  {h.slice(0, 10)}…{h.slice(-6)} ↗
                </a>
              ))}
            </div>
          )}
        </div>

        <div className="card table-card">
          <div className="table-card-head">
            <h3>Recorded on-chain</h3>
            <button className="btn-ghost" onClick={() => void load()}>
              Refresh
            </button>
          </div>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Token</th>
                  <th className="num">Owed</th>
                  <th className="num">Held</th>
                  <th>State</th>
                  <th className="num">Block</th>
                </tr>
              </thead>
              <tbody>
                {!rows && (
                  <tr>
                    <td colSpan={5} className="empty">
                      Reading the checker on {CHECKER.network}…
                    </td>
                  </tr>
                )}
                {rows?.map((r) => {
                  const st = r.status ? stateName(r.status.state) : null;
                  return (
                    <tr key={r.target + r.token} className={st && st !== "Healthy" ? "row-warn" : ""}>
                      <td>
                        <div className="tok">
                          <span className="tok-sym">{r.symbol ?? "?"}</span>
                          <span className="tok-id mono">{r.token}</span>
                        </div>
                      </td>
                      <td className="num mono">{r.status ? formatUnits(r.status.owed, r.decimals) : "—"}</td>
                      <td className="num mono">{r.status ? formatUnits(r.status.held, r.decimals) : "—"}</td>
                      <td>
                        {st ? (
                          <span className={`pill pill-${st === "Healthy" ? "ok" : "warn"}`}>{st}</span>
                        ) : (
                          <span className="pill pill-muted">{r.error ? "Unreadable" : "Not attested yet"}</span>
                        )}
                      </td>
                      <td className="num mono muted">{r.status ? r.status.last_block.toLocaleString("en-US") : "—"}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          <div className="breaches">
            <p className="eyebrow">Breach log</p>
            {breaches === null ? (
              <p className="muted small">Loading…</p>
            ) : breaches.length === 0 ? (
              <p className="muted small">No breaches recorded. Every attestation so far found the protocol fully backed.</p>
            ) : (
              <ul>
                {breaches.map((b, i) => (
                  <li key={i} className="mono small">
                    {b.confirmed ? "CONFIRMED" : "suspected"} · {b.token} · block {b.block} · owed {b.owed} / held {b.held}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
