import DotField from "../DotField";
import { REPO } from "../config";
import { INCIDENTS, PILLARS, REPLAYS } from "../content";
import { href, navigate, type Page } from "../router";
import { useSnapshot } from "../snapshot";
import { AccountButton, Brand, ThemeButton } from "../ui";
import { useWallet } from "../wallet";
import { ReplayCompare } from "./Replays";

const APP_PAGES: { page: Page; title: string; body: string; icon: string }[] = [
  { page: "dashboard", title: "Live backing", body: "Every token in NEAR Intents and Rhea lending: owed vs held at one block. Re-verify any row from your browser.", icon: "◉" },
  { page: "checker", title: "On-chain checker", body: "Read attestations recorded by the checker contract, and sign a new one with your wallet.", icon: "⛓" },
  { page: "replays", title: "Attack replays", body: "The three 2026 incidents replayed against a naive vault and a Backed vault, step by step.", icon: "↻" },
  { page: "build", title: "Build with Backed", body: "The Rust library, the CLI and the AI-agent skill, with copy-paste examples.", icon: "</>" },
];

export default function Home() {
  const snap = useSnapshot();
  const { accountId, connect, ready } = useWallet();
  const goApp = () => navigate({ name: "app", page: "dashboard" });
  const primary = () => (accountId ? goApp() : connect(goApp));
  const intents = REPLAYS[0];

  return (
    <>
      <header className="nav">
        <Brand />
        <nav aria-label="Sections">
          <a href="#problem">Problem</a>
          <a href="#proof">Proof</a>
          <a href="#how">How it works</a>
          <a href="#inside">Inside the app</a>
        </nav>
        <div className="nav-actions">
          <ThemeButton />
          {accountId ? (
            <a className="btn btn-small" href={href({ name: "app", page: "dashboard" })}>
              Open app →
            </a>
          ) : (
            <AccountButton onConnected={goApp} />
          )}
        </div>
      </header>

      <main id="top">
        <section className="hero">
          <div className="aurora" aria-hidden />
          <div className="hero-copy">
            <a className="badge" href={href({ name: "app", page: "dashboard" })}>
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
              On NEAR, nothing rolls back. Backed makes the async ledger bugs behind $22M of 2026 losses impossible to ship,
              replays every attack to prove it, and checks on-chain that protocols hold what they owe.
            </p>
            <div className="hero-ctas">
              <button className="btn btn-ok btn-lg" onClick={primary} disabled={!ready && !accountId}>
                {accountId ? "Open the app" : "Connect wallet"} <span aria-hidden>→</span>
              </button>
              <a className="btn btn-ghost-lg btn-lg" href={href({ name: "app", page: "dashboard" })}>
                Explore without a wallet
              </a>
            </div>
            <dl className="hero-stats">
              <div>
                <dt>Intents tokens checked</dt>
                <dd>{snap?.checked ?? "—"}</dd>
              </div>
              <div>
                <dt>Attacks blocked</dt>
                <dd className="ok">3 / 3</dd>
              </div>
              <div>
                <dt>Under-backed found</dt>
                <dd className="warn">{snap?.under_backed ?? "—"}</dd>
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

        <div className="marquee" aria-label="What Backed covers">
          <span>NEP-141</span>
          <span>NEP-245 · intents.near</span>
          <span>Rhea / Burrow lending</span>
          <span>near-sdk 5.x</span>
          <span>near-sandbox</span>
          <span>Async receipts &amp; callbacks</span>
          <span>Open source · MIT / Apache-2.0</span>
        </div>

        <section className="section" id="problem">
          <div className="section-head">
            <p className="eyebrow">The problem</p>
            <h2>
              $22M in 2026. <em>One bug family.</em>
            </h2>
            <p className="lede">
              NEAR calls between contracts are asynchronous. When a callback fails or is skipped, nothing rolls back, and a
              contract's ledger keeps numbers no real tokens back. On Ethereum a transaction is all-or-nothing, so this
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

        <section className="section" id="proof">
          <div className="section-head">
            <p className="eyebrow">Proof, not promises</p>
            <h2>
              Same attack. <em>Two vaults.</em>
            </h2>
            <p className="lede">
              Replayed on a real local NEAR sandbox: real contracts, real receipts. The ledger is checked against the
              token's balance after every step.
            </p>
          </div>
          <ReplayCompare replay={intents} compact />
          <p className="center-link">
            <a className="btn btn-ghost-lg" href={href({ name: "app", page: "replays" })}>
              See all three replays →
            </a>
          </p>
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

        <section className="section" id="inside">
          <div className="section-head">
            <p className="eyebrow">Inside the app</p>
            <h2>
              Four pages. <em>One question.</em>
            </h2>
            <p className="lede">Does this protocol hold what it owes? Reading is free and needs no wallet. Signing an on-chain attestation does.</p>
          </div>
          <div className="grid-4">
            {APP_PAGES.map((p) => (
              <a className="card app-card" key={p.page} href={href({ name: "app", page: p.page })}>
                <span className="app-icon mono" aria-hidden>
                  {p.icon}
                </span>
                <h3>{p.title}</h3>
                <p>{p.body}</p>
                <span className="app-go">Open →</span>
              </a>
            ))}
          </div>
        </section>

        <section className="cta-band">
          <div className="cta-inner card">
            <div>
              <h2>
                Check it <em>yourself.</em>
              </h2>
              <p className="muted">Connect a NEAR testnet wallet and record your first on-chain attestation in under a minute.</p>
            </div>
            <button className="btn btn-ok btn-lg" onClick={primary}>
              {accountId ? "Open the app" : "Connect wallet"} <span aria-hidden>→</span>
            </button>
          </div>
        </section>
      </main>

      <Footer />
    </>
  );
}

export function Footer() {
  return (
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
            <a href={`${REPO}/blob/main/skill/SKILL.md`}>Agent skill</a>
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
        Data: NEAR mainnet and testnet via public RPC. Under-backed is a finding to investigate, not proof of an exploit.
      </p>
    </footer>
  );
}
