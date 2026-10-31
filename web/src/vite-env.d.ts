/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "mainnet" moves the wallet and checker to mainnet; anything else means testnet. */
  readonly VITE_NETWORK?: string;
  /** Checker contract account; defaults per network in config.ts. */
  readonly VITE_CHECKER_ACCOUNT?: string;
}
