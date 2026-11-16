import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const designDir = fileURLToPath(new URL("../../design", import.meta.url));

// The UI imports the design tokens and component styles straight from design/ (A-002),
// so the design stays the single source of the visual language.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: { alias: { "@design": designDir } },
  server: {
    port: 5173,
    strictPort: true,
    fs: { allow: [fileURLToPath(new URL("..", import.meta.url)), designDir] },
  },
  build: { target: "safari16", outDir: "dist", emptyOutDir: true },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
