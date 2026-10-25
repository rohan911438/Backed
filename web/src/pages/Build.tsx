import { REPO } from "../config";

const MCP_URL = "https://backed-near.vercel.app/api/mcp";

const SECTIONS = [
  ["why", "Why"],
  ["motto", "Principles"],
  ["edge", "Why Backed"],
  ["roadmap", "Roadmap"],
  ["start", "Get started"],
  ["rules", "The six rules"],
] as const;

const WHY = [
  {
    stat: "$22M+",
    title: "Lost to one bug family in 2026",
    body: "NEAR Intents ($3.8M), Rhea Finance ($18.4M) and the SWEAT/HOT zero-day all had the same root: a contract's books said it held tokens it didn't.",
  },
  {
    stat: "0",
    title: "Rollbacks on NEAR",
    body: "Calls between contracts are asynchronous. When a callback fails or trusts the wrong number, nothing undoes the earlier steps. On Ethereum this family barely exists.",
  },
  {
    stat: "1 script",
    title: "Is what NEAR had after the exploit",
    body: "NEAR's engineers compared Intents' books with its real balances by hand, with a shell script. No library prevented the bug, and nothing checked continuously.",
  },
];

const PRINCIPLES = [
  ["Delivered, not claimed", "A balance grows only from tokens that verifiably arrived, never from what a caller, a venue or a token says arrived."],
  ["Verify, don't trust", "Anyone can check a protocol by reading its books and the token's balance at the same block. No reporter, no oracle, no backend."],
  ["Prevent first, detect second", "The library makes the bug impossible to write; the checker and daily scan catch whatever slips through anywhere else."],
];

const EDGE = [
  ["Exact", "Owed and held are read at the same block height, so transfers in flight can't fake a shortfall or hide one."],
  ["Trustless", "The checker contract reads both numbers itself. Nobody has to believe us, or the protocol."],
  ["Preventive", "A Rust library where crediting an unverified amount doesn't compile. Not just another dashboard."],
  ["Proven", "All three 2026 attacks replayed on a real NEAR sandbox: the naive vault loses money, the Backed vault doesn't."],
  ["Agent-native", "An MCP server and a skill file: AI agents check before they deposit, and write safe contracts by default."],
  ["Found something real", "It traced 3 under-backed Intents tokens to the exact transactions, including a live tax-refund bug."],
];

const ROADMAP = [
  {
    phase: "Shipped",
    state: "done",
    items: [
      "backed library + six rules",
      "Three incident replays in CI",
      "Checker contract (testnet), abuse-resistant",
      "Live backing for Intents, Rhea, Ref",
      "Daily scan, history, issue + chat alerts",
      "MCP server (hosted + CLI) and agent skill",
      "Draft NEP: backed_solvency",
    ],
  },
  {
    phase: "Next",
    state: "next",
    items: [
      "Checker on mainnet with Intents and Rhea adapters",
      "Report the tax-refund finding to NEAR Intents",
      "Daily job records attestations on-chain",
      "NEP discussion with NEAR protocol teams",
    ],
  },
  {
    phase: "Later",
    state: "later",
    items: [
      "Wallet warning before depositing an under-backed token",
      "Protocols self-register via backed_solvency",
      "Checker ownership to a multisig or DAO",
      "Chaos harness for any contract's wasm",
    ],
  },
];

const RULES = [
  ["R1", "Credit only what was delivered", "A balance grows only from an amount the token vouched for, never from arguments, msg fields, route minimums or a venue's reply."],
  ["R2", "Every callback is #[private]", "Any method that reads promise results can't be called from outside."],
  ["R3", "Resolve callbacks never panic", "Refunds capped, result reads bounded, logs under 16 KB, gas checked at entry."],
  ["R4", "Debit before tokens leave", "Then a timing gap can only make holdings look higher than the books, never lower."],
  ["R5", "Reject unknown tokens", "ft_on_transfer from an unregistered token returns everything as unused."],
  ["R6", "Expose backed_solvency", "Anyone, including the checker, can verify owed ≤ held."],
];

function jump(id: string) {
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

export default function Build() {
  return (
    <div className="page build">
      <div className="page-head">
        <p className="eyebrow">Why we built it · how to use it</p>
        <h1>
          Never owe more <em>than you hold.</em>
        </h1>
        <p className="lede">
          That's the whole idea. Backed makes it true inside NEAR contracts, checkable by anyone on-chain, and the default
          for the AI agents that now write and use those contracts.
        </p>
        <nav className="jumpbar" aria-label="On this page">
          {SECTIONS.map(([id, label]) => (
            <button key={id} onClick={() => jump(id)}>
              {label}
            </button>
          ))}
        </nav>
      </div>

      <section id="why" className="build-section">
        <div className="section-head small-head">
          <p className="eyebrow">01 · Why</p>
          <h2>
            A bug NEAR keeps paying for, <em>and no tool for it.</em>
          </h2>
        </div>
        <div className="grid-3">
          {WHY.map((w) => (
            <article className="card why" key={w.title}>
              <span className="why-stat">{w.stat}</span>
              <h3>{w.title}</h3>
              <p>{w.body}</p>
            </article>
          ))}
        </div>
      </section>

      <section id="motto" className="build-section">
        <div className="motto card">
          <p className="eyebrow">02 · Our principles</p>
          <blockquote>“Trust what arrived, not what you were told.”</blockquote>
          <div className="grid-3 principles">
            {PRINCIPLES.map(([t, b], i) => (
              <div key={t}>
                <span className="pillar-num mono">0{i + 1}</span>
                <h3>{t}</h3>
                <p>{b}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section id="edge" className="build-section">
        <div className="section-head small-head">
          <p className="eyebrow">03 · Why Backed</p>
          <h2>
            Not proof of reserves. <em>Proof of backing.</em>
          </h2>
          <p className="lede">
            Proof of reserves trusts a reporter and only watches. Backed reads the chain itself, prevents the bug in code,
            and proves the prevention with real attacks.
          </p>
        </div>
        <div className="grid-3">
          {EDGE.map(([t, b]) => (
            <article className="card edge" key={t}>
              <span className="pill pill-ok">{t}</span>
              <p>{b}</p>
            </article>
          ))}
        </div>
      </section>

      <section id="roadmap" className="build-section">
        <div className="section-head small-head">
          <p className="eyebrow">04 · Roadmap</p>
          <h2>
            From one protocol <em>to a NEAR standard.</em>
          </h2>
        </div>
        <ol className="roadmap">
          {ROADMAP.map((r) => (
            <li key={r.phase} className={`card stage stage-${r.state}`}>
              <div className="stage-head">
                <span className="stage-dot" aria-hidden />
                <h3>{r.phase}</h3>
              </div>
              <ul>
                {r.items.map((i) => (
                  <li key={i}>{i}</li>
                ))}
              </ul>
            </li>
          ))}
        </ol>
      </section>

      <section id="start" className="build-section">
        <div className="section-head small-head">
          <p className="eyebrow">05 · Get started</p>
          <h2>
            Pick your role. <em>No permission needed.</em>
          </h2>
        </div>
        <div className="grid-3">
          <article className="card start">
            <p className="eyebrow">AI agents · wallets</p>
            <h3>Hosted MCP server</h3>
            <p>Ask “is it safe to deposit here?” from any MCP client. Read-only, nothing to install.</p>
            <pre className="mono snippet">{`claude mcp add --transport http \\
  backed ${MCP_URL}`}</pre>
            <p className="muted small">Tools: check_backing · intents_backing_summary · checker_status</p>
          </article>
          <article className="card start">
            <p className="eyebrow">Contract developers</p>
            <h3>Rust library + CI</h3>
            <p>Build deposits, withdrawals and refunds on Backed's Ledger, then run the attack replays in CI.</p>
            <pre className="mono snippet">{`backed = { git = "${REPO}" }
backed chaos --scenario intents`}</pre>
            <p className="muted small">Example contract: contracts/examples/backed-vault</p>
          </article>
          <article className="card start">
            <p className="eyebrow">Protocols</p>
            <h3>Be verifiable</h3>
            <p>Expose one view, owed per token, and any wallet, agent or checker can verify you. No adapter needed.</p>
            <pre className="mono snippet">{`fn backed_solvency(&self, token: AccountId)
  -> Option<Solvency>`}</pre>
            <a className="small" href={`${REPO}/blob/main/docs/NEP-backed-solvency.md`} target="_blank" rel="noreferrer">
              Read the draft NEP ↗
            </a>
          </article>
        </div>
      </section>

      <section id="rules" className="build-section">
        <div className="section-head small-head">
          <p className="eyebrow">06 · The six rules</p>
          <h2>
            What the library enforces, <em>and the skill teaches.</em>
          </h2>
        </div>
        <div className="grid-3 rules">
          {RULES.map(([id, title, body]) => (
            <article className="card rule" key={id}>
              <span className="pillar-num mono">{id}</span>
              <h3>{title}</h3>
              <p>{body}</p>
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

// Deposit + notify: the refund is capped, the resolve can't panic.
let unused = self.ledger.finalize_followup(&pending, requested);

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
usdt.tether-token.near   backed   553605606751   553626036477

$ backed scan-intents
intents.near: 370 NEP-141 tokens
  backed: 362   under-backed: 3   could not read: 5

$ backed scan-ref          # Ref Finance, 8,991 pools at one block
$ backed chaos --scenario intents
== replay_intents_log_limit vs Naive vault: BROKEN
== replay_intents_log_limit vs Backed vault: SAFE`}</code>
            </pre>
          </div>
        </div>

        <div className="card skill-card">
          <div>
            <p className="eyebrow">AI coding agents</p>
            <h3>skill/SKILL.md</h3>
            <p className="muted">
              Drop it into Claude Code, Codex or any agent that reads skills. When the agent writes a NEAR contract that
              holds tokens, it follows the six rules, uses the backed building blocks and proves the result with{" "}
              <code>backed chaos</code>.
            </p>
          </div>
          <a className="btn" href={`${REPO}/blob/main/skill/SKILL.md`} target="_blank" rel="noreferrer">
            Read the skill ↗
          </a>
        </div>
      </section>
    </div>
  );
}
