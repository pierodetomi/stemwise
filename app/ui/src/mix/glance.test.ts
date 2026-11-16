import { describe, expect, it } from "vitest";
import { defaultChain, demoProject } from "../backend/demo";
import { chainEdit } from "../backend/mockFx";
import type { Fx } from "../bindings/Fx";
import type { Track } from "../bindings/Track";
import { blockSummary, kindOfId, playsIn, specKey } from "./fx";
import { cellValue, chainGlance, fitMarks, overviewKinds, relations, shortHz } from "./glance";

const track = (f: (t: Track) => void = () => {}): Track => {
  const t: Track = { id: "t1", kind: "audio", name: "T", color: 1, role: null, channels: 1, output: { kind: "mix_bus" }, clips: [], chain: defaultChain(), ret: null, sends: [], automation: [], locked: false, mute: false, solo: false };
  f(t);
  return t;
};
const fx = (t: Track, id: string) => t.chain.blocks.find((b) => b.id === id)!.fx;
const states = (t: Track) => Object.fromEntries(chainGlance(t).marks.map((m) => [m.id, m.state]));

describe("chain marks (UI-8, UI-12)", () => {
  // req: UI-8, UI-12
  it("one mark per block in chain order: bypassed blocks off, filters and compressor working when on", () => {
    const g = chainGlance(track());
    expect(g.marks.map((m) => m.sig)).toEqual(["HP", "LP", "EQ", "CMP", "SAT"]);
    expect(states(track())).toEqual({ hpf: "off", lpf: "off", eq: "neutral", comp: "off", sat: "off" });
    const on = track((t) => ["hpf", "lpf", "comp", "sat"].forEach((id) => (fx(t, id).bypass = false)));
    expect(states(on)).toEqual({ hpf: "on", lpf: "on", eq: "neutral", comp: "on", sat: "on" });
    expect(chainGlance(on).marks.every((m) => m.key === null && m.parallel === null)).toBe(true);
  });

  // req: UI-8
  it("an EQ is neutral without active bands, or with active bands all at 0 dB, and works with any active band off 0 dB", () => {
    const eq = (t: Track) => fx(t, "eq") as Extract<Fx, { type: "eq" }>;
    expect(states(track()).eq).toBe("neutral");
    expect(states(track((t) => eq(t).bands.forEach((b) => (b.enabled = true)))).eq).toBe("neutral");
    expect(states(track((t) => (eq(t).bands[1].gain_db = 4))).eq).toBe("neutral");
    const working = track((t) => (eq(t).bands[2] = { ...eq(t).bands[2], enabled: true, gain_db: -0.5 }));
    expect(states(working).eq).toBe("on");
    const bypassed = track((t) => {
      eq(t).bands[2] = { ...eq(t).bands[2], enabled: true, gain_db: 3 };
      eq(t).bypass = true;
    });
    expect(states(bypassed).eq).toBe("off");
    const b = working.chain.blocks.find((x) => x.id === "eq")!;
    expect(cellValue(b, "on")).toBe("1 band");
  });

  // req: UI-8
  it("saturation with mix at 0 is neutral", () => {
    const t = track((x) => Object.assign(fx(x, "sat"), { bypass: false, mix: 0 }));
    expect(states(t).sat).toBe("neutral");
    expect(cellValue(t.chain.blocks.find((b) => b.id === "sat")!, "neutral")).toBe("dry");
  });

  // req: UI-8, UI-12
  it("a keyed block names its source, a parallel compressor its mix; a bypassed one neither", () => {
    const t = track((x) => Object.assign(fx(x, "comp"), { bypass: false, mix: 0.35, sidechain: { source: "k", hpf_hz: 100 } }));
    const m = chainGlance(t).marks.find((x) => x.id === "comp")!;
    expect(m.key).toBe("k");
    expect(m.parallel).toBeCloseTo(0.35);
    fx(t, "comp").bypass = true;
    const off = chainGlance(t).marks.find((x) => x.id === "comp")!;
    expect(off.key).toBeNull();
    expect(off.parallel).toBeNull();
    // A gate with a sidechain is keyed too.
    chainEdit(t.chain, { op: "add", kind: "gate", at: 0 }, false);
    Object.assign(fx(t, "gate"), { bypass: false, sidechain: "snare" });
    expect(chainGlance(t).marks[0]).toMatchObject({ sig: "GT", key: "snare" });
  });

  // req: UI-12
  it("long chains show the first four marks and a +n naming the rest", () => {
    const t = track();
    expect(fitMarks(chainGlance(t).marks).more).toHaveLength(0);
    chainEdit(t.chain, { op: "add", kind: "deesser", at: null }, false);
    chainEdit(t.chain, { op: "add", kind: "doubler", at: null }, false);
    const { shown, more } = fitMarks(chainGlance(t).marks);
    expect(shown.map((m) => m.sig)).toEqual(["HP", "LP", "EQ", "CMP"]);
    expect(more.map((m) => m.sig)).toEqual(["SAT", "DS", "DBL"]);
  });

  // req: UI-8
  it("automation is the assistant's only when it wrote every lane with points", () => {
    const lane = (param: string, author: "ai" | "user", n = 1) => ({ param, author, visible: false, points: Array.from({ length: n }, (_, i) => ({ frame: i, value: 0 })) });
    expect(chainGlance(track()).automation).toBeNull();
    expect(chainGlance(track((t) => (t.automation = [lane("fader", "ai", 0)]))).automation).toBeNull();
    expect(chainGlance(track((t) => (t.automation = [lane("fader", "ai")]))).automation).toEqual({ by: "ai", params: ["fader"] });
    expect(chainGlance(track((t) => (t.automation = [lane("fader", "ai"), lane("pan", "user")]))).automation?.by).toBe("user");
  });

  // req: UI-11
  it("writes frequencies short", () => {
    expect([shortHz(90), shortHz(9000), shortHz(2500), shortHz(12000)]).toEqual(["90", "9k", "2.5k", "12k"]);
  });
});

describe("blocks of a chain (UI-12)", () => {
  // req: UI-12
  it("block ids name their kind and parameter ids find their spec", () => {
    expect(kindOfId("comp2")).toBe("compressor");
    expect(kindOfId("deq")).toBe("dynamic_eq");
    expect(specKey("eq2.3.gain")).toBe("eq.3.gain");
    expect(specKey("bypass.comp2")).toBe("bypass.comp");
    expect(specKey("fader")).toBe("fader");
  });

  // req: UI-12, UI-11
  it("rows and cells say what a block is set to; the Overview lists the kinds the project has, the five first", () => {
    const p = demoProject();
    const kinds = overviewKinds(p);
    expect(kinds.slice(0, 5)).toEqual(["hpf", "lpf", "eq", "compressor", "saturation"]);
    expect(kinds).toContain("gate");
    expect(kinds).not.toContain("harmonizer");
    const vox = p.tracks.find((t) => t.name === "Lead Vox")!;
    expect(blockSummary(fx(vox, "sat"))).toBe("Tube · 4 dB");
    expect(blockSummary(fx(vox, "hpf"))).toBe("90 Hz");
    // The doubler plays in the choruses only (a bypass lane): the row says so.
    expect(playsIn(vox, "dbl", p.sections)).toBe("Choruses");
    expect(playsIn(vox, "eq", p.sections)).toBeNull();
  });
});

describe("selection links (UI-10)", () => {
  const p = demoProject();
  const id = (n: string) => p.tracks.find((t) => t.name === n)!.id;

  // req: UI-10
  it("a return marks whoever sends to it, with the level", () => {
    const r = relations(p, id("Plate"));
    expect(r.ret).toBe(id("Plate"));
    expect(r.reasons.get(id("Snare"))).toBe("sends −18.0 dB");
    expect(r.reasons.get(id("Lead Vox"))).toBe("sends −14.0 dB");
    expect(r.reasons.has(id("Kick"))).toBe(false);
  });

  // req: UI-10
  it("a track marks its group, its returns and whoever listens to it", () => {
    const kick = relations(p, id("Kick"));
    expect(kick.ret).toBeNull();
    expect(kick.reasons.get(id("Drums"))).toBe("takes Kick");
    expect(kick.reasons.get(id("Bass DI"))).toBe("listens to Kick");
    const snare = relations(p, id("Snare"));
    expect(snare.reasons.get(id("Plate"))).toBe("Snare −18.0 dB");
    expect(snare.reasons.has(id("Snare"))).toBe(false);
    expect(relations(p, null).reasons.size).toBe(0);
    // The guitars' dynamic EQs listen to the vocal.
    expect(relations(p, id("Lead Vox")).reasons.get(id("Gtr Left"))).toBe("listens to Lead Vox");
  });
});
