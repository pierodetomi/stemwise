// The blocks of a chain (FX-10, UI-12), as the interface reads them: names, marks, families and where each parameter
// lives come from the catalog the core generates (`fx_catalog.json`); here only reads of the mirror and the words the
// rows and marks print.
import catalog from "../bindings/fx_catalog.json";
import specs from "../bindings/param_specs.json";
import type { Chain } from "../bindings/Chain";
import type { Fx } from "../bindings/Fx";
import type { FxBlock } from "../bindings/FxBlock";
import type { FxInfo } from "../bindings/FxInfo";
import type { FxKind } from "../bindings/FxKind";
import type { Track } from "../bindings/Track";

export const CATALOG = catalog as unknown as FxInfo[];
/** Labels of the parameters, by spec id (`gate.range` → "Range"). */
export const SPEC_LABEL = new Map((specs as { id: string; label: string }[]).map((s) => [s.id, s.label]));
export const INFO = Object.fromEntries(CATALOG.map((c) => [c.kind, c])) as Record<FxKind, FxInfo>;
const BY_KEY = new Map(CATALOG.map((c) => [c.key, c]));
/** Per-kind and total limits (FX-10, FX-11). */
export const PER_KIND = 2;
export const TRACK_BLOCKS = 12;
export const RETURN_BLOCKS = 6;
/** The five blocks a new track starts with, in their starting order (FX-1). */
export const CLASSIC: FxKind[] = ["hpf", "lpf", "eq", "compressor", "saturation"];

/** The kind a block id names (`comp2` → compressor). */
export function kindOfId(id: string): FxKind | null {
  return BY_KEY.get(id.replace(/\d+$/, ""))?.kind ?? null;
}

/** A parameter id with its block id replaced by the kind's key: the id its spec is listed under (`eq2.1.gain` → `eq.1.gain`). */
export function specKey(param: string): string {
  if (param.startsWith("bypass.")) {
    const k = kindOfId(param.slice(7));
    return k ? `bypass.${INFO[k].key}` : param;
  }
  const dot = param.indexOf(".");
  if (dot < 0) return param;
  const k = kindOfId(param.slice(0, dot));
  // Specs list one band or voice per kind (the first): every band reads its range from it, except an EQ's four
  // bands, which are listed one by one since schema 3.
  const rest = k && k !== "eq" ? param.slice(dot).replace(/^\.\d+\./, ".1.").replace(/^\.xover\.\d+$/, ".xover.1") : param.slice(dot);
  return k ? `${INFO[k].key}${rest}` : param;
}

function locate(fx: Fx, key: string): { obj: Record<string, unknown>; field: string } | null {
  const info = INFO[fx.type];
  let index: number | null = null;
  let plain = key;
  const m = /^(\d+)\.(.+)$/.exec(key) ?? null;
  const x = /^xover\.(\d+)$/.exec(key);
  let pattern: string;
  if (x) {
    index = Number(x[1]) - 1;
    pattern = "xover.<n>";
  } else if (m) {
    index = Number(m[1]) - 1;
    plain = m[2];
    pattern = `<n>.${plain}`;
  } else pattern = plain;
  const p = info.params.find((q) => q.key === pattern);
  if (!p) return null;
  const parts = p.path.replace("<n>", String(index ?? 0)).split(".");
  let obj = fx as unknown as Record<string, unknown>;
  for (const part of parts.slice(0, -1)) {
    const next = obj?.[part];
    if (next === null || next === undefined || typeof next !== "object") return null;
    obj = next as Record<string, unknown>;
  }
  return { obj, field: parts[parts.length - 1] };
}

/** A block parameter's value (`freq`, `2.gain`, `xover.1`), or null if the block has no such band or voice. */
export function fxGet(fx: Fx, key: string): number | null {
  const l = locate(fx, key);
  const v = l?.obj[l.field];
  return typeof v === "number" ? v : null;
}

/** Sets a block parameter in place (the mock backend; the real one does it in the core). */
export function fxSet(fx: Fx, key: string, v: number): boolean {
  const l = locate(fx, key);
  if (!l || typeof l.obj[l.field] !== "number") return false;
  l.obj[l.field] = v;
  if ((fx.type === "modulation" || fx.type === "tremolo") && key === "rate") fx.sync = null;
  return true;
}

/** The id a new block of a kind gets: its key, then the key numbered from 2 (the core's `Chain::free_id`). */
export function freeId(c: Chain, kind: FxKind): string {
  const key = INFO[kind].key;
  for (let n = 1; ; n++) {
    const id = n === 1 ? key : `${key}${n}`;
    if (!c.blocks.some((b) => b.id === id)) return id;
  }
}

/**
 * Where a block plays when it is on in some sections only (16a "Choruses"): the sections a bypass lane leaves it on in.
 * `null` when it has no such lane or plays in all of them.
 */
export function playsIn(t: Track, id: string, sections: { name: string; start: number; end: number }[]): string | null {
  const lane = t.automation.find((l) => l.param === `bypass.${id}` && l.points.length > 0);
  if (!lane || !sections.length) return null;
  const at = (f: number) => {
    let v = lane.points[0].value;
    for (const p of lane.points) if (p.frame <= f) v = p.value;
    return v;
  };
  const on = sections.filter((s) => at((s.start + s.end) / 2) < 0.5);
  if (!on.length || on.length === sections.length) return on.length ? null : "Never";
  const names = [...new Set(on.map((s) => s.name.replace(/\s*\d+$/, "")))];
  if (names.length === 1) return on.length > 1 ? `${names[0]}${/(s|x|z|ch|sh)$/.test(names[0]) ? "es" : "s"}` : on[0].name;
  return on.length > 2 ? `${on[0].name} +${on.length - 1}` : on.map((s) => s.name).join(" · ");
}

export const blockOf = (t: Track, id: string): FxBlock | undefined => t.chain.blocks.find((b) => b.id === id);
export const firstOf = (c: Chain, kind: FxKind): FxBlock | undefined => c.blocks.find((b) => b.fx.type === kind);

/** Bypassed, on but changing nothing, working (UI-8; the core's `Fx::neutral`). */
export type BlockState = "off" | "neutral" | "on";

export function neutral(fx: Fx): boolean {
  switch (fx.type) {
    case "eq":
      return fx.bands.every((b) => !b.enabled || b.gain_db === 0);
    case "saturation":
      return fx.mix === 0;
    case "transient":
      return fx.mix === 0 || (fx.attack_db === 0 && fx.sustain_db === 0);
    case "exciter":
      return fx.amount === 0;
    case "bass_enhancer":
      return fx.mix === 0 || fx.harmonics === 0;
    case "width":
      return fx.width === 1 && fx.mono_below_hz === 0;
    case "modulation":
      return fx.mix === 0 || fx.depth === 0;
    case "tremolo":
      return fx.depth === 0;
    case "harmonizer":
      return fx.mix === 0 || fx.voices.every((v) => !v.enabled);
    case "doubler":
      return fx.mix === 0;
    case "dynamic_eq":
      return fx.bands.every((b) => !b.enabled || b.range_db === 0);
    case "deesser":
      return fx.max_reduction_db === 0;
    case "gate":
      return fx.range_db === 0;
    case "multiband":
      return fx.bands.every((b) => b.bypass || (b.ratio === 1 && b.gain_db === 0));
    default:
      return false;
  }
}

export function blockState(fx: Fx): BlockState {
  return fx.bypass ? "off" : neutral(fx) ? "neutral" : "on";
}

/** The name a block shows: its kind, numbered in chain order when there are two ("EQ 1", "EQ 2"). */
export function blockName(c: Chain, id: string): string {
  const b = c.blocks.find((x) => x.id === id);
  if (!b) return id;
  const same = c.blocks.filter((x) => x.fx.type === b.fx.type);
  const label = INFO[b.fx.type].label;
  return same.length < 2 ? label : `${label} ${same.indexOf(b) + 1}`;
}

/** Sources a block listens to (FX-3, FX-13, FX-15). */
export function sidechainsOf(fx: Fx): string[] {
  switch (fx.type) {
    case "compressor":
      return fx.sidechain ? [fx.sidechain.source] : [];
    case "gate":
      return fx.sidechain ? [fx.sidechain] : [];
    case "dynamic_eq":
      return fx.bands.filter((b) => b.enabled && b.sidechain).map((b) => b.sidechain as string);
    default:
      return [];
  }
}

const n1 = (v: number) => Number(v.toFixed(1)).toString().replace("-", "−");
export const hzText = (f: number) => (f >= 1000 ? `${n1(f / 1000)} kHz` : `${Math.round(f)} Hz`);
const pct = (v: number) => `${Math.round(v * 100)} %`;
const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
const signedDb = (v: number) => `${v > 0 ? "+" : ""}${n1(v)} dB`;

/** What a block is set to, in a few words (the value of its row, 16a). */
export function blockSummary(fx: Fx): string {
  switch (fx.type) {
    case "hpf":
    case "lpf":
      return Math.abs(fx.resonance - Math.SQRT1_2) < 1e-3 ? hzText(fx.freq_hz) : `${hzText(fx.freq_hz)} · Q ${n1(fx.resonance)}`;
    case "eq": {
      const n = fx.bands.filter((x) => x.enabled).length;
      return n ? `${n} band${n > 1 ? "s" : ""}` : "No band set";
    }
    case "compressor":
      return fx.mix < 1 ? `${n1(fx.ratio)}:1 · ${pct(fx.mix)}` : `${n1(fx.ratio)}:1`;
    case "saturation":
      return `${cap(fx.character)} · ${n1(fx.drive_db)} dB`;
    case "gate":
      return `${n1(fx.threshold_db)} dB · ${n1(fx.range_db)} dB range`;
    case "deesser":
      return hzText(fx.freq_hz);
    case "dynamic_eq": {
      const b = fx.bands.filter((x) => x.enabled);
      return b.length ? b.map((x) => hzText(x.freq_hz)).join(" · ") : "No band set";
    }
    case "transient":
      return `${signedDb(fx.attack_db)} attack`;
    case "clipper":
      return `${n1(fx.threshold_db)} dBFS`;
    case "multiband":
      return `${fx.bands.length} bands`;
    case "exciter":
      return `${hzText(fx.freq_hz)} · ${pct(fx.amount)}`;
    case "bass_enhancer":
      return `${hzText(fx.freq_hz)} · ${pct(fx.harmonics)}`;
    case "width":
      return pct(fx.width);
    case "modulation":
      return `${cap(fx.mode)} · ${fx.sync ?? `${n1(fx.rate_hz)} Hz`}`;
    case "tremolo":
      return `${fx.mode === "autopan" ? "Auto-pan" : "Tremolo"} · ${fx.sync ?? `${n1(fx.rate_hz)} Hz`}`;
    case "harmonizer": {
      const v = fx.voices.filter((x) => x.enabled).map((x) => `${x.semitones > 0 ? "+" : ""}${Math.round(x.semitones)}`);
      return v.length ? `${v.join(" ")} st` : "No voice";
    }
    case "doubler":
      return `${n1(fx.detune_cents)} ct`;
  }
}

/** Words for a mark's tooltip and screen-reader label (UI-8, UI-12). */
export function blockTitle(c: Chain, b: FxBlock, keyName: string | null): string {
  const name = blockName(c, b.id);
  const s = blockState(b.fx);
  if (s === "off") return `${name} · off`;
  if (s === "neutral") return `${name} · on, changes nothing`;
  const parts: string[] = [];
  if (keyName) parts.push(`listens to ${keyName}`);
  if (b.fx.type === "compressor" && b.fx.mix < 1) parts.push(`parallel, ${pct(b.fx.mix)}`);
  return `${name} · ${parts.length ? parts.join(", ") : "on"}`;
}

/** The kinds the "Add block" menu offers and, for the others, why not (16b). */
export function addable(c: Chain, ret: boolean): { kind: FxKind; why: string | null }[] {
  const cap = ret ? RETURN_BLOCKS : TRACK_BLOCKS;
  return CATALOG.map((i) => {
    const n = c.blocks.filter((b) => b.fx.type === i.kind).length;
    const why = c.blocks.length >= cap ? `The chain holds ${cap} blocks: remove one first` : n >= PER_KIND ? "Two already: remove one first" : null;
    return { kind: i.kind, why };
  });
}

/** What each kind does, in plain words (the "Add block" menu, 16b). */
export const PURPOSE: Record<FxKind, string> = {
  gate: "Silences what sits between the hits",
  deesser: "Tames the s and t that stick out",
  dynamic_eq: "A band that moves only when needed",
  transient: "More or less attack, no threshold",
  compressor: "Evens out the level",
  clipper: "Shaves the tallest peaks",
  multiband: "A compressor per range",
  hpf: "Removes the lows below a frequency",
  lpf: "Removes the highs above a frequency",
  eq: "Four bands on the spectrum",
  saturation: "Warmth and grit, four characters",
  exciter: "Adds sparkle to the top",
  bass_enhancer: "Bass you hear on small speakers",
  width: "Narrower or wider stereo",
  modulation: "Chorus, flanger, phaser",
  tremolo: "Level or pan that moves in time",
  harmonizer: "Voices at a fixed interval",
  doubler: "A second performance, inside the track",
};

/** Columns of the "Add block" menu (16b): Space and Movement share one, Space having a single block. */
export const MENU_COLUMNS: { title: string; kinds: FxKind[] }[] = [
  { title: "Dynamics", kinds: ["gate", "deesser", "dynamic_eq", "transient", "compressor", "clipper", "multiband"] },
  { title: "Tone", kinds: ["hpf", "lpf", "eq", "saturation", "exciter", "bass_enhancer"] },
  { title: "Space & movement", kinds: ["width", "modulation", "tremolo"] },
  { title: "Pitch", kinds: ["harmonizer", "doubler"] },
];

/** Order of the kinds in the Overview (16u): the five of always, then the others in menu order. */
export const OVERVIEW_ORDER: FxKind[] = [...CLASSIC, ...MENU_COLUMNS.flatMap((c) => c.kinds).filter((k) => !CLASSIC.includes(k))];
