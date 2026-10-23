import { useEffect, useState } from "react";
import DotField from "./DotField";
import Explorer from "./Explorer";
import type { Reading } from "./near";

const REPO = "https://github.com/rohan911438/Backed";

type Snapshot = { block: number; checked: number; backed: number; under_backed: number; errors: number; readings: Reading[] };

const INCIDENTS = [
  {
    name: "NEAR Intents",
    date: "Oct 1, 2026",
    loss: "$3.8M",
    what: "A refund log passed NEAR's 16,384-byte limit, so the resolve callback failed. The token was refunded; the balance stayed recorded.",
    fix: "Refunds capped first, log size measured on capped values, resolve can't panic.",
  },
  {
    name: "Rhea Finance",
    date: "Apr 16, 2026",
    loss: "$18.4M",
    what: "Margin trading credited the amount a swap route claimed. The protocol received about 0.00002% of it.",
    fix: "Balances only grow from Delivered amounts. Crediting a claimed number doesn't compile.",
  },
  {
    name: "SWEAT & HOT",
    date: "Zero-day, Fireblocks",
    loss: "8.5M SWEAT",
    what: "A refund callback anyone could call paid out refunds nobody was owed.",
    fix: "Resolvers are private and refunds are capped by the deposit and the balance.",
  },
];

const PILLARS = [
  {
    tag: "Prevent",
    title: "backed",
    kind: "Rust library",
    body: "Accounting building blocks for near-sdk contracts. Owed ≤ held is checked on every state change, and reconcile compares the books with the token's real balance without false alarms from transfers in flight.",
  },
  {
    tag: "Prove",
    title: "backed-chaos",
    kind: "Test harness",
    body: "Fails every async step on purpose: callbacks out of gas, log-limit overflow, absurd refunds, direct callback calls, lying venues, fee and burn tokens. Replays all three 2026 incidents.",
  },
  {
    tag: "Verify",
    title: "Checker",
    kind: "Mainnet contract",
    body: "Anyone can record, on-chain, whether a protocol holds what it owes. Owed and held come from the protocol's and the token's own views. No trusted reporter.",
  },
];

const FLOWS = [
  {
    who: "Contract developers",
    steps: ["cargo add backed", "Build deposits, withdrawals and refunds on Ledger", "Run backed chaos in CI", "Expose backed_solvency and self-register"],
  },
  {
    who: "Protocols & the NEAR team",
    steps: ["Nothing to install", "Intents and Rhea are checked today from their own views", "See issues here and on-chain", "Adopt the standard when ready"],
  },
  {
    who: "Agents, wallets & users",
    steps: ["Before depositing: backed check <protocol>", "Or read get_status on the checker", "Rule of thumb: never deposit into a confirmed breach"],
  },
];

function useTheme() {
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    try {
      const t = localStorage.getItem("theme");
      if (t === "dark" || t === "light") return t;
    } catch {
      /* storage unavailable */
    }
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  });
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("theme", theme);
    } catch {
      /* storage unavailable */
    }
  }, [theme]);
  return [theme, () => setTheme((t) => (t === "dark" ? "light" : "dark"))] as const;
}

export default function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [theme, toggleTheme] = useTheme();

  useEffect(() => {
    fetch("./intents-snapshot.json")
      .then((r) => r.json())
      .then(setSnap)
      .catch(() => setSnap(null));
  }, []);

  return (
    <>
      <div className="grain" aria-hidden />
      <header className="nav">
        <a className="brand" href="#top">
          <span className="brand-mark" aria-hidden>
            <svg viewBox="0 0 32 32" width="22" height="22">
              <rect width="32" height="32" rx="8" fill="currentColor" opacity="0.12" />
              <path d="M9 16.5l4.5 4.5L23 11.5" stroke="var(--ok)" strokeWidth="3" fill="none" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </span>
          Backed
        </a>
        <nav aria-label="Main">
          <a href="#problem">Problem</a>
          <a href="#how">How it works</a>
          <a href="#live">Live</a>
          <a href="#build">Build</a>
        </nav>
        <div className="nav-actions">
          <button className="icon-btn" onClick={toggleTheme} aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}>
            {theme === "dark" ? "☀" : "☾"}
          </button>
          <a className="btn btn-small" href={REPO} target="_blank" rel="noreferrer">
            GitHub
          </a>
        </div>
      </header>

      <main id="top">
        <section className="hero">
          <div className="aurora" aria-hidden />
          <div className="hero-copy">
            <a className="badge" href="#live">
              <span className="pulse" aria-hidden />
              {snap ? (
                <>
                  intents.near · <b>{snap.backed}</b> of {snap.checked} tokens backed
                </>
              ) : (
                "Reading NEAR mainnet…"
              )}
            </a>
            <h1>
              <span className="nowrap">Never owe more</span>
              <br />
              than you <em>hold.</em>
            </h1>
            <p className="hero-sub">
              On NEAR, nothing rolls back. Backed makes the async ledger bugs behind $22M of 2026 losses impossible to
              ship, and proves on-chain that protocols hold what they owe.
            </p>
            <div className="hero-ctas">
              <a className="btn" href="#live">
                See live backing
              </a>
              <a className="btn btn-ghost-lg" href="#build">
                Build with Backed
              </a>
            </div>
            <dl className="hero-stats">
              <div>
                <dt>Tokens checked</dt>
                <dd>{snap?.checked ?? "—"}</dd>
              </div>
              <div>
                <dt>Under-backed</dt>
                <dd className="warn">{snap?.under_backed ?? "—"}</dd>
              </div>
              <div>
                <dt>Snapshot block</dt>
                <dd className="mono">{snap ? snap.block.toLocaleString("en-US") : "—"}</dd>
              </div>
            </dl>
          </div>
          <div className="hero-visual">
            <DotField readings={snap?.readings ?? []} />
            <p className="visual-caption">
              Each dot is a token held by NEAR Intents. <span className="ok">Green</span> is backed,{" "}
              <span className="warn">amber</span> is under-backed.
            </p>
          </div>
        </section>

        <section className="section" id="problem">
          <div className="section-head">
            <p className="eyebrow">The problem</p>
            <h2>
              $22M in 2026. <em>One bug family.</em>
            </h2>
            <p className="lede">
              NEAR calls between contracts are asynchronous. When a callback fails or is skipped, nothing rolls back, and
              a contract's ledger keeps numbers no real tokens back. On Ethereum, a transaction is all-or-nothing, so this
              family barely exists there.
            </p>
          </div>
          <div className="grid-3">
            {INCIDENTS.map((i) => (
              <article className="card incident" key={i.name}>
                <div className="incident-top">
                  <span className="mono muted">{i.date}</span>
                  <span className="loss">{i.loss}</span>
                </div>
                <h3>{i.name}</h3>
                <p>{i.what}</p>
                <div className="fix">
                  <span className="pill pill-ok">Backed fix</span>
                  <p>{i.fix}</p>
                </div>
              </article>
            ))}
          </div>
        </section>

        <section className="section" id="how">
          <div className="section-head">
            <p className="eyebrow">How it works</p>
            <h2>
              Prevent. Prove. <em>Verify.</em>
            </h2>
          </div>
          <div className="grid-3">
            {PILLARS.map((p, i) => (
              <article className="card pillar" key={p.tag}>
                <span className="pillar-num mono">0{i + 1}</span>
                <p className="eyebrow">{p.tag}</p>
                <h3>
                  {p.title} <span className="muted kind">{p.kind}</span>
                </h3>
                <p>{p.body}</p>
              </article>
            ))}
          </div>
        </section>

        <Explorer snapshot={snap} />

        <section className="section" id="build">
          <div className="section-head">
            <p className="eyebrow">Who uses it</p>
            <h2>
              Three ways in. <em>No permission needed.</em>
            </h2>
          </div>
          <div className="grid-3">
            {FLOWS.map((f) => (
              <article className="card flow" key={f.who}>
                <h3>{f.who}</h3>
                <ol>
                  {f.steps.map((s) => (
                    <li key={s}>{s}</li>
                  ))}
                </ol>
              </article>
            ))}
          </div>

          <div className="code-grid">
            <div className="card code">
              <div className="code-head mono">
                <span>vault.rs</span>
                <span className="muted">backed</span>
              </div>
              <pre>
                <code>{`// Only a registered token can credit a balance.
let Some(amount) = Delivered::from_token_transfer(&token, amount.0)
else { return PromiseOrValue::Value(amount) };
self.ledger.credit(&token, &sender_id, amount);

// Debit before sending; settle in a private callback.
let pending = self.ledger.debit_for_withdraw(&token, &owner, Claimed(x));

// Books vs reality, no false alarms from transfers in flight.
let ticket = self.ledger.begin_reconcile(&token);`}</code>
              </pre>
            </div>
            <div className="card code">
              <div className="code-head mono">
                <span>terminal</span>
                <span className="muted">backed CLI</span>
              </div>
              <pre>
                <code>{`$ backed check intents.near --token usdt.tether-token.near
intents.near at block 221254666
usdt.tether-token.near   backed   529429980519   529450410245

$ backed scan-intents
intents.near: 370 NEP-141 tokens
  backed: 362   under-backed: 3   could not read: 5`}</code>
              </pre>
            </div>
          </div>
        </section>
      </main>

      <footer className="footer">
        <div className="footer-top">
          <div>
            <p className="footer-word">Backed</p>
            <p className="muted">Verified settlement for NEAR. Built at NEAR HACKATHON, 2026.</p>
          </div>
          <div className="footer-links">
            <div>
              <p className="eyebrow">Project</p>
              <a href={REPO}>GitHub</a>
              <a href={`${REPO}/tree/main/Idea`}>Design docs</a>
              <a href="#live">Live backing</a>
            </div>
            <div>
              <p className="eyebrow">NEAR</p>
              <a href="https://nearhackathon.com">NEAR HACKATHON</a>
              <a href="https://docs.near.org">NEAR docs</a>
              <a href="https://github.com/near/intents">near/intents</a>
            </div>
          </div>
        </div>
        <p className="footer-note mono muted">
          Data: NEAR mainnet via public RPC. Under-backed is a finding to investigate, not proof of an exploit.
        </p>
      </footer>
    </>
  );
}
