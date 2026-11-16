// Explain this mix (P-019): what the map and the text are built from. Pure reads of the project mirror, like the
// marks of P-013: who goes where (UI-15), what the selection is connected to, what plays and what changes in a
// section. Nothing here calls the backend or a model; the same project gives the same result.
import type { AutomationLane } from "../../bindings/AutomationLane";
import type { Author } from "../../bindings/Author";
import type { Project } from "../../bindings/Project";
import type { Track } from "../../bindings/Track";
import families from "../../bindings/role_families.json";
import { faderPos } from "../../shared/format";
import { blockName, blockState, sidechainsOf } from "../fx";

/** The families of AI-22, in the order the balance lists them. */
export const FAMILY_ORDER: string[] = families.families;
const FAMILY_OF = families.of as Record<string, string>;
export const familyOf = (t: Track): string => FAMILY_OF[t.role ?? "other"] ?? "other";
export const FAMILY_LABEL: Record<string, { name: string; plural: boolean }> = {
  lead_vocal: { name: "Lead vocal", plural: false },
  backing_vocal: { name: "Backing vocals", plural: true },
  drums: { name: "Drums", plural: true },
  percussion: { name: "Percussion", plural: false },
  bass: { name: "Bass", plural: false },
  guitars: { name: "Guitars", plural: true },
  keys: { name: "Keys", plural: true },
  other: { name: "Other tracks", plural: true },
};
export const familyName = (key: string) => FAMILY_LABEL[key]?.name ?? key;

export const MIX = "mix";
export const tintOf = (t: Track) => (t.kind === "audio" ? `t${t.color || 1}` : "tb");
const byId = (p: Project) => new Map(p.tracks.map((t) => [t.id, t]));

/** The group a track's output goes to, when it exists. */
export function parentOf(p: Project, t: Track): Track | null {
  if (t.output.kind !== "group") return null;
  const id = t.output.id;
  const g = p.tracks.find((x) => x.id === id);
  return g && g.kind === "group" ? g : null;
}

/** Tracks and groups whose output passes through a group, at any depth. */
export function membersOf(p: Project, group: string): Track[] {
  const out: Track[] = [];
  const walk = (id: string, depth: number) => {
    if (depth > 16) return;
    for (const t of p.tracks) {
      if (t.output.kind === "group" && t.output.id === id && t.kind !== "return") {
        out.push(t);
        if (t.kind === "group") walk(t.id, depth + 1);
      }
    }
  };
  walk(group, 0);
  return out;
}

/** Who wrote the automation of a track: the assistant only if it wrote every lane that has points. */
function autoBy(t: Track): Author | null {
  const lanes = t.automation.filter((l) => l.points.length > 0);
  if (!lanes.length) return null;
  return lanes.every((l) => l.author === "ai") ? "ai" : "user";
}

export interface MapSend {
  ret: string;
  db: number;
}
/** A row of the first column: a track, or the tracks of a folded group as one. */
export interface MapRow {
  id: string;
  name: string;
  tint: string;
  family: string;
  /** Fader position 0..1: the thickness of its output. */
  level: number;
  lock: boolean;
  auto: Author | null;
  note: number | null;
  /** The folded group this row stands for, with how many tracks. */
  fold: { group: string; count: number } | null;
  to: string;
}
export interface MapGroup {
  id: string;
  name: string;
  level: number;
  /** 1 for a group of tracks only, one more for each level of groups inside it: its column. */
  depth: number;
  count: number;
  folded: boolean;
  auto: Author | null;
  note: number | null;
  to: string;
}
export interface MapReturn {
  id: string;
  name: string;
  sub: string;
  level: number;
  senders: number;
  to: string;
}
export interface MapLink {
  id: string;
  kind: "out" | "send" | "key";
  from: string;
  to: string;
  tint: string;
  level: number;
  /** Send level, dB. */
  db: number | null;
  /** For a sidechain: the block that listens. */
  label: string | null;
}
export interface MixMap {
  families: { key: string; label: string; rows: MapRow[] }[];
  groups: MapGroup[];
  returns: MapReturn[];
  links: MapLink[];
  /** Columns of groups. */
  depth: number;
}

/** What a return is, in a few words ("plate reverb · 1.2 s"). */
export function returnSub(r: Track): string {
  const fx = r.ret;
  if (!fx) return "return";
  if (fx.type === "reverb") return `${fx.algorithm} reverb · ${n1(fx.decay_s)} s`;
  return `${fx.mode === "ping_pong" ? "ping-pong delay" : "delay"} · ${fx.sync ?? `${Math.round(fx.time_ms)} ms`}`;
}
const n1 = (v: number) => Number(v.toFixed(1)).toString().replace("-", "−");

/**
 * The map (UI-15): rows by family, groups by column, returns, and every link between them. A track inside a folded
 * group is drawn as part of that group's single row; links that start or end inside it start or end at that row.
 */
export function buildMap(p: Project, folded: string[] = []): MixMap {
  const tracks = byId(p);
  const notes = new Map(p.ai.notes.filter((n) => n.track).map((n) => [n.track!, n.number]));
  // The outermost folded group a track sits in.
  const foldOf = (t: Track): Track | null => {
    let found: Track | null = null;
    let g = parentOf(p, t);
    for (let i = 0; g && i < 16; i++) {
      if (folded.includes(g.id)) found = g;
      g = parentOf(p, g);
    }
    return found;
  };
  const rep = (id: string): string => {
    const t = tracks.get(id);
    if (!t) return id;
    const f = foldOf(t);
    return f ? `fold:${f.id}` : id;
  };
  const audio = p.tracks.filter((t) => t.kind === "audio");
  const groups = p.tracks.filter((t) => t.kind === "group" && !foldOf(t));
  const returns = p.tracks.filter((t) => t.kind === "return");
  const destOf = (t: Track) => parentOf(p, t)?.id ?? MIX;

  const rows: MapRow[] = [];
  for (const t of audio) {
    if (foldOf(t)) continue;
    rows.push({ id: t.id, name: t.name, tint: tintOf(t), family: familyOf(t), level: faderPos(t.chain.fader_db), lock: t.locked, auto: autoBy(t), note: notes.get(t.id) ?? null, fold: null, to: destOf(t) });
  }
  for (const g of groups) {
    if (!folded.includes(g.id)) continue;
    const inside = membersOf(p, g.id).filter((t) => t.kind === "audio");
    if (!inside.length) continue;
    const insideNotes = inside.map((t) => notes.get(t.id)).filter((n): n is number => n !== undefined);
    rows.push({
      id: `fold:${g.id}`,
      name: `${inside.length} track${inside.length > 1 ? "s" : ""}`,
      tint: tintOf(inside[0]),
      family: familyOf(inside[0]),
      level: Math.max(...inside.map((t) => faderPos(t.chain.fader_db))),
      lock: inside.every((t) => t.locked),
      auto: inside.some((t) => autoBy(t) === "user") ? "user" : inside.some((t) => autoBy(t) === "ai") ? "ai" : null,
      note: insideNotes.length ? Math.min(...insideNotes) : null,
      fold: { group: g.id, count: inside.length },
      to: g.id,
    });
  }
  // Rows keep the order of the project inside each family; a folded group sits where its first track was.
  const order = new Map(p.tracks.map((t, i) => [t.id, i]));
  const pos = (r: MapRow) => (r.fold ? Math.min(...membersOf(p, r.fold.group).map((t) => order.get(t.id) ?? 0)) : (order.get(r.id) ?? 0));
  const fams = FAMILY_ORDER.map((key) => ({ key, label: familyName(key), rows: rows.filter((r) => r.family === key).sort((a, b) => pos(a) - pos(b)) })).filter((f) => f.rows.length > 0);

  const depthOf = (g: Track, n = 0): number => {
    if (n > 16 || folded.includes(g.id)) return 1;
    const kids = groups.filter((x) => parentOf(p, x)?.id === g.id);
    return kids.length ? 1 + Math.max(...kids.map((k) => depthOf(k, n + 1))) : 1;
  };
  const mapGroups: MapGroup[] = groups.map((g) => ({
    id: g.id,
    name: g.name,
    level: faderPos(g.chain.fader_db),
    depth: depthOf(g),
    count: membersOf(p, g.id).filter((t) => t.kind === "audio").length,
    folded: folded.includes(g.id),
    auto: autoBy(g),
    note: notes.get(g.id) ?? null,
    to: destOf(g),
  }));

  const links: MapLink[] = [];
  for (const r of rows) links.push({ id: `o:${r.id}`, kind: "out", from: r.id, to: r.to, tint: r.tint, level: r.level, db: null, label: null });
  for (const g of mapGroups) links.push({ id: `o:${g.id}`, kind: "out", from: g.id, to: g.to, tint: "tb", level: g.level, db: null, label: null });
  for (const r of returns) links.push({ id: `o:${r.id}`, kind: "out", from: r.id, to: MIX, tint: "tb", level: faderPos(r.chain.fader_db), db: null, label: null });
  // Sends: the loudest one stands for a folded group's.
  const sends = new Map<string, MapLink>();
  for (const t of p.tracks) {
    if (t.kind === "return") continue;
    for (const s of t.sends) {
      if (!tracks.has(s.target)) continue;
      const from = rep(t.id);
      const id = `s:${from}>${s.target}`;
      const had = sends.get(id);
      if (!had || (had.db ?? -Infinity) < s.level_db) sends.set(id, { id, kind: "send", from, to: s.target, tint: from.startsWith("fold:") ? (rows.find((r) => r.id === from)?.tint ?? "tb") : tintOf(t), level: 0, db: s.level_db, label: null });
    }
  }
  links.push(...sends.values());
  // Sidechains: from the source to whoever listens.
  const keys = new Set<string>();
  for (const t of p.tracks) {
    for (const b of t.chain.blocks) {
      if (blockState(b.fx) !== "on") continue;
      for (const src of sidechainsOf(b.fx)) {
        const s = tracks.get(src);
        if (!s) continue;
        const from = rep(src);
        const to = rep(t.id);
        const id = `k:${from}>${to}`;
        if (from === to || keys.has(id)) continue;
        keys.add(id);
        links.push({ id, kind: "key", from, to, tint: from.startsWith("fold:") ? (rows.find((r) => r.id === from)?.tint ?? "tb") : tintOf(s), level: 0, db: null, label: blockName(t.chain, b.id) });
      }
    }
  }
  return {
    families: fams,
    groups: mapGroups,
    returns: returns.map((r) => ({ id: r.id, name: r.name, sub: returnSub(r), level: faderPos(r.chain.fader_db), senders: p.tracks.filter((t) => t.sends.some((s) => s.target === r.id)).length, to: MIX })),
    links,
    depth: Math.max(0, ...mapGroups.map((g) => g.depth)),
  };
}

/** What stays lit when an element of the map is selected (UI-15): the nodes and the links it is connected to. */
export function related(map: MixMap, sel: string | null): { nodes: Set<string>; links: Set<string> } {
  const nodes = new Set<string>();
  const links = new Set<string>();
  if (!sel) return { nodes, links };
  const out = (id: string) => map.links.find((l) => l.kind === "out" && l.from === id);
  // Its way down to the mix bus.
  const down = (id: string) => {
    for (let l = out(id), i = 0; l && i < 16; l = out(l.to), i++) {
      links.add(l.id);
      nodes.add(l.to);
    }
  };
  // Whatever comes into it, at any depth.
  const up = (id: string, depth = 0) => {
    if (depth > 16) return;
    for (const l of map.links) {
      if (l.kind === "out" && l.to === id) {
        links.add(l.id);
        nodes.add(l.from);
        up(l.from, depth + 1);
      }
    }
  };
  nodes.add(sel);
  down(sel);
  if (map.groups.some((g) => g.id === sel)) up(sel);
  for (const l of map.links) {
    if (l.kind === "send" && l.from === sel) {
      links.add(l.id);
      nodes.add(l.to);
      const o = out(l.to);
      if (o) links.add(o.id);
    }
    if (l.kind === "send" && l.to === sel) {
      links.add(l.id);
      nodes.add(l.from);
    }
    if (l.kind === "key" && (l.from === sel || l.to === sel)) {
      links.add(l.id);
      nodes.add(l.from);
      nodes.add(l.to);
    }
  }
  return { nodes, links };
}

// ---------- sections ----------

/** Sections with the same name count as one: "Chorus" is every chorus. */
export interface SectionGroup {
  name: string;
  ids: string[];
  spans: [number, number][];
}
export function sectionGroups(p: Project): SectionGroup[] {
  const out: SectionGroup[] = [];
  for (const s of [...p.sections].sort((a, b) => a.start - b.start)) {
    const g = out.find((x) => x.name === s.name);
    if (g) {
      g.ids.push(s.id);
      g.spans.push([s.start, s.end]);
    } else out.push({ name: s.name, ids: [s.id], spans: [[s.start, s.end]] });
  }
  return out;
}

/** Whether a track is heard between two frames: an unmuted clip overlaps them by at least half a second. */
export function playsBetween(t: Track, a: number, b: number, rate: number): boolean {
  const need = Math.min(rate / 2, (b - a) / 2);
  return t.clips.some((c) => !c.mute && Math.min(b, c.start + c.length) - Math.max(a, c.start) >= need);
}

/** Value of a lane at a frame: straight lines between its points, flat before the first and after the last. */
export function laneAt(lane: AutomationLane, frame: number): number {
  const pts = lane.points;
  if (!pts.length) return 0;
  if (frame <= pts[0].frame) return pts[0].value;
  for (let i = 1; i < pts.length; i++) {
    if (frame <= pts[i].frame) {
      const a = pts[i - 1];
      const b = pts[i];
      return b.frame === a.frame ? b.value : a.value + ((b.value - a.value) * (frame - a.frame)) / (b.frame - a.frame);
    }
  }
  return pts[pts.length - 1].value;
}
function laneMean(lane: AutomationLane, spans: [number, number][]): number {
  let sum = 0;
  let n = 0;
  for (const [a, b] of spans) {
    for (let i = 0; i < 9; i++) {
      sum += laneAt(lane, a + ((b - a) * (i + 0.5)) / 9);
      n++;
    }
  }
  return n ? sum / n : 0;
}

export interface SectionChange {
  track: string;
  /** Fader level here minus its level in the other sections, dB. */
  delta: number;
  by: Author;
}
export interface SectionFacts {
  name: string;
  spans: [number, number][];
  /** Audio tracks that have clips at all. */
  total: number;
  playing: string[];
  /** Tracks that play here and in no section of another name. */
  onlyHere: string[];
  /** Tracks that play elsewhere and not here. */
  silent: string[];
  changes: SectionChange[];
  /** Blocks a bypass lane leaves on here and off everywhere else. */
  blocks: { track: string; block: string }[];
}

/** The least fader difference worth a sentence, dB. */
const CHANGE_DB = 0.5;

export function sectionFacts(p: Project, name: string): SectionFacts | null {
  const groups = sectionGroups(p);
  const here = groups.find((g) => g.name === name);
  if (!here) return null;
  const rate = p.sample_rate ?? 48000;
  const others = groups.filter((g) => g !== here).flatMap((g) => g.spans);
  const audio = p.tracks.filter((t) => t.kind === "audio" && t.clips.some((c) => !c.mute));
  const inSpans = (t: Track, spans: [number, number][]) => spans.some(([a, b]) => playsBetween(t, a, b, rate));
  const playing = audio.filter((t) => inSpans(t, here.spans));
  const changes: SectionChange[] = [];
  const blocks: { track: string; block: string }[] = [];
  if (others.length) {
    for (const t of p.tracks) {
      const lane = t.automation.find((l) => l.param === "fader" && l.points.length > 0);
      if (lane) {
        const delta = laneMean(lane, here.spans) - laneMean(lane, others);
        if (Math.abs(delta) >= CHANGE_DB) changes.push({ track: t.id, delta: Math.round(delta * 10) / 10, by: lane.author });
      }
      for (const b of t.chain.blocks) {
        const by = t.automation.find((l) => l.param === `bypass.${b.id}` && l.points.length > 0);
        if (!by) continue;
        // A bypass lane holds its value: read it as steps, at the middle of each section.
        const step = (f: number) => {
          let v = by.points[0].value;
          for (const pt of by.points) if (pt.frame <= f) v = pt.value;
          return v < 0.5;
        };
        if (here.spans.every(([a, z]) => step((a + z) / 2)) && others.every(([a, z]) => !step((a + z) / 2))) blocks.push({ track: t.id, block: blockName(t.chain, b.id) });
      }
    }
  }
  return {
    name,
    spans: here.spans,
    total: audio.length,
    playing: playing.map((t) => t.id),
    onlyHere: others.length ? playing.filter((t) => !inSpans(t, others)).map((t) => t.id) : [],
    silent: audio.filter((t) => !playing.includes(t)).map((t) => t.id),
    changes,
    blocks,
  };
}
