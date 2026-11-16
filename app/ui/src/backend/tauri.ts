import { invoke as rawInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { baseName, parentDir } from "../shared/path";
import type { MasterWave } from "../bindings/MasterWave";
import type { ChainOp } from "../bindings/ChainOp";
import type { MixLevels } from "../bindings/MixLevels";
import type {
  AiConfig, AiStatus, AppInfo, ConnectOutcome, ProviderPage, MasterStatus, AudioPrefs, Backend, CallDetail, CostEstimate, DeviceInfo, Edit, FileDropEvent, HistoryEntry, ImportCandidate, ImportChoice, MeterSnapshot, ParamSpec,
  Phase, Project, ProjectEvent, RecentProject, TourInfo, TransportState, Unsubscribe,
} from "./types";

function sub<T>(event: string, cb: (p: T) => void): Unsubscribe {
  let off: (() => void) | null = null;
  let dead = false;
  void listen<T>(event, (e) => cb(e.payload)).then((u) => (dead ? u() : (off = u)));
  return () => {
    dead = true;
    off?.();
  };
}

// Commands go out one after the other: a gesture's begin, values and end must reach the core in order.
let chain: Promise<unknown> = Promise.resolve();
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const p = chain.then(() => rawInvoke<T>(cmd, args));
  chain = p.catch(() => undefined);
  return p;
}

// Backend backed by the Rust core through Tauri commands and events (FOUNDATION §5.6).
export class TauriBackend implements Backend {
  readonly kind = "tauri" as const;
  appInfo = () => invoke<AppInfo>("app_info");
  uiReady = () => invoke<number>("ui_ready");
  recentProjects = () => invoke<RecentProject[]>("recent_projects");
  async pickFolder(title: string) {
    const r = await open({ directory: true, title });
    return typeof r === "string" ? r : null;
  }
  async pickProject() {
    const r = await open({ directory: true, title: "Open a Stemwise project (.stemwise)" });
    return typeof r === "string" ? r : null;
  }
  async pickAudioFiles() {
    const r = await open({ multiple: true, filters: [{ name: "Audio", extensions: ["wav", "flac", "aif", "aiff"] }] });
    return Array.isArray(r) ? r : r ? [r] : [];
  }
  createProject = (parent: string, name: string) => invoke<void>("create_project", { parent, name });
  openProject = (path: string) => invoke<boolean>("open_project", { path });
  recoverProject = (accept: boolean) => invoke<void>("recover_project", { accept });
  closeProject = () => invoke<void>("close_project");
  saveProject = () => invoke<void>("save_project");
  async pickSaveLocation(name: string) {
    const r = await save({ title: "Save a copy of the project", defaultPath: `${name}.stemwise` });
    if (!r) return null;
    const file = baseName(r);
    return { parent: parentDir(r), name: file.replace(/\.stemwise$/i, "") };
  }
  saveProjectAs = (parent: string, name: string) => invoke<void>("save_project_as", { parent, name });
  tourInfo = () => invoke<TourInfo>("tour_info");
  tourMarkShown = () => invoke<void>("tour_mark_shown");
  // Building the demo takes a moment: it does not hold up the commands that follow (a Skip, the status).
  tourStart = () => rawInvoke<void>("tour_start");
  tourProposal = () => invoke<boolean>("tour_proposal");
  tourEnd = () => invoke<void>("tour_end");
  getProject = () => invoke<Project>("get_project");
  onProject = (cb: (e: ProjectEvent) => void) => sub("project-changed", cb);
  onMediaReady = (cb: (media: string) => void) => sub("media-ready", cb);
  onFileDrop(cb: (e: FileDropEvent) => void): Unsubscribe {
    let off: (() => void) | null = null;
    let dead = false;
    // The webview reports the pointer in physical pixels: the screens compare it with element rectangles in CSS px.
    const at = (p: { position: { x: number; y: number } }) => ({ x: p.position.x / window.devicePixelRatio, y: p.position.y / window.devicePixelRatio });
    let paths: string[] = [];
    void getCurrentWebview()
      .onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "enter") paths = p.paths;
        if (p.type === "enter" || p.type === "over") cb({ type: "over", paths, ...at(p) });
        else if (p.type === "drop") cb({ type: "drop", paths: p.paths, ...at(p) });
        else cb({ type: "leave", paths: [] });
      })
      .then((u) => (dead ? u() : (off = u)));
    return () => {
      dead = true;
      off?.();
    };
  }
  importCandidates = (paths: string[], adding: boolean, rate: number | null) =>
    invoke<ImportCandidate[]>("import_candidates", { paths, adding, rate });
  importFiles = (choices: ImportChoice[]) => invoke<void>("import_files", { choices });
  relinkMedia = (media: string, path: string) => invoke<boolean>("relink_media", { media, path });
  paramSpecs = () => invoke<ParamSpec[]>("param_specs");
  setParam = (track: string, param: string, value: number, label: string) =>
    invoke<void>("set_param", { track, param, value, label });
  beginGesture = (label: string, phase?: Phase) => invoke<void>("begin_gesture", { label, phase: phase ?? null });
  gestureParam = (track: string, param: string, value: number) => invoke<void>("gesture_param", { track, param, value });
  endGesture = (cancel: boolean) => invoke<void>("end_gesture", { cancel });
  setView = (track: string, what: "mute" | "solo", on: boolean) => invoke<void>("set_view", { track, what, on });
  edit = (edit: Edit, label: string) => invoke<void>("edit", { edit, label });
  gestureEdit = (edit: Edit) => invoke<void>("gesture_edit", { edit });
  setLaneVisible = (track: string, param: string, visible: boolean) =>
    invoke<void>("set_lane_visible", { track, param, visible });
  undo = (phase: Phase) => invoke<string>("undo", { phase });
  redo = (phase: Phase) => invoke<string>("redo", { phase });
  history = (phase: Phase) => invoke<[HistoryEntry[], number]>("history", { phase });
  historyGoto = (phase: Phase, position: number) => invoke<void>("history_goto", { phase, position });
  play = () => invoke<void>("play");
  pause = () => invoke<void>("pause");
  stop = () => invoke<void>("stop");
  locate = (frame: number) => invoke<void>("locate", { frame: Math.max(0, Math.round(frame)) });
  setLoop = (start: number | null, end: number | null) => invoke<void>("set_loop", { start, end });
  transportState = () => invoke<TransportState>("transport_state");
  onMeters = (cb: (m: MeterSnapshot) => void) => sub("meters", cb);
  watchSpectrum = (track: string | null) => invoke<void>("watch_spectrum", { track });
  audioDevices = () => invoke<DeviceInfo[]>("audio_devices");
  audioPrefs = () => invoke<AudioPrefs>("audio_prefs");
  setAudioPrefs = (prefs: AudioPrefs) => invoke<void>("set_audio_prefs", { prefs });
  peaks = (media: string, start: number, end: number, columns: number) =>
    rawInvoke<[number, number][][]>("peaks", { media, start, end, columns });

  aiStatus = () => rawInvoke<AiStatus>("ai_status");
  onAiStatus = (cb: (s: AiStatus) => void) => sub("ai-status", cb);
  onAiError = (cb: (m: string) => void) => sub("ai-error", cb);
  aiEstimate = (revision: boolean) => invoke<CostEstimate>("ai_estimate", { revision });
  aiStart = (request: string | null, scope: string[] | null) => invoke<void>("ai_start", { request, scope });
  aiCancel = () => rawInvoke<void>("ai_cancel");
  aiApply = (only: string[] | null, trims: boolean) => invoke<string>("ai_apply", { only, trims });
  aiDiscard = () => invoke<void>("ai_discard");
  aiReopen = (id: string) => invoke<void>("ai_reopen", { id });
  aiListen = (what: string) => invoke<void>("ai_listen", { what });
  aiDetail = (id: string) => invoke<CallDetail>("ai_detail", { id });
  async aiExport(json: boolean) {
    const path = await save({ title: "Export AI usage", defaultPath: json ? "ai-usage.json" : "ai-usage.csv", filters: [{ name: json ? "JSON" : "CSV", extensions: [json ? "json" : "csv"] }] });
    if (!path) return false;
    await invoke<void>("ai_export", { path, json });
    return true;
  }
  aiSetConfig = (config: AiConfig) => invoke<void>("ai_set_config", { config });
  aiSetKey = (key: string | null) => invoke<void>("ai_set_key", { key });
  aiConnectStart = () => rawInvoke<void>("ai_connect_start");
  aiConnectCancel = () => rawInvoke<void>("ai_connect_cancel");
  aiConnectReopen = () => rawInvoke<void>("ai_connect_reopen");
  onAiConnect = (cb: (o: ConnectOutcome) => void) => sub("ai-connect", cb);
  aiOpenPage = (which: ProviderPage) => rawInvoke<void>("ai_open_page", { which });
  setSpendCap = (usd: number | null) => invoke<void>("set_spend_cap", { usd });
  setNormalizeTrim = (on: boolean) => invoke<void>("set_normalize_trim", { on });
  setAllowStructure = (on: boolean) => invoke<void>("set_allow_structure", { on });
  setAllowIdeas = (on: boolean) => invoke<void>("set_allow_ideas", { on });
  aiSetIdeas = (ids: string[]) => invoke<void>("ai_set_ideas", { ideas: ids });
  snapshotSave = (name: string) => invoke<void>("snapshot_save", { name });
  snapshotRestore = (id: string) => invoke<void>("snapshot_restore", { id });
  snapshotDelete = (id: string) => invoke<void>("snapshot_delete", { id });
  async pickReference() {
    const r = await open({ title: "Add a reference track", filters: [{ name: "Audio", extensions: ["wav", "flac", "aif", "aiff", "mp3", "m4a", "aac"] }] });
    return typeof r === "string" ? r : null;
  }
  addReference = (path: string) => invoke<void>("add_reference", { path });
  removeReference = (id: string) => invoke<void>("remove_reference", { id });

  mixLevels = () => rawInvoke<MixLevels | null>("mix_levels");
  onMixLevels = (cb: (l: MixLevels) => void) => sub("mix-levels", cb);
  masterStatus = () => rawInvoke<MasterStatus>("master_status");
  onMasterStatus = (cb: (s: MasterStatus) => void) => sub("master-status", cb);
  setPhase = (master: boolean) => invoke<void>("set_phase", { master });
  masterPrepareRetry = () => invoke<void>("master_prepare_retry");
  masterListen = (what: string) => invoke<void>("master_listen", { what });
  masterListenSame = (on: boolean) => invoke<void>("master_listen_same", { on });
  masterChain = (op: ChainOp) => invoke<string | null>("master_chain", { op });
  masterSolo = (block: string | null, mask: number) => invoke<void>("master_solo", { block, mask });
  listenBlock = (track: string, block: string, on: boolean) => invoke<void>("listen_block", { track, block, on });
  masterJobCancel = () => rawInvoke<void>("master_job_cancel");
  bounceStart = () => invoke<void>("bounce_start");
  exportStart = () => invoke<void>("export_start");
  masterAiStart = (request: string | null) => invoke<void>("master_ai_start", { request });
  masterAiApply = (only: string[] | null) => invoke<string>("master_ai_apply", { only });
  masterAiDiscard = () => invoke<void>("master_ai_discard");
  masterAiReopen = (id: string) => invoke<void>("master_ai_reopen", { id });
  mixdownPeaks = (columns: number) => rawInvoke<[number, number][]>("mixdown_peaks", { columns });
  masterWave = (columns: number) => rawInvoke<MasterWave>("master_wave", { columns });
  async pickImage() {
    const r = await open({ title: "Cover image", filters: [{ name: "Image", extensions: ["jpg", "jpeg", "png"] }] });
    return typeof r === "string" ? r : null;
  }
  setCover = (path: string | null) => invoke<void>("set_cover", { path });
}
