import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Relative base so the build works on GitHub Pages under /Backed/ or any path.
export default defineConfig({
  base: "./",
  plugins: [react()],
});
