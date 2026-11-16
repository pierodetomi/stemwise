import { defineConfig, devices } from "@playwright/test";

// The suite runs on the engine the app ships with on this system (A-104): WebKit on macOS (WKWebView) and Linux
// (WebKitGTK), Edge on Windows (WebView2 is Edge's engine). Playwright's WebKit for Windows draws in software: it
// measures no frame time the app would have there, and under the load of the suite it loses clicks on a bubble that
// is still moving. The Mac user agent stays, so the interface reads as the tests expect.
const browser =
  process.platform === "win32"
    ? { ...devices["Desktop Edge"], channel: "msedge", userAgent: devices["Desktop Safari"].userAgent }
    : devices["Desktop Safari"];

// Playwright drives the UI against the in-memory mock backend (FOUNDATION §5.6).
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  reporter: [["list"]],
  outputDir: "test-results",
  use: { baseURL: "http://localhost:5174", viewport: { width: 1440, height: 900 }, trace: "off" },
  projects: [
    // `webkit` by its history: on Windows it runs Edge (see above).
    { name: "webkit", grepInvert: /@perf/, use: { ...browser, viewport: { width: 1440, height: 900 } } },
    // Frame-time measures (NFR-3) run alone, after every other test: in parallel with three other browsers they
    // would measure the machine's load, not the UI.
    { name: "perf", grep: /@perf/, dependencies: ["webkit"], fullyParallel: false, use: { ...browser, viewport: { width: 1440, height: 900 } } },
  ],
  webServer: {
    command: "pnpm exec vite --port 5174 --strictPort",
    url: "http://localhost:5174",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
