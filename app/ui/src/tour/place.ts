// Where the tour's bubble goes (design/README.md, "Tour guidato"): the first side with room — the step's own
// preference, then right, left, below, above — at a gap from the lit area and never closer than an edge margin to
// the window; inside the lit area, bottom-left and without an arrow, when no side fits.

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}
export type Side = "r" | "l" | "b" | "t";

/** Tokens of the design: --tour-pad, --tour-gap, --tour-edge, the ring, the arrow's distance from the corners. */
export const TOUR_GEOM = { pad: 4, gap: 12, edge: 16, ring: 2, corner: 24 };

/** The lit hole: the target grown by the pad, kept inside the window so its ring shows. */
export function holeOf(t: Rect, win: { w: number; h: number }): Rect {
  const { pad, ring } = TOUR_GEOM;
  const x = Math.max(ring, t.x - pad);
  const y = Math.max(ring, t.y - pad);
  return { x, y, w: Math.max(0, Math.min(win.w - ring, t.x + t.w + pad) - x), h: Math.max(0, Math.min(win.h - ring, t.y + t.h + pad) - y) };
}

export interface Placement {
  hole: Rect;
  x: number;
  y: number;
  /** The side of the bubble that faces the target (where its arrow is), or null inside the hole. */
  arrowAt: "l" | "r" | "t" | "b" | null;
  /** The arrow's position along that side. */
  arrow: number;
}

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(Math.max(lo, hi), v));

export function place(target: Rect, bubble: { w: number; h: number }, win: { w: number; h: number }, opts: { side?: Side; anchor?: { x: number; y: number } } = {}): Placement {
  const { gap, edge, corner } = TOUR_GEOM;
  const hole = holeOf(target, win);
  const ax = opts.anchor?.x ?? target.x + target.w / 2;
  const ay = opts.anchor?.y ?? target.y + target.h / 2;
  const fits: Record<Side, boolean> = {
    r: win.w - (hole.x + hole.w) >= bubble.w + gap + edge,
    l: hole.x >= bubble.w + gap + edge,
    b: win.h - (hole.y + hole.h) >= bubble.h + gap + edge,
    t: hole.y >= bubble.h + gap + edge,
  };
  const side = ([opts.side, "r", "l", "b", "t"] as (Side | undefined)[]).find((s) => s && fits[s]);
  if (side === "r" || side === "l") {
    const x = side === "r" ? hole.x + hole.w + gap : hole.x - gap - bubble.w;
    const y = clamp(ay - bubble.h / 2, edge, win.h - edge - bubble.h);
    return { hole, x, y, arrowAt: side === "r" ? "l" : "r", arrow: clamp(ay - y, corner, bubble.h - corner) };
  }
  if (side === "b" || side === "t") {
    const y = side === "b" ? hole.y + hole.h + gap : hole.y - gap - bubble.h;
    const x = clamp(ax - bubble.w / 2, edge, win.w - edge - bubble.w);
    return { hole, x, y, arrowAt: side === "b" ? "t" : "b", arrow: clamp(ax - x, corner, bubble.w - corner) };
  }
  // No side has room: inside the hole, bottom-left — still inside the window.
  const x = clamp(hole.x + edge, edge, win.w - edge - bubble.w);
  const y = clamp(hole.y + hole.h - edge - bubble.h, edge, win.h - edge - bubble.h);
  return { hole, x, y, arrowAt: null, arrow: 0 };
}
