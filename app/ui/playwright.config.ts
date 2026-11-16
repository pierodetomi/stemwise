import { defineConfig, devices } from "@playwright/test";

// Playwright drives the UI against the in-memory mock backend (FOUNDATION §5.6).
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  reporter: [["list"]],
  outputDir: "test-results",
  use: { baseURL: "http://localhost:5174", viewport: { width: 1440, height: 900 }, trace: "off" },
  projects: [
    { name: "webkit", grepInvert: /@perf/, use: { ...devices["Desktop Safari"], viewport: { width: 1440, height: 900 } } },
    // Frame-time measures (NFR-3) run alone, after every other test: in parallel with three other browsers they
    // would measure the machine's load, not the UI.
    { name: "perf", grep: /@perf/, dependencies: ["webkit"], fullyParallel: false, use: { ...devices["Desktop Safari"], viewport: { width: 1440, height: 900 } } },
  ],
  webServer: {
    command: "pnpm exec vite --port 5174 --strictPort",
    url: "http://localhost:5174",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
