import { useEffect, useMemo, useRef, type ReactNode } from "react";
import type { Backend } from "../backend";
import { Icon } from "../shared/Icon";
import { createAppStore } from "../state/store";
import { StoreContext, useApp, useBackend, useStore } from "../state/context";
import { Settings } from "../settings/Settings";
import { shortcutOf } from "../shared/shortcuts";
import { useTransportPoll } from "../shared/StateBanners";
import { Welcome } from "../welcome/Welcome";
import { NewProject } from "../welcome/NewProject";
import { AiWorkspace } from "../ai/AiWorkspace";
import { Setup } from "../ai/Setup";
import { Usage } from "../ai/Usage";
import { BounceDialog } from "../master/Bounce";
import { MasterWorkspace } from "../master/MasterWorkspace";
import { ProjectSettings } from "../project/ProjectSettings";
import { usePrepToasts } from "../master/Prep";
import { useProjectCommands } from "../project/commands";
import { Tour } from "../tour/Tour";
import { beginTour } from "../tour/useTour";

/** Confirmation dialog (design 13g): focus trapped, Esc cancels, Enter confirms unless destructive. */
function Dialog() {
  const d = useApp((s) => s.dialog);
  const setView = useApp((s) => s.setView);
  // A confirmation asked from the lit area of the tour (discard the proposal?) shows above the tour.
  const overTour = useApp((s) => !!s.tour);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (d) ref.current?.querySelector<HTMLElement>(d.danger ? "[data-cancel]" : "[data-ok]")?.focus();
  }, [d]);
  if (!d) return null;
  const close = () => setView({ dialog: null });
  return (
    <div className={`scrim ${overTour ? "over-tour" : ""}`} onPointerDown={(e) => e.target === e.currentTarget && close()}>
      <div
        className="dialog s"
        role="dialog"
        aria-modal="true"
        aria-label={d.title}
        ref={ref}
        onKeyDown={(e) => {
          if (e.key === "Escape") close();
          if (e.key === "Tab") {
            const f = [...(ref.current?.querySelectorAll<HTMLElement>("button") ?? [])];
            const i = f.indexOf(document.activeElement as HTMLElement);
            e.preventDefault();
            f[(i + (e.shiftKey ? -1 : 1) + f.length) % f.length]?.focus();
          }
        }}
      >
        <div className="dlg-h">
          <span className="h l">{d.title}</span>
          {d.sub && <span className="dim text">{d.sub}</span>}
        </div>
        <div className="dlg-f">
          <button type="button" className="btn quiet" data-cancel onClick={close}>
            Cancel
          </button>
          {d.alt && (
            <button
              type="button"
              className={`btn ${d.alt.danger ? "danger" : ""}`}
              onClick={() => {
                close();
                d.alt!.onSelect();
              }}
            >
              {d.alt.label}
            </button>
          )}
          <button
            type="button"
            className={`btn ${d.danger ? "danger" : "primary"}`}
            data-ok
            onClick={() => {
              close();
              d.onConfirm();
            }}
          >
            {d.confirm}
          </button>
        </div>
      </div>
    </div>
  );
}

/** Notifications (design 13h): bottom right, stacked; errors stay until dismissed. */
function Toasts() {
  const toasts = useApp((s) => s.toasts);
  const dismiss = useApp((s) => s.dismissToast);
  const icon = { ok: "check", warn: "warn", err: "warn", ai: "spark", info: "info" } as const;
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`} role={t.kind === "err" ? "alert" : undefined}>
          <Icon name={icon[t.kind]} />
          <div className="col g1 grow">
            <span className="b">{t.title}</span>
            {t.text && <span className="dim">{t.text}</span>}
          </div>
          {t.action && (
            <button
              type="button"
              className="btn sm"
              onClick={() => {
                dismiss(t.id);
                t.action!.run();
              }}
            >
              {t.action.label}
            </button>
          )}
          <button type="button" className="ib" aria-label="Dismiss" onClick={() => dismiss(t.id)}>
            <Icon name="x" size="m" />
          </button>
        </div>
      ))}
    </div>
  );
}

/** Compact layout below 1360 px wide and small lanes below 800 px high (design "Ridimensionamento"). */
function useLayoutClasses() {
  const setView = useApp((s) => s.setView);
  const compact = useApp((s) => s.compact);
  const store = useStore();
  useEffect(() => {
    let short = false;
    const on = () => {
      const c = window.innerWidth < 1360;
      const sh = window.innerHeight < 800;
      setView({ compact: c });
      if (sh !== short) {
        short = sh;
        setView({ lane: sh ? "s" : store.getState().prefs.lane });
      }
    };
    on();
    window.addEventListener("resize", on);
    return () => window.removeEventListener("resize", on);
  }, [setView, store]);
  return compact;
}

function TransportPoll() {
  useTransportPoll();
  return null;
}

function Root({ children }: { children?: ReactNode }) {
  const project = useApp((s) => s.project);
  const screen = useApp((s) => s.screen);
  const setup = useApp((s) => s.setup);
  const backend = useBackend();
  const setSpecs = useApp((s) => s.setSpecs);
  const setError = useApp((s) => s.setError);
  const setView = useApp((s) => s.setView);
  const ms = useApp((s) => s.masterStatus);
  const bounceOpen = useApp((s) => s.bounceOpen);
  const projectOpen = useApp((s) => s.projectOpen);
  const compact = useLayoutClasses();
  // The output state (13d), polled while a project is open: the device can go away while nothing plays.
  const polling = !!project;
  const reduceMotion = useApp((s) => s.prefs.reduceMotion);
  const openSettings = useApp((s) => s.openSettings);
  const cmd = useProjectCommands();
  // Shortcuts of the app menu that work on every screen (UI-20).
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      const id = shortcutOf(e);
      if (id !== "settings" && id !== "newProject" && id !== "openProject") return;
      e.preventDefault();
      if (id === "settings") openSettings();
      else if (id === "newProject") cmd.newProject();
      else cmd.open();
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [openSettings, cmd]);
  // Master: straight in whenever a mixdown exists, even an out-of-date one (the master says so and offers to bounce
  // again, keeping its settings, MDN-2); through the bounce dialog only for the first one (design 8a, MDN-1).
  const onMaster = async () => {
    try {
      if (project?.master.mixdown) await backend.setPhase(true);
      else setView({ bounceOpen: true });
    } catch (e) {
      setError(String(e));
    }
  };
  const master = !!project && ms?.phase === "master";
  // The master prepared while in Mix: a toast when it is ready or failed (9g).
  usePrepToasts(ms);
  useEffect(() => {
    backend.paramSpecs().then(setSpecs).catch(() => {});
  }, [backend, setSpecs]);
  const view =
    screen === "settings" ? (
      <Settings />
    ) : screen === "new" ? (
      <NewProject />
    ) : project && screen === "usage" ? (
      <Usage />
    ) : project && screen === "workspace" ? (
      master && ms ? <MasterWorkspace ms={ms} /> : <AiWorkspace onMaster={() => void onMaster()} />
    ) : (
      <Welcome />
    );
  return (
    <div className={`sw app ${compact ? "compact" : ""} ${reduceMotion ? "reduce-motion" : ""}`} data-phase={master ? "master" : "mix"}>
      {polling && <TransportPoll />}
      {view}
      {bounceOpen && project && <BounceDialog onClose={() => setView({ bounceOpen: false })} />}
      {projectOpen && project && screen === "workspace" && <ProjectSettings onClose={() => setView({ projectOpen: false })} />}
      {children}
      {setup && <Setup key={setup} />}
      <Dialog />
      <Toasts />
      <Tour />
    </div>
  );
}

/** One question per backend, also under StrictMode's double effects. */
const autoAsked = new WeakSet<Backend>();

export function App({ backend }: { backend: Backend }) {
  const store = useMemo(() => createAppStore(backend), [backend]);
  useEffect(() => {
    const offP = backend.onProject((e) => store.getState().setProjectEvent(e));
    const offM = backend.onMeters((m) => store.getState().setMeters(m));
    // The assistant (AI-12): status as it changes, a toast when a proposal is ready or a call ends without one.
    const offA = backend.onAiStatus((a) => {
      const s = store.getState();
      if (s.ai?.running && !a.running && a.proposal && !s.ai.proposal) s.toast("ai", "Proposal ready", "Review it when you like: nothing changes until you apply it.");
      if (a.running) store.setState({ aiOffline: false, aiRefusal: null });
      // A new key, or none: what the provider said about the old one no longer holds.
      if (s.ai && s.ai.has_key !== a.has_key) store.setState({ aiRefusal: null });
      s.setAi(a);
    });
    const offE = backend.onAiError((m) => {
      const s = store.getState();
      const last = [...(s.project?.ai.calls ?? [])].sort((a, b) => b.started_ms - a.started_ms)[0];
      const spent = last ? `$${last.cost_usd.toFixed(2)}` : "$0.00";
      // 13f: no network — the assistant column says so in place; the toast stays for the details.
      if (/^network unavailable/i.test(m)) store.setState({ aiOffline: true });
      // 14i–14k: the provider refused for lack of credit, or no longer takes the key.
      const refusal = /account has no credit left/.test(m) ? "credit" : /key has reached its own credit limit/.test(m) ? "cap" : /\(401\): the API key was refused/.test(m) ? "revoked" : null;
      if (refusal) store.setState({ aiRefusal: refusal });
      if (m === "cancelled") s.toast("ai", "Assistant cancelled", `Nothing was changed. The run spent ${spent}, counted in AI usage.`);
      else s.toast("err", "The assistant could not answer", `${m}. ${spent} spent. Your mix is unchanged.`);
    });
    backend.aiStatus().then((a) => store.getState().setAi(a)).catch(() => {});
    // After the first paint: the cold start ends when the window shows a usable UI (NFR-3).
    requestAnimationFrame(() => void backend.uiReady().catch(() => {}));
    const offMs = backend.onMasterStatus((m) => store.getState().setMasterStatus(m));
    // UI-17: on the very first launch the tour starts by itself; never in the self-test, never on the mock unless
    // asked (the core and the mock both say so in `auto`).
    if (!autoAsked.has(backend)) {
      autoAsked.add(backend);
      backend.tourInfo().then((t) => t.auto && void beginTour(store)).catch(() => {});
    }
    backend.masterStatus().then((m) => store.getState().setMasterStatus(m)).catch(() => {});
    return () => {
      offP();
      offM();
      offA();
      offE();
      offMs();
    };
  }, [backend, store]);
  return (
    <StoreContext.Provider value={store}>
      <Root />
    </StoreContext.Provider>
  );
}
