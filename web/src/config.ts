// Where things live. One switch moves the wallet and the checker between networks:
//   VITE_NETWORK=mainnet VITE_CHECKER_ACCOUNT=<account>.near npm run build
// Mainnet protocols are always read live and read-only, whatever the switch says.

export const REPO = "https://github.com/rohan911438/Backed";

export type NetworkId = "mainnet" | "testnet";

const env = import.meta.env;

/** The network the wallet signs on (where the checker contract is deployed). */
export const WALLET_NETWORK: NetworkId = env.VITE_NETWORK === "mainnet" ? "mainnet" : "testnet";

const DEFAULT_CHECKER: Record<NetworkId, string> = {
  testnet: "simplekilt7490.testnet",
  mainnet: "luckycity5453.near",
};

export const CHECKER = {
  network: WALLET_NETWORK,
  account: (env.VITE_CHECKER_ACCOUNT as string | undefined) || DEFAULT_CHECKER[WALLET_NETWORK],
};

export type CheckerTarget = { label: string; target: string; adapter: "burrow" | "nep245" | "backed"; tokens: string[] };

/** Targets the checker has an adapter for on each network, with tokens worth checking. */
const TARGETS: Record<NetworkId, CheckerTarget[]> = {
  testnet: [
    {
      label: "Rhea / Burrow lending (testnet)",
      target: "contract.1638481328.burrow.testnet",
      adapter: "burrow",
      tokens: ["wrap.testnet", "usdc.fakes.testnet"],
    },
    {
      label: "Example Backed vault (backed_solvency standard)",
      target: "dev-20261025164059-33623400177607.testnet",
      adapter: "backed",
      tokens: ["wrap.testnet"],
    },
  ],
  mainnet: [
    {
      label: "NEAR Intents",
      target: "intents.near",
      adapter: "nep245",
      tokens: ["wrap.near", "usdt.tether-token.near", "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"],
    },
    {
      label: "Rhea / Burrow lending",
      target: "contract.main.burrow.near",
      adapter: "burrow",
      tokens: ["wrap.near", "usdt.tether-token.near"],
    },
  ],
};

export const CHECKER_TARGETS = TARGETS[WALLET_NETWORK];

export const RPCS: Record<NetworkId, string[]> = {
  mainnet: ["https://free.rpc.fastnear.com", "https://rpc.mainnet.fastnear.com"],
  testnet: ["https://test.rpc.fastnear.com", "https://rpc.testnet.near.org"],
};

export function explorerTx(network: NetworkId, hash: string) {
  return `https://${network === "testnet" ? "testnet." : ""}nearblocks.io/txns/${hash}`;
}

export function explorerAccount(network: NetworkId, account: string) {
  return `https://${network === "testnet" ? "testnet." : ""}nearblocks.io/address/${account}`;
}
