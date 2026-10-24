import { useEffect, useState } from "react";
import { href } from "./router";
import { useWallet } from "./wallet";

export function useTheme() {
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

export function Brand() {
  return (
    <a className="brand" href={href({ name: "home" })}>
      <span className="brand-mark" aria-hidden>
        <svg viewBox="0 0 32 32" width="22" height="22">
          <rect width="32" height="32" rx="8" fill="currentColor" opacity="0.12" />
          <path d="M9 16.5l4.5 4.5L23 11.5" stroke="var(--ok)" strokeWidth="3" fill="none" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </span>
      Backed
    </a>
  );
}

export function ThemeButton() {
  const [theme, toggle] = useTheme();
  return (
    <button className="icon-btn" onClick={toggle} aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}>
      {theme === "dark" ? "☀" : "☾"}
    </button>
  );
}

export function short(account: string) {
  return account.length > 22 ? `${account.slice(0, 10)}…${account.slice(-8)}` : account;
}

/** Connect button, or the connected account with a sign-out menu. */
export function AccountButton({ onConnected }: { onConnected?: () => void }) {
  const { accountId, connect, disconnect, ready, network } = useWallet();
  const [open, setOpen] = useState(false);
  if (!accountId) {
    return (
      <button className="btn btn-small btn-ok" onClick={() => connect(onConnected)} disabled={!ready}>
        {ready ? "Connect wallet" : "Loading…"}
      </button>
    );
  }
  return (
    <div className="acct">
      <button className="acct-btn" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className="acct-dot" aria-hidden />
        <span className="mono">{short(accountId)}</span>
      </button>
      {open && (
        <div className="acct-menu card" onMouseLeave={() => setOpen(false)}>
          <p className="muted mono acct-net">{network}</p>
          <p className="mono acct-full">{accountId}</p>
          <button
            className="btn-ghost"
            onClick={() => {
              setOpen(false);
              void disconnect();
            }}
          >
            Disconnect
          </button>
        </div>
      )}
    </div>
  );
}
