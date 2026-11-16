// The demo project of the mock backend (FOUNDATION §5.6): the song of the design boards, built as a real
// `Project` document so every screen can be exercised in the browser. Mock data only; never shipped as logic.
import { defaultChain as defaultMasterChain } from "./mockChain";
import { chainEdit, classicBlocks, compOf, eqOf, hpfOf, lpfOf, satOf } from "./mockFx";
import type { FxKind } from "../bindings/FxKind";
import type { Chain } from "../bindings/Chain";
import type { Clip } from "../bindings/Clip";
import type { MasterState } from "../bindings/MasterState";
import type { Project } from "../bindings/Project";
import type { Role } from "../bindings/Role";
import type { Track } from "../bindings/Track";

export const DEMO_RATE = 48000;
const S = DEMO_RATE;

export function defaultChain(): Chain {
  return { trim_db: 0, blocks: classicBlocks(), fader_db: 0, pan: 0 };
}

function track(id: string, name: string, role: Role | null, channels: number, color: number): Track {
  return {
    id,
    kind: "audio",
    name,
    color,
    role,
    channels,
    output: { kind: "mix_bus" },
    clips: [],
    chain: defaultChain(),
    ret: null,
    sends: [],
    automation: [],
    locked: false,
    mute: false,
    solo: false,
  };
}

function masterDefault(): MasterState {
  return {
    chain: defaultMasterChain(),
    target: { preset: "streaming", lufs: -14, true_peak_db: -1 },
    reference: null,
    mixdown: null,
    metadata: { title: "", artist: "", album: "", year: "", genre: "", track_number: "", isrc: "", cover: null },
    export: { wav: true, flac: false, aac: false, sample_rate: 48000, bit_depth: 24, aac_kbps: 256, folder: null },
    target_kept: false,
    sections: [],
  };
}

// Song of the design boards (design/tools/lib.mjs): 222 s, 12 stems, two groups, a plate and an echo.
const TRACKS: [string, Role, number, number, number, string | null, number][] = [
  // name, role, channels, fader dB, pan, group, colour
  ["Kick", "kick", 1, -6.5, 0, "g1", 2],
  ["Snare", "snare", 1, -8.0, 0, "g1", 2],
  ["Overheads", "overheads", 2, -11.5, 0, "g1", 2],
  ["Room", "room", 2, -16.0, 0, "g1", 2],
  ["Bass DI", "bass", 1, -7.2, 0, null, 1],
  ["Acoustic", "acoustic_guitar", 2, -12.4, -0.3, "g2", 5],
  ["Gtr Left", "electric_guitar", 1, -10.1, -0.8, "g2", 4],
  ["Gtr Right", "electric_guitar", 1, -10.3, 0.8, "g2", 4],
  ["Rhodes", "keys", 2, -14.0, 0.2, null, 8],
  ["Strings", "strings", 2, -15.5, 0, null, 6],
  ["Lead Vox", "lead_vocal", 1, -5.8, 0, null, 7],
  ["BVs", "backing_vocal", 2, -13.0, 0, null, 11],
];
const CLIPS: [number, number][][] = [
  [[14, 144], [168, 214]], [[16, 144], [168, 214]], [[0, 144], [166, 222]], [[0, 144], [166, 222]],
  [[16, 112], [112, 144], [168, 216]], [[0, 222]], [[48, 80], [112, 144], [168, 208]],
  [[48, 80], [112, 144], [168, 208]], [[0, 48], [80, 112], [144, 168], [208, 222]],
  [[80, 144], [144, 208]], [[16, 46], [48, 80], [82, 110], [112, 144], [146, 166], [168, 208]],
  [[48, 80], [112, 144], [168, 208]],
];
const SECTIONS: [string, number, number][] = [
  ["Intro", 0, 16], ["Verse 1", 16, 48], ["Chorus", 48, 80], ["Verse 2", 80, 112],
  ["Chorus", 112, 144], ["Bridge", 144, 168], ["Chorus", 168, 208], ["Outro", 208, 222],
];

export function demoProject(): Project {
  let next = 1;
  const id = (p: string) => `${p}${next++}`;
  const p: Project = {
    schema_version: 1,
    name: "Confidence",
    sample_rate: DEMO_RATE,
    brief: { genre: "indie rock", mood: "warm, intimate, a little rough", notes: "The vocal must sit in front. Dry drums." },
    tempo: { bpm: 92, beats_per_bar: 4, beat_unit: 4 },
    media: [],
    references: [],
    tracks: [],
    mix_bus: { fader_db: 0 },
    sections: [],
    snapshots: [],
    master: masterDefault(),
    ai: { normalize_trim: true, allow_structure: true, allow_ideas: true, spend_cap_usd: null, calls: [], notes: [], summary: null },
    next_id: 0,
  };
  const g1 = { ...track("g1", "Drums", null, 2, 0), kind: "group" as const, chain: { ...defaultChain(), fader_db: -4 } };
  const g2 = { ...track("g2", "Guitars", null, 2, 0), kind: "group" as const, chain: { ...defaultChain(), fader_db: -6.5 } };
  next = 3;
  TRACKS.forEach(([name, role, ch, db, pan, group, color], i) => {
    const t = track(id("t"), name, role, ch, color);
    t.chain.fader_db = db;
    t.chain.pan = pan;
    if (group) t.output = { kind: "group", id: group };
    const mediaId = `m${i + 1}`.padEnd(32, "0");
    p.media.push({
      id: mediaId, path: `audio/${mediaId}.wav`, original_name: `${name}.wav`, format: "wav", channels: ch,
      sample_rate: DEMO_RATE, frames: 222 * S, encoding: "PCM 24-bit", converted: false,
    });
    t.clips = CLIPS[i].map(([a, b]): Clip => ({
      id: id("c"), media: mediaId, start: a * S, offset: a * S, length: (b - a) * S, gain_db: 0,
      fade_in: { length: 0, shape: "equal_power" }, fade_out: { length: 0, shape: "equal_power" }, mute: false,
    }));
    if (name === "BVs") t.locked = true;
    p.tracks.push(t);
  });
  // What each track has on, as in the design boards (lib.mjs GL, P-013): every state of the chain marks shows in the
  // demo — working, neutral and bypassed blocks, a keyed and a parallel compressor, sends to both returns,
  // automation by hand and by the assistant.
  const by = (n: string) => p.tracks.find((t) => t.name === n)!;
  const band = (i: number, freq_hz: number, gain_db: number) => ({ enabled: true, kind: (i === 0 ? "low_shelf" : i === 3 ? "high_shelf" : "peak") as "peak", freq_hz, gain_db, q: 1 });
  const chain = (n: string, o: { hp?: number; lp?: number; eq?: [number, number, number][]; ratio?: number; mix?: number; sat?: number }) => {
    const c = by(n).chain;
    if (o.hp) Object.assign(hpfOf(c), { bypass: false, freq_hz: o.hp });
    if (o.lp) Object.assign(lpfOf(c), { bypass: false, freq_hz: o.lp });
    for (const [i, f, g] of o.eq ?? []) eqOf(c).bands[i] = band(i, f, g);
    if (o.ratio) Object.assign(compOf(c), { bypass: false, ratio: o.ratio, mix: o.mix ?? 1 });
    if (o.sat) Object.assign(satOf(c), { bypass: false, drive_db: o.sat, mix: 1 });
  };
  chain("Kick", { hp: 40, eq: [[0, 60, 2], [1, 350, -3], [2, 3000, 2]], ratio: 4 });
  chain("Snare", { hp: 90, eq: [[1, 200, 1.5], [2, 5000, 2]], ratio: 3, mix: 0.35, sat: 3 });
  chain("Overheads", { hp: 200, eq: [[1, 500, -2], [3, 10000, 1.5]] });
  chain("Room", { ratio: 6 });
  chain("Bass DI", { hp: 40, eq: [[0, 80, 1.5], [1, 250, -2]], ratio: 4 });
  chain("Acoustic", { hp: 80, eq: [[0, 120, -2], [1, 800, -1.5], [3, 9000, 2]] });
  chain("Gtr Left", { hp: 100, lp: 9000, eq: [[1, 400, -1.5], [2, 3000, -2.5]] });
  chain("Gtr Right", { hp: 100, lp: 9000, eq: [[1, 400, -1.5], [2, 3000, -2.5]] });
  chain("Rhodes", { eq: [[1, 300, -2]] });
  chain("Strings", { hp: 120 });
  chain("BVs", { hp: 110, eq: [[1, 300, -2], [3, 9000, 1.5]], ratio: 2.5 });
  const vox = by("Lead Vox");
  Object.assign(hpfOf(vox.chain), { bypass: false, freq_hz: 90 });
  eqOf(vox.chain).bands[1] = { enabled: true, kind: "peak", freq_hz: 220, gain_db: -2.5, q: 1.4 };
  eqOf(vox.chain).bands[2] = { enabled: true, kind: "peak", freq_hz: 3200, gain_db: 2, q: 1.2 };
  Object.assign(compOf(vox.chain), { bypass: false, threshold_db: -18, ratio: 3 });
  Object.assign(satOf(vox.chain), { bypass: false, drive_db: 4, mix: 1, character: "tube" });
  // The blocks of P-018 as the boards show them (16a, 16t): new kinds added where they serve, in free order.
  const add = (n: string, kind: FxKind, at: number | null, on = true, f: (fx: Record<string, unknown>) => void = () => {}) => {
    const c = by(n).chain;
    const id = chainEdit(c, { op: "add", kind, at }, false)!;
    const b = c.blocks.find((x) => x.id === id)!;
    b.fx.bypass = !on;
    f(b.fx as unknown as Record<string, unknown>);
  };
  const drop = (n: string, id: string) => chainEdit(by(n).chain, { op: "remove", id }, false);
  drop("Lead Vox", "lpf");
  add("Lead Vox", "deesser", 3, true, (x) => Object.assign(x, { threshold_db: -28 }));
  add("Lead Vox", "doubler", null, false, (x) => Object.assign(x, { detune_cents: 12 }));
  drop("Kick", "lpf");
  drop("Kick", "sat");
  add("Kick", "gate", 1);
  add("Kick", "transient", null, true, (x) => Object.assign(x, { attack_db: 4 }));
  drop("Snare", "lpf");
  add("Snare", "gate", 1);
  add("Snare", "transient", 2, true, (x) => Object.assign(x, { attack_db: 3 }));
  drop("Bass DI", "lpf");
  add("Bass DI", "bass_enhancer", null, true);
  drop("Gtr Left", "comp");
  drop("Gtr Left", "sat");
  add("Gtr Left", "dynamic_eq", null, true);
  drop("Gtr Right", "comp");
  drop("Gtr Right", "sat");
  add("Gtr Right", "dynamic_eq", null, true);
  add("Gtr Right", "tremolo", null, true);
  for (const id of ["hpf", "lpf", "comp", "sat"]) drop("Rhodes", id);
  add("Rhodes", "modulation", null, true);
  add("Rhodes", "width", null, true);
  drop("Strings", "comp");
  drop("Strings", "sat");
  add("Strings", "width", null, true);
  vox.sends = [{ target: "r20", level_db: -14 }, { target: "r21", level_db: -20 }];
  by("Snare").sends = [{ target: "r20", level_db: -18 }];
  by("Acoustic").sends = [{ target: "r20", level_db: -20 }];
  by("BVs").sends = [{ target: "r20", level_db: -12 }];
  by("Rhodes").sends = [{ target: "r21", level_db: -16 }];
  compOf(by("Bass DI").chain).sidechain = { source: by("Kick").id, hpf_hz: 100 };
  for (const n of ["Gtr Left", "Gtr Right"]) {
    const d = by(n).chain.blocks.find((b) => b.fx.type === "dynamic_eq")!.fx;
    if (d.type === "dynamic_eq") d.bands[0] = { ...d.bands[0], sidechain: vox.id, range_db: -2.5 };
  }
  by("Bass DI").automation = [{
    param: "fader", author: "user", visible: false,
    points: [{ frame: 112 * S, value: -7.2 }, { frame: 144 * S, value: -6.2 }],
  }];
  // The doubler plays in the choruses only: a bypass lane the assistant wrote (AI-6).
  const steps = (on: [number, number][]) => on.flatMap(([a, b]) => [{ frame: a * S, value: 0 }, { frame: b * S, value: 1 }]);
  const dblLane = { param: "bypass.dbl", author: "ai" as const, visible: false, points: [{ frame: 0, value: 1 }, ...steps([[48, 80], [112, 144], [168, 208]])] };
  vox.automation = [dblLane, {
    param: "fader", author: "ai", visible: true,
    points: [
      { frame: 46 * S, value: -5.8 }, { frame: 48 * S, value: -4.3 }, { frame: 80 * S, value: -4.3 }, { frame: 82 * S, value: -5.8 },
    ],
  }];
  eqOf(g1.chain).bands[1] = band(1, 250, -1.5);
  Object.assign(compOf(g1.chain), { bypass: false, ratio: 2, mix: 0.5 });
  eqOf(g2.chain).bands[1] = band(1, 600, -1);
  eqOf(g2.chain).bands[2] = band(2, 2800, -1.5);
  p.tracks.splice(0, 0, g1);
  p.tracks.splice(7, 0, g2);
  p.tracks.push({ ...track("r20", "Plate", null, 2, 0), kind: "return", ret: { type: "reverb", bypass: false, algorithm: "plate", decay_s: 1.2, size: 0.5, damping: 0.5, modulation: 0.2, predelay_ms: 30, hpf_hz: 300, lpf_hz: 12000, shimmer_semitones: 12, shimmer_amount: 0 }, chain: { ...defaultChain(), blocks: [], fader_db: -12 } });
  p.tracks.push({ ...track("r21", "Echo", null, 2, 0), kind: "return", ret: { type: "delay", bypass: false, time_ms: 326, sync: "1/8.", feedback: 0.25, feedback_hpf_hz: 200, feedback_lpf_hz: 6000, mode: "stereo", character: "clean", wow: 0, drive_db: 0 }, chain: { ...defaultChain(), blocks: [], fader_db: -15 } });
  const tid = (n: string) => p.tracks.find((t) => t.name === n)!.id;
  // The notes of the design boards (3a): numbered in plan order, one per changed track.
  p.ai.summary = "Vocal forward, guitars opened to the sides, fuller choruses. Drums 1 dB down to make room; bass ducks slightly under the kick.";
  p.ai.notes = [
    { number: 1, track: tid("Kick"), text: "Drums sit 1 dB lower as a group so the vocal has room; the kick keeps its click at 3 kHz." },
    { number: 2, track: tid("Bass DI"), text: "Ducks 2 dB on every kick (sidechain) to free the low end without thinning the bass." },
    { number: 3, track: tid("Gtr Left"), text: "Opened wide and dipped 2.5 dB at 3 kHz, only where they masked the vocal." },
    { number: 4, track: tid("Lead Vox"), text: "Lead vocal sat 2.1 LU under the reference vocal-to-mix balance and was masked by the guitars around 2–4 kHz in the choruses. Raised it 1.8 dB, cut some 220 Hz mud, added presence at 3.2 kHz, and lifted it a further 1.5 dB in the choruses only." },
  ];
  // Calls of the design's AI usage board (12a): applied, cancelled, applied in part, failed, discarded.
  const day = Date.UTC(2026, 10, 10, 20, 0);
  const call = (min: number, kind: "auto_mix" | "revision" | "master" | "master_revision", model: string, effort: string, its: number, att: number, tin: number, tout: number, cost: number, dur: number, outcome: "applied" | "applied_partly" | "discarded" | "failed" | "cancelled", request: string | null = null) => {
    const id = `${day + min * 60000}-${kind === "auto_mix" ? "mix" : kind === "revision" ? "rev" : "master"}`;
    return { id, kind, started_ms: day + min * 60000, duration_ms: dur * 1000, model, effort, decider: "openrouter", attempts: att, iterations: its, tokens_in: tin, tokens_out: tout, cost_usd: cost, cost_estimated: false, outcome, history_label: outcome.startsWith("applied") ? (kind === "auto_mix" ? "Auto mix applied · 11 tracks" : `Revision applied: “${request ?? ""}”`) : null, request, path: `ai/${id}`, error: outcome === "failed" ? "the model did not return a valid plan after 2 retries" : null, stored: outcome === "applied" || outcome === "applied_partly" || outcome === "discarded" };
  };
  p.ai.calls = [
    call(40, "revision", "anthropic/claude-sonnet-5.5", "medium", 1, 1, 3900, 400, 0.02, 9, "applied", "less reverb on the snare"),
    call(58, "auto_mix", "openai/gpt-6.1-sol", "medium", 3, 3, 31800, 4400, 0.13, 91, "discarded"),
    call(72, "revision", "openai/gpt-6.1-sol", "high", 1, 3, 13600, 1300, 0.09, 38, "failed", "brighter guitars"),
    call(80, "revision", "openai/gpt-6.1-sol", "medium", 1, 1, 4600, 500, 0.04, 11, "applied_partly", "vocal a touch warmer in the verses"),
    call(101, "auto_mix", "openai/gpt-6.1-sol", "medium", 1, 1, 8600, 1200, 0.07, 42, "cancelled"),
    call(104, "auto_mix", "openai/gpt-6.1-sol", "medium", 2, 2, 27800, 3600, 0.11, 66, "applied"),
  ];
  SECTIONS.forEach(([name, a, b], i) => p.sections.push({ id: `s${i + 1}`, name, color: (i % 12) + 1, start: a * S, end: b * S }));
  p.next_id = 100;
  return p;
}

/** Deterministic pseudo-waveform for a media file (mock peaks). */
export function demoPeaks(media: string, start: number, end: number, columns: number, channels: number): [number, number][][] {
  const seed = [...media].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);
  const out: [number, number][][] = [];
  for (let c = 0; c < channels; c++) {
    const col: [number, number][] = [];
    for (let i = 0; i < columns; i++) {
      const t = (start + ((end - start) * i) / columns) / S;
      const beat = Math.abs(Math.sin(t * Math.PI * (1.5 + (seed % 5) * 0.25)));
      const v = 0.15 + 0.55 * beat ** 3 + 0.12 * Math.abs(Math.sin(t * 7.3 + seed + c));
      col.push([-v, v * 0.96]);
    }
    out.push(col);
  }
  return out;
}
