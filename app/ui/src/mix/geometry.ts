// Drawing geometry of the workspace: song length, time ↔ pixels, track tree order, snap targets (CLP-3, A-017).
// View computations only: what an edit means is decided by the core.
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";

/** Song length in frames: the end of the last clip, at least 30 s. */
export function songFrames(p: Project): number {
  const rate = p.sample_rate ?? 48000;
  let end = 0;
  for (const t of p.tracks) for (const c of t.clips) end = Math.max(end, c.start + c.length);
  for (const s of p.sections) end = Math.max(end, s.end);
  return Math.max(end, 30 * rate);
}

export type Row =
  | { kind: "track"; t: Track; depth: number }
  | { kind: "group"; t: Track; depth: number; members: number; open: boolean }
  | { kind: "return"; t: Track; depth: 0 }
  | { kind: "lane"; t: Track; param: string; depth: number };

/** Tracks in tree order: each group followed by its members (nested, TRK-3), returns at the end. */
export function treeRows(p: Project, folded: string[]): Row[] {
  const out: Row[] = [];
  const children = (gid: string | null) =>
    p.tracks.filter((t) => t.kind !== "return" && (gid === null ? t.output.kind === "mix_bus" : t.output.kind === "group" && t.output.id === gid));
  const walk = (t: Track, depth: number, hidden: boolean) => {
    if (t.kind === "group") {
      const kids = children(t.id);
      const open = !folded.includes(t.id);
      if (!hidden) out.push({ kind: "group", t, depth, members: kids.length, open });
      kids.forEach((k) => walk(k, depth + 1, hidden || !open));
    } else if (!hidden) {
      out.push({ kind: "track", t, depth });
    }
    if (!hidden && t.kind !== "return") for (const l of t.automation) if (l.visible) out.push({ kind: "lane", t, param: l.param, depth: depth + 1 });
  };
  children(null).forEach((t) => walk(t, 0, false));
  // Tracks routed to a group that does not exist any more would be lost: show them at the top level.
  const seen = new Set(out.map((r) => r.t.id));
  p.tracks.filter((t) => t.kind !== "return" && !seen.has(t.id) && !isUnderFolded(p, t, folded)).forEach((t) => walk(t, 0, false));
  for (const t of p.tracks.filter((x) => x.kind === "return")) {
    out.push({ kind: "return", t, depth: 0 });
    for (const l of t.automation) if (l.visible) out.push({ kind: "lane", t, param: l.param, depth: 1 });
  }
  return out;
}

function isUnderFolded(p: Project, t: Track, folded: string[]): boolean {
  let o = t.output;
  for (let i = 0; i < 8 && o.kind === "group"; i++) {
    const gid: string = o.id;
    if (folded.includes(gid)) return true;
    const g = p.tracks.find((x) => x.id === gid);
    if (!g) return false;
    o = g.output;
  }
  return false;
}

/** Mixer order: same tree, groups before their members, returns, then the mix bus (drawn apart). */
export function stripOrder(p: Project): { t: Track; depth: number; parent: string | null }[] {
  const rows = treeRows(p, []).filter((r) => r.kind !== "lane");
  return rows.map((r) => ({
    t: r.t,
    depth: r.depth,
    parent: r.t.output.kind === "group" ? (p.tracks.find((x) => x.id === (r.t.output as { id: string }).id)?.name ?? null) : null,
  }));
}

/** Outputs a track may take without a loop (TRK-3): the mix bus and every group that is not inside it. */
export function outputChoices(p: Project, t: Track): Track[] {
  const inside = (g: Track): boolean => {
    let o = g.output;
    let cur: Track | undefined = g;
    for (let i = 0; i < 16 && cur; i++) {
      if (cur.id === t.id) return true;
      if (o.kind !== "group") return false;
      const id: string = o.id;
      cur = p.tracks.find((x) => x.id === id);
      o = cur?.output ?? { kind: "mix_bus" };
    }
    return false;
  };
  return p.tracks.filter((g) => g.kind === "group" && g.id !== t.id && !inside(g));
}

/** Snap: the nearest target within `dist` px of `frame`, if any (CLP-3). */
export function snapFrame(
  frame: number, targets: { frame: number; what: string }[], pps: number, rate: number, dist = 8,
): { frame: number; what: string } | null {
  let best: { frame: number; what: string } | null = null;
  let bd = dist;
  for (const t of targets) {
    const d = (Math.abs(t.frame - frame) / rate) * pps;
    if (d <= bd) {
      bd = d;
      best = t;
    }
  }
  return best;
}

/** Grid step in seconds for the ruler and the snap grid at a zoom (bars when there is a tempo, TRN-2). */
export function gridStep(pps: number, p: Project): { step: number; bars: boolean } {
  if (p.tempo) {
    const bar = (60 / p.tempo.bpm) * p.tempo.beats_per_bar;
    let k = 1;
    while (bar * k * pps < 70) k *= 2;
    return { step: bar * k, bars: true };
  }
  for (const s of [1, 2, 5, 10, 15, 30, 60, 120]) if (s * pps >= 70) return { step: s, bars: false };
  return { step: 300, bars: false };
}

/** Every snap target of the project except the clips being moved. */
export function snapTargets(p: Project, rate: number, cursor: number, pps: number, exclude: Set<string>): { frame: number; what: string }[] {
  const out: { frame: number; what: string }[] = [{ frame: cursor, what: "cursor" }, { frame: 0, what: "start" }];
  for (const s of p.sections) {
    out.push({ frame: s.start, what: `section “${s.name}”` }, { frame: s.end, what: `section “${s.name}”` });
  }
  for (const t of p.tracks)
    for (const c of t.clips) {
      if (exclude.has(`${t.id}/${c.id}`)) continue;
      out.push({ frame: c.start, what: "clip edge" }, { frame: c.start + c.length, what: "clip edge" });
    }
  const g = gridStep(pps, p).step;
  const end = songFrames(p) / rate;
  for (let s = 0; s <= end; s += g) out.push({ frame: Math.round(s * rate), what: "grid" });
  return out;
}

/** Friendly band names (design: "Mud", "Presence"…), from the band's frequency. Labels only. */
export function bandName(freq: number, enabled: boolean): string {
  if (!enabled) return "Off";
  if (freq < 80) return "Sub";
  if (freq < 180) return "Low end";
  if (freq < 500) return "Mud";
  if (freq < 1500) return "Body";
  if (freq < 5000) return "Presence";
  if (freq < 8500) return "Harsh";
  return "Air";
}

/** Track header width (tokens --head-w, --head-w-min): the design's width is the minimum (issue #9, A-084). */
export const HEAD_MIN = 344;
export const HEAD_MIN_COMPACT = 304;
/** Widest header: past this a name gains nothing the tooltip does not already give. */
export const HEAD_MAX = 640;
/** Lane width a wider header always leaves to the clips. */
export const HEAD_LANE_MIN = 320;

/**
 * The header width the timeline draws: what the user asked for, never under the layout's minimum, never over
 * {@link HEAD_MAX} nor so wide that the lanes keep less than {@link HEAD_LANE_MIN} (until the window grows again).
 */
export function headWidth(want: number, compact: boolean, timelineW: number): number {
  const min = compact ? HEAD_MIN_COMPACT : HEAD_MIN;
  const max = Math.max(min, Math.min(HEAD_MAX, timelineW - HEAD_LANE_MIN));
  return Math.round(Math.max(min, Math.min(max, Number.isNaN(want) ? min : want)));
}
