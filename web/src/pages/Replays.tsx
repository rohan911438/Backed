import { useState } from "react";
import { REPLAYS, type Book, type Replay, type ReplayRun } from "../content";

/** The token the attack targets: the one where the naive vault falls short. */
function focusToken(r: Replay): string {
  for (const s of r.naive.steps) for (const b of s.books) if (b.owed > b.held) return b.token;
  return r.naive.steps[0].books[0].token;
}

function BookBar({ book, max }: { book: Book; max: number }) {
  const short = book.owed > book.held;
  const pct = (n: number) => `${Math.max(2, (n / max) * 100)}%`;
  return (
    <div className="bookbar" title={`${book.token}: owed ${book.owed}, held ${book.held}`}>
      <div className="bookbar-track">
        <span className="bookbar-held" style={{ width: pct(book.held) }} />
        <span className={`bookbar-owed ${short ? "short" : ""}`} style={{ width: pct(book.owed) }} />
      </div>
      <span className="bookbar-num mono">
        {book.owed}
        <span className="muted"> / {book.held}</span>
        {short && <b className="warn"> −{book.owed - book.held}</b>}
      </span>
    </div>
  );
}

function RunColumn({ label, run, token, max }: { label: string; run: ReplayRun; token: string; max: number }) {
  const broke = run.verdict === "BROKEN";
  return (
    <div className={`run card ${broke ? "run-broken" : "run-safe"}`}>
      <div className="run-head">
        <div>
          <p className="eyebrow">{label}</p>
          <h3>{broke ? "Owes more than it holds" : "Books match reality"}</h3>
        </div>
        <span className={`pill ${broke ? "pill-warn" : "pill-ok"}`}>{run.verdict}</span>
      </div>
      <ol className="run-steps">
        {run.steps.map((s, i) => {
          const book = s.books.find((b) => b.token === token);
          return (
            <li key={i} className={s.ok ? "" : "step-failed"}>
              <div className="step-top">
                <span className="step-actor mono">{s.actor}</span>
                <span className="step-note">
                  {s.note}
                  {s.tag && <span className="step-tag mono">{s.tag}</span>}
                </span>
                <span className={`step-status mono ${s.ok ? "muted" : "warn"}`}>{s.ok ? "ok" : "failed"}</span>
              </div>
              {s.error && <p className="step-error mono">{s.error}</p>}
              {book && <BookBar book={book} max={max} />}
            </li>
          );
        })}
      </ol>
      <div className="run-foot">
        <span className="muted">Attacker gain</span>
        <b className={run.gain > 0 ? "warn" : "ok"}>{run.gain > 0 ? `+${run.gain}` : run.gain}</b>
      </div>
    </div>
  );
}

export function ReplayCompare({ replay, compact }: { replay: Replay; compact?: boolean }) {
  const token = focusToken(replay);
  const max = Math.max(
    ...[replay.naive, replay.backed].flatMap((r) => r.steps.flatMap((s) => s.books.filter((b) => b.token === token).map((b) => Math.max(b.owed, b.held)))),
  );
  return (
    <div className={`replay ${compact ? "replay-compact" : ""}`}>
      <div className="replay-intro">
        <p className="mono muted">{replay.incident}</p>
        <h3>{replay.title}</h3>
        <p>{replay.summary}</p>
        <p className="legend muted">
          <span className="lg lg-owed" /> owed (vault's books) <span className="lg lg-held" /> held (token's real balance) · shown for{" "}
          <span className="mono">{token}</span>
        </p>
      </div>
      <div className="replay-cols">
        <RunColumn label="Naive vault" run={replay.naive} token={token} max={max} />
        <RunColumn label="Backed vault" run={replay.backed} token={token} max={max} />
      </div>
    </div>
  );
}

export default function Replays() {
  const [id, setId] = useState(REPLAYS[0].id);
  const replay = REPLAYS.find((r) => r.id === id) ?? REPLAYS[0];
  return (
    <div className="page">
      <div className="page-head">
        <p className="eyebrow">backed-chaos · local NEAR sandbox</p>
        <h1>
          Attack replays <em>— naive vs Backed</em>
        </h1>
        <p className="lede">
          Each 2026 incident runs as real transactions against two vaults with the same features. After every step, the
          vault's books (owed) are compared with what the token contract says it holds.
        </p>
      </div>

      <div className="tabs replay-tabs" role="tablist">
        {REPLAYS.map((r) => (
          <button key={r.id} role="tab" aria-selected={r.id === id} onClick={() => setId(r.id)}>
            {r.incident.split(" · ")[0]}
          </button>
        ))}
      </div>

      <ReplayCompare replay={replay} />

      <div className="card code run-yourself">
        <div className="code-head mono">
          <span>run it yourself</span>
          <span className="muted">Linux · macOS · WSL</span>
        </div>
        <pre>
          <code>{`git clone https://github.com/rohan911438/Backed && cd Backed
bash scripts/build-wasm.sh
cargo run -p backed-cli --features chaos -- chaos          # all three, both vaults
cargo run -p backed-cli --features chaos -- chaos --scenario rhea --json
cargo test -p backed-chaos --features sandbox replay_      # the same, as tests`}</code>
        </pre>
      </div>
    </div>
  );
}
