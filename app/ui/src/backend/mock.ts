import type { ChainOp } from "../bindings/ChainOp";
import { mockLevels } from "./mockLevels";
import type { MixLevels } from "./types";
import type {
  AppInfo, AudioPrefs, Backend, DeviceInfo, Edit, HistoryEntry, ImportCandidate, ImportChoice, MeterSnapshot, ParamSpec,
  Phase, Project, ProjectEvent, RecentProject, TourInfo, TransportState, Unsubscribe,
} from "./types";
import { DEMO_RATE, demoPeaks, demoProject } from "./demo";
import specs from "../bindings/param_specs.json";
import { runEdit, setParam } from "./mockEdit";
import { MockAi } from "./mockAi";
import { MockMaster } from "./mockMaster";
import type { AiConfig, AiStatus, CallDetail, ConnectOutcome, FileDropEvent, MasterStatus, ProviderPage } from "./types";

type Step = { label: string; before: string; after: string; aiCall?: string };

// Generated from the core (`make schemas`): the mock never redefines parameter ranges.
const SPECS = specs as ParamSpec[];

// In-memory backend used by the browser build, Playwright and visual checks (FOUNDATION §5.6). It mirrors the
// core's behaviour coarsely (one history step per change, transport clock, simulated meters); it is not the core.
export class MockBackend implements Backend {
  readonly kind = "mock" as const;
  private project: Project | null = null;
  private revision = 0;
  private done: Step[] = [];
  private undone: Step[] = [];
  private gesture: { label: string; before: string } | null = null;
  private listeners = new Set<(e: ProjectEvent) => void>();
  private meterListeners = new Set<(m: MeterSnapshot) => void>();
  private transport = { playing: false, frame: 0, startedAt: 0, startFrame: 0, playFrom: 0 };
  private loop: [number, number] | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;

  async uiReady() {
    return 0;
  }
  async appInfo(): Promise<AppInfo> {
    return { version: "0.1.0", backend: "mock" };
  }

  /** Test hooks: a first launch (no recent projects, no key), and the provider refusing the next call. */
  private firstLaunch = false;
  simulateFirstLaunch() {
    this.firstLaunch = true;
    this.ai.setKey(null);
  }
  private refusal: string | null = null;
  simulateRefusal(kind: "credit" | "cap" | "revoked" | null) {
    this.refusal =
      kind === "credit" ? "the provider answered with an error (402): the OpenRouter account has no credit left: Insufficient credits"
      : kind === "cap" ? "the provider answered with an error (402): the key has reached its own credit limit: Key limit exceeded"
      : kind === "revoked" ? "the provider answered with an error (401): the API key was refused: No auth credentials found"
      : null;
  }

  async recentProjects(): Promise<RecentProject[]> {
    if (this.firstLaunch) return [];
    const day = 86_400_000;
    const now = Date.UTC(2026, 10, 11);
    return [
      { path: "/Users/demo/Music/Confidence.stemwise", name: "Confidence", opened_ms: now - day },
      { path: "/Users/demo/Music/Low Tide.stemwise", name: "Low Tide", opened_ms: now - 4 * day },
      { path: "/Users/demo/Music/Paper Moons.stemwise", name: "Paper Moons", opened_ms: now - 12 * day },
    ];
  }

  async pickFolder() {
    return "/Users/demo/Music";
  }
  async pickProject() {
    return "/Users/demo/Music/Confidence.stemwise";
  }
  async pickAudioFiles() {
    return ["01 Kick In.wav", "02 Snare Top.wav", "03 OH.wav", "05 Bass DI.wav", "07 Gtr L.wav", "10 Strings.wav", "11 Lead Vox.wav"].map((f) => `/Users/demo/Stems/${f}`);
  }

  // ---------- simulated states for the tests (UI-6, design 13d, 13f) ----------
  private missingMedia = new Set<string>();
  private deviceDown = false;
  private networkDown = false;
  /** Marks the first `n` media of the open project as missing from the package. */
  simulateMissing(n: number) {
    this.missingMedia = new Set((this.project?.media ?? []).slice(0, n).map((m) => m.id));
    this.emit();
  }
  /** The master's preparation (9d–9j): how long each step lasts, and a failure to simulate on the next one. */
  simulatePrep(stepMs: number, failure: "mixdown_unreadable" | "internal" | "reference" | null = null, remeasureMs = 0) {
    this.master.prepStepMs = stepMs;
    this.master.prepFailure = failure;
    this.master.remeasureMs = remeasureMs;
  }
  /** The references the master compares with (boards 9p–9u). */
  simulateReferences(refs: { id: string; name: string; lufs: number; true_peak_db?: number }[]) {
    this.master.setReferences(refs);
  }
  /** Copies audio tracks of the open project until it has `n` (NFR-3: the UI with 32 tracks). */
  simulateTracks(n: number) {
    if (!this.project) return;
    const audio = this.project.tracks.filter((t) => t.kind === "audio");
    const extra = [];
    for (let k = audio.length; k < n; k++) {
      const src = audio[k % audio.length];
      const id = `tx${k}`;
      extra.push({ ...structuredClone(src), id, name: `${src.name} ${k}`, clips: src.clips.map((c, i) => ({ ...c, id: `${id}c${i}` })), sends: [] });
    }
    const at = this.project.tracks.findIndex((t) => t.kind === "return");
    const tracks = [...this.project.tracks];
    tracks.splice(at < 0 ? tracks.length : at, 0, ...extra);
    this.project = { ...this.project, tracks };
    this.emit();
  }
  /** Adds returns until the project has `n` (UI-9: more than two returns, design 3f); a few tracks send to them. */
  simulateReturns(n: number) {
    if (!this.project) return;
    const p = structuredClone(this.project);
    const names = ["Slap", "Throw", "Hall", "Chorus", "Room Verb", "Long Echo"];
    const audio = p.tracks.filter((t) => t.kind === "audio");
    for (let k = p.tracks.filter((t) => t.kind === "return").length, i = 0; k < n; k++, i++) {
      const id = `rx${k}`;
      const src = p.tracks.find((t) => t.kind === "return" && t.ret?.type === (i % 2 ? "delay" : "reverb"))!;
      p.tracks.push({ ...structuredClone(src), id, name: names[i % names.length] });
      [audio[(i * 5 + 3) % audio.length], audio[(i * 7 + 10) % audio.length]].forEach((t, j) => t.sends.push({ target: id, level_db: -16 - 3 * j - i }));
    }
    this.project = p;
    this.emit();
  }
  /** Empties the open project (13a): no tracks, no media, no sections, no rate yet. */
  simulateEmpty() {
    if (!this.project) return;
    this.project = { ...this.project, tracks: [], media: [], sections: [], sample_rate: null };
    this.emit();
  }
  simulateDisconnect(down: boolean) {
    this.deviceDown = down;
    if (down) this.transport = { ...this.transport, playing: false };
  }
  simulateNetworkDown(down: boolean) {
    this.networkDown = down;
  }
  async relinkMedia(media: string, path: string) {
    if (!this.missingMedia.has(media)) throw new Error("that file is not missing");
    if (!path) throw new Error("no file chosen");
    this.missingMedia.delete(media);
    this.emit();
    return false;
  }

  /** Changes since the last save, and where the project is (Save As moves it). */
  private dirty = false;
  private folder = "/Users/demo/Music";
  private emit() {
    if (!this.project) return;
    this.revision++;
    const e: ProjectEvent = {
      revision: this.revision,
      project: this.project,
      canUndo: this.done.length > 0,
      canRedo: this.undone.length > 0,
      dirty: this.dirty,
      path: `${this.folder}/${this.project.name}.stemwise`,
      missing: [...this.missingMedia].filter((id) => this.project!.media.some((m) => m.id === id)),
      demo: this.demo,
    };
    this.listeners.forEach((l) => l(e));
  }

  private mutate(label: string, f: (p: Project) => void, aiCall?: string) {
    if (!this.project) throw new Error("no project is open");
    const before = JSON.stringify(this.project);
    const next = JSON.parse(before) as Project;
    f(next);
    this.project = next;
    if (!this.gesture) {
      this.done.push({ label, before, after: JSON.stringify(next), aiCall });
      this.undone = [];
    }
    this.dirty = true;
    if (JSON.stringify(JSON.parse(before).tracks) !== JSON.stringify(next.tracks)) this.master?.mixChanged();
    const was = JSON.parse(before) as Project;
    if (JSON.stringify(was.master.chain) !== JSON.stringify(next.master.chain)) this.master?.masterChanged();
    if (!this.gesture && (was.master.target.lufs !== next.master.target.lufs || was.master.target.true_peak_db !== next.master.target.true_peak_db)) this.master?.targetChanged(was.master.target);
    this.emit();
    if (this.master?.status.phase === "master") this.master.emit();
  }

  /** A change outside the history (call log, snapshot list: HIS-6-like view state). */
  private viewChange(f: (p: Project) => void) {
    if (!this.project) return;
    const next = structuredClone(this.project);
    f(next);
    this.project = next;
    this.emit();
  }

  private master = new MockMaster(
    { project: () => this.project, mutate: (l, f, a) => this.mutate(l, f, a), view: (f) => this.viewChange(f) },
    (m) => this.ai.fail(m),
  );

  private ai = new MockAi({
    project: () => this.project,
    mutate: (label, f, aiCall) => this.mutate(label, f, aiCall),
    view: (f) => this.viewChange(f),
  });

  async createProject(_parent: string, name: string) {
    const p = demoProject();
    p.name = name;
    p.tracks = p.tracks.filter((t) => t.kind === "return");
    p.media = [];
    p.sections = [];
    p.sample_rate = null;
    this.install(p);
  }

  async openProject(path: string) {
    const p = demoProject();
    const name = path.split("/").pop()?.replace(/\.stemwise$/, "");
    if (name) p.name = name;
    this.install(p);
    return false;
  }

  private install(p: Project, demo = false) {
    this.demo = demo;
    this.project = p;
    this.dirty = false;
    this.folder = "/Users/demo/Music";
    this.done = [];
    this.undone = [];
    this.transport = { playing: false, frame: 0, startedAt: 0, startFrame: 0, playFrom: 0 };
    this.emit();
  }

  async recoverProject() {}
  async closeProject() {
    this.project = null;
    this.dirty = false;
    this.demo = false;
  }
  async saveProject() {
    if (!this.project) throw new Error("no project is open");
    if (this.demo) throw new Error("the demo song of the tour is not saved and cannot be exported");
    this.dirty = false;
    this.emit();
  }
  async pickSaveLocation(name: string) {
    return { parent: "/Users/demo/Music/Copies", name };
  }
  async saveProjectAs(parent: string, name: string) {
    if (!this.project) throw new Error("no project is open");
    if (this.demo) throw new Error("the demo song of the tour is not saved and cannot be exported");
    this.project = { ...this.project, name };
    this.folder = parent;
    this.dirty = false;
    this.emit();
  }
  // ---------- guided tour (P-020): the demo song the mock already has stands in for the core's (PRJ-9) ----------
  private demo = false;
  /** Test hooks: the tour starts by itself only when asked (`?tour`), so the other tests never see it. */
  tourAuto = false;
  tourShown = false;
  /** How long the demo takes to build, and whether it fails (design 18w). */
  tourBuildMs = 300;
  tourFails = false;
  /** Every tour that ended: the demo was closed and deleted. */
  toursEnded = 0;
  private tourEpoch = 0;
  async tourInfo(): Promise<TourInfo> {
    return { shown: this.tourShown, auto: this.tourAuto && !this.tourShown };
  }
  async tourMarkShown() {
    this.tourShown = true;
  }
  async tourStart() {
    this.ai.reset();
    this.project = null;
    this.demo = false;
    const epoch = this.tourEpoch;
    await new Promise((r) => setTimeout(r, this.tourBuildMs));
    if (epoch !== this.tourEpoch) throw new Error("cancelled");
    if (this.tourFails) throw new Error("the demo song could not be written: No space left on device");
    const p = demoProject();
    p.name = "Demo song";
    p.references = [];
    // As the core's: no proposal received yet, nothing applied (PRJ-9).
    p.ai = { ...p.ai, calls: [], notes: [], summary: null };
    this.install(p, true);
    this.master.reset();
  }
  async tourProposal() {
    if (!this.demo) throw new Error("no tour is running");
    if (this.ai.status.proposal) return true;
    if (!this.ai.status.running) this.ai.start(null, null, true);
    return false;
  }
  async tourEnd() {
    this.tourEpoch++;
    this.transport.playing = false;
    if (this.demo) {
      this.ai.reset();
      this.master.reset();
      this.project = null;
      this.demo = false;
      this.dirty = false;
      this.toursEnded++;
    }
  }

  async getProject() {
    if (!this.project) throw new Error("no project is open");
    this.emit();
    return this.project;
  }

  onProject(cb: (e: ProjectEvent) => void): Unsubscribe {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  private dropListeners = new Set<(e: FileDropEvent) => void>();
  onMediaReady(cb: (media: string) => void): Unsubscribe {
    // The demo's peaks are computed on request: nothing to wait for.
    void cb;
    return () => {};
  }
  onFileDrop(cb: (e: FileDropEvent) => void): Unsubscribe {
    this.dropListeners.add(cb);
    return () => this.dropListeners.delete(cb);
  }
  /** Test hook: simulates files dragged from the Finder (Playwright cannot drag from the OS). */
  simulateDrop(type: "over" | "drop" | "leave", paths: string[] = [], at?: { x: number; y: number }) {
    this.dropListeners.forEach((l) => l({ type, paths, ...at }));
  }

  async importCandidates(paths: string[], adding: boolean, rate: number | null): Promise<ImportCandidate[]> {
    // As the shell: adding needs the open project; a new project has none yet.
    if (adding && !this.project) throw new Error("no project is open");
    void rate;
    const guess = (n: string): ImportCandidate["role"] => {
      const l = n.toLowerCase();
      const table: [string, ImportCandidate["role"]][] = [
        ["kick", "kick"], ["snare", "snare"], ["oh", "overheads"], ["room", "room"], ["bass", "bass"], ["acoustic", "acoustic_guitar"],
        ["gtr", "electric_guitar"], ["rhodes", "keys"], ["keys", "keys"], ["string", "strings"], ["vox", "lead_vocal"], ["bv", "backing_vocal"],
      ];
      return table.find(([k]) => l.includes(k))?.[1] ?? "other";
    };
    return paths.map((path) => {
      const file = path.split("/").pop()!;
      const name = file.replace(/\.[^.]+$/, "").replace(/^\d+\s*/, "");
      const ext = file.split(".").pop()!.toLowerCase();
      const strings = /strings/i.test(name);
      return {
        path, name, format: ext === "flac" ? "flac" : ext === "aif" || ext === "aiff" ? "aiff" : "wav",
        channels: /oh|room|acoustic|rhodes|strings|bv/i.test(name) ? 2 : 1, sample_rate: strings ? 44100 : 48000, frames: 222 * DEMO_RATE,
        encoding: strings ? "PCM 16-bit" : "PCM 24-bit", role: guess(name), rate_mismatch: strings, error: null,
      };
    });
  }

  async importFiles(choices: ImportChoice[]) {
    this.mutate(`Import ${choices.length} files`, (p) => {
      p.sample_rate ??= DEMO_RATE;
      choices.forEach((c, i) => {
        const id = `t${p.next_id++}`;
        const mediaId = `mx${p.next_id++}`.padEnd(32, "0");
        const channels = /oh|room|acoustic|rhodes|strings|bv/i.test(c.name) ? 2 : 1;
        p.media.push({
          id: mediaId, path: `audio/${mediaId}.wav`, original_name: c.path.split("/").pop() ?? c.name, format: "wav", channels,
          sample_rate: DEMO_RATE, frames: 222 * DEMO_RATE, encoding: "PCM 24-bit", converted: c.convert,
        });
        const t = structuredClone(demoProject().tracks[1]);
        Object.assign(t, {
          id, kind: "audio", name: c.name, role: c.role, channels, color: (i % 12) + 1, sends: [], automation: [], output: { kind: "mix_bus" }, locked: false,
          clips: [{ id: `c${p.next_id++}`, media: mediaId, start: 0, offset: 0, length: 222 * DEMO_RATE, gain_db: 0, fade_in: { length: 0, shape: "equal_power" }, fade_out: { length: 0, shape: "equal_power" }, mute: false }],
        });
        t.chain.fader_db = 0;
        const at = p.tracks.findIndex((x) => x.kind === "return");
        p.tracks.splice(at < 0 ? p.tracks.length : at, 0, t);
      });
    });
  }

  async paramSpecs() {
    return SPECS;
  }

  private setValue(p: Project, track: string, param: string, value: number) {
    setParam(p, track, param, value);
  }

  async setParam(track: string, param: string, value: number, label: string) {
    this.mutate(label, (p) => this.setValue(p, track, param, value));
  }

  async beginGesture(label: string, _phase?: Phase) {
    if (!this.project) throw new Error("no project is open");
    this.gesture = { label, before: JSON.stringify(this.project) };
  }

  async gestureParam(track: string, param: string, value: number) {
    this.mutate("", (p) => this.setValue(p, track, param, value));
  }

  async endGesture(cancel: boolean) {
    const g = this.gesture;
    this.gesture = null;
    if (!g || !this.project) return;
    if (cancel) {
      this.project = JSON.parse(g.before);
    } else if (JSON.stringify(this.project) !== g.before) {
      this.done.push({ label: g.label, before: g.before, after: JSON.stringify(this.project) });
      this.undone = [];
    }
    this.emit();
  }

  async setView(track: string, what: "mute" | "solo", on: boolean) {
    if (!this.project) return;
    const t = this.project.tracks.find((x) => x.id === track);
    if (t) t[what] = on;
    this.project = structuredClone(this.project);
    this.emit();
  }

  async edit(e: Edit, label: string) {
    this.mutate(label, (p) => runEdit(p, e));
  }

  async gestureEdit(e: Edit) {
    if (!this.gesture || !this.project) throw new Error("no gesture is open");
    const base = JSON.parse(this.gesture.before) as Project;
    runEdit(base, e);
    this.project = base;
    this.emit();
  }

  async setLaneVisible(track: string, param: string, visible: boolean) {
    if (!this.project) return;
    const p = structuredClone(this.project);
    const t = p.tracks.find((x) => x.id === track);
    if (!t) return;
    const l = t.automation.find((x) => x.param === param);
    if (l) l.visible = visible;
    else if (visible) t.automation.push({ param, points: [], author: "user", visible: true });
    this.project = p;
    this.emit();
  }

  /** As in the app: the log of AI calls is not a history step, so undo and redo leave it as it is (AI-14). */
  private keepCalls(p: Project): Project {
    if (this.project) p.ai.calls = this.project.ai.calls;
    return p;
  }

  async undo(_phase: Phase) {
    const s = this.done.pop();
    if (!s) throw new Error("nothing to undo");
    this.project = this.keepCalls(JSON.parse(s.before));
    this.undone.push(s);
    this.emit();
    // As the app: the master follows its history (measures of the state gone back to).
    if (this.master?.status.phase === "master") this.master.emit();
    return s.label;
  }

  async redo(_phase: Phase) {
    const s = this.undone.pop();
    if (!s) throw new Error("nothing to redo");
    this.project = this.keepCalls(JSON.parse(s.after));
    this.done.push(s);
    this.emit();
    // As the app: the master follows its history (measures of the state gone back to).
    if (this.master?.status.phase === "master") this.master.emit();
    return s.label;
  }

  async history(_phase: Phase): Promise<[HistoryEntry[], number]> {
    const e = [...this.done.map((s) => ({ s, undone: false })), ...[...this.undone].reverse().map((s) => ({ s, undone: true }))];
    return [e.map((x, i) => ({ index: i + 1, label: x.s.label, ai_call: x.s.aiCall ?? null, undone: x.undone })), this.done.length];
  }

  async historyGoto(phase: Phase, position: number) {
    while (this.done.length > position) await this.undo(phase);
    while (this.done.length < position && this.undone.length) await this.redo(phase);
  }

  private now() {
    if (!this.transport.playing) return this.transport.frame;
    const f = this.transport.startFrame + ((performance.now() - this.transport.startedAt) / 1000) * DEMO_RATE;
    if (this.loop && f >= this.loop[1]) {
      const len = this.loop[1] - this.loop[0];
      return this.loop[0] + ((f - this.loop[0]) % len);
    }
    return f;
  }

  private tick = () => {
    const frame = this.now();
    const playing = this.transport.playing;
    const t = frame / DEMO_RATE;
    const tracks = (this.project?.tracks ?? []).map((tr, i): [string, [number, number, number, number, number]] => {
      if (!playing || tr.mute) return [tr.id, [-200, -200, -200, -200, 0]];
      const v = -18 + 6 * Math.abs(Math.sin(t * 3 + i)) + tr.chain.fader_db * 0.5;
      return [tr.id, [v + 6, v + 5.5, v, v - 0.5, !tr.chain.blocks.some((b) => b.fx.type === "compressor" && !b.fx.bypass) ? 0 : -2 * Math.abs(Math.sin(t * 5 + i))]];
    });
    const lv = playing ? -10 + 2 * Math.sin(t) : -200;
    // A pink-ish moving spectrum for the watched track (96 log bands, 20 Hz … 20 kHz).
    const spectrum = this.watched && playing
      ? Array.from({ length: 96 }, (_, k) => -28 - k * 0.32 + 6 * Math.sin(k * 0.35 + t * 2) * Math.exp(-k / 70) - (k > 88 ? (k - 88) * 4 : 0))
      : null;
    this.meterListeners.forEach((l) =>
      l({ frame, playing, tracks, bus: [lv + 6, lv + 5.8, lv, lv, playing ? -3.2 : -200, playing ? -15.3 : -200, playing ? -14.8 : -200, 0.82], load: playing ? 0.12 : 0.01, spectrum, masterGr: [playing ? -1.2 : 0, playing ? -2.4 + Math.sin(t * 3) : 0], ...this.master.blockMeters(playing, t) }),
    );
  };

  private watched: string | null = null;
  async watchSpectrum(track: string | null) {
    this.watched = track;
  }

  async play() {
    this.transport = { ...this.transport, playing: true, startedAt: performance.now(), startFrame: this.transport.frame, playFrom: this.transport.frame };
  }
  async pause() {
    this.transport = { ...this.transport, frame: this.now(), playing: false };
  }
  async stop() {
    this.transport = { ...this.transport, frame: this.transport.playFrom, playing: false };
  }
  async locate(frame: number) {
    this.transport = { ...this.transport, frame: Math.max(0, frame), startFrame: Math.max(0, frame), startedAt: performance.now() };
  }
  async setLoop(start: number | null, end: number | null) {
    this.loop = start !== null && end !== null && end > start ? [start, end] : null;
  }

  async transportState(): Promise<TransportState> {
    return {
      playing: this.transport.playing, frame: this.now(), rate: DEMO_RATE, load: 0.12, xruns: 0,
      device: "Demo output", deviceRate: DEMO_RATE, deviceError: null, disconnected: this.deviceDown,
    };
  }

  onMeters(cb: (m: MeterSnapshot) => void): Unsubscribe {
    this.meterListeners.add(cb);
    this.timer ??= setInterval(this.tick, 33);
    return () => {
      this.meterListeners.delete(cb);
      if (this.meterListeners.size === 0 && this.timer) {
        clearInterval(this.timer);
        this.timer = null;
      }
    };
  }

  async audioDevices(): Promise<DeviceInfo[]> {
    return [
      { name: "MacBook Speakers", default: true, sample_rates: [44100, 48000, 96000], min_buffer: 32, max_buffer: 4096 },
      { name: "Scarlett 2i2", default: false, sample_rates: [44100, 48000, 88200, 96000], min_buffer: 16, max_buffer: 2048 },
    ];
  }
  private prefs: AudioPrefs = { device: null, buffer: 512 };
  async audioPrefs() {
    return this.prefs;
  }
  async setAudioPrefs(p: AudioPrefs) {
    this.prefs = p;
    // Choosing a device brings the output back.
    this.deviceDown = false;
  }

  async peaks(media: string, start: number, end: number, columns: number) {
    const m = this.project?.media.find((x) => x.id === media);
    return demoPeaks(media, start, end, columns, m?.channels ?? 1);
  }

  // ---------- assistant (F7) ----------
  async aiStatus(): Promise<AiStatus> {
    return structuredClone(this.ai.status);
  }
  onAiStatus(cb: (s: AiStatus) => void): Unsubscribe {
    return this.ai.on(cb);
  }
  onAiError(cb: (m: string) => void): Unsubscribe {
    return this.ai.onError(cb);
  }
  async aiEstimate(revision: boolean) {
    return this.ai.estimate(revision);
  }
  async aiStart(request: string | null, scope: string[] | null) {
    if (this.missingMedia.size) throw new Error(`${this.missingMedia.size} audio files are missing from the project folder: locate them before asking the assistant`);
    if (this.networkDown && !this.ai.status.config.offline_demo) {
      // As the real client: the error event, nothing spent beyond the estimate, the mix untouched.
      setTimeout(() => this.ai.fail("network unavailable: could not reach openrouter.ai"), 50);
      return;
    }
    if (this.refusal && !this.ai.status.config.offline_demo) {
      const m = this.refusal;
      setTimeout(() => this.ai.fail(m), 50);
      return;
    }
    this.ai.start(request, scope);
  }
  async aiCancel() {
    this.ai.cancel();
  }
  async aiApply(only: string[] | null, trims: boolean) {
    return this.ai.apply(only, trims);
  }
  async aiDiscard() {
    this.ai.discard();
  }
  async aiReopen(id: string) {
    this.ai.reopen(id);
  }
  async aiListen(what: string) {
    this.ai.listen(what);
  }
  async aiDetail(id: string): Promise<CallDetail> {
    return this.ai.detail(id);
  }
  /** Test hook: the last export, instead of a file. */
  lastExport = "";
  async aiExport(json: boolean) {
    const calls = this.project?.ai.calls ?? [];
    this.lastExport = json ? JSON.stringify(calls, null, 2) : ["id,kind,model,cost_usd,outcome", ...calls.map((c) => `${c.id},${c.kind},${c.model},${c.cost_usd},${c.outcome}`)].join("\n");
    return true;
  }
  async aiSetConfig(c: AiConfig) {
    this.ai.setConfig(c);
  }
  async aiSetKey(key: string | null) {
    this.ai.setKey(key);
  }
  /** Test hooks: how the next connection ends, the pages opened in the "browser", a first launch. */
  connectOutcome: ConnectOutcome = "connected";
  connectDelay = 400;
  opened: string[] = [];
  private connectTimer: ReturnType<typeof setTimeout> | null = null;
  private connectListeners = new Set<(o: ConnectOutcome) => void>();
  async aiConnectStart() {
    if (this.connectTimer) clearTimeout(this.connectTimer);
    this.opened.push("auth");
    const outcome = this.connectOutcome;
    this.connectTimer = setTimeout(() => {
      this.connectTimer = null;
      if (outcome === "connected") {
        this.ai.setKey("sk-or-connected");
        this.ai.setConfig({ ...this.ai.config, offline_demo: false });
      }
      this.connectListeners.forEach((l) => l(outcome));
    }, this.connectDelay);
  }
  async aiConnectCancel() {
    if (!this.connectTimer) return;
    clearTimeout(this.connectTimer);
    this.connectTimer = null;
    this.connectListeners.forEach((l) => l("cancelled"));
  }
  async aiConnectReopen() {
    if (!this.connectTimer) throw new Error("no connection is waiting");
    this.opened.push("auth");
  }
  onAiConnect(cb: (o: ConnectOutcome) => void): Unsubscribe {
    this.connectListeners.add(cb);
    return () => this.connectListeners.delete(cb);
  }
  async aiOpenPage(which: ProviderPage) {
    this.opened.push(which);
  }
  async setSpendCap(usd: number | null) {
    this.viewChange((p) => (p.ai.spend_cap_usd = usd));
  }
  async setAllowStructure(on: boolean) {
    this.viewChange((p) => (p.ai.allow_structure = on));
  }
  async setAllowIdeas(on: boolean) {
    this.viewChange((p) => (p.ai.allow_ideas = on));
  }
  async aiSetIdeas(ids: string[]) {
    this.ai.setIdeas(ids);
  }
  async setNormalizeTrim(on: boolean) {
    this.mutate(on ? "Level tracks to −23 LUFS first" : "Keep the recorded levels", (p) => (p.ai.normalize_trim = on));
  }
  private snapshots = new Map<string, Project["tracks"]>();
  async snapshotSave(name: string) {
    if (!this.project) return;
    const id = `snap${this.project.next_id}`;
    this.snapshots.set(id, structuredClone(this.project.tracks));
    this.viewChange((p) => {
      p.next_id++;
      p.snapshots = [...p.snapshots, { id, name, created_ms: Date.now(), path: `snapshots/${id}.json` }];
    });
  }
  async snapshotRestore(id: string) {
    const tracks = this.snapshots.get(id);
    const s = this.project?.snapshots.find((x) => x.id === id);
    if (!tracks || !s) throw new Error("no such snapshot");
    this.mutate(`Restore snapshot “${s.name}”`, (p) => (p.tracks = structuredClone(tracks)));
    this.ai.listen("current");
  }
  async snapshotDelete(id: string) {
    this.viewChange((p) => (p.snapshots = p.snapshots.filter((s) => s.id !== id)));
  }
  async pickReference() {
    return "/Users/demo/Music/Reference A.mp3";
  }
  async addReference(path: string) {
    this.mutate("Add reference", (p) => {
      if (p.references.length >= 3) throw new Error("at most 3 reference tracks");
      const n = p.references.length + 1;
      const name = path.split("/").pop() ?? "Reference";
      p.references.push({
        id: `ref${n}`,
        media: { id: `ref${n}`.padEnd(32, "0"), path: `references/ref${n}.wav`, original_name: name, format: "wav", channels: 2, sample_rate: 48000, frames: 231 * 48000, encoding: "MP3", converted: true },
      });
    });
  }
  async removeReference(id: string) {
    this.mutate("Remove reference", (p) => (p.references = p.references.filter((r) => r.id !== id)));
  }

  // ---------- levels for the Explain view (ANA-20) ----------
  private levelsKey = "";
  private levels: MixLevels | null = null;
  private levelListeners = new Set<(l: MixLevels) => void>();
  /** As in the app: the first ask on a mix answers `null` and the levels follow in an event. */
  async mixLevels(): Promise<MixLevels | null> {
    const p = this.project;
    if (!p) throw new Error("no project is open");
    const key = JSON.stringify([p.tracks, p.sections, p.mix_bus]);
    if (key === this.levelsKey) return this.levels;
    this.levelsKey = key;
    this.levels = null;
    setTimeout(() => {
      if (this.levelsKey !== key || !this.project) return;
      this.levels = mockLevels(this.project);
      for (const cb of this.levelListeners) cb(structuredClone(this.levels));
    }, 120);
    return null;
  }
  onMixLevels(cb: (l: MixLevels) => void): Unsubscribe {
    this.levelListeners.add(cb);
    return () => this.levelListeners.delete(cb);
  }

  // ---------- master phase (F8) ----------
  async masterStatus() {
    return structuredClone(this.master.status);
  }
  onMasterStatus(cb: (s: MasterStatus) => void): Unsubscribe {
    return this.master.on(cb);
  }
  async masterPrepareRetry() {
    this.master.retry();
  }
  async setPhase(master: boolean) {
    this.master.setPhase(master);
  }
  async masterListen(what: string) {
    this.master.listen(what);
  }
  async masterListenSame(on: boolean) {
    this.master.listenSame(on);
  }
  async masterJobCancel() {
    this.master.cancel();
  }
  async masterChain(op: ChainOp) {
    return this.master.chainOp(op);
  }
  listening: string | null = null;
  async listenBlock(track: string, block: string, on: boolean) {
    this.listening = on ? `${track}/${block}` : null;
  }

  async masterSolo(block: string | null, mask: number) {
    this.master.solo(block, mask);
  }
  async bounceStart() {
    this.master.bounce();
  }
  async exportStart() {
    if (this.demo) throw new Error("the demo song of the tour is not saved and cannot be exported");
    this.master.export();
  }
  async masterAiStart(request: string | null) {
    this.master.aiStart(request);
  }
  async masterAiApply(only: string[] | null) {
    return this.master.aiApply(only);
  }
  async masterAiDiscard() {
    this.master.aiDiscard();
  }
  async masterAiReopen(id: string) {
    this.master.aiReopen(id);
  }
  async mixdownPeaks(columns: number) {
    return this.master.peaks(columns);
  }
  async masterWave(columns: number) {
    return this.master.wave(columns);
  }
  async pickImage() {
    return "/Users/demo/Pictures/cover.jpg";
  }
  async setCover(path: string | null) {
    this.mutate("Cover", (p) => (p.master.metadata.cover = path ? "masters/cover.jpg" : null));
  }
}
