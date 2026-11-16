// req: NFR-7
// The webview makes no network calls of its own: no fetch, sockets or beacons in the UI sources, and the
// content security policy of the window only allows the app itself and the IPC bridge. The assistant's calls go
// through the core (the OpenRouter client), only on the user's request.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const ui = join(__dirname, "..");

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    if (statSync(p).isDirectory()) return n === "bindings" ? [] : sources(p);
    return /\.(ts|tsx)$/.test(n) && !n.endsWith(".test.ts") ? [p] : [];
  });
}

describe("network policy of the UI (NFR-7)", () => {
  it("has no network calls in its sources", () => {
    const files = sources(join(ui, "src"));
    expect(files.length).toBeGreaterThan(30);
    for (const f of files) {
      const code = readFileSync(f, "utf8")
        .split("\n")
        .map((l) => l.replace(/\/\/.*$/, ""))
        .join("\n");
      for (const api of ["fetch(", "XMLHttpRequest", "new WebSocket", "sendBeacon", "EventSource", "navigator.connection"]) {
        expect(code.includes(api), `${f} uses ${api}`).toBe(false);
      }
    }
  });

  it("is confined by the window's content security policy", () => {
    const conf = JSON.parse(readFileSync(join(ui, "../src-tauri/tauri.conf.json"), "utf8"));
    const csp: string = conf.app.security.csp;
    const connect = csp.split(";").map((d) => d.trim()).find((d) => d.startsWith("connect-src"))!;
    expect(connect.split(/\s+/).slice(1).sort()).toEqual(["'self'", "http://ipc.localhost", "ipc:"].sort());
    expect(csp).toContain("default-src 'self'");
  });
});
