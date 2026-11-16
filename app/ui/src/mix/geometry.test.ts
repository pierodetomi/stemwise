import { demoProject } from "../backend/demo";
import { HEAD_LANE_MIN, HEAD_MAX, HEAD_MIN, HEAD_MIN_COMPACT, gridStep, headWidth, outputChoices, snapFrame, snapTargets, treeRows } from "./geometry";

const R = 48000;

// req: CLP-3
test("snap catches the nearest target within 8 px and nothing farther", () => {
  const t = [{ frame: 10 * R, what: "grid" }, { frame: 12 * R, what: "section" }];
  // 4 px per second: 1.5 s away = 6 px → snaps; 3 s away = 12 px → does not.
  expect(snapFrame(Math.round(11.5 * R), t, 4, R)).toEqual({ frame: 12 * R, what: "section" });
  expect(snapFrame(15 * R, t, 4, R)).toBeNull();
  const p = demoProject();
  const targets = snapTargets(p, R, 3 * R, 10, new Set());
  for (const what of ["cursor", "grid", "clip edge"]) expect(targets.some((x) => x.what === what)).toBe(true);
  expect(targets.some((x) => x.what.startsWith("section"))).toBe(true);
});

// req: TRN-2
test("the grid follows bars when there is a tempo and seconds otherwise", () => {
  const p = demoProject();
  const bars = gridStep(10, p);
  expect(bars.bars).toBe(true);
  const bar = (60 / p.tempo!.bpm) * p.tempo!.beats_per_bar;
  expect(bars.step / bar).toBe(Math.round(bars.step / bar));
  const secs = gridStep(10, { ...p, tempo: null });
  expect(secs).toEqual({ step: 10, bars: false });
});

// req: TRK-3, TRK-4
test("tree order nests groups and the output picker never offers a loop", () => {
  const p = demoProject();
  const rows = treeRows(p, []);
  const drums = rows.findIndex((r) => r.t.name === "Drums");
  expect(rows[drums + 1].t.name).toBe("Kick");
  expect(rows[drums + 1].depth).toBe(1);
  expect(rows.at(-1)!.kind === "return" || rows.at(-1)!.kind === "lane").toBe(true);
  const folded = treeRows(p, ["g1"]);
  expect(folded.some((r) => r.t.name === "Kick")).toBe(false);
  const drumsTrack = p.tracks.find((t) => t.name === "Drums")!;
  const guitars = p.tracks.find((t) => t.name === "Guitars")!;
  guitars.output = { kind: "group", id: drumsTrack.id };
  expect(outputChoices(p, drumsTrack).map((g) => g.name)).not.toContain("Guitars");
});

// req: UI-4
test("the track header widens on request but never under the layout's width nor over the lanes' share", () => {
  // Unset (0) or narrower than the minimum: the design's width, also in the compact layout.
  expect(headWidth(0, false, 1600)).toBe(HEAD_MIN);
  expect(headWidth(200, false, 1600)).toBe(HEAD_MIN);
  expect(headWidth(200, true, 1600)).toBe(HEAD_MIN_COMPACT);
  expect(headWidth(Number.NaN, false, 1600)).toBe(HEAD_MIN);
  // Wider: honoured, up to the maximum.
  expect(headWidth(480.4, false, 1600)).toBe(480);
  expect(headWidth(2000, false, 1600)).toBe(HEAD_MAX);
  expect(headWidth(Infinity, false, 1600)).toBe(HEAD_MAX);
  // A narrow window keeps room for the lanes, but never pushes the header under its minimum.
  expect(headWidth(600, false, 800)).toBe(800 - HEAD_LANE_MIN);
  expect(headWidth(600, false, 500)).toBe(HEAD_MIN);
  // Before the timeline is measured (width 0) the minimum.
  expect(headWidth(600, false, 0)).toBe(HEAD_MIN);
});
