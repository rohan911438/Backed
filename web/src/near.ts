// Browser-side NEAR reads. Every check pins owed and held to one block height,
// the same method as the `backed` CLI, so in-flight transfers can't skew it.

const RPCS = ["https://free.rpc.fastnear.com", "https://rpc.mainnet.fastnear.com"];

async function rpc(method: string, params: unknown): Promise<any> {
  let lastError: unknown;
  for (let attempt = 0; attempt < 6; attempt++) {
    const url = RPCS[attempt % RPCS.length];
    try {
      const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
      });
      if (res.status === 429 || res.status >= 500) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      if (json.error) {
        const msg = JSON.stringify(json.error);
        if (/limit|Too many|discontinued/i.test(msg)) throw new Error(msg);
        throw Object.assign(new Error(msg), { fatal: true });
      }
      return json.result;
    } catch (e: any) {
      if (e?.fatal) throw e;
      lastError = e;
      await new Promise((r) => setTimeout(r, 300 * (attempt + 1)));
    }
  }
  throw lastError;
}

export async function finalBlock(): Promise<number> {
  const r = await rpc("block", { finality: "final" });
  return r.header.height;
}

export async function view(account: string, method: string, args: object, block: number): Promise<string> {
  const r = await rpc("query", {
    request_type: "call_function",
    block_id: block,
    account_id: account,
    method_name: method,
    args_base64: btoa(JSON.stringify(args)),
  });
  if (r.error) throw new Error(r.error);
  return new TextDecoder().decode(new Uint8Array(r.result));
}

export function parseU128(s: string): bigint | null {
  const t = s.trim().replace(/^"|"$/g, "");
  return /^\d+$/.test(t) ? BigInt(t) : null;
}

/** Rhea lending: owed = (supplied + reserved + prot_fee - borrowed) / 10^extra_decimals. */
export function burrowOwed(json: string): bigint | null {
  try {
    const a = JSON.parse(json);
    const inner =
      BigInt(a.supplied.balance) + BigInt(a.reserved) + BigInt(a.prot_fee ?? "0") - BigInt(a.borrowed.balance);
    return inner / 10n ** BigInt(a.config.extra_decimals);
  } catch {
    return null;
  }
}

export type Adapter = "nep245" | "burrow";

export type Status = "backed" | "UNDER-BACKED" | "error";

export interface Reading {
  token: string;
  symbol?: string;
  decimals?: number;
  owed: string | null;
  held: string | null;
  status: Status;
  shortfall?: string | null;
  error?: string | null;
  block: number;
  live?: boolean;
}

export async function checkToken(
  target: string,
  adapter: Adapter,
  token: string,
  block?: number,
): Promise<Pick<Reading, "owed" | "held" | "status" | "shortfall" | "error" | "block" | "live">> {
  const b = block ?? (await finalBlock());
  try {
    const [owedRaw, heldRaw] = await Promise.all([
      adapter === "nep245"
        ? view(target, "mt_supply", { token_id: `nep141:${token}` }, b)
        : view(target, "get_asset", { token_id: token }, b),
      view(token, "ft_balance_of", { account_id: target }, b),
    ]);
    const owed = adapter === "nep245" ? parseU128(owedRaw) : burrowOwed(owedRaw);
    const held = parseU128(heldRaw);
    if (owed === null || held === null) throw new Error("unreadable response");
    const under = held < owed;
    return {
      owed: owed.toString(),
      held: held.toString(),
      status: under ? "UNDER-BACKED" : "backed",
      shortfall: under ? (owed - held).toString() : null,
      error: null,
      block: b,
      live: true,
    };
  } catch (e: any) {
    return { owed: null, held: null, status: "error", shortfall: null, error: String(e?.message ?? e), block: b, live: true };
  }
}

export async function tokenMeta(token: string, block: number): Promise<{ symbol?: string; decimals?: number }> {
  try {
    const m = JSON.parse(await view(token, "ft_metadata", {}, block));
    return { symbol: m.symbol, decimals: m.decimals };
  } catch {
    return {};
  }
}

/** Human amount: 1234567 with 6 decimals → "1.23" ; large numbers compact. */
export function formatUnits(raw: string | null | undefined, decimals?: number): string {
  if (raw == null) return "—";
  if (decimals == null) return BigInt(raw).toLocaleString("en-US");
  const v = BigInt(raw);
  const base = 10n ** BigInt(decimals);
  const whole = v / base;
  const frac = Number(v % base) / Number(base);
  const n = Number(whole) + frac;
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  if (n >= 1) return n.toFixed(2);
  if (n === 0) return "0";
  return n.toPrecision(3);
}

/** held / owed as a percentage string, e.g. "100.004%". */
export function coverage(owed: string | null, held: string | null): string {
  if (owed == null || held == null) return "—";
  const o = BigInt(owed);
  if (o === 0n) return held === "0" ? "—" : "∞";
  const bps = (BigInt(held) * 1_000_000n) / o; // millionths
  return (Number(bps) / 10_000).toFixed(3) + "%";
}
