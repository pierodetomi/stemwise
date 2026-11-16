import { describe, expect, it } from "vitest";
import { isCard, keyEffect, move, TOUR_STEPS } from "./machine";
import { holeOf, place, TOUR_GEOM, type Rect } from "./place";
import { STEP_TEXT, TOUR_TEXT } from "./text";

const key = (k: string, m: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({ key: k, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...m });

describe("the tour's steps", () => {
  // req: UI-18
  it("has fifteen steps, a title and one or two sentences each, cards first and last", () => {
    expect(TOUR_STEPS).toBe(15);
    for (const s of STEP_TEXT) {
      expect(s.title.length).toBeGreaterThan(3);
      const sentences = s.text.split(/(?<=\.)\s+/).filter(Boolean);
      expect(sentences.length).toBeGreaterThanOrEqual(1);
      expect(sentences.length).toBeLessThanOrEqual(2);
    }
    expect(isCard(1) && isCard(15)).toBe(true);
    expect([...Array(13)].map((_, i) => isCard(i + 2)).some(Boolean)).toBe(false);
    expect(TOUR_TEXT.counter(3, 15)).toBe("3 of 15");
  });

  // req: UI-18, UI-19
  it("goes forward and back one step at a time and ends on Done, Skip or past the last step", () => {
    let s = 1;
    const seen = [s];
    for (;;) {
      const n = move(s, "next");
      if (n === "end") break;
      s = n;
      seen.push(s);
    }
    expect(seen).toEqual([...Array(15)].map((_, i) => i + 1));
    for (let i = 15; i > 1; i--) expect(move(i, "back")).toBe(i - 1);
    expect(move(1, "back")).toBe(1);
    for (const i of [1, 7, 15]) {
      expect(move(i, "skip")).toBe("end");
      expect(move(i, "done")).toBe("end");
    }
  });

  // req: UI-18
  it("keys: → and ↩ next, ← back, Esc ends, Space stays with the transport, other shortcuts wait", () => {
    expect(keyEffect(key("ArrowRight"), 3, false)).toBe("next");
    expect(keyEffect(key("Enter"), 3, false)).toBe("next");
    expect(keyEffect(key("Enter"), 15, false)).toBe("done");
    expect(keyEffect(key("ArrowLeft"), 3, false)).toBe("back");
    expect(keyEffect(key("Escape"), 3, false)).toBe("skip");
    expect(keyEffect(key(" "), 3, false)).toBe("pass");
    expect(keyEffect(key("Tab"), 3, false)).toBe("pass");
    for (const k of [key("z", { metaKey: true }), key("e", { metaKey: true }), key("b"), key("m"), key("s", { metaKey: true })]) expect(keyEffect(k, 5, false)).toBe("hold");
    // Typing in the request field of the lit assistant is the user's, Esc still ends the tour.
    expect(keyEffect(key("Enter"), 7, true)).toBe("pass");
    expect(keyEffect(key("b"), 7, true)).toBe("pass");
    expect(keyEffect(key("Escape"), 7, true)).toBe("skip");
  });

  // req: UI-18, UI-7
  it("keys on a focused control: ↩ presses the button, arrows adjust a fader, and in a field only text editing gets ⌘", () => {
    // Back or Skip tour reached with Tab: ↩ is that button's, not Next.
    expect(keyEffect(key("Enter"), 7, false, { button: true })).toBe("native");
    expect(keyEffect(key("ArrowRight"), 7, false, { button: true })).toBe("next");
    // A fader or the phase selector of the lit area.
    expect(keyEffect(key("ArrowLeft"), 5, false, { adjust: true })).toBe("pass");
    expect(keyEffect(key("ArrowUp"), 5, false, { adjust: true })).toBe("pass");
    expect(keyEffect(key("ArrowLeft"), 5, false)).toBe("back");
    // In the request field ⌘A and ⌘Z edit the text; ⌘, ⌘S, ⌘N and ⌘E are the app's and wait.
    for (const k of ["a", "z", "v"]) expect(keyEffect(key(k, { metaKey: true }), 7, true)).toBe("native");
    for (const k of [",", "s", "n", "e", "m"]) expect(keyEffect(key(k, { metaKey: true }), 7, true)).toBe("hold");
  });
});

describe("where the bubble goes", () => {
  const bubble = { w: 340, h: 240 };
  const inside = (p: { x: number; y: number }, win: { w: number; h: number }) => {
    const e = TOUR_GEOM.edge;
    expect(p.x).toBeGreaterThanOrEqual(e);
    expect(p.y).toBeGreaterThanOrEqual(e);
    expect(p.x + bubble.w).toBeLessThanOrEqual(win.w - e);
    expect(p.y + bubble.h).toBeLessThanOrEqual(win.h - e);
  };
  const overlaps = (a: Rect, b: Rect) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

  // req: UI-18, UI-4
  it("stays inside the minimum window (1280 × 720) for every area of the tour, beside it when there is room", () => {
    const win = { w: 1280, h: 720 };
    // The areas of design 18r and 3d at the minimum window.
    const areas: Rect[] = [
      { x: 0, y: 0, w: 1280, h: 52 }, // top bar
      { x: 8, y: 52, w: 640, h: 400 }, // timeline
      { x: 8, y: 392, w: 304, h: 26 }, // track header, low in the window
      { x: 8, y: 462, w: 640, h: 250 }, // mixer
      { x: 696, y: 52, w: 288, h: 660 }, // inspector
      { x: 984, y: 52, w: 288, h: 660 }, // assistant
      { x: 696, y: 52, w: 576, h: 660 }, // proposal
      { x: 8, y: 52, w: 1264, h: 660 }, // Explain
      { x: 285, y: 12, w: 135, h: 28 }, // phase selector
      { x: 512, y: 62, w: 760, h: 595 }, // export dialog, docked right
    ];
    for (const a of areas) {
      const p = place(a, bubble, win);
      inside(p, win);
      if (p.arrowAt) expect(overlaps({ x: p.x, y: p.y, ...bubble }, p.hole)).toBe(false);
    }
  });

  // req: UI-18
  it("prefers the step's side, then right, left, below, above, and points the arrow at the anchor", () => {
    const win = { w: 1440, h: 900 };
    const top = place({ x: 0, y: 0, w: 1440, h: 52 }, bubble, win, { side: "b", anchor: { x: 491, y: 26 } });
    expect(top.arrowAt).toBe("t");
    expect(top.y).toBe(top.hole.y + top.hole.h + TOUR_GEOM.gap);
    expect(top.x + top.arrow).toBeCloseTo(491, 0);
    expect(place({ x: 8, y: 52, w: 776, h: 528 }, bubble, win).arrowAt).toBe("l");
    expect(place({ x: 1116, y: 52, w: 316, h: 840 }, bubble, win).arrowAt).toBe("r");
    const full = place({ x: 8, y: 52, w: 1424, h: 840 }, bubble, win);
    expect(full.arrowAt).toBeNull();
    inside(full, win);
  });

  // req: UI-18
  it("keeps the lit hole inside the window so its ring shows", () => {
    const h = holeOf({ x: 0, y: 0, w: 1440, h: 52 }, { w: 1440, h: 900 });
    expect(h.x).toBe(TOUR_GEOM.ring);
    expect(h.y).toBe(TOUR_GEOM.ring);
    expect(h.x + h.w).toBe(1440 - TOUR_GEOM.ring);
    expect(h.h).toBe(52 + TOUR_GEOM.pad - TOUR_GEOM.ring);
  });
});
