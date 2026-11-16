import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/fonts.css";
import "@design/tokens.css";
import "@design/components/components.css";
import "@design/screens/screens.css";
import "./styles/app.css";
import { App } from "./app/App";
import { createBackend } from "./backend";
import { runSelfTestIfRequested } from "./selftest";

const backend = createBackend();
// Test hook for Playwright: the mock backend can simulate files dropped from the Finder.
if (backend.kind === "mock") {
  (window as unknown as { __stemwiseMock: unknown }).__stemwiseMock = backend;
  // `?first-launch`: the mock starts with no recent projects and no key (design 1b).
  if (new URLSearchParams(location.search).has("first-launch")) (backend as unknown as { simulateFirstLaunch(): void }).simulateFirstLaunch();
  // `?tour`: the tour starts by itself, as on the very first launch of the app (UI-17); never otherwise.
  if (new URLSearchParams(location.search).has("tour")) (backend as unknown as { tourAuto: boolean }).tourAuto = true;
}
void runSelfTestIfRequested(backend);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App backend={backend} />
  </StrictMode>,
);
