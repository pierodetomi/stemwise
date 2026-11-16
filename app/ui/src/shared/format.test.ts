import { formatDb, formatParam, formatTime } from "./format";

// req: TRN-2
test("time and level formatting", () => {
  expect(formatTime(48000 * 62.48, 48000)).toBe("1:02.48");
  expect(formatTime(0, 48000)).toBe("0:00.00");
  expect(formatDb(-144)).toBe("−∞");
  expect(formatDb(-5.83)).toBe("−5.8");
  expect(formatDb(1.5)).toBe("+1.5");
  expect(formatDb(0.01)).toBe("0.0");
});

// req: FX-5
test("a reverb decay reads in seconds", () => {
  expect(formatParam({ id: "rev.decay", unit: "s" }, 1.2)).toBe("1.2 s");
  expect(formatParam({ id: "rev.decay", unit: "s" }, 0.35)).toBe("0.35 s");
  expect(formatParam({ id: "comp.mix", unit: "percent" }, 0.35)).toBe("35 %");
});
