// Where things live. The checker contract runs on testnet; mainnet protocols are read-only.

export const REPO = "https://github.com/rohan911438/Backed";

export type NetworkId = "mainnet" | "testnet";

/** The network the wallet signs on (where the checker contract is deployed). */
export const WALLET_NETWORK: NetworkId = "testnet";

export const CHECKER = {
  network: "testnet" as NetworkId,
  account: "simplekilt7490.testnet",
};

/** Targets the testnet checker has an adapter for, with tokens worth checking. */
export const CHECKER_TARGETS = [
  {
    label: "Rhea / Burrow lending (testnet)",
    target: "contract.1638481328.burrow.testnet",
    adapter: "burrow" as const,
    tokens: ["wrap.testnet", "usdc.fakes.testnet"],
  },
];

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
