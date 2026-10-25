import { useEffect, useMemo, useState } from "react";
import { REPO } from "./config";
import { checkToken, coverage, finalBlock, formatUnits, tokenMeta, type Adapter, type Reading } from "./near";

const RHEA_TOKENS = [
  "wrap.near",
  "usdt.tether-token.near",
  "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1",
  "eth.bridge.near",
  "nbtc.bridge.near",
  "zec.omft.near",
];
const MAJOR = ["USDC", "USDT", "wNEAR", "ETH", "nBTC", "ZEC", "WBTC"];

type Filter = "all" | "issues" | "backed";

interface Props {
  snapshot: { block: number; readings: Reading[] } | null;
}

function rank(r: Reading) {
  if (r.status === "UNDER-BACKED") return -2;
  if (r.status === "error") return 2;
  const m = MAJOR.indexOf(r.symbol ?? "");
  return m >= 0 ? -1 + m / 100 : 1;
}

export default function Explorer({ snapshot }: Props) {
  const [tab, setTab] = useState<"intents" | "rhea">("intents");
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Reading[]>([]);
  const [rhea, setRhea] = useState<Reading[] | null>(null);
  const [checking, setChecking] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    if (snapshot) setRows([...snapshot.readings].sort((a, b) => rank(a) - rank(b)));
  }, [snapshot]);

  // Rhea lending is checked live in the browser at one block height.
  useEffect(() => {
    if (tab !== "rhea" || rhea) return;
    (async () => {
      const block = await finalBlock();
      const out = await Promise.all(
        RHEA_TOKENS.map(async (t) => ({
          token: t,
          ...(await tokenMeta(t, block)),
          ...(await checkToken("contract.main.burrow.near", "burrow", t, block)),
        })),
      );
      setRhea(out);
    })();
  }, [tab, rhea]);

  const target = tab === "intents" ? "intents.near" : "contract.main.burrow.near";
  const adapter: Adapter = tab === "intents" ? "nep245" : "burrow";
  const source = tab === "intents" ? rows : (rhea ?? []);

  const stats = useMemo(
    () => ({
      total: source.length,
      backed: source.filter((r) => r.status === "backed").length,
      under: source.filter((r) => r.status === "UNDER-BACKED").length,
      error: source.filter((r) => r.status === "error").length,
    }),
    [source],
  );

  const visible = source.filter((r) => {
    if (filter === "issues" && r.status === "backed") return false;
    if (filter === "backed" && r.status !== "backed") return false;
    const q = query.trim().toLowerCase();
    return !q || r.token.toLowerCase().includes(q) || (r.symbol ?? "").toLowerCase().includes(q);
  });
  const shown = showAll || query || filter !== "all" ? visible : visible.slice(0, 12);

  async function verifyLive(token: string) {
    setChecking(token);
    const res = await checkToken(target, adapter, token);
    const update = (list: Reading[]) => list.map((r) => (r.token === token ? { ...r, ...res } : r));
    if (tab === "intents") setRows(update);
    else setRhea((x) => (x ? update(x) : x));
    setChecking(null);
  }

  return (
    <section className="section" id="live">
      <div className="section-head">
        <p className="eyebrow">Live on NEAR mainnet</p>
        <h2>
          Does it hold <em>what it owes?</em>
        </h2>
        <p className="lede">
          Owed comes from the protocol's own accounting. Held comes from each token contract's <code>ft_balance_of</code>.
          Both are read at the same block, so the answer is exact. Press <strong>Verify</strong> to re-check any row
          from your browser.
        </p>
      </div>

      <div className="explorer card">
        <div className="explorer-bar">
          <div className="tabs" role="tablist">
            <button role="tab" aria-selected={tab === "intents"} onClick={() => setTab("intents")}>
              NEAR Intents
            </button>
            <button role="tab" aria-selected={tab === "rhea"} onClick={() => setTab("rhea")}>
              Rhea lending
            </button>
          </div>
          <div className="stats">
            <span className="stat">
              <b>{stats.backed}</b> backed
            </span>
            <span className={`stat ${stats.under ? "stat-warn" : ""}`}>
              <b>{stats.under}</b> under-backed
            </span>
            <span className="stat stat-muted">
              <b>{stats.error}</b> unreadable
            </span>
            <span className="stat stat-muted mono">
              {tab === "intents" && snapshot ? `block ${snapshot.block.toLocaleString("en-US")}` : rhea ? `block ${rhea[0]?.block.toLocaleString("en-US")}` : "…"}
            </span>
          </div>
        </div>

        <div className="explorer-tools">
          <input
            type="search"
            placeholder="Search token or symbol"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            aria-label="Search tokens"
          />
          <div className="seg">
            {(["all", "issues", "backed"] as Filter[]).map((f) => (
              <button key={f} aria-pressed={filter === f} onClick={() => setFilter(f)}>
                {f === "all" ? "All" : f === "issues" ? "Issues" : "Backed"}
              </button>
            ))}
          </div>
        </div>

        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Token</th>
                <th className="num">Owed</th>
                <th className="num">Held</th>
                <th className="num">Coverage</th>
                <th>Status</th>
                <th aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {tab === "rhea" && !rhea && (
                <tr>
                  <td colSpan={6} className="empty">
                    Reading Rhea lending from mainnet…
                  </td>
                </tr>
              )}
              {shown.map((r) => (
                <tr key={r.token} className={r.status !== "backed" ? `row-${r.status === "error" ? "muted" : "warn"}` : ""}>
                  <td>
                    <div className="tok">
                      <span className="tok-sym">{r.symbol ?? "?"}</span>
                      <span className="tok-id mono" title={r.token}>
                        {r.token.length > 34 ? r.token.slice(0, 16) + "…" + r.token.slice(-12) : r.token}
                      </span>
                    </div>
                  </td>
                  <td className="num mono">{formatUnits(r.owed, r.decimals)}</td>
                  <td className="num mono">{formatUnits(r.held, r.decimals)}</td>
                  <td className="num mono">{coverage(r.owed, r.held)}</td>
                  <td>
                    <span className={`pill pill-${r.status === "backed" ? "ok" : r.status === "error" ? "muted" : "warn"}`}>
                      {r.status === "backed" ? "Backed" : r.status === "error" ? "Unreadable" : "Under-backed"}
                    </span>
                    {r.live && <span className="live-tag mono">live · {r.block.toLocaleString("en-US")}</span>}
                  </td>
                  <td className="num">
                    <button className="btn-ghost" onClick={() => verifyLive(r.token)} disabled={checking === r.token}>
                      {checking === r.token ? "Checking…" : "Verify"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!showAll && !query && filter === "all" && visible.length > 12 && (
          <button className="btn-more" onClick={() => setShowAll(true)}>
            Show all {visible.length} tokens
          </button>
        )}
      </div>
      <p className="footnote">
        Under-backed means the token contract reports less than the protocol's ledger records. We traced all three to
        their transactions: two memecoins take a sell tax, then report it to Intents as refunded, and Intents credits it;
        a test token reported a deposit that never moved tokens.{" "}
        <a href={`${REPO}/blob/main/docs/FINDINGS.md`} target="_blank" rel="noreferrer">
          Read the findings
        </a>
        .
      </p>
    </section>
  );
}
