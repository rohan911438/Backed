import { REPO } from "../config";
import { FLOWS } from "../content";

const RULES = [
  ["R1", "Credit only what was delivered", "A balance grows only from an amount the token vouched for, never from arguments, msg fields, route minimums or a venue's reply."],
  ["R2", "Every callback is #[private]", "Any method that reads promise results can't be called from outside."],
  ["R3", "Resolve callbacks never panic", "Refunds capped, result reads bounded, logs under 16 KB, gas checked at entry."],
  ["R4", "Debit before tokens leave", "Then a timing gap can only make holdings look higher than the books, never lower."],
  ["R5", "Reject unknown tokens", "ft_on_transfer from an unregistered token returns everything as unused."],
  ["R6", "Expose backed_solvency", "Anyone, including the checker, can verify owed ≤ held."],
];

export default function Build() {
  return (
    <div className="page">
      <div className="page-head">
        <p className="eyebrow">For developers and AI agents</p>
        <h1>
          Build with <em>Backed.</em>
        </h1>
        <p className="lede">
          A Rust library for near-sdk contracts, a CLI for checks and chaos replays, and a skill file that makes AI coding
          agents use both by default.
        </p>
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
intents.near at block 221254666
usdt.tether-token.near   backed   529429980519   529450410245

$ backed scan-intents
intents.near: 370 NEP-141 tokens
  backed: 362   under-backed: 3   could not read: 5

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
            Drop it into Claude Code, Codex or any agent that reads skills. When the agent writes a NEAR contract that holds
            tokens, it uses the backed building blocks, runs the review checklist, and proves the result with{" "}
            <code>backed chaos</code>.
          </p>
        </div>
        <a className="btn" href={`${REPO}/blob/main/skill/SKILL.md`} target="_blank" rel="noreferrer">
          Read the skill ↗
        </a>
      </div>

      <div className="section-head small-head">
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
    </div>
  );
}
