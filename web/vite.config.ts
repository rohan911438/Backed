import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Relative base so the build works on GitHub Pages under /Backed/ or any path.
export default defineConfig({
  base: "./",
  plugins: [react()],
  // Wallet SDKs import Node's `buffer`; use the browser polyfill instead.
  resolve: { alias: { buffer: "buffer/" } },
  define: { global: "globalThis" },
});
