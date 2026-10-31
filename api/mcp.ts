// Hosted Backed MCP server (Streamable HTTP, stateless, read-only).
// Connect: claude mcp add --transport http backed https://backed-near.vercel.app/api/mcp
// Same tools as `backed mcp` (the CLI's stdio server). It never signs anything.

type Json = any;

const RPCS: Record<"mainnet" | "testnet", string[]> = {
  mainnet: ["https://free.rpc.fastnear.com", "https://rpc.mainnet.fastnear.com"],
  testnet: ["https://test.rpc.fastnear.com", "https://rpc.testnet.near.org"],
};
const SNAPSHOT_URL = "https://backed-near.vercel.app/intents-snapshot.json";
const FINDINGS_URL = "https://github.com/rohan911438/Backed/blob/main/docs/FINDINGS.md";
const DEFAULT_CHECKER_TESTNET = "simplekilt7490.testnet";
const VERSION = "0.1.0";

async function rpc(method: string, params: Json, network: "mainnet" | "testnet" = "mainnet"): Promise<Json> {
  let last: unknown;
  for (let i = 0; i < 4; i++) {
    const url = RPCS[network][i % 2];
    try {
      const r = await fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }) });
      if (r.status === 429 || r.status >= 500) throw new Error(`HTTP ${r.status}`);
      const j = await r.json();
      if (j.error) throw Object.assign(new Error(JSON.stringify(j.error).slice(0, 300)), { fatal: !/limit|Too many/i.test(JSON.stringify(j.error)) });
      return j.result;
    } catch (e: any) {
      if (e?.fatal) throw e;
      last = e;
      await new Promise((res) => setTimeout(res, 250 * (i + 1)));
    }
  }
  throw last;
}

async function finalBlock(network: "mainnet" | "testnet" = "mainnet"): Promise<number> {
  return (await rpc("block", { finality: "final" }, network)).header.height;
}

async function view(account: string, method: string, args: Json, block: number, network: "mainnet" | "testnet" = "mainnet"): Promise<string> {
  const r = await rpc(
    "query",
    { request_type: "call_function", block_id: block, account_id: account, method_name: method, args_base64: Buffer.from(JSON.stringify(args)).toString("base64") },
    network,
  );
  return Buffer.from(r.result).toString();
}

const u128 = (s: string): bigint | null => {
  const t = s.trim().replace(/^"|"$/g, "");
  return /^\d+$/.test(t) ? BigInt(t) : null;
};

type Adapter = "nep245" | "burrow" | "backed";

function adapterFor(target: string, explicit?: string): Adapter {
  if (explicit === "nep245" || explicit === "burrow" || explicit === "backed") return explicit;
  if (target === "intents.near") return "nep245";
  if (target === "contract.main.burrow.near") return "burrow";
  return "backed";
}

function parseOwed(adapter: Adapter, raw: string): bigint | null {
  try {
    if (adapter === "nep245") return u128(raw);
    const v = JSON.parse(raw);
    if (adapter === "backed") return v && typeof v.owed === "string" ? BigInt(v.owed) : null;
    // Rhea lending: (supplied + reserved + prot_fee - borrowed) / 10^extra_decimals
    const inner = BigInt(v.supplied.balance) + BigInt(v.reserved) + BigInt(v.prot_fee ?? "0") - BigInt(v.borrowed.balance);
    return inner / 10n ** BigInt(v.config.extra_decimals);
  } catch {
    return null;
  }
}

async function read(target: string, adapter: Adapter, token: string, block: number) {
  const [method, args] =
    adapter === "nep245" ? ["mt_supply", { token_id: `nep141:${token}` }] : adapter === "burrow" ? ["get_asset", { token_id: token }] : ["backed_solvency", { token }];
  try {
    const [o, h] = await Promise.all([view(target, method as string, args, block), view(token, "ft_balance_of", { account_id: target }, block)]);
    const owed = parseOwed(adapter, o);
    const held = u128(h);
    if (owed === null || held === null) throw new Error("unreadable response");
    const under = held < owed;
    return { token, owed: owed.toString(), held: held.toString(), status: under ? "UNDER-BACKED" : "backed", shortfall: under ? (owed - held).toString() : null };
  } catch (e: any) {
    return { token, owed: null, held: null, status: "error", error: String(e?.message ?? e).slice(0, 200) };
  }
}

const TOOLS = [
  {
    name: "check_backing",
    description:
      "Read what a NEAR protocol owes (its own accounting view) and what it holds (each token's ft_balance_of) at the same block, so the answer is exact. Use before depositing into a protocol. Works for intents.near, contract.main.burrow.near (Rhea lending) and any contract implementing backed_solvency.",
    inputSchema: {
      type: "object",
      properties: {
        target: { type: "string", description: "Protocol account, e.g. intents.near" },
        tokens: { type: "array", items: { type: "string" }, minItems: 1, maxItems: 10, description: "NEP-141 token contracts, e.g. usdt.tether-token.near" },
        adapter: { type: "string", enum: ["nep245", "burrow", "backed"], description: "How to read 'owed'. Guessed for intents.near and contract.main.burrow.near; default backed (backed_solvency)." },
      },
      required: ["target", "tokens"],
    },
    annotations: { readOnlyHint: true, openWorldHint: true },
  },
  {
    name: "intents_backing_summary",
    description: "The latest daily scan of every NEP-141 token in NEAR Intents (intents.near): how many are backed, which are under-backed and by how much, and why.",
    inputSchema: { type: "object", properties: {} },
    annotations: { readOnlyHint: true, openWorldHint: true },
  },
  {
    name: "checker_status",
    description: "What the on-chain Backed checker contract last recorded for a (protocol, token) pair: Healthy, Suspected or Confirmed breach, with owed, held and block.",
    inputSchema: {
      type: "object",
      properties: {
        target: { type: "string" },
        token: { type: "string" },
        checker: { type: "string", description: "Checker contract account. Default: the testnet deployment." },
        network: { type: "string", enum: ["testnet", "mainnet"], description: "Default: testnet" },
      },
      required: ["target", "token"],
    },
    annotations: { readOnlyHint: true, openWorldHint: true },
  },
];

async function callTool(name: string, args: Json): Promise<{ text: string; data: Json }> {
  if (name === "check_backing") {
    const target = String(args?.target ?? "");
    const tokens: string[] = Array.isArray(args?.tokens) ? args.tokens.map(String).slice(0, 10) : [];
    if (!target || tokens.length === 0) throw new Error("target and tokens are required");
    const adapter = adapterFor(target, args?.adapter);
    const block = await finalBlock();
    const rows = await Promise.all(tokens.map((t) => read(target, adapter, t, block)));
    const under = rows.filter((r) => r.status === "UNDER-BACKED").map((r) => r.token);
    const errors = rows.filter((r) => r.status === "error").length;
    const verdict = under.length
      ? `UNDER-BACKED for ${under.join(", ")}: the protocol owes more of these tokens than it holds. Do not deposit them without investigating.`
      : errors
        ? "UNKNOWN for some tokens: a view call failed. Treat those as unverified."
        : "BACKED: for every token checked, the protocol holds at least what it owes.";
    const lines = rows.map(
      (r) => `- ${r.token}: ${r.status} (owed ${r.owed ?? "?"}, held ${r.held ?? "?"}${r.shortfall ? `, short ${r.shortfall}` : (r as any).error ? `, error: ${(r as any).error}` : ""})`,
    );
    return { text: `${target} at block ${block}: ${verdict}\n${lines.join("\n")}`, data: { target, block, verdict, readings: rows } };
  }
  if (name === "intents_backing_summary") {
    const s = await (await fetch(SNAPSHOT_URL)).json();
    const under = (s.readings as Json[]).filter((r) => r.status === "UNDER-BACKED");
    const lines = under.map((r) => `- ${r.token} (${r.symbol ?? "?"}): owed ${r.owed}, held ${r.held}`);
    return {
      text: `intents.near at block ${s.block}: ${s.backed} of ${s.checked} NEP-141 tokens backed, ${s.under_backed} under-backed, ${s.errors} unreadable.\n${lines.join("\n")}\nCauses: ${FINDINGS_URL}`,
      data: { block: s.block, checked: s.checked, backed: s.backed, under_backed: under },
    };
  }
  if (name === "checker_status") {
    const target = String(args?.target ?? "");
    const token = String(args?.token ?? "");
    if (!target || !token) throw new Error("target and token are required");
    const network = args?.network === "mainnet" ? "mainnet" : "testnet";
    const checker = args?.checker ?? (network === "testnet" ? DEFAULT_CHECKER_TESTNET : null);
    if (!checker) throw new Error("pass `checker`: no default mainnet checker yet");
    const block = await finalBlock(network);
    const status = JSON.parse(await view(checker, "get_status", { target, token }, block, network));
    const text = status
      ? `${checker}: (${target}, ${token}) is ${JSON.stringify(status.state)} as of block ${status.last_block} (owed ${status.owed}, held ${status.held}).`
      : `${checker} has no attestation for (${target}, ${token}) yet.`;
    return { text, data: { checker, network, status } };
  }
  throw new Error(`unknown tool ${name}`);
}

async function handle(msg: Json): Promise<Json | null> {
  if (msg?.id === undefined || msg?.id === null) return null; // notification
  const ok = (result: Json) => ({ jsonrpc: "2.0", id: msg.id, result });
  switch (msg.method) {
    case "initialize":
      return ok({
        protocolVersion: msg.params?.protocolVersion ?? "2025-06-18",
        capabilities: { tools: {} },
        serverInfo: { name: "backed", title: "Backed: NEAR solvency checks", version: VERSION },
        instructions:
          "Backed checks whether a NEAR protocol holds the tokens its own books say it owes, reading both at the same block. Call check_backing before depositing into a NEAR protocol; never deposit into one that is UNDER-BACKED without investigating.",
      });
    case "ping":
      return ok({});
    case "tools/list":
      return ok({ tools: TOOLS });
    case "tools/call":
      try {
        const { text, data } = await callTool(msg.params?.name, msg.params?.arguments ?? {});
        return ok({ content: [{ type: "text", text }], structuredContent: data, isError: false });
      } catch (e: any) {
        return ok({ content: [{ type: "text", text: `error: ${e?.message ?? e}` }], isError: true });
      }
    default:
      return { jsonrpc: "2.0", id: msg.id, error: { code: -32601, message: `method not found: ${msg.method}` } };
  }
}

export default async function handler(req: any, res: any) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Headers", "content-type, mcp-protocol-version, mcp-session-id, accept");
  res.setHeader("Access-Control-Allow-Methods", "POST, GET, OPTIONS");
  if (req.method === "OPTIONS") return res.status(204).end();
  if (req.method === "GET") {
    // No server-initiated stream: this server is stateless.
    return res.status(405).json({ name: "backed", transport: "streamable-http", usage: "POST JSON-RPC 2.0 here. claude mcp add --transport http backed https://backed-near.vercel.app/api/mcp" });
  }
  if (req.method !== "POST") return res.status(405).end();
  let body = req.body;
  if (typeof body === "string") {
    try {
      body = JSON.parse(body);
    } catch {
      return res.status(400).json({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "parse error" } });
    }
  }
  if (Array.isArray(body)) {
    const out = (await Promise.all(body.map(handle))).filter(Boolean);
    return out.length ? res.status(200).json(out) : res.status(202).end();
  }
  const out = await handle(body);
  return out ? res.status(200).json(out) : res.status(202).end();
}
