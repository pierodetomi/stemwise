// The assistant of the mock backend (FOUNDATION §5.6: "una proposta AI di esempio"): a timed run through the
// phases, a proposal shaped like the design boards, apply/discard, the call log. The real loop is in Rust
// (`stemwise_ai::mixing`); this only lets the browser build and Playwright exercise the screens.
import type { AiCallRecord } from "../bindings/AiCallRecord";
import type { AiConfig } from "../bindings/AiConfig";
import type { AiStatus } from "../bindings/AiStatus";
import type { CallDetail } from "../bindings/CallDetail";
import type { DiffRow } from "../bindings/DiffRow";
import type { Project } from "../bindings/Project";
import type { ProposalView } from "../bindings/ProposalView";
import type { Track } from "../bindings/Track";
import type { NewElementView } from "../bindings/NewElementView";
import type { IdeaView } from "../bindings/IdeaView";
import { reverbOf, setParam } from "./mockEdit";
import { chainEdit, compOf, eqOf, lpfOf } from "./mockFx";

export interface MockAiHost {
  project(): Project | null;
  mutate(label: string, f: (p: Project) => void, aiCall?: string): void;
  view(f: (p: Project) => void): void;
}

type Change = { track: string; param: string; value: number };
/** A new return's sends, or a new group's members, as the proposal creates them. */
type Creation = { el: NewElementView; level_db: number; sends: { track: string; db: number }[] };
interface Plan {
  view: ProposalView;
  changes: Change[];
  creations: Creation[];
  notes: { track: string; text: string }[];
  /** AI-27: the creative ideas, each with what it adds; off until ticked. */
  ideas: { view: IdeaView; add: (p: Project) => void }[];
  /** AI-25: blocks the plan itself adds, per track. */
  blocks: { track: string; add: (p: Project) => void }[];
}

const now = () => Date.now();
let seq = 0;

export class MockAi {
  config: AiConfig = {
    base_url: "https://openrouter.ai/api/v1", model: "openai/gpt-6.1-sol", effort: "medium", temperature: 0.2, offline_demo: false,
    price_in_per_mtok: 1.25, price_out_per_mtok: 10, max_output_tokens: 32000,
  };
  key: string | null = "sk-or-demo";
  status: AiStatus = { running: null, proposal: null, listening: "current", has_key: true, key_place: "keychain", config: this.config, typical_mix_usd: 0.12, typical_revision_usd: 0.04, total_usd: 0.48, total_calls: 7, total_projects: 2 };
  private plan: Plan | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private listeners = new Set<(s: AiStatus) => void>();
  private errors = new Set<(m: string) => void>();

  constructor(private host: MockAiHost) {}

  on(cb: (s: AiStatus) => void) {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }
  onError(cb: (m: string) => void) {
    this.errors.add(cb);
    return () => this.errors.delete(cb);
  }
  /** Reports an error or a cancel like the real backend's "ai-error" event. */
  fail(m: string) {
    this.errors.forEach((e) => e(m));
  }

  emit() {
    this.status = { ...this.status, has_key: !!this.key, config: this.config };
    this.listeners.forEach((l) => l(structuredClone(this.status)));
  }

  estimate(revision: boolean) {
    const p = this.host.project();
    const spent = p?.ai.calls.reduce((a, c) => a + c.cost_usd, 0) ?? 0;
    const cost = this.offline() ? 0 : revision ? 0.04 : 0.13;
    const cap = p?.ai.spend_cap_usd ?? null;
    return { tokens_in: revision ? 5200 : 31000, tokens_out: revision ? 600 : 5400, cost_usd: cost, spent_usd: spent, cap_usd: cap, over_cap: cap !== null && spent + cost > cap };
  }

  /** The tour's calls: the offline demo whatever the settings, out of the totals of all projects (PRJ-9). */
  private tour = false;
  start(request: string | null, scope: string[] | null, tour = false) {
    if (this.status.running) throw new Error("the assistant is already working");
    if (this.status.proposal) throw new Error("a proposal is in review: apply or discard it first");
    this.tour = tour;
    if (!this.offline() && !this.key) throw new Error("no-key");
    const revision = request !== null;
    const of = revision ? 1 : 2;
    const phases: ["analysis" | "decision" | "render" | "evaluation", number][] = [["analysis", 0]];
    for (let i = 1; i <= of; i++) phases.push(["decision", i], ["render", i], ["evaluation", i]);
    const started = now();
    this.status.running = { kind: revision ? "revision" : "auto_mix", started_ms: started, progress: null, request, estimate_usd: this.estimate(revision).cost_usd };
    this.emit();
    let k = 0;
    const tick = () => {
      if (k < phases.length) {
        const [phase, iteration] = phases[k++];
        this.status.running = { ...this.status.running!, progress: { phase, iteration, of, fraction: 0.5 } };
        this.emit();
        this.timer = setTimeout(tick, 220);
        return;
      }
      this.timer = null;
      this.finish(request, scope, started, of);
    };
    this.timer = setTimeout(tick, 120);
  }

  cancel() {
    if (!this.timer || !this.status.running) return;
    clearTimeout(this.timer);
    this.timer = null;
    const r = this.status.running;
    this.status.running = null;
    this.log(this.record(r.kind, r.started_ms, 1, 0.01, "cancelled", r.request, "cancelled"));
    this.emit();
    // Cancelling the wait for the tour's proposal is not an error (as in the core).
    if (!this.tour) this.errors.forEach((e) => e("cancelled"));
  }

  private record(kind: AiCallRecord["kind"], started: number, iterations: number, cost: number, outcome: AiCallRecord["outcome"], request: string | null, error: string | null): AiCallRecord {
    const id = `${started}-${seq++}-${kind === "revision" ? "rev" : "mix"}`;
    return {
      id, kind, started_ms: started, duration_ms: now() - started, model: this.offline() ? "offline-demo" : this.config.model, effort: this.config.effort,
      decider: this.offline() ? "offline-demo" : "openrouter", attempts: iterations, iterations, tokens_in: 15600 * iterations, tokens_out: 2700 * iterations,
      cost_usd: this.offline() ? 0 : cost, cost_estimated: false, outcome, history_label: null, request, path: `ai/${id}`, error, stored: outcome === "pending",
    };
  }

  private log(rec: AiCallRecord) {
    this.host.view((p) => {
      p.ai.calls = [...p.ai.calls.filter((c) => c.id !== rec.id), rec];
    });
    if (this.tour) return;
    this.status.total_calls += 1;
    this.status.total_usd += rec.cost_usd;
  }

  private offline() {
    return this.config.offline_demo || this.tour;
  }

  /** The project closed (the tour's demo): nothing runs, nothing is in review. */
  reset() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.plan = null;
    this.tour = false;
    this.status = { ...this.status, running: null, proposal: null, listening: "current" };
    this.emit();
  }

  private finish(request: string | null, scope: string[] | null, started: number, iterations: number) {
    const p = this.host.project();
    if (!p) return;
    this.plan = buildPlan(p, request, scope);
    const rec = this.record(request ? "revision" : "auto_mix", started, iterations, request ? 0.04 : 0.11, "pending", request, null);
    this.plan.view.record = rec;
    this.plan.view.call_id = rec.id;
    this.plan.view.reopened = false;
    this.status.running = null;
    this.status.proposal = this.plan.view;
    this.emit();
  }

  apply(only: string[] | null, trims: boolean): string {
    const plan = this.plan;
    if (!plan || !this.status.proposal) throw new Error("no proposal to apply");
    void trims;
    const keep = (t: string) => !only || only.includes(t);
    const changed = new Set(plan.changes.filter((c) => keep(c.track)).map((c) => c.track));
    const ticked = plan.ideas.filter((i) => i.view.on && i.view.tracks.some(keep)).map((i) => i.view.name);
    let label = plan.view.request ? `Revision applied: “${plan.view.request.slice(0, 48)}”` : `Auto mix applied · ${changed.size} ${changed.size === 1 ? "track" : "tracks"}`;
    if (ticked.length) label = `${label} · ${ticked.length === 1 ? "idea" : "ideas"} ${ticked.join(", ")}`;
    const rec = { ...plan.view.record, outcome: only ? ("applied_partly" as const) : ("applied" as const), history_label: label };
    this.host.mutate(label, (p) => {
      // AI-20: a new element is created if it is ticked and enough of the tracks it exists for are applied.
      for (const c of plan.creations) {
        const deps = c.el.depends_on.filter((d) => keep(d) && p.tracks.some((t) => t.id === d));
        if (!keep(c.el.id) || deps.length < c.el.needs) continue;
        const proto = p.tracks.find((t) => t.kind === (c.el.kind === "return" ? "return" : "group")) ?? p.tracks[0];
        const made: Track = {
          ...structuredClone(proto), id: c.el.id, kind: c.el.kind, name: c.el.name, color: 0, role: null, channels: 2, clips: [], sends: [], automation: [],
          ret: c.el.ret, output: { kind: "mix_bus" }, locked: false, mute: false, solo: false,
        };
        made.chain.fader_db = c.level_db;
        if (c.el.kind === "return") {
          p.tracks.push(made);
          for (const s of c.sends) if (keep(s.track)) p.tracks.find((t) => t.id === s.track)?.sends.push({ target: made.id, level_db: s.db });
        } else {
          const members = p.tracks.filter((t) => deps.includes(t.id));
          made.output = structuredClone(members[0].output);
          p.tracks.splice(p.tracks.indexOf(members[0]), 0, made);
          for (const m of members) m.output = { kind: "group", id: made.id };
        }
      }
      for (const c of plan.changes) if (keep(c.track)) setParam(p, c.track, c.param, c.value);
      for (const b of plan.blocks) if (keep(b.track)) b.add(p);
      // AI-27: the ticked ideas, in the same step; UI-13: their name in the notes of the tracks they touch.
      for (const i of plan.ideas) if (i.view.on && i.view.tracks.some(keep)) i.add(p);
      // UI-13: like the core, the idea is named in the note of each track it touches.
      const kept = plan.notes.filter((n) => keep(n.track)).map((n) => ({ ...n }));
      for (const i of plan.ideas.filter((x) => x.view.on)) {
        for (const track of i.view.tracks.filter(keep)) {
          const said = `Idea “${i.view.name}”: ${i.view.rationale}`;
          const n = kept.find((x) => x.track === track);
          if (n) n.text = `${n.text} ${said}`;
          else kept.push({ track, text: said });
        }
      }
      p.ai.notes = kept.map((n, i) => ({ number: i + 1, track: n.track, text: n.text }));
      p.ai.summary = plan.view.summary;
    }, rec.id);
    this.log(rec);
    this.plan = null;
    this.status.proposal = null;
    this.status.listening = "current";
    this.emit();
    return label;
  }

  /** AI-23: a proposal received earlier goes back in review; its plan is built again on the project as it is now. */
  reopen(id: string) {
    if (this.status.running) throw new Error("the assistant is working: wait for it to finish");
    if (this.status.proposal) throw new Error("a proposal is in review: apply or discard it first");
    const p = this.host.project();
    const rec = p?.ai.calls.find((c) => c.id === id && (c.kind === "auto_mix" || c.kind === "revision"));
    if (!p || !rec) throw new Error("no such proposal");
    if (!rec.stored) throw new Error("this proposal was not kept with the project: it cannot be reviewed again");
    this.plan = buildPlan(p, rec.request, null);
    this.plan.view = { ...this.plan.view, record: rec, call_id: rec.id, reopened: true };
    this.status.proposal = this.plan.view;
    this.emit();
  }

  discard() {
    if (!this.plan) throw new Error("no proposal to discard");
    // A proposal opened again keeps what its record says happened to it (AI-23).
    if (!this.plan.view.reopened) this.log({ ...this.plan.view.record, outcome: "discarded" });
    this.plan = null;
    this.status.proposal = null;
    this.status.listening = "current";
    this.emit();
  }

  /** AI-27: the ideas ticked in the review. */
  setIdeas(ids: string[]) {
    const plan = this.plan;
    if (!plan || !this.status.proposal) throw new Error("no proposal in review");
    for (const i of plan.ideas) i.view.on = ids.includes(i.view.id);
    this.status.proposal = { ...plan.view, ideas: plan.ideas.map((i) => ({ ...i.view })) };
    plan.view = this.status.proposal;
    this.emit();
  }

  listen(what: string) {
    if ((what === "proposal" || what.startsWith("idea:")) && !this.status.proposal) throw new Error("no proposal to listen to");
    this.status.listening = what;
    this.emit();
  }

  detail(id: string): CallDetail {
    const p = this.host.project()!;
    const record = p.ai.calls.find((c) => c.id === id);
    if (!record) throw new Error("no such call");
    const tracks = p.tracks.filter((t) => t.kind === "audio");
    const ids = new Map(tracks.map((t, i) => [t.id, `t${i + 1}`]));
    const analysis = {
      version: 2,
      tracks: Object.fromEntries(tracks.map((t, i) => [ids.get(t.id), { name: t.name, role: t.role, locked: t.locked, lufs: -18 - (i % 5) * 1.3, peak_db: -4 - (i % 4), crest_db: 12 + (i % 6), active_pct: 40 + ((i * 13) % 55) }])),
    };
    const brief = p.brief;
    const request = {
      model: record.model,
      messages: [
        { role: "system", content: "You are a mixing engineer. You never hear the audio: you decide mix parameters from numeric measurements…" },
        { role: "user", content: `## Brief\n${JSON.stringify(brief)}\n\n## Analysis\n${JSON.stringify(analysis)}${record.request ? `\n\n## Author's request (top priority)\n${record.request}` : ""}` },
      ],
    };
    const vox = tracks.find((t) => t.role === "lead_vocal");
    const plan = {
      version: 2, iteration: record.iterations - 1, summary: p.ai.summary ?? "Balance by role.",
      tracks: vox ? { [ids.get(vox.id)!]: { gain_db: vox.chain.fader_db, pan: 0, hpf_hz: 90, lpf_hz: null, eq: [{ type: "peak", freq_hz: 220, gain_db: -2.5, q: 1.2 }], compressor: null, saturation: null, sends_db: {}, sections: {}, rationale: "Less mud, more presence." } } : {},
    };
    const response = { model: record.model, choices: [{ message: { content: JSON.stringify(plan) } }], usage: { prompt_tokens: record.tokens_in, completion_tokens: record.tokens_out, cost: record.cost_usd } };
    return {
      record,
      files: [
        ["analysis.json", JSON.stringify(analysis, null, 2)],
        ["plan-0.json", JSON.stringify(plan, null, 2)],
        ["request-0-0.json", JSON.stringify(request)],
        ["response-0-0.json", JSON.stringify(response)],
      ],
    };
  }

  setConfig(c: AiConfig) {
    this.config = c;
    this.emit();
  }
  setKey(k: string | null) {
    this.key = k && k.trim() ? k.trim() : null;
    this.emit();
  }
}

/** A proposal like the design's (4a): every unlocked track in scope gets a reasoned change. */
function buildPlan(p: Project, request: string | null, scope: string[] | null): Plan {
  const changes: Change[] = [];
  const notes: { track: string; text: string }[] = [];
  const diff: ProposalView["diff"] = [];
  const untouched: string[] = [];
  const blocks: Plan["blocks"] = [];
  const rows = (_t: Track, items: [string, string, number, number][]): DiffRow[] => items.map(([label, unit, before, after]) => ({ label, unit, before, after }));
  const targets: Record<string, [number, string]> = {
    kick: [-7.5, "Keeps its click at 3 kHz; 1 dB down with the drum group."],
    snare: [-8.5, "A little more body at 200 Hz, faster compressor release."],
    overheads: [-12, "High-pass at 180 Hz so the kick stays clean."],
    room: [-18, "Down 2 dB and darker: the brief asks for dry drums."],
    bass: [-7.2, "Ducks 2 dB on every kick through a sidechain."],
    acoustic_guitar: [-12.9, "Thinned below 120 Hz, panned slightly left."],
    electric_guitar: [-10.6, "Wide, 2.5 dB dip at 3 kHz where it masked the vocal."],
    keys: [-14.5, "Sent to Echo in the bridge only."],
    strings: [-14, "Up 1.5 dB in the last chorus."],
    lead_vocal: [-4, "Lead vocal sat 2.1 LU under the reference vocal-to-mix balance and was masked by the guitars around 2–4 kHz in the choruses. Raised it 1.8 dB, cut some 220 Hz mud, added presence at 3.2 kHz, and lifted it a further 1.5 dB in the choruses only."],
  };
  for (const t of p.tracks.filter((x) => x.kind === "audio")) {
    if (t.locked || (scope && !scope.includes(t.id))) {
      untouched.push(t.id);
      continue;
    }
    const tg = targets[t.role ?? ""];
    if (!tg) continue;
    let value = tg[0];
    let why = tg[1];
    if (request) {
      const r = request.toLowerCase();
      const named = r.includes(t.name.toLowerCase()) || (t.role === "lead_vocal" && /vocal|voce|vox/.test(r));
      if (!named && scope === null) continue;
      value = Math.round((t.chain.fader_db + (/down|less|quieter|meno|giù/.test(r) ? -1.5 : 1.5)) * 10) / 10;
      why = `As asked: “${request}”.`;
    }
    if (Math.abs(value - t.chain.fader_db) < 0.05 && t.role !== "lead_vocal") continue;
    changes.push({ track: t.id, param: "fader", value });
    const r: [string, string, number, number][] = [["Fader", "dB", t.chain.fader_db, value]];
    if (t.role === "lead_vocal" && !request) {
      changes.push({ track: t.id, param: "eq.2.gain", value: -2.5 }, { track: t.id, param: "eq.3.gain", value: 2 }, { track: t.id, param: "comp.threshold", value: -18 });
      r.push(["EQ 2 · 220 Hz", "dB", eqOf(t.chain).bands[1].gain_db, -2.5], ["EQ 3 · 3.2 kHz", "dB", eqOf(t.chain).bands[2].gain_db, 2], ["Comp threshold", "dB", compOf(t.chain).threshold_db, -18], ["Chorus · fader offset", "dB", 0, 1.5]);
    }
    const dr = rows(t, r);
    // AI-25 (design 4c): a block of the plan is a row of its track's diff, marked new.
    if (t.role === "lead_vocal" && !request && t.chain.blocks.filter((b) => b.fx.type === "deesser").length < 2) {
      dr.push({ label: "De-esser · new block", unit: "", before: null, after: null, text: "6.5 kHz" });
      why = `${why} A second de-esser, narrow at 6.5 kHz, after the compressor.`;
      blocks.push({
        track: t.id,
        add: (q) => {
          const x = q.tracks.find((y) => y.id === t.id);
          if (!x) return;
          const ci = x.chain.blocks.findIndex((b) => b.fx.type === "compressor");
          const id = chainEdit(x.chain, { op: "add", kind: "deesser", at: ci < 0 ? null : ci + 1 }, false);
          const b = x.chain.blocks.find((y) => y.id === id);
          if (b && b.fx.type === "deesser") Object.assign(b.fx, { bypass: false, freq_hz: 6500 });
        },
      });
    }
    notes.push({ track: t.id, text: why });
    diff.push({ track: t.id, name: t.name, note: null, rationale: why, rows: dr });
  }
  // Notes are numbered in plan order; the vocal first, as on the board.
  diff.sort((a, b) => (a.name === "Lead Vox" ? -1 : b.name === "Lead Vox" ? 1 : 0));
  diff.forEach((d, i) => (d.note = i < 4 ? i + 1 : null));
  const summary = request
    ? `Revision: ${request}`
    : "Vocal forward, guitars opened to the sides, fuller choruses. Drums 1 dB down to make room; bass ducks slightly under the kick.";
  // AI-17: an auto mix on the whole project may create returns and groups (design 4a).
  const creations: Creation[] = [];
  const returns = p.tracks.filter((t) => t.kind === "return").length;
  const free = (role: string) => p.tracks.filter((t) => t.kind === "audio" && t.role === role && !untouched.includes(t.id) && t.output.kind === "mix_bus");
  if (!request && !scope && p.ai.allow_structure) {
    const vox = free("lead_vocal")[0];
    const stamp = `${now()}`;
    if (vox && returns < 7 && !p.tracks.some((t) => t.name === "Vocal plate")) {
      creations.push({
        el: { id: `r${stamp}1`, kind: "return", name: "Vocal plate", rationale: "A short, bright space for the vocal alone.", ret: reverbOf("plate"), members: [], depends_on: [vox.id], needs: 1, fits: true, idea: null },
        level_db: -8, sends: [{ track: vox.id, db: -14 }],
      });
      creations.push({
        el: { id: `r${stamp}2`, kind: "return", name: "Slap", rationale: "Slapback: body on the vocal without a tail.", ret: { type: "delay", bypass: false, time_ms: 110, sync: null, feedback: 0, feedback_hpf_hz: 200, feedback_lpf_hz: 5000, mode: "stereo", character: "clean", wow: 0, drive_db: 0 }, members: [], depends_on: [vox.id], needs: 1, fits: true, idea: null },
        level_db: -12, sends: [{ track: vox.id, db: -18 }],
      });
    }
    const pads = [...free("keys"), ...free("strings")];
    if (pads.length >= 2 && !p.tracks.some((t) => t.kind === "group" && t.name === "Pads")) {
      creations.push({
        el: { id: `g${stamp}3`, kind: "group", name: "Pads", rationale: "One bus to tuck the pads under the vocal together.", ret: null, members: pads.map((t) => t.id), depends_on: pads.map((t) => t.id), needs: 2, fits: true, idea: null },
        level_db: 0, sends: [],
      });
    }
  }
  // AI-25 → AI-27: an auto mix on the whole project may carry up to three ideas (design 4c), off by default.
  const ideas: Plan["ideas"] = [];
  const role = (r: string) => p.tracks.find((t) => t.kind === "audio" && t.role === r && !untouched.includes(t.id));
  const named = (ids: string[]) => p.sections.filter((x) => ids.includes(x.name)).length;
  if (!request && !scope && p.ai.allow_ideas) {
    const keys = role("keys");
    const strings = role("strings");
    const vox = role("lead_vocal");
    if (keys) {
      ideas.push({
        view: { id: "i1", name: "Chorus on the keys", rationale: `The ${keys.name} open to the sides in the choruses and leave the centre to the vocal.`, sections: named(["Chorus"]) ? ["Choruses"] : [], elements: [`${keys.name} · Modulation, chorus 1/4, 35 % · new block`, `On in the ${p.sections.filter((x) => x.name === "Chorus").length} choruses only`], tracks: [keys.id], on: false },
        add: (q) => {
          const t = q.tracks.find((x) => x.id === keys.id);
          if (!t) return;
          const id = chainEdit(t.chain, { op: "add", kind: "modulation", at: null }, false);
          const b = t.chain.blocks.find((x) => x.id === id);
          if (b && b.fx.type === "modulation") Object.assign(b.fx, { bypass: false, mix: 0.35 });
        },
      });
    }
    if (strings) {
      ideas.push({
        view: { id: "i2", name: "Filter build to the end", rationale: `The ${strings.name} start dark in the bridge and open into the last chorus, so it lands bigger.`, sections: named(["Bridge"]) ? ["Bridge"] : [], elements: [`${strings.name} · Low-pass resonance 2.0`, "Bridge · low-pass 3 octaves lower"], tracks: [strings.id], on: false },
        add: (q) => {
          const t = q.tracks.find((x) => x.id === strings.id);
          if (t) Object.assign(lpfOf(t.chain), { bypass: false, resonance: 2 });
        },
      });
    }
    if (vox && returns < 7) {
      ideas.push({
        view: { id: "i3", name: "Octave halo in the bridge", rationale: "A shimmer return fed by the lead vocal in the bridge only: a halo an octave up.", sections: named(["Bridge"]) ? ["Bridge"] : [], elements: ["New return · Shimmer +12, 4.5 s", `${vox.name} · send −18 dB in the bridge`], tracks: [vox.id], on: false },
        add: (q) => {
          const proto = q.tracks.find((t) => t.kind === "return") ?? q.tracks[0];
          const id = `r${now()}h`;
          const halo = reverbOf("shimmer");
          if (halo.type === "reverb") halo.decay_s = 4.5;
          const made: Track = { ...structuredClone(proto), id, kind: "return", name: "Halo", color: 0, role: null, channels: 2, clips: [], sends: [], automation: [], ret: halo, output: { kind: "mix_bus" }, locked: false, mute: false, solo: false };
          made.chain.fader_db = -10;
          q.tracks.push(made);
          q.tracks.find((t) => t.id === vox.id)?.sends.push({ target: id, level_db: -18 });
        },
      });
    }
  }
  const view: ProposalView = {
    ideas: ideas.map((i) => ({ ...i.view })),
    call_id: "", kind: request ? "revision" : "auto_mix", summary, diff, new_elements: creations.map((c) => c.el),
    before: { distance_db: 4.8, lufs: -17.9, true_peak_db: -4.2, true_peak_ok: true, low_corr_ok: true, dc_ok: true },
    after: { distance_db: request ? 4.1 : 2.1, lufs: -16.4, true_peak_db: -3.1, true_peak_ok: true, low_corr_ok: true, dc_ok: true },
    iterations: request ? [{ iteration: 1, distance_db: 4.1, valid: true, best: true }] : [{ iteration: 1, distance_db: 3.4, valid: true, best: false }, { iteration: 2, distance_db: 2.1, valid: true, best: true }],
    warnings: [], record: {} as AiCallRecord, trims: !request && p.ai.normalize_trim, untouched, request, reopened: false,
  };
  return { view, changes, creations, notes, ideas, blocks };
}
