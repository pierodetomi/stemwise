import { MockBackend } from "./mock";
import { TauriBackend } from "./tauri";
import type { Backend } from "./types";

export type { Backend } from "./types";

/** The Tauri backend when running inside the app, the mock otherwise (browser, Playwright). */
export function createBackend(): Backend {
  const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  return inTauri ? new TauriBackend() : new MockBackend();
}
