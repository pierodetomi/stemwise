import type { MasterStatus } from "../bindings/MasterStatus";
import { GAP_LU, gapWords, listenRows, placeOf, refLevels, targetGap } from "./level";

const refs = refLevels({
  references: [
    { id: "a", name: "a.wav", lufs: -9.6, true_peak_db: -0.3 },
    { id: "b", name: "b.wav", lufs: -11.2, true_peak_db: -0.5 },
    { id: "c", name: "c.wav", lufs: null, true_peak_db: -1 },
  ],
});

// req: MAS-2
test("the destination is compared with the range of the measured references", () => {
  expect(refs.map((r) => r.label)).toEqual(["Reference A", "Reference B"]);
  // Inside the range, or within 3 LU of its edges: nothing.
  expect(targetGap(-10, refs)).toBeNull();
  expect(targetGap(-11.2 - GAP_LU, refs)).toBeNull();
  expect(targetGap(-9.6 + GAP_LU, refs)).toBeNull();
  // Below the quietest: a warning measured from it, every reference listed nearest first.
  const q = targetGap(-18, refs)!;
  expect(q.kind).toBe("quieter");
  expect(q.edge.label).toBe("Reference B");
  expect(q.lu).toBeCloseTo(6.8, 6);
  expect(q.refs.map((r) => r.ref.label)).toEqual(["Reference B", "Reference A"]);
  // Above the loudest: a note measured from it.
  const l = targetGap(-6, [refs[1]])!;
  expect(l.kind).toBe("louder");
  expect(l.lu).toBeCloseTo(5.2, 6);
  expect(targetGap(-18, [])).toBeNull();
});

// req: MAS-2
test("the warning and the menu say it in words", () => {
  const w = gapWords(targetGap(-18, [refs[0]])!, -18, -1);
  expect(w.title).toBe("Much quieter than your reference");
  expect(w.text).toBe("At −18 LUFS this master will sound 8.4 LU quieter than Reference A (−9.6 LUFS): clearly softer when the two play one after the other.");
  expect(w.fix).toBe("Match the reference");
  expect(w.keep).toBe("Keep −18 LUFS");
  expect(w.foot).toBe("Sets the target to −9.6 LUFS and keeps the −1 dBTP ceiling · one undo step");
  expect(placeOf(-14, refs)).toEqual({ text: "2.8 LU quieter", warn: false });
  expect(placeOf(-9, refs)).toEqual({ text: "0.6 LU louder", warn: false });
  expect(placeOf(-10, refs)).toEqual({ text: "Between your references", warn: false });
  expect(placeOf(-18, refs)).toEqual({ text: "6.8 LU quieter", warn: true });
  expect(placeOf(-14, [])).toBeNull();
});

// req: MAS-5
test("the Listening rows: offsets at the real level, gains and trim at the same loudness", () => {
  const ms = {
    listening: "master",
    listen_same: false,
    ab_trim_db: 2.5,
    measures: { lufs: -14.1, true_peak_db: -1, lra: 6, distance_db: 1, bands_db: null },
    mixdown_measures: { lufs: -18.6, true_peak_db: -3, lra: 7, distance_db: 3, bands_db: null },
    references: [{ id: "r", name: "r.wav", lufs: -12.9, true_peak_db: -0.8 }],
    proposal: null,
  } as unknown as MasterStatus;
  const real = listenRows(ms, null);
  expect(real.rows.map((r) => r.offset)).toEqual(["as exported", "4.5 LU quieter", "1.2 LU louder"]);
  expect(real.note).toBeNull();
  const same = listenRows({ ...ms, listen_same: true }, null);
  expect(same.rows.map((r) => r.offset)).toEqual(["lowered 2.5 dB", "raised 2.0 dB", "lowered 3.7 dB"]);
  expect(same.note).toBe("Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so everything plays 2.5 dB lower. The export is not affected.");
});
