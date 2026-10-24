import Explorer from "../Explorer";
import { CHECKER } from "../config";
import { href, type Page } from "../router";
import { useSnapshot } from "../snapshot";
import { AccountButton, Brand, ThemeButton } from "../ui";
import { useWallet } from "../wallet";
import Build from "./Build";
import Checker from "./Checker";
import { Footer } from "./Home";
import Replays from "./Replays";

const TABS: { page: Page; label: string }[] = [
  { page: "dashboard", label: "Live backing" },
  { page: "checker", label: "Checker" },
  { page: "replays", label: "Replays" },
  { page: "build", label: "Build" },
];

export default function AppShell({ page }: { page: Page }) {
  const snap = useSnapshot();
  const { accountId, connect } = useWallet();

  return (
    <>
      <header className="nav app-nav">
        <Brand />
        <nav className="app-tabs" aria-label="App">
          {TABS.map((t) => (
            <a key={t.page} href={href({ name: "app", page: t.page })} aria-current={t.page === page ? "page" : undefined}>
              {t.label}
            </a>
          ))}
        </nav>
        <div className="nav-actions">
          <span className="net-pill mono" title={`Wallet and checker on ${CHECKER.network}`}>
            {CHECKER.network}
          </span>
          <ThemeButton />
          <AccountButton />
        </div>
      </header>

      {!accountId && (
        <div className="readonly-bar">
          <span>
            <b>Read-only.</b> Everything here is public data. Connect a wallet to sign on-chain attestations.
          </span>
          <button className="btn-ghost" onClick={() => connect()}>
            Connect wallet
          </button>
        </div>
      )}

      <main className="app-main">
        {page === "dashboard" && (
          <div className="page">
            <Explorer snapshot={snap} />
          </div>
        )}
        {page === "checker" && <Checker />}
        {page === "replays" && <Replays />}
        {page === "build" && <Build />}
      </main>
      <Footer />
    </>
  );
}
