// The mix at a glance (P-013, P-018): what a track has on, read from the document without selecting it (UI-8, UI-12),
// and what the selection is connected to (UI-10). Pure reads of the mirror: the only rule here is UI-8's reading of
// "neutral" (in `fx.ts`).
import type { Chain } from "../bindings/Chain";
import type { FxBlock } from "../bindings/FxBlock";
import type { FxKind } from "../bindings/FxKind";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { formatDb } from "../shared/format";
import { INFO, OVERVIEW_ORDER, blockName, blockState, blockTitle, sidechainsOf, type BlockState } from "./fx";

export type { BlockState } from "./fx";

/** One mark of a chain (UI-8, UI-12): a block, in chain order. */
export interface Mark {
  id: string;
  kind: FxKind;
  sig: string;
  state: BlockState;
  /** Track or group the block listens to, when it is on. */
  key: string | null;
  /** Compressor mix (0..1) when it is on and below 100 %: parallel compression. */
  parallel: number | null;
  title: string;
}

export interface ChainGlance {
  marks: Mark[];
  /** Automation present, and who wrote it: the assistant only if it wrote every lane with points. */
  automation: null | { by: "ai" | "user"; params: string[] };
}

/** The marks of a chain, keys named from the project when it is given. */
export function marksOf(c: Chain, p?: Project): Mark[] {
  return c.blocks.map((b) => {
    const state = blockState(b.fx);
    const key = state === "on" ? (sidechainsOf(b.fx)[0] ?? null) : null;
    const keyName = key ? (p?.tracks.find((t) => t.id === key)?.name ?? key) : null;
    return {
      id: b.id,
      kind: b.fx.type,
      sig: INFO[b.fx.type].mark,
      state,
      key,
      parallel: state === "on" && b.fx.type === "compressor" && b.fx.mix < 1 ? b.fx.mix : null,
      title: blockTitle(c, b, keyName),
    };
  });
}

/** State of the chain of a track, group or return (UI-8, UI-12). */
export function chainGlance(t: Track, p?: Project): ChainGlance {
  const lanes = t.automation.filter((l) => l.points.length > 0);
  return {
    marks: marksOf(t.chain, p),
    automation: lanes.length ? { by: lanes.every((l) => l.author === "ai") ? "ai" : "user", params: lanes.map((l) => l.param) } : null,
  };
}

/** Marks that fit (16t): all of them up to five, else the first four and the rest behind a "+n". */
export const MARKS_SHOWN = 5;
export function fitMarks(marks: Mark[]): { shown: Mark[]; more: Mark[] } {
  if (marks.length <= MARKS_SHOWN) return { shown: marks, more: [] };
  return { shown: marks.slice(0, MARKS_SHOWN - 1), more: marks.slice(MARKS_SHOWN - 1) };
}

/** Level of a send, as the marks write it ("−14.0"). */
export const sendDb = (db: number) => formatDb(db);

/** What the selection is connected to (UI-10): the reason, in words, for each track, group or return it touches. */
export interface Relations {
  reasons: Map<string, string>;
  /** The selected track when it is a return: its send dots and bars are marked. */
  ret: string | null;
}

export const NO_RELATIONS: Relations = { reasons: new Map(), ret: null };

export function relations(p: Project, sel: string | null): Relations {
  const s = sel ? p.tracks.find((t) => t.id === sel) : undefined;
  if (!s) return NO_RELATIONS;
  const reasons = new Map<string, string>();
  const add = (id: string, why: string) => {
    if (id !== s.id && !reasons.has(id)) reasons.set(id, why);
  };
  // The group that takes its output.
  if (s.output.kind === "group") add(s.output.id, `takes ${s.name}`);
  // The returns it sends to, with the level.
  for (const snd of s.sends) {
    const r = p.tracks.find((t) => t.id === snd.target);
    if (r) add(r.id, `${s.name} ${sendDb(snd.level_db)} dB`);
  }
  // Whoever it listens to.
  for (const b of s.chain.blocks) {
    for (const k of sidechainsOf(b.fx)) {
      const src = p.tracks.find((t) => t.id === k);
      if (src) add(src.id, `${s.name}'s ${blockName(s.chain, b.id)} listens to it`);
    }
  }
  for (const t of p.tracks) {
    // Whoever sends to it, with the level.
    const snd = t.sends.find((x) => x.target === s.id);
    if (snd) add(t.id, `sends ${sendDb(snd.level_db)} dB`);
    // Whoever's block listens to it.
    if (t.chain.blocks.some((b) => sidechainsOf(b.fx).includes(s.id))) add(t.id, `listens to ${s.name}`);
  }
  return { reasons, ret: s.kind === "return" ? s.id : null };
}

/** Frequency as a cell writes it: "90", "9k", "2.5k". */
export function shortHz(hz: number): string {
  if (hz < 1000) return String(Math.round(hz));
  const k = hz / 1000;
  return `${k >= 10 ? Math.round(k) : Number(k.toFixed(1))}k`;
}

const n1 = (v: number) => Number(v.toFixed(1)).toString().replace("-", "−");

/** The main value of a block, as the Overview writes it (UI-11). */
export function cellValue(b: FxBlock, state: BlockState): string {
  const fx = b.fx;
  if (state === "off") return "off";
  switch (fx.type) {
    case "hpf":
    case "lpf":
      return shortHz(fx.freq_hz);
    case "eq": {
      const n = fx.bands.filter((x) => x.enabled && x.gain_db !== 0).length;
      return n ? `${n} band${n === 1 ? "" : "s"}` : "flat";
    }
    case "compressor":
      return `${n1(fx.ratio)}:1`;
    case "saturation":
      return state === "neutral" ? "dry" : `${n1(fx.drive_db)} dB`;
    case "gate":
      return `${n1(fx.threshold_db)}`;
    case "deesser":
      return shortHz(fx.freq_hz);
    case "dynamic_eq":
      return shortHz(fx.bands.find((x) => x.enabled)?.freq_hz ?? fx.bands[0].freq_hz);
    case "transient":
      return `${fx.attack_db > 0 ? "+" : ""}${n1(fx.attack_db)}`;
    case "clipper":
      return `${n1(fx.threshold_db)}`;
    case "multiband":
      return `${fx.bands.length} bands`;
    case "exciter":
    case "bass_enhancer":
      return shortHz(fx.freq_hz);
    case "width":
      return `${Math.round(fx.width * 100)}%`;
    case "modulation":
      return fx.mode;
    case "tremolo":
      return fx.mode === "autopan" ? "pan" : "trem";
    case "harmonizer":
      return fx.voices.filter((v) => v.enabled).map((v) => `${v.semitones > 0 ? "+" : ""}${Math.round(v.semitones)}`).join(" ") || "—";
    case "doubler":
      return `${n1(fx.detune_cents)} ct`;
  }
}

/** The Overview's block rows (16u): one per kind present in any chain of the project, the five of always first. */
export function overviewKinds(p: Project): FxKind[] {
  const present = new Set(p.tracks.flatMap((t) => t.chain.blocks.map((b) => b.fx.type)));
  return OVERVIEW_ORDER.filter((k, i) => i < 5 || present.has(k));
}
