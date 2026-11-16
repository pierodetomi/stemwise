// Explain this mix (P-019, UI-15): where the map draws its boxes and lines (design 17b, 17g, 17i). Drawing only:
// tracks by family on the left, one column of groups per level of nesting, returns under the first of them, the
// mix bus on the right. A line bends only in the gutter before the box it enters.
import { MIX, type MixMap } from "./model";

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
  cy: number;
}
export interface LaidLink {
  id: string;
  kind: "out" | "send" | "key";
  tint: string;
  d: string;
  width: number;
  /** Arrow head of a sidechain. */
  head: string | null;
  /** Where its label goes when it is lit. */
  at: [number, number];
  text: string | null;
}
export interface Layout {
  width: number;
  height: number;
  boxes: Map<string, Box>;
  links: LaidLink[];
  labels: { text: string; x: number; y: number; strong: boolean }[];
  /** The mix bus going on to the master. */
  exit: { d: string; head: string };
}
export type MapMode = "normal" | "dense" | "compact";

/** Line thickness of an output: 2 px for a fader at the bottom, 9 px at the top. */
export const thickness = (level: number) => 2 + level * 7;
const f1 = (v: number) => (Math.round(v * 10) / 10).toString();
const bez = (x1: number, y1: number, x2: number, y2: number) => `C${f1((x1 + x2) / 2)} ${f1(y1)} ${f1((x1 + x2) / 2)} ${f1(y2)} ${f1(x2)} ${f1(y2)}`;

const SIZES: Record<MapMode, { row: number; gap: number; famGap: number; famLabel: number; ret: number; retGap: number; retTop: number; group: number }> = {
  normal: { row: 30, gap: 5, famGap: 14, famLabel: 16, ret: 40, retGap: 8, retTop: 30, group: 44 },
  dense: { row: 26, gap: 4, famGap: 10, famLabel: 14, ret: 28, retGap: 5, retTop: 22, group: 40 },
  compact: { row: 22, gap: 4, famGap: 10, famLabel: 14, ret: 32, retGap: 6, retTop: 22, group: 34 },
};
/** The roomy sizes while the map fits the height it has; the tighter ones before it scrolls (17g). */
export function modeFor(map: MixMap, compact: boolean, width: number, height: number): MapMode {
  if (compact) return "compact";
  return layoutMap(map, width, "normal").height <= height ? "normal" : "dense";
}

export function layoutMap(map: MixMap, width: number, mode: MapMode): Layout {
  const z = SIZES[mode];
  const c = mode === "compact";
  const X1 = c ? 60 : 72;
  const W1 = c ? 176 : 196;
  const R1 = X1 + W1;
  const W3 = 128;
  const X3 = Math.max(R1 + 260, width - 40 - W3);
  const cols = Math.max(1, map.depth);
  const W2 = Math.max(110, Math.min(168, (X3 - R1 - (cols + 1) * 64) / cols));
  const gutter = (X3 - R1 - cols * W2) / (cols + 1);
  const colX = (k: number) => R1 + gutter * k + W2 * (k - 1);
  const top = 30;
  const boxes = new Map<string, Box>();
  const labels: Layout["labels"] = [{ text: "Tracks", x: X1, y: 4, strong: true }];

  // Column of tracks.
  let y = top;
  map.families.forEach((f, fi) => {
    if (fi) y += z.famGap;
    labels.push({ text: f.label, x: X1, y, strong: false });
    y += z.famLabel;
    f.rows.forEach((r, i) => {
      if (i) y += z.gap;
      boxes.set(r.id, { x: X1, y, w: W1, h: z.row, cy: y + z.row / 2 });
      y += z.row;
    });
  });
  const tracksEnd = y;

  // Groups, the shallowest first: each beside what comes into it and as tall as that.
  const incoming = (id: string) => map.links.filter((l) => l.kind === "out" && l.to === id && boxes.has(l.from)).sort((a, b) => boxes.get(a.from)!.cy - boxes.get(b.from)!.cy);
  for (let k = 1; k <= cols; k++) {
    const here = map.groups.filter((g) => g.depth === k);
    for (const g of here) {
      const ins = incoming(g.id);
      const sum = ins.reduce((a, l) => a + thickness(l.level), 0) + Math.max(0, ins.length - 1) * 4;
      const h = Math.max(z.group, sum + 14);
      const cy = ins.length ? (boxes.get(ins[0].from)!.cy + boxes.get(ins[ins.length - 1].from)!.cy) / 2 : top + h / 2;
      boxes.set(g.id, { x: colX(k), y: cy - h / 2, w: W2, h, cy });
    }
    // Groups of a column that would overlap are pushed apart, top to bottom.
    const sorted = here.map((g) => boxes.get(g.id)!).sort((a, b) => a.y - b.y);
    sorted.forEach((b, i) => {
      const min = i ? sorted[i - 1].y + sorted[i - 1].h + 8 : top;
      if (b.y < min) {
        b.cy += min - b.y;
        b.y = min;
      }
    });
  }
  if (map.groups.length) labels.push({ text: "Groups", x: colX(1), y: 4, strong: true });
  const groupsEnd = Math.max(0, ...map.groups.map((g) => boxes.get(g.id)!).map((b) => b.y + b.h));

  // Returns, under the first column of groups and under every track.
  let ry = Math.max(tracksEnd, groupsEnd) + z.retTop;
  if (map.returns.length) labels.push({ text: "Shared effects", x: colX(1), y: ry - (mode === "normal" ? 22 : 18), strong: true });
  for (const r of map.returns) {
    boxes.set(r.id, { x: colX(1), y: ry, w: W2, h: z.ret, cy: ry + z.ret / 2 });
    ry += z.ret + z.retGap;
  }
  const end = map.returns.length ? ry - z.retGap : Math.max(tracksEnd, groupsEnd);

  // The mix bus: as tall as what reaches it, centred on the tracks.
  const toMix = incoming(MIX);
  const msum = toMix.reduce((a, l) => a + thickness(l.level), 0) + Math.max(0, toMix.length - 1) * 9;
  const mh = Math.max(72, msum + 36);
  const mcy = Math.max(top + mh / 2, Math.min((top + tracksEnd) / 2 + 20, end - mh / 2));
  boxes.set(MIX, { x: X3, y: mcy - mh / 2, w: W3, h: mh, cy: mcy });
  labels.push({ text: "Mix bus", x: X3, y: 4, strong: true });

  // Where each output enters its target.
  const port = new Map<string, number>();
  for (const id of [...map.groups.map((g) => g.id), MIX]) {
    const ins = incoming(id);
    const pad = id === MIX ? 9 : 4;
    const sum = ins.reduce((a, l) => a + thickness(l.level), 0) + Math.max(0, ins.length - 1) * pad;
    let py = boxes.get(id)!.cy - sum / 2;
    for (const l of ins) {
      port.set(l.id, py + thickness(l.level) / 2);
      py += thickness(l.level) + pad;
    }
  }

  const links: LaidLink[] = [];
  for (const l of map.links) {
    const a = boxes.get(l.from);
    const b = boxes.get(l.to);
    if (!a || !b) continue;
    if (l.kind === "out") {
      const sx = a.x + a.w;
      const py = port.get(l.id) ?? b.cy;
      const bend = Math.max(sx, b.x - gutter);
      links.push({ id: l.id, kind: "out", tint: l.tint, width: thickness(l.level), head: null, at: [(bend + b.x) / 2, (a.cy + py) / 2], text: null, d: `M${f1(sx)} ${f1(a.cy)}${bend > sx ? `H${f1(bend)}` : ""}${bez(bend, a.cy, b.x, py)}` });
    } else if (l.kind === "send") {
      const from = map.links.filter((x) => x.kind === "send" && x.to === l.to && boxes.has(x.from)).sort((p, q) => boxes.get(p.from)!.cy - boxes.get(q.from)!.cy);
      const k = from.findIndex((x) => x.id === l.id);
      const py = b.cy + (k - (from.length - 1) / 2) * (mode === "normal" ? 6 : 4);
      const text = l.db === null ? null : `${Number(l.db.toFixed(1)).toString().replace("-", "−")} dB`;
      if (a.x >= b.x) {
        // From a group: down from its foot to the return under it, or back from a deeper column.
        const sx = a.x + 18 + map.returns.findIndex((r) => r.id === l.to) * 10;
        const d = a.x === b.x ? `M${f1(sx)} ${f1(a.y + a.h)}V${f1(b.y)}` : `M${f1(a.x + a.w / 2)} ${f1(a.y + a.h)}${bez(a.x + a.w / 2, a.y + a.h + 30, b.x + b.w, py).replace("C", "C")}`;
        links.push({ id: l.id, kind: "send", tint: l.tint, width: 1.5, head: null, at: [sx + 30, (a.y + a.h + b.y) / 2], text, d });
      } else {
        const y1 = a.cy + (mode === "normal" ? 8 : 5);
        links.push({ id: l.id, kind: "send", tint: l.tint, width: 1.5, head: null, at: [(a.x + a.w + b.x) / 2 + (k % 2 ? 26 : -26), (y1 + py) / 2], text, d: `M${f1(a.x + a.w)} ${f1(y1)}${bez(a.x + a.w, y1, b.x, py)}` });
      }
    } else {
      const bow = Math.min(a.x, b.x) - 34;
      links.push({
        id: l.id,
        kind: "key",
        tint: l.tint,
        width: 1.5,
        d: `M${f1(a.x)} ${f1(a.cy)}C${f1(bow)} ${f1(a.cy)} ${f1(bow)} ${f1(b.cy)} ${f1(b.x - 6)} ${f1(b.cy)}`,
        head: `M${f1(b.x - 7)} ${f1(b.cy - 4)}L${f1(b.x - 1)} ${f1(b.cy)}L${f1(b.x - 7)} ${f1(b.cy + 4)}Z`,
        at: [Math.max(34, Math.min(a.x, b.x) - 38), (a.cy + b.cy) / 2],
        text: "listens",
      });
    }
  }
  const w = Math.max(width, X3 + W3 + 40);
  return {
    width: w,
    height: Math.max(end, mcy + mh / 2) + 12,
    boxes,
    links,
    labels,
    exit: { d: `M${X3 + W3} ${f1(mcy)}H${f1(w - 8)}`, head: `M${f1(w - 10)} ${f1(mcy - 6)}L${f1(w - 2)} ${f1(mcy)}L${f1(w - 10)} ${f1(mcy + 6)}Z` },
  };
}
