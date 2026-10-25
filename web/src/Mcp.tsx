import { useState } from "react";

export const MCP_URL = "https://backed-near.vercel.app/api/mcp";
export const MCP_NAME = "io.github.rohan911438/backed";
export const MCP_REGISTRY = `https://registry.modelcontextprotocol.io/v0/servers?search=${MCP_NAME}`;

const CLIENTS = {
  "Claude Code": `claude mcp add --transport http backed ${MCP_URL}`,
  "Cursor / VS Code": `{ "mcpServers": { "backed": { "url": "${MCP_URL}" } } }`,
  "Local (CLI)": "claude mcp add backed -- backed mcp",
} as const;
type Client = keyof typeof CLIENTS;

export const MCP_TOOLS: [string, string][] = [
  ["check_backing", "Owed vs held at one block, with a BACKED / UNDER-BACKED verdict"],
  ["intents_backing_summary", "Latest scan of every NEP-141 token in NEAR Intents"],
  ["checker_status", "What the on-chain checker last recorded for a pair"],
];

/** Connect instructions for the Backed MCP server, with a copy button. */
export function McpConnect({ compact = false }: { compact?: boolean }) {
  const [client, setClient] = useState<Client>("Claude Code");
  const [copied, setCopied] = useState(false);
  const cmd = CLIENTS[client];

  async function copy() {
    try {
      await navigator.clipboard.writeText(cmd);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard blocked: the text is still selectable */
    }
  }

  return (
    <div className="mcp">
      <a className="registry-badge" href={MCP_REGISTRY} target="_blank" rel="noreferrer">
        <span className="dot" aria-hidden />
        {compact ? (
          "In the official MCP Registry ↗"
        ) : (
          <>
            Listed in the official MCP Registry · <span className="mono">{MCP_NAME}</span>
          </>
        )}
      </a>
      <div className="mcp-tabs" role="tablist" aria-label="MCP client">
        {(Object.keys(CLIENTS) as Client[]).map((c) => (
          <button key={c} role="tab" aria-selected={c === client} onClick={() => setClient(c)}>
            {c}
          </button>
        ))}
      </div>
      <div className="mcp-cmd">
        <pre className="mono">{cmd}</pre>
        <button className="btn-ghost btn-small" onClick={copy} aria-label="Copy command">
          {copied ? "Copied ✓" : "Copy"}
        </button>
      </div>
      {!compact && (
        <ul className="mcp-tools">
          {MCP_TOOLS.map(([name, what]) => (
            <li key={name}>
              <code>{name}</code>
              <span>{what}</span>
            </li>
          ))}
        </ul>
      )}
      <p className="muted small">Read-only: it never signs or moves funds. Hosted, free, no API key.</p>
    </div>
  );
}
