// The master phase of the mock backend (FOUNDATION §5.6): a timed bounce, measures that follow the destination,
// equal-loudness A/B, a timed export with its report, a master proposal. The real work is in Rust
// (`stemwise_engine::mixdown`, `master`, `stemwise_export::pipeline`); this only drives the screens in the browser.
import type { AiCallRecord } from "../bindings/AiCallRecord";
import type { FileReport } from "../bindings/FileReport";
import type { MasterState } from "../bindings/MasterState";
import type { MasterStatus } from "../bindings/MasterStatus";
import type { PrepStep } from "../bindings/PrepStep";
import type { Project } from "../bindings/Project";
import type { ChainOp } from "../bindings/ChainOp";
import type { MasterChange } from "../bindings/MasterChange";
import type { MasterMeasures } from "../bindings/MasterMeasures";
import type { MasterWave } from "../bindings/MasterWave";
import { addable, chainEdit, chainView } from "./mockChain";

export interface MockMasterHost {
  project(): Project | null;
  mutate(label: string, f: (p: Project) => void, aiCall?: string): void;
  view(f: (p: Project) => void): void;
}

type Bands = [number, number, number, number, number, number, number, number];
const BANDS_MIX: Bands = [-14.2, -9.8, -8.1, -9.5, -6.3, -11.8, -15.6, -21.4];
const BANDS_REF: Bands = [-13.5, -10.5, -9.0, -9.8, -6.6, -10.9, -13.9, -18.8];

export class MockMaster {
  status: MasterStatus = {
    phase: "mix", stale: false, listening: "master", listen_same: false, loud_gain_db: 4.5, analysing: false, measures: null, mixdown_measures: null,
    target_bands: BANDS_REF, references: [{ id: "ref1", name: "Reference A", lufs: -12.9, true_peak_db: -0.8 }], job: null, bounced: null, last_export: null, aac: true, ab_trim_db: 0, proposal: null,
    prep: null, retarget: null, skipped_references: [], solo: null, chain: [], addable: [], strained: null,
  };
  /** The mixdown whose measures are ready (the real app keeps them in the package). */
  private preparedFor: number | null = null;
  private prepTimer: ReturnType<typeof setInterval> | null = null;
  /** Test hooks: how long each step of the preparation lasts, and a failure to simulate. */
  prepStepMs = 350;
  prepFailure: "mixdown_unreadable" | "internal" | "reference" | null = null;
  private listeners = new Set<(s: MasterStatus) => void>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private pendingState: MasterState | null = null;

  constructor(private host: MockMasterHost, private onError: (m: string) => void) {}

  /** The project closed (the tour's demo): no job, no preparation, back to the Mix. */
  reset() {
    for (const t of [this.timer, this.prepTimer, this.retargetTimer]) if (t) clearInterval(t);
    if (this.remeasure) clearTimeout(this.remeasure);
    this.timer = this.prepTimer = this.retargetTimer = null;
    this.remeasure = null;
    this.preparedFor = null;
    this.pendingState = null;
    this.status = { ...this.status, phase: "mix", job: null, prep: null, retarget: null, proposal: null, bounced: null, last_export: null, measures: null, mixdown_measures: null, stale: false };
    this.emit();
  }

  on(cb: (s: MasterStatus) => void) {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  emit() {
    const p = this.host.project();
    const t = p?.master.target;
    this.status.chain = p ? chainView(p.master.chain) : [];
    this.status.addable = p ? addable(p.master.chain) : [];
    if (this.status.solo && !p?.master.chain.blocks.some((b) => b.id === this.status.solo![0])) this.status.solo = null;
    if (p?.master.mixdown && this.preparedFor === p.master.mixdown.number) {
      this.status.mixdown_measures = { ...this.measuresOf(p.master, false), lufs: -18.6, true_peak_db: -3.0, lra: 7.4, distance_db: 3.4, bands_db: BANDS_MIX, peak_to_loudness_db: 15.6, short_crest_db: 13.4, limiter: null, blocks: [] };
      // While a new target is reached the measures are still those of the old one (9v).
      if (!this.status.retarget) this.status.measures = t ? this.measuresOf(p.master, true) : null;
    }
    const lim = this.status.measures?.limiter;
    this.status.strained = lim && t && lim.over_3_pct > 5 ? { over_3_pct: lim.over_3_pct, easier_lufs: Math.floor((t.lufs - Math.max(1, lim.max_db - 3)) * 2) / 2 } : null;
    // As the app: raising a source to the master's loudness must keep its peaks under −1 dBTP (S7).
    const ml = this.status.measures?.lufs;
    const over = (l: number | null | undefined, tp: number) => (ml == null || l == null ? 0 : Math.max(0, ml - l + tp + 1));
    const mx = this.status.mixdown_measures;
    this.status.ab_trim_db = Math.max(mx ? over(mx.lufs, mx.true_peak_db) : 0, ...this.status.references.map((r) => over(r.lufs, r.true_peak_db)));
    this.listeners.forEach((l) => l(structuredClone(this.status)));
  }

  /** What the app would measure on a master state: the louder the target and the fewer blocks before the limiter,
   *  the harder the limiter works (numbers for the screens only). */
  measuresOf(m: MasterState, matched: boolean): MasterMeasures {
    const on = m.chain.blocks.filter((b) => !b.block.bypass);
    const push = Math.max(0, m.target.lufs + 18.6 + 3 + m.target.true_peak_db * -1 - 4.6);
    let left = push;
    const blocks = on.flatMap((b) => {
      const k = b.block.type;
      if (k !== "compressor" && k !== "multiband" && k !== "clipper") return [];
      const take = k === "clipper" ? Math.min(left, b.block.type === "clipper" ? b.block.take_off_db : 0) : Math.min(left, k === "compressor" ? 2.6 : 0.8);
      left -= take;
      return [{ id: b.id, work: { max_db: +(take + (k === "clipper" ? 0 : 0.4)).toFixed(1), mean_db: +(take / 4).toFixed(1), active_pct: k === "clipper" ? 0.6 : 34, over_3_pct: 0 } }];
    });
    const over3 = Math.max(0, Math.min(60, (left - 4.5) * 8));
    const match = on.some((b) => b.block.type === "match_eq");
    return {
      lufs: m.target.lufs - 0.1, true_peak_db: m.target.true_peak_db, lra: 6.2, distance_db: matched && match ? 1.2 : 2.4, bands_db: BANDS_REF,
      peak_to_loudness_db: m.target.true_peak_db - m.target.lufs + 0.1, short_crest_db: +(13.4 - push * 0.3).toFixed(1),
      limiter: { max_db: +(left + 0.4).toFixed(1), mean_db: +(left / 5).toFixed(1), active_pct: Math.min(100, left * 12), over_3_pct: +over3.toFixed(0) },
      blocks, clip_peak: [0.7, 0],
    };
  }

  /** Per-block and per-band gain reduction for the meters, as the engine sends it. */
  blockMeters(playing: boolean, t: number): { masterBlocks: number[]; masterBands: number[] } {
    const p = this.host.project();
    const blocks = Array.from({ length: 9 }, (_, i) => {
      const b = p?.master.chain.blocks[i];
      if (!playing || !b || b.block.bypass) return 0;
      const k = b.block.type;
      return k === "compressor" ? -1.6 + 0.6 * Math.sin(t * 4) : k === "multiband" ? -1.2 + 0.4 * Math.sin(t * 3) : k === "clipper" ? -Math.max(0, 2 * Math.sin(t * 7)) : 0;
    });
    const bands = Array.from({ length: 12 }, (_, i) => (playing && i < 6 ? -[1.2, 0.4, 0, 0.8, 0.5, 0.2][i] * (0.7 + 0.3 * Math.sin(t * 3 + i)) : 0));
    return { masterBlocks: blocks, masterBands: bands };
  }

  /** MAS-15, MAS-12: a structural change of the chain as one master step. */
  chainOp(op: ChainOp): string | null {
    const p = this.host.project();
    if (!p) throw new Error("no project");
    const name = (id: string) => chainView(p.master.chain).find((b) => b.id === id)?.name ?? "";
    const r = chainEdit(p.master, op);
    const label = op.op === "add" ? "Master · block added" : op.op === "reset_order" ? "Master · order reset" : op.op === "remove" ? `Master · ${name(op.id)} removed` : op.op === "move" ? `Master · ${name(op.id)} moved` : op.op === "reset" ? `Master · ${name(op.id)} reset` : `Master · ${name(op.id)} bands`;
    this.host.mutate(label, (x) => (x.master = r.master));
    this.masterChanged();
    this.emit();
    return r.added;
  }

  solo(block: string | null, mask: number) {
    this.status.solo = block && mask ? [block, mask] : null;
    this.emit();
  }

  /** Test hook: how long the loudness is measured again after a master change (9k). */
  remeasureMs = 0;
  private remeasure: ReturnType<typeof setTimeout> | null = null;

  /** A master change: the level match is measured again in the background (9k), like the app. */
  masterChanged() {
    if (!this.remeasureMs || this.status.prep) return;
    this.status.analysing = true;
    if (this.remeasure) clearTimeout(this.remeasure);
    this.remeasure = setTimeout(() => {
      this.status.analysing = false;
      this.emit();
    }, this.remeasureMs);
  }

  private retargetTimer: ReturnType<typeof setInterval> | null = null;

  /** The destination changed: the gain is found again and the workspace waits under an overlay (9v), like the app. */
  targetChanged(from: { lufs: number; true_peak_db: number }) {
    if (!this.remeasureMs || this.status.prep || this.preparedFor === null) return;
    const started = Date.now();
    this.status.retarget = { from_lufs: this.status.retarget?.from_lufs ?? from.lufs, from_true_peak_db: this.status.retarget?.from_true_peak_db ?? from.true_peak_db, fraction: 0, elapsed_ms: 0 };
    this.status.analysing = true;
    if (this.retargetTimer) clearInterval(this.retargetTimer);
    this.retargetTimer = setInterval(() => {
      const rt = this.status.retarget;
      const el = Date.now() - started;
      if (!rt || el >= this.remeasureMs) {
        clearInterval(this.retargetTimer!);
        this.retargetTimer = null;
        this.status.retarget = null;
        this.status.analysing = false;
      } else {
        rt.fraction = el / this.remeasureMs;
        rt.elapsed_ms = el;
      }
      this.emit();
    }, 60);
  }

  /** Mix edits make the mixdown out of date (MDN-2). */
  mixChanged() {
    if (this.host.project()?.master.mixdown) {
      this.status.stale = true;
      this.emit();
    }
  }

  setPhase(master: boolean) {
    const md = this.host.project()?.master.mixdown;
    if (master && !md) throw new Error("no-mixdown");
    this.status.phase = master ? "master" : "mix";
    this.status.listening = "master";
    // A reopened project measures its mixdown when Master is first opened (9e).
    if (master && md && this.preparedFor !== md.number && !this.status.prep) this.prepare(false);
    this.emit();
  }

  /** The preparation of the master (9d–9j), with timed steps like the real one. */
  prepare(fromBounce: boolean) {
    const p = this.host.project();
    const md = p?.master.mixdown;
    if (!p || !md) return;
    if (this.prepTimer) clearInterval(this.prepTimer);
    this.status.mixdown_measures = null;
    this.status.measures = null;
    this.status.skipped_references = [];
    const step = (kind: string, extra: Partial<PrepStep> = {}): PrepStep => ({ kind, state: "waiting", source: null, fraction: 0, ms: null, references: [], from_rate: null, detail: null, ...extra });
    const refs = this.status.references.map((r) => r.name);
    const steps: PrepStep[] = [
      step("read_mixdown", fromBounce ? { state: "done", source: "bounce", fraction: 1 } : {}),
      step("measure_mixdown"),
      ...refs.slice(0, 1).map((r) => step("convert_reference", { references: [r], from_rate: 44100 })),
      ...(refs.length ? [step("measure_references", { references: refs })] : []),
      step("loudness"),
    ];
    const started = Date.now();
    this.status.prep = { mixdown: md.number, frames: md.frames, rate: p.sample_rate ?? 48000, references: refs.length, fraction: 0, elapsed_ms: 0, eta_ms: null, error: null, steps };
    this.status.analysing = true;
    let i = steps.findIndex((s) => s.state === "waiting");
    let stepStart = Date.now();
    const fail = this.prepFailure;
    this.prepTimer = setInterval(() => {
      const prep = this.status.prep;
      if (!prep) return;
      const cur = prep.steps[i];
      const now = Date.now();
      if (cur.state === "waiting") {
        cur.state = "running";
        stepStart = now;
      }
      cur.fraction = Math.min(1, (now - stepStart) / this.prepStepMs);
      const failsHere =
        (fail === "mixdown_unreadable" && cur.kind === "measure_mixdown") ||
        (fail === "internal" && cur.kind === "measure_references") ||
        (fail === "reference" && cur.kind === "convert_reference");
      if (failsHere && fail === "reference") {
        cur.state = "skipped";
        cur.detail = "The file is missing or damaged";
        this.status.skipped_references = cur.references;
      } else if (failsHere) {
        const at = fail === "mixdown_unreadable" ? prep.steps[0] : cur;
        at.state = "failed";
        at.detail = fail === "mixdown_unreadable" ? "The file ends early (2:18 of 3:42)" : "out of memory";
        prep.steps.forEach((s) => s.state === "waiting" || s.state === "running" ? (s.state = "not_started") : null);
        prep.error = { kind: fail, step: at.kind, detail: at.detail };
        this.status.analysing = false;
        clearInterval(this.prepTimer!);
        this.prepTimer = null;
        this.emit();
        return;
      } else if (cur.fraction >= 1) {
        cur.state = "done";
        cur.ms = now - stepStart;
      }
      if (cur.state === "done" || cur.state === "skipped") i++;
      const done = prep.steps.filter((s) => s.state === "done" || s.state === "skipped").length;
      prep.fraction = (done + (prep.steps[i]?.fraction ?? 0)) / prep.steps.length;
      prep.elapsed_ms = now - started;
      prep.eta_ms = done > 0 ? Math.round((prep.elapsed_ms * (1 - prep.fraction)) / Math.max(0.01, prep.fraction)) : null;
      if (i >= prep.steps.length) {
        clearInterval(this.prepTimer!);
        this.prepTimer = null;
        this.status.prep = null;
        this.status.analysing = false;
        this.preparedFor = md.number;
      }
      this.emit();
    }, 50);
  }

  /** "Try again" (9j). */
  retry() {
    this.status.prep = null;
    this.prepFailure = null;
    this.prepare(false);
  }

  listen(what: string) {
    this.status.listening = what;
    this.emit();
  }

  listenSame(on: boolean) {
    this.status.listen_same = on;
    this.emit();
  }

  /** Test hook: the references the A/B offers, with their loudness (boards 9p–9u). */
  setReferences(refs: { id: string; name: string; lufs: number; true_peak_db?: number }[]) {
    this.status.references = refs.map((r) => ({ true_peak_db: -1, ...r }));
    this.emit();
  }

  private run(kind: string, ms: number, done: () => void) {
    if (this.status.job) throw new Error(`busy: ${this.status.job.kind} in progress`);
    const started = Date.now();
    this.status.job = { kind, progress: 0, started_ms: started };
    this.emit();
    this.timer = setInterval(() => {
      const f = Math.min(1, (Date.now() - started) / ms);
      this.status.job = { kind, progress: f, started_ms: started };
      if (f >= 1) {
        clearInterval(this.timer!);
        this.timer = null;
        this.status.job = null;
        done();
      }
      this.emit();
    }, 60);
  }

  cancel() {
    if (!this.timer || !this.status.job) return;
    clearInterval(this.timer);
    this.timer = null;
    const kind = this.status.job.kind;
    this.status.job = null;
    this.emit();
    this.onError(`${kind} cancelled`);
  }

  bounce() {
    this.run("bounce", 900, () => {
      const p = this.host.project();
      if (!p) return;
      const n = (p.master.mixdown?.number ?? 2) + 1;
      const md = { number: n, path: `mixdowns/mixdown-${n}.wav`, mix_hash: "demo", trim_db: -0.8, frames: 222 * 48000, created_ms: Date.now(), lufs: -18.6, true_peak_db: -3.0, lra: 7.4 };
      this.host.view((q) => (q.master.mixdown = md));
      this.status.bounced = md;
      this.status.stale = false;
      // As the app: the preparation starts as soon as the mixdown is written, with its samples in memory (9d).
      this.prepare(true);
    });
  }

  export() {
    this.run("export", 900, () => {
      const p = this.host.project();
      if (!p) return;
      const x = p.master.export;
      const t = p.master.target;
      const base = `/Users/demo/Music/Stemwise exports/${p.name}`;
      const r: FileReport[] = [];
      const row = (path: string, format: string, l: number): FileReport => ({ path, format, bytes: 41_000_000, lufs: l, true_peak_db: t.true_peak_db, lra: 6.2, distance_db: 1.2 });
      if (x.wav) r.push(row(`${base}.wav`, `WAV ${x.bit_depth === 32 ? "32-bit float" : `${x.bit_depth}-bit`}`, t.lufs - 0.1));
      if (x.flac) r.push(row(`${base}.flac`, `FLAC ${Math.min(24, x.bit_depth)}-bit`, t.lufs - 0.1));
      if (x.aac && x.sample_rate <= 48000) r.push(row(`${base}.m4a`, `AAC ${x.aac_kbps} kbit/s`, t.lufs - 0.4));
      this.status.last_export = r;
    });
  }

  aiStart(request: string | null) {
    this.propose(request, null);
  }

  /** AI-23: a master proposal received earlier goes back in review, measured again like a new one. */
  aiReopen(id: string) {
    const rec = this.host.project()?.ai.calls.find((c) => c.id === id && (c.kind === "master" || c.kind === "master_revision"));
    if (!rec) throw new Error("no such master proposal");
    if (!rec.stored) throw new Error("this proposal was not kept with the project: it cannot be reviewed again");
    this.propose(rec.request, rec);
  }

  private propose(request: string | null, again: AiCallRecord | null) {
    if (this.status.proposal) throw new Error("a master proposal is in review: apply or discard it first");
    this.run("master_ai", again ? 300 : 700, () => {
      const p = this.host.project();
      if (!p) return;
      // A plan as the assistant makes one: a compressor first, the EQ after it, a second EQ it adds, a clipper, and a
      // limiter that recovers more slowly.
      const m: MasterState = structuredClone(p.master);
      const c = m.chain;
      const first = (k: string) => c.blocks.find((b) => b.block.type === k);
      const eq = first("eq");
      if (eq && eq.block.type === "eq") {
        eq.block.bypass = false;
        eq.block.bands[5] = { enabled: true, kind: "high_shelf", freq_hz: 10000, gain_db: request ? 0.4 : 1.0, q: 0.7 };
        eq.block.bands[1] = { enabled: true, kind: "peak", freq_hz: 250, gain_db: -1.0, q: 1.2 };
      }
      const comp = first("compressor");
      if (comp && comp.block.type === "compressor") Object.assign(comp.block, { bypass: false, ratio: request ? 2.0 : 1.8, threshold_db: request ? -15 : -14, mix: 0.6, listen_above_hz: 120 });
      const match = first("match_eq");
      if (match) match.block.bypass = false;
      const clip = first("clipper");
      if (clip && clip.block.type === "clipper") Object.assign(clip.block, { bypass: false, take_off_db: 3 });
      // The compressor moves before the EQ.
      if (comp && eq) {
        c.blocks = c.blocks.filter((b) => b.id !== comp.id);
        c.blocks.splice(c.blocks.findIndex((b) => b.id === eq.id), 0, comp);
      }
      c.limiter = { ...c.limiter, release_ms: request ? 120 : 80 };
      const sec = p.sections.find((s) => /chorus/i.test(s.name));
      if (sec && !request) m.sections = p.sections.filter((s) => s.name === sec.name).map((s) => ({ section: s.id, level_db: 0.5, side_gain_db: 0, limiter_release_ms: 0, eq: [] }));
      this.pendingState = m;
      const id = again?.id ?? `${Date.now()}-master`;
      const was = (bid: string) => !p.master.chain.blocks.find((b) => b.id === bid)?.block.bypass;
      const change = (key: string, name: string, state: string, rationale: string, rows: MasterChange["rows"]): MasterChange => ({ key, name, state, rationale, rows });
      const changes: MasterChange[] = [];
      if (comp && eq && was(comp.id) && was(eq.id)) changes.push(change("order", "Order", "changed", "The compressor first, so the EQ shapes the tone without changing what it reacts to.", [{ label: "Compressor", unit: "", before: 2, after: 1 }]));
      if (comp) changes.push(change(`block:${comp.id}`, "Compressor", was(comp.id) ? "changed" : "on", "Only a little glue, blended at 60 %, so the loud sections keep their lift.", [{ label: "Ratio", unit: ":1", before: was(comp.id) ? 1.8 : null, after: request ? 2.0 : 1.8 }, { label: "Mix", unit: "%", before: was(comp.id) ? 100 : null, after: 60 }]));
      if (eq) changes.push(change(`block:${eq.id}`, "EQ", "changed", "Slightly dark against the reference above 8 kHz: a gentle high shelf, and 1 dB less at 250 Hz.", [{ label: "Band 6 · 10.0 kHz", unit: "dB", before: null, after: request ? 0.4 : 1.0 }, { label: "Band 2 · 250 Hz", unit: "dB", before: null, after: -1.0 }]));
      if (match) changes.push(change(`block:${match.id}`, "Match EQ", was(match.id) ? "same" : "on", "Half-way toward the reference profile; full match would thin the low mids.", was(match.id) ? [] : [{ label: "Amount", unit: "%", before: null, after: 50 }]));
      if (clip) changes.push(change(`block:${clip.id}`, "Clipper", was(clip.id) ? "same" : "on", "The tallest peaks are few and short: 3 dB off them is that much less for the limiter.", was(clip.id) ? [] : [{ label: "Take off up to", unit: "dB", before: null, after: 3 }]));
      changes.push(change("limiter", "Limiter", request ? "changed" : "same", "It holds the ceiling of the target.", request ? [{ label: "Release", unit: "ms", before: 80, after: 120 }] : []));
      if (m.sections.length && JSON.stringify(m.sections) !== JSON.stringify(p.master.sections)) changes.push(change("sections", "Sections", "changed", "The choruses lift a little without a louder song.", m.sections.map((s) => ({ label: `${p.sections.find((x) => x.id === s.section)?.name ?? s.section} · level`, unit: "dB", before: null, after: s.level_db }))));
      this.status.proposal = {
        call_id: id,
        summary: request ? "A touch less bright and a little more glue, as you asked. Loudness and ceiling stay on the target." : "Mixdown measured −18.6 LUFS and slightly dark against the reference. A gentle shelf and half match close the gap; a clipper takes the tallest peaks before the limiter, which brings it to target.",
        diff: { track: "master", name: "Master", note: null, rationale: "", rows: changes.flatMap((x) => x.rows.map((r) => ({ ...r, label: `${x.name} · ${r.label}` }))) },
        record: again ?? {
          id, kind: request ? "master_revision" : "master", started_ms: Date.now() - 14000, duration_ms: 14000, model: "openai/gpt-6.1-sol", effort: "medium", decider: "openrouter", attempts: 1, iterations: 2,
          tokens_in: 9600, tokens_out: 900, cost_usd: 0.04, cost_estimated: false, outcome: "pending", history_label: null, request, path: `ai/${id}`, error: null, stored: true,
        },
        reopened: !!again,
        request,
        changes,
        preview: m,
        before: this.status.measures,
        after: { ...this.measuresOf(m, true), lra: 6.0, distance_db: 1.0 },
      };
    });
  }

  aiApply(only: string[] | null): string {
    const prop = this.status.proposal;
    const next = this.pendingState;
    if (!prop || !next) throw new Error("no master proposal to apply");
    const label = only ? "Master proposal applied in part" : prop.request ? `Master revision applied: “${prop.request}”` : "Master proposal applied";
    const take = (k: string) => !only || only.includes(k);
    this.host.mutate(label, (p) => {
      const c = p.master.chain;
      const n = next.chain;
      // Chosen blocks take the proposal's settings; the order and the sections only when chosen.
      for (const b of n.blocks) {
        if (!take(`block:${b.id}`)) continue;
        const i = c.blocks.findIndex((x) => x.id === b.id);
        if (i >= 0) c.blocks[i] = structuredClone(b);
        else c.blocks.push(structuredClone(b));
      }
      if (take("order")) c.blocks.sort((x, y) => n.blocks.findIndex((b) => b.id === x.id) - n.blocks.findIndex((b) => b.id === y.id));
      if (take("limiter")) c.limiter = n.limiter;
      if (take("sections")) p.master.sections = structuredClone(next.sections);
      p.ai.calls = [...p.ai.calls.filter((c) => c.id !== prop.call_id), { ...prop.record, outcome: only ? "applied_partly" : "applied", history_label: label }];
    }, prop.call_id);
    this.status.proposal = null;
    this.pendingState = null;
    this.status.listening = "master";
    this.emit();
    return label;
  }

  aiDiscard() {
    const prop = this.status.proposal;
    if (!prop) throw new Error("no master proposal to discard");
    // A proposal opened again keeps what its record says happened to it (AI-23).
    if (!prop.reopened) this.host.view((p) => (p.ai.calls = [...p.ai.calls, { ...prop.record, outcome: "discarded" }]));
    this.status.proposal = null;
    this.pendingState = null;
    this.emit();
  }

  /** The waveform as the app sends it: the mixdown, and the master pushed to the target and held at its ceiling. */
  wave(columns: number): MasterWave {
    const p = this.host.project();
    const mix = this.peaks(columns).map(([lo, hi]): [number, number, number] => [lo, hi, hi * 0.28]);
    if (!p || !this.status.measures) return { mixdown: mix, master: null };
    const g = 10 ** ((p.master.target.lufs + 18.6) / 20);
    const c = 10 ** (p.master.target.true_peak_db / 20);
    return { mixdown: mix, master: mix.map(([lo, hi, rms]): [number, number, number] => [Math.max(-c, lo * g), Math.min(c, hi * g), Math.min(c * 0.7, rms * g)]) };
  }

  peaks(columns: number): [number, number][] {
    return Array.from({ length: columns }, (_, i) => {
      const t = (i / columns) * 222;
      const sec = t < 16 ? 0.45 : t < 48 ? 0.65 : t < 80 ? 1 : t < 112 ? 0.68 : t < 144 ? 1 : t < 168 ? 0.55 : t < 208 ? 1 : 0.4;
      const v = sec * (0.55 + 0.25 * Math.abs(Math.sin(t * 1.7)) + 0.1 * Math.sin(t * 13.1));
      return [-v * 0.7, v * 0.7];
    });
  }
}
