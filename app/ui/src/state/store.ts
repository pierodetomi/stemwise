// UI state: a read-only mirror of the core (FOUNDATION §5.3, §4.1 "zustand"), plus view state (HIS-6: selection,
// zoom, tool and scroll never enter the history).
import { create } from "zustand";
import { loadPrefs, savePrefs, type UiPrefs } from "./prefs";
import { specKey } from "../mix/fx";
import type { MixLevels } from "../bindings/MixLevels";
import type { AiStatus, MasterStatus, Backend, MeterSnapshot, ParamSpec, Project, ProjectEvent, TransportState } from "../backend/types";

export type Tool = "select" | "split" | "auto";
/** Blocks of the inspector's chain strip. */
/** What the inspector has open: a block of the chain (its id), the trim, the fader or the sends (UI-12). */
export type InspBlock = string;
/** Default mixer height (design token --mixer-h): its strips carry the chain marks (P-013). */
export const MIXER_H = 304;
/** Same, in a window under 800 px high (design 3d, token --mixer-h-short). */
export const MIXER_H_SHORT = 250;
export type LaneSize = "s" | "m" | "l";
/** A notification (13h); one with an action stays until dismissed or acted on. */
export type Toast = { id: number; kind: "ok" | "warn" | "err" | "ai" | "info"; title: string; text: string; action?: { label: string; run: () => void } };

/** The guided tour (P-020, UI-17 → UI-19): the step on screen and the state of its demo song. */
export interface TourState {
  step: number;
  /** The demo song: being built by the core, open, or it could not be prepared (design 18w). */
  demo: "building" | "ready" | "failed";
  error?: string;
  /** What the tour changes and gives back at the end (UI-19): panels, mixer, views. */
  saved: Partial<ViewState>;
}
/** A clip reference as the UI keys it: `track/clip`. */
export type ClipKey = string;
export const clipKey = (track: string, clip: string): ClipKey => `${track}/${clip}`;
export const splitKey = (k: ClipKey): [string, string] => {
  const i = k.indexOf("/");
  return [k.slice(0, i), k.slice(i + 1)];
};

export interface DialogState {
  title: string;
  sub?: string;
  confirm: string;
  danger?: boolean;
  onConfirm: () => void;
  /** A third choice, between Cancel and the confirmation (e.g. "Don't save"). */
  alt?: { label: string; danger?: boolean; onSelect: () => void };
}

export interface ViewState {
  selTrack: string | null;
  /** Tracks selected together (⌘-click, ⇧-click): grouped or dragged as one; `selTrack` is the last one clicked. */
  selTracks: string[];
  selClips: ClipKey[];
  selSection: string | null;
  tool: Tool;
  snap: boolean;
  /** Horizontal zoom in pixels per second (TRN-3). */
  pps: number;
  lane: LaneSize;
  scrollX: number;
  follow: boolean;
  loop: [number, number] | null;
  loopOn: boolean;
  asstTab: number;
  /** Folded groups in the timeline. */
  folded: string[];
  /** Collapsible panels (UI-4). */
  hideAssistant: boolean;
  hideInspector: boolean;
  hideMixer: boolean;
  mixerH: number;
  /** The mixer's view (UI-11): faders, or the Overview grid on the same columns. */
  mixerView: "faders" | "grid";
  /** Block of the selected track open in the inspector. */
  inspBlock: InspBlock;
  screen: "welcome" | "new" | "workspace" | "usage" | "settings";
  /** Where Settings returns to, and the section it shows (design 11). */
  settingsBack: "welcome" | "new" | "workspace" | "usage";
  settingsTab: "audio" | "ai" | "appearance" | "shortcuts";
  /** Call shown in AI usage (12b), and the readable or raw view of it. */
  usageCall: string | null;
  renameTrack: string | null;
  dialog: DialogState | null;
  /** Project settings (name, brief, references) are open. */
  projectOpen: boolean;
  /** The assistant setup is open (design 14), and where it starts: connecting, pasting a key, or the credit step. */
  setup: null | "connect" | "manual" | "credit";
  bounceOpen: boolean;
  exportOpen: boolean;
  /** What the master's editor shows: "input", "limiter", "dither", or the id of a free block of the chain. */
  masterModule: string;
  /** A section whose offsets are open in the master's editor instead of a block (MAS-14); `null`: none. */
  masterSection: string | null;
  /** A request written for the mix assistant by another screen (15g): the request field takes it, once. */
  assistDraft: string | null;
  /** Window narrower than 1360 px (design: compact layout, 3d). */
  compact: boolean;
  /** The Explain view takes the workspace (UI-14, design 17). */
  explain: boolean;
  /** What it explains: a track, group or return, or every section of a name; neither: the whole mix. */
  explainSel: string | null;
  explainSection: string | null;
  /** Groups folded in its map (UI-15). */
  explainFolded: string[];
  /** ANA-20: the levels of the mix as it is; `null` while the core measures them. */
  mixLevels: MixLevels | null;
  /** The guided tour, while it runs. */
  tour: TourState | null;
}

export interface AppState extends ViewState {
  backend: Backend;
  specs: Map<string, ParamSpec>;
  project: Project | null;
  path: string;
  revision: number;
  canUndo: boolean;
  canRedo: boolean;
  dirty: boolean;
  savedAt: number | null;
  /** Media missing from the package (UI-6, 13d). */
  missing: string[];
  /** The open project is the tour's demo (PRJ-9): not saved, not exported. */
  demo: boolean;
  /** The last assistant call failed for lack of network (13f), until the next one starts. */
  aiOffline: boolean;
  /** The provider refused the last call: no credit (14i), the key's own limit (14j), a key it no longer accepts (14k). */
  aiRefusal: null | "credit" | "cap" | "revoked";
  transport: TransportState | null;
  meters: MeterSnapshot | null;
  error: string | null;
  toasts: Toast[];
  ai: AiStatus | null;
  setAi(s: AiStatus): void;
  masterStatus: MasterStatus | null;
  setMasterStatus(s: MasterStatus): void;
  setProjectEvent(e: ProjectEvent): void;
  setMeters(m: MeterSnapshot): void;
  setTransport(t: TransportState): void;
  setError(e: string | null): void;
  setSpecs(s: ParamSpec[]): void;
  toast(kind: Toast["kind"], title: string, text?: string, action?: Toast["action"]): void;
  dismissToast(id: number): void;
  setView(v: Partial<ViewState>): void;
  prefs: UiPrefs;
  setPrefs(p: Partial<UiPrefs>): void;
  openSettings(tab?: ViewState["settingsTab"]): void;
  closeProject(): void;
}

let toastId = 0;

export function createAppStore(backend: Backend) {
  return create<AppState>((set, get) => ({
    backend,
    specs: new Map(),
    project: null,
    path: "",
    revision: 0,
    canUndo: false,
    canRedo: false,
    dirty: false,
    savedAt: null,
    missing: [],
    demo: false,
    tour: null,
    aiOffline: false,
    aiRefusal: null,
    transport: null,
    meters: null,
    error: null,
    toasts: [],
    selTrack: null,
    selTracks: [],
    selClips: [],
    selSection: null,
    tool: "select",
    snap: true,
    pps: 0,
    lane: "m",
    scrollX: 0,
    follow: true,
    loop: null,
    loopOn: false,
    asstTab: 0,
    folded: [],
    hideAssistant: false,
    hideInspector: false,
    hideMixer: false,
    mixerH: MIXER_H,
    mixerView: "faders",
    inspBlock: "eq",
    screen: "welcome",
    settingsBack: "welcome",
    settingsTab: "audio",
    prefs: loadPrefs(),
    setPrefs: (p) => {
      const prefs = { ...get().prefs, ...p };
      savePrefs(prefs);
      set({ prefs });
    },
    openSettings: (tab) => {
      const s = get();
      if (s.screen === "settings") {
        if (tab) set({ settingsTab: tab });
        return;
      }
      set({ screen: "settings", settingsBack: s.screen, ...(tab ? { settingsTab: tab } : {}) });
    },
    usageCall: null,
    ai: null,
    setAi: (ai) => set({ ai }),
    masterStatus: null,
    setMasterStatus: (masterStatus) => set({ masterStatus }),
    renameTrack: null,
    dialog: null,
    compact: false,
    explain: false,
    explainSel: null,
    explainSection: null,
    explainFolded: [],
    mixLevels: null,
    projectOpen: false,
    setup: null,
    bounceOpen: false,
    exportOpen: false,
    masterModule: "limiter",
    masterSection: null,
    assistDraft: null,
    setProjectEvent: (e) =>
      set((s) => {
        if (e.revision < s.revision && s.project) return s;
        const first = !s.project || s.path !== e.path;
        const tracks = e.project.tracks;
        const selTrack =
          s.selTrack && tracks.some((t) => t.id === s.selTrack) ? s.selTrack : (tracks.find((t) => t.kind === "audio")?.id ?? null);
        const alive = new Set(tracks.flatMap((t) => t.clips.map((c) => clipKey(t.id, c.id))));
        return {
          project: e.project,
          revision: e.revision,
          canUndo: e.canUndo,
          canRedo: e.canRedo,
          dirty: e.dirty,
          savedAt: !e.dirty && (s.dirty || first) ? Date.now() : s.savedAt,
          path: e.path,
          missing: e.missing,
          demo: e.demo,
          selTrack,
          selTracks: s.selTracks.filter((id) => tracks.some((t) => t.id === id)),
          selClips: s.selClips.filter((k) => alive.has(k)),
          // The tour's opening card stays on the Welcome screen while its demo opens behind it (design 18a).
          screen: s.tour && s.tour.step === 1 ? s.screen : s.screen === "new" && tracks.some((t) => t.kind === "audio") ? "workspace" : s.screen === "welcome" ? "workspace" : s.screen,
          // A changed mix has no levels until the core has measured it again (ANA-20).
          mixLevels: null,
          ...(first ? { explain: false, explainSel: null, explainSection: null, explainFolded: [] } : {}),
          ...(first ? { pps: 0, scrollX: 0, lane: window.innerHeight < 800 ? "s" : get().prefs.lane, mixerH: window.innerHeight < 800 ? MIXER_H_SHORT : MIXER_H } : {}),
        };
      }),
    setMeters: (meters) => set({ meters }),
    setTransport: (transport) => set({ transport }),
    setError: (error) => {
      set({ error });
      if (error) get().toast("err", "Something went wrong", error);
    },
    setSpecs: (s) => set({ specs: new Map(s.map((x) => [x.id, x])) }),
    toast: (kind, title, text = "", action) => {
      // The tour sits above notifications: what happens to its demo is not news (design token --z-tour).
      if (get().tour) return;
      const id = ++toastId;
      set((s) => ({ toasts: [...s.toasts.slice(-3), { id, kind, title, text, action }] }));
      if (kind !== "err" && !action) setTimeout(() => get().dismissToast(id), 6000);
    },
    dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
    // Selecting one track (anywhere) makes it the whole track selection, unless the caller sets both.
    setView: (v) => set("selTrack" in v && !("selTracks" in v) ? { ...v, selTracks: v.selTrack ? [v.selTrack] : [] } : v),
    closeProject: () => set({ explain: false, mixLevels: null, project: null, demo: false, revision: 0, meters: null, screen: "welcome", selClips: [], selTrack: null, selTracks: [] }),
  }));
}

export type AppStore = ReturnType<typeof createAppStore>;

/** The spec of a parameter id; `send.<id>` shares one spec. */
export function specOf(specs: Map<string, ParamSpec>, id: string): ParamSpec {
  const s = specs.get(id.startsWith("send.") ? "send.<return>" : specKey(id));
  if (s) return { ...s, id };
  // The mix bus fader is not a track parameter: same range as a track fader.
  const f = specs.get("fader");
  return { id, label: id, min: f?.min ?? -144, max: f?.max ?? 12, default: 0, unit: "db", log: false, stepped: false };
}
