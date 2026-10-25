// Static content shared by the home page and the app.

export const INCIDENTS = [
  {
    name: "NEAR Intents",
    date: "Oct 1, 2026",
    loss: "$3.8M",
    what: "A refund log passed NEAR's 16,384-byte limit, so the resolve callback failed. The token was refunded; the balance stayed recorded.",
    fix: "Refunds capped first, log size measured on capped values, resolve can't panic.",
    sources: [
      { label: "NEAR's regression test", url: "https://github.com/near/intents/commit/0c1a530c8da337008e28baa9987a816249d707e6" },
      { label: "Postmortem", url: "https://x.com/AlexAuroraDev/status/2111124226962870770" },
      { label: "Crypto Times", url: "https://www.cryptotimes.io/2026/10/17/near-intents-traces-3-8m-exploit-to-smart-contract-deposit-bug/" },
    ],
  },
  {
    name: "Rhea Finance",
    date: "Apr 16, 2026",
    loss: "$18.4M",
    what: "Margin trading validated positions against the swap route's claimed output, then accepted a delivered amount about 4 million times smaller.",
    fix: "Balances and checks use only Delivered amounts. Crediting a claimed number doesn't compile.",
    sources: [
      { label: "The Block", url: "https://www.theblock.co/news/ecosystems/2026-04-17-rhea-finance-post-mortem-exploit-losses-18-4-million-double-initial-estimates-397961" },
      { label: "Rekt analysis", url: "https://rekt.news/rhea-finance-rekt" },
    ],
  },
  {
    name: "SWEAT & HOT",
    date: "Late Apr 2026",
    loss: "8.5M SWEAT",
    what: "The token's refund callback wasn't #[private], so anyone could call it and take refunds nobody was owed.",
    fix: "Resolvers are private and refunds are capped by the deposit and the balance.",
    sources: [{ label: "Fireblocks", url: "https://www.fireblocks.com/blog/near-zero-day-sweat-hot-token-exploit" }],
  },
];

export const PILLARS = [
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
    body: "Replays all three 2026 incidents on a local NEAR sandbox with real contracts. The naive vault loses money every time; the Backed vault blocks every attack.",
  },
  {
    tag: "Verify",
    title: "Checker",
    kind: "On-chain contract",
    body: "Anyone can record, on-chain, whether a protocol holds what it owes, read from the protocol's and the token's own views. No trusted reporter. Live on NEAR testnet.",
  },
];

export const FLOWS = [
  {
    who: "Contract developers",
    steps: ["cargo add backed", "Build deposits, withdrawals and refunds on Ledger", "Run backed chaos in CI", "Expose backed_solvency and self-register"],
  },
  {
    who: "Protocols & the NEAR team",
    steps: ["Nothing to install", "Intents and Rhea are checked from their own views", "See issues here and on-chain", "Adopt the standard when ready"],
  },
  {
    who: "Agents, wallets & users",
    steps: ["Before depositing: backed check <protocol>", "Or read get_status on the checker", "Rule of thumb: never deposit into a confirmed breach"],
  },
];

// ---------- chaos replays (output of `backed chaos`, local NEAR sandbox) ----------

export type Book = { token: string; owed: number; held: number };
export type ReplayStep = { actor: string; note: string; ok: boolean; error?: string; tag?: string; books: Book[] };
export type ReplayRun = { verdict: "BROKEN" | "SAFE"; gain: number; steps: ReplayStep[] };
export type Replay = { id: string; title: string; incident: string; summary: string; naive: ReplayRun; backed: ReplayRun };

const A = "token-a";
const B = "token-b";

export const REPLAYS: Replay[] = [
  {
    id: "intents",
    title: "Failing resolve callback",
    incident: "NEAR Intents · Oct 1, 2026 · $3.8M",
    summary:
      "The attacker deposits with a notify call and asks for an absurd refund. The naive resolve panics, the token refunds the deposit, but the vault keeps the credit.",
    naive: {
      verdict: "BROKEN",
      gain: 100,
      steps: [
        { actor: "lp", note: "Honest liquidity in the vault", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Notify receiver asks for an absurd refund (u128::MAX)",
          tag: "F4",
          ok: false,
          error: "attempt to subtract with overflow",
          books: [{ token: A, owed: 1100, held: 1000 }],
        },
        { actor: "attacker", note: "Withdraw the credit the token already refunded", ok: true, books: [{ token: A, owed: 1000, held: 900 }] },
      ],
    },
    backed: {
      verdict: "SAFE",
      gain: 0,
      steps: [
        { actor: "lp", note: "Honest liquidity in the vault", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        { actor: "attacker", note: "Notify receiver asks for an absurd refund (u128::MAX)", tag: "F4", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Withdraw the credit the token already refunded",
          ok: false,
          error: "BACKED: insufficient balance",
          books: [{ token: A, owed: 1000, held: 1000 }],
        },
      ],
    },
  },
  {
    id: "sweat",
    title: "Public resolver",
    incident: "SWEAT / HOT · Apr 2026 · Fireblocks",
    summary:
      "After a real withdrawal, the attacker chains a failing call into the vault's resolve_withdraw. A public resolver sees “transfer failed” and re-credits.",
    naive: {
      verdict: "BROKEN",
      gain: 100,
      steps: [
        { actor: "lp", note: "Honest liquidity", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        { actor: "attacker", note: "Attacker deposits", ok: true, books: [{ token: A, owed: 1100, held: 1100 }] },
        { actor: "attacker", note: "Real withdrawal succeeds", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Chain a failing call into the vault's resolve_withdraw",
          tag: "F5",
          ok: false,
          error: "evil-receiver: this call always fails",
          books: [{ token: A, owed: 1100, held: 1000 }],
        },
        { actor: "attacker", note: "Withdraw again", ok: true, books: [{ token: A, owed: 1000, held: 900 }] },
      ],
    },
    backed: {
      verdict: "SAFE",
      gain: 0,
      steps: [
        { actor: "lp", note: "Honest liquidity", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        { actor: "attacker", note: "Attacker deposits", ok: true, books: [{ token: A, owed: 1100, held: 1100 }] },
        { actor: "attacker", note: "Real withdrawal succeeds", ok: true, books: [{ token: A, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Chain a failing call into the vault's resolve_withdraw",
          tag: "F5",
          ok: false,
          error: "The chain fails as planned, but the #[private] resolver rejects the outside call: no re-credit",
          books: [{ token: A, owed: 1000, held: 1000 }],
        },
        { actor: "attacker", note: "Withdraw again", ok: false, error: "BACKED: insufficient balance", books: [{ token: A, owed: 1000, held: 1000 }] },
      ],
    },
  },
  {
    id: "rhea",
    title: "Crediting a claimed amount",
    incident: "Rhea Finance · Apr 16, 2026 · $18.4M",
    summary:
      "A swap venue delivers 1 unit but reports 900, and the route claims a 900 minimum. The naive vault credits 900; the attacker drains honest liquidity.",
    naive: {
      verdict: "BROKEN",
      gain: 800,
      steps: [
        { actor: "lp", note: "Honest liquidity (token B)", ok: true, books: [{ token: B, owed: 1000, held: 1000 }] },
        { actor: "attacker", note: "Attacker deposits token A", ok: true, books: [{ token: A, owed: 100, held: 100 }, { token: B, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Swap: venue delivers 1, reports 900, route claims 900",
          tag: "F7",
          ok: true,
          books: [{ token: A, owed: 0, held: 100 }, { token: B, owed: 1901, held: 1001 }],
        },
        { actor: "attacker", note: "Drain honest liquidity", ok: true, books: [{ token: A, owed: 0, held: 100 }, { token: B, owed: 1001, held: 101 }] },
      ],
    },
    backed: {
      verdict: "SAFE",
      gain: -99,
      steps: [
        { actor: "lp", note: "Honest liquidity (token B)", ok: true, books: [{ token: B, owed: 1000, held: 1000 }] },
        { actor: "attacker", note: "Attacker deposits token A", ok: true, books: [{ token: A, owed: 100, held: 100 }, { token: B, owed: 1000, held: 1000 }] },
        {
          actor: "attacker",
          note: "Swap: venue delivers 1, reports 900, route claims 900",
          tag: "F7",
          ok: true,
          books: [{ token: A, owed: 0, held: 100 }, { token: B, owed: 1001, held: 1001 }],
        },
        {
          actor: "attacker",
          note: "Drain honest liquidity",
          ok: false,
          error: "BACKED: insufficient balance",
          books: [{ token: A, owed: 0, held: 100 }, { token: B, owed: 1001, held: 1001 }],
        },
        { actor: "anyone", note: "Reconcile: books vs real balance", ok: true, books: [{ token: A, owed: 0, held: 100 }, { token: B, owed: 1001, held: 1001 }] },
      ],
    },
  },
];
