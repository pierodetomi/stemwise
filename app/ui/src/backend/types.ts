// Contract between the UI and the core (FOUNDATION §5.6). Domain types are generated from Rust into
// ../bindings; this file declares the transport-level interface implemented by `tauri.ts` and `mock.ts`.
import type { MasterWave } from "../bindings/MasterWave";
import type { ChainOp } from "../bindings/ChainOp";
import type { Project } from "../bindings/Project";
import type { Phase } from "../bindings/Phase";
import type { HistoryEntry } from "../bindings/HistoryEntry";
import type { ImportCandidate } from "../bindings/ImportCandidate";
import type { ImportChoice } from "../bindings/ImportChoice";
import type { ParamSpec } from "../bindings/ParamSpec";
import type { RecentProject } from "../bindings/RecentProject";
import type { Edit } from "../bindings/Edit";

export type { Edit, Project, Phase, HistoryEntry, ImportCandidate, ImportChoice, ParamSpec, RecentProject };

export interface AppInfo {
  version: string;
  backend: "tauri" | "mock";
}

// Messages generated from Rust (stemwise-engine::dto): no hand-written copies (FOUNDATION §5.6).
export type { ProjectEvent } from "../bindings/ProjectEvent";
export type { TransportState } from "../bindings/TransportState";
export type { MeterSnapshot } from "../bindings/MeterSnapshot";
export type { DeviceInfo } from "../bindings/DeviceInfo";
export type { AudioPrefs } from "../bindings/AudioPrefs";
export type { TourInfo } from "../bindings/TourInfo";
import type { TourInfo } from "../bindings/TourInfo";
import type { ProjectEvent } from "../bindings/ProjectEvent";
import type { TransportState } from "../bindings/TransportState";
import type { MeterSnapshot } from "../bindings/MeterSnapshot";
import type { DeviceInfo } from "../bindings/DeviceInfo";
import type { AudioPrefs } from "../bindings/AudioPrefs";

export type { AiStatus } from "../bindings/AiStatus";
/** How a connection to the provider ended (AI-24). */
export type ConnectOutcome = "connected" | "denied" | "expired" | "offline" | "refused" | "local" | "cancelled";
export type ProviderPage = "home" | "keys" | "credits";
export type { ProposalView } from "../bindings/ProposalView";
export type { AiConfig } from "../bindings/AiConfig";
export type { CostEstimate } from "../bindings/CostEstimate";
export type { CallDetail } from "../bindings/CallDetail";
import type { AiStatus } from "../bindings/AiStatus";
import type { AiConfig } from "../bindings/AiConfig";
import type { CostEstimate } from "../bindings/CostEstimate";
import type { CallDetail } from "../bindings/CallDetail";

export type { MixLevels } from "../bindings/MixLevels";
import type { MixLevels } from "../bindings/MixLevels";
export type { MasterStatus } from "../bindings/MasterStatus";
import type { MasterStatus } from "../bindings/MasterStatus";

export type Unsubscribe = () => void;

export interface Backend {
  readonly kind: "tauri" | "mock";
  appInfo(): Promise<AppInfo>;
  /** NFR-3: tells the shell the UI is drawn; returns the cold-start time in ms (0 in the browser). */
  uiReady(): Promise<number>;
  recentProjects(): Promise<RecentProject[]>;
  /** Native folder pickers (mock: fixed paths). */
  pickFolder(title: string): Promise<string | null>;
  pickProject(): Promise<string | null>;
  pickAudioFiles(): Promise<string[]>;

  createProject(parent: string, name: string): Promise<void>;
  /** Returns true when an autosave from a crashed session can be recovered. */
  openProject(path: string): Promise<boolean>;
  recoverProject(accept: boolean): Promise<void>;
  closeProject(): Promise<void>;
  saveProject(): Promise<void>;
  /** Asks where to save a copy (Save As): the folder and the project's name, or null if cancelled. */
  pickSaveLocation(name: string): Promise<{ parent: string; name: string } | null>;
  /** Save As (PRJ-2): the copy becomes the open project; the original stays as it was. */
  saveProjectAs(parent: string, name: string): Promise<void>;
  getProject(): Promise<Project>;

  // Guided tour (P-020): the demo project lives in the session (PRJ-9), the "shown" mark in the app's data (UI-17).
  tourInfo(): Promise<TourInfo>;
  tourMarkShown(): Promise<void>;
  /** Closes the open project and opens the demo song (built and synthesised by the core); rejects if it can't. */
  tourStart(): Promise<void>;
  /** Step 8: the offline demo's proposal in review; false while it is still being made (it then arrives by itself). */
  tourProposal(): Promise<boolean>;
  /** UI-19: playback stops, the demo closes and is deleted. */
  tourEnd(): Promise<void>;
  onProject(cb: (e: ProjectEvent) => void): Unsubscribe;

  /** Files dragged over or dropped on the window (paths), or the drag left it. */
  /** Files dragged from the Finder over the window; `x`, `y` is the pointer in CSS pixels (absent on leave). */
  onFileDrop(cb: (e: FileDropEvent) => void): Unsubscribe;
  /** Probes files for the import screen. `adding`: to the open project (its rate); otherwise for a project not
   *  created yet, against `rate` (the first file already listed) or none. */
  importCandidates(paths: string[], adding: boolean, rate: number | null): Promise<ImportCandidate[]>;
  importFiles(choices: ImportChoice[]): Promise<void>;
  /** UI-6: a missing file located again; `true` when the clips now use another file (one undoable step). */
  relinkMedia(media: string, path: string): Promise<boolean>;
  /** 13c: a file's waveform overview became available (built in the background after opening). */
  onMediaReady(cb: (media: string) => void): Unsubscribe;

  paramSpecs(): Promise<ParamSpec[]>;
  setParam(track: string, param: string, value: number, label: string): Promise<void>;
  /** Opens a gesture in the mix history, or in the master history with `phase` "master" (HIS-5). */
  beginGesture(label: string, phase?: Phase): Promise<void>;
  gestureParam(track: string, param: string, value: number): Promise<void>;
  endGesture(cancel: boolean): Promise<void>;
  setView(track: string, what: "mute" | "solo", on: boolean): Promise<void>;
  /** One user edit as one history step; the core computes the commands (§5.6). */
  edit(e: Edit, label: string): Promise<void>;
  /** An edit inside the open gesture, recomputed from the gesture's start. */
  gestureEdit(e: Edit): Promise<void>;
  /** Shows or hides an automation lane under its track (view state, HIS-6). */
  setLaneVisible(track: string, param: string, visible: boolean): Promise<void>;

  undo(phase: Phase): Promise<string>;
  redo(phase: Phase): Promise<string>;
  history(phase: Phase): Promise<[HistoryEntry[], number]>;
  historyGoto(phase: Phase, position: number): Promise<void>;

  play(): Promise<void>;
  pause(): Promise<void>;
  stop(): Promise<void>;
  locate(frame: number): Promise<void>;
  setLoop(start: number | null, end: number | null): Promise<void>;
  transportState(): Promise<TransportState>;
  onMeters(cb: (m: MeterSnapshot) => void): Unsubscribe;
  /** The track whose spectrum the meters carry (FX-7); `null` stops it. */
  watchSpectrum(track: string | null): Promise<void>;

  /** ANA-20: the levels of the mix as it is, for the Explain view. `null` while they are being measured in the
   *  background: they then arrive through `onMixLevels`. Levels with nothing in them: there is nothing to measure. */
  mixLevels(): Promise<MixLevels | null>;
  onMixLevels(cb: (l: MixLevels) => void): Unsubscribe;

  audioDevices(): Promise<DeviceInfo[]>;
  audioPrefs(): Promise<AudioPrefs>;
  setAudioPrefs(p: AudioPrefs): Promise<void>;

  peaks(media: string, start: number, end: number, columns: number): Promise<[number, number][][]>;

  // Assistant (F7): calls never block the interface; progress and the proposal arrive as events (AI-12).
  aiStatus(): Promise<AiStatus>;
  onAiStatus(cb: (s: AiStatus) => void): Unsubscribe;
  onAiError(cb: (message: string) => void): Unsubscribe;
  aiEstimate(revision: boolean): Promise<CostEstimate>;
  /** Auto mix (no request) or revision (a request); `scope` limits the tracks (AI-5). "no-key" when no key is set. */
  aiStart(request: string | null, scope: string[] | null): Promise<void>;
  aiCancel(): Promise<void>;
  /** Applies the proposal, or only the given tracks; `trims` keeps the −23 LUFS input trims (AI-7). */
  aiApply(only: string[] | null, trims: boolean): Promise<string>;
  aiDiscard(): Promise<void>;
  /** AI-23: puts a proposal received earlier (a call with `stored`) back in review, on the mix as it is now. */
  aiReopen(id: string): Promise<void>;
  /** A/B: "current", "proposal" or "snapshot:<id>", without a history step (AI-2, HIS-4). */
  aiListen(what: string): Promise<void>;
  aiDetail(id: string): Promise<CallDetail>;
  /** Exports the call history after asking where (AI-14); false when cancelled. */
  aiExport(json: boolean): Promise<boolean>;
  aiSetConfig(c: AiConfig): Promise<void>;
  aiSetKey(key: string | null): Promise<void>;
  /** AI-24: opens the provider's approval page in the browser and waits for it to come back; the end arrives through
   *  `onAiConnect`. Rejects with a `ConnectOutcome` when it cannot even start. */
  aiConnectStart(): Promise<void>;
  aiConnectCancel(): Promise<void>;
  /** 14b: opens the approval page of the waiting connection again. */
  aiConnectReopen(): Promise<void>;
  onAiConnect(cb: (outcome: ConnectOutcome) => void): Unsubscribe;
  /** Opens one of the provider's pages in the browser: its home, the user's keys, the user's credit. */
  aiOpenPage(which: ProviderPage): Promise<void>;
  setSpendCap(usd: number | null): Promise<void>;
  setNormalizeTrim(on: boolean): Promise<void>;
  /** AI-18: whether the assistant may propose new returns and groups; kept in the project, not a history step. */
  setAllowStructure(on: boolean): Promise<void>;
  /** AI-27: whether the assistant may propose creative ideas (kept in the project, not a history step). */
  setAllowIdeas(on: boolean): Promise<void>;
  /** AI-27: the ideas ticked in the review; listening to the proposal follows them. */
  aiSetIdeas(ids: string[]): Promise<void>;
  snapshotSave(name: string): Promise<void>;
  snapshotRestore(id: string): Promise<void>;
  snapshotDelete(id: string): Promise<void>;
  pickReference(): Promise<string | null>;
  addReference(path: string): Promise<void>;
  removeReference(id: string): Promise<void>;

  // Master phase (F8).
  masterStatus(): Promise<MasterStatus>;
  onMasterStatus(cb: (s: MasterStatus) => void): Unsubscribe;
  /** Switches phase (UI-1); "no-mixdown" when Master has nothing to play yet (MDN-1). */
  setPhase(master: boolean): Promise<void>;
  /** "Try again" after the master could not be prepared (9j). */
  masterPrepareRetry(): Promise<void>;
  /** A/B at equal loudness (MAS-5): "master", "mixdown", "reference:<id>" or "proposal". */
  masterListen(what: string): Promise<void>;
  /** Same loudness (MAS-5) on or off; off is the real level (design 9l). */
  masterListenSame(on: boolean): Promise<void>;
  masterJobCancel(): Promise<void>;
  /** MAS-15, MAS-12: a block added, removed or moved, or the bands of a multiband changed; one master step. Answers
   *  with the id of the block it added. */
  masterChain(op: ChainOp): Promise<string | null>;
  /** Bands of a multiband heard alone (a bit per band); no block or no bit ends it. Listening only. */
  masterSolo(block: string | null, mask: number): Promise<void>;
  /** Hear only what a de-esser takes (FX-14): listening only, never saved. */
  listenBlock(track: string, block: string, on: boolean): Promise<void>;
  bounceStart(): Promise<void>;
  exportStart(): Promise<void>;
  masterAiStart(request: string | null): Promise<void>;
  /** Applies the master proposal, or only some of its entries (the keys of its `changes`). */
  masterAiApply(only: string[] | null): Promise<string>;
  masterAiDiscard(): Promise<void>;
  /** AI-23: the same for a master proposal; it is measured again before the review. */
  masterAiReopen(id: string): Promise<void>;
  mixdownPeaks(columns: number): Promise<[number, number][]>;
  /** The waveform of the master workspace: mixdown and measured master, (min, max, RMS) per column. */
  masterWave(columns: number): Promise<MasterWave>;
  pickImage(): Promise<string | null>;
  setCover(path: string | null): Promise<void>;
}

export type FileDropEvent = { type: "over" | "drop" | "leave"; paths: string[]; x?: number; y?: number };
