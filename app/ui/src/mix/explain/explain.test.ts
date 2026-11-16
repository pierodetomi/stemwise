import { describe, expect, it } from "vitest";
import { demoProject } from "../../backend/demo";
import { mockLevels } from "../../backend/mockLevels";
import type { Project } from "../../bindings/Project";
import { layoutMap, modeFor, thickness } from "./layout";
import { MIX, buildMap, familyOf, laneAt, related, sectionFacts, sectionGroups } from "./model";
import { GLOSSARY, blockWords, explainElement, explainMix, explainSection, toPlainText, type Block, type Doc, type Part } from "./text";

const id = (p: Project, name: string) => p.tracks.find((t) => t.name === name)!.id;
const partText = (x: Part) => (typeof x === "string" ? x : "b" in x ? x.b : "name" in x ? x.name : x.text);
const blockText = (b: Block): string =>
  b.kind === "p" ? b.parts.map(partText).join("") : b.kind === "item" ? b.head.map(partText).join("") + " " + b.body.map(partText).join("") : b.kind === "steps" ? b.steps.map((s) => `${s.sig} ${s.name} ${s.value} ${s.body.map(partText).join("")}`).join("\n") : b.kind === "note" ? b.text : b.kind === "bars" ? b.bars.map((r) => `${r.label} ${r.text}`).join("\n") : b.kind === "tags" ? b.tags.map((t) => t.text).join(" ") : b.kind === "pending" ? b.text : "";
const sec = (d: Doc, sid: string) => d.sections.find((s) => s.id === sid);
const text = (d: Doc, sid: string) => (sec(d, sid)?.blocks ?? []).map(blockText).join("\n");
/** Every element a text names, as chips. */
const chips = (d: Doc) =>
  d.sections.flatMap((s) => s.blocks).flatMap((b) => (b.kind === "p" ? b.parts : b.kind === "item" ? [...b.head, ...b.body] : b.kind === "steps" ? b.steps.flatMap((x) => x.body) : [])).filter((x): x is { id: string; name: string; tint: string } => typeof x !== "string" && "id" in x);

describe("the map of the mix (UI-15)", () => {
  // req: UI-15
  it("lists tracks by family, groups beside them and returns, with one output per element down to the mix bus", () => {
    const p = demoProject();
    const m = buildMap(p);
    expect(m.families.map((f) => f.label)).toEqual(["Lead vocal", "Backing vocals", "Drums", "Bass", "Guitars", "Keys"]);
    expect(m.families.find((f) => f.key === "drums")!.rows.map((r) => r.name)).toEqual(["Kick", "Snare", "Overheads", "Room"]);
    expect(familyOf(p.tracks.find((t) => t.name === "Strings")!)).toBe("keys");
    expect(m.groups.map((g) => [g.name, g.count, g.depth])).toEqual([["Drums", 4, 1], ["Guitars", 3, 1]]);
    expect(m.returns.map((r) => [r.name, r.sub, r.senders])).toEqual([["Plate", "plate reverb · 1.2 s", 4], ["Echo", "delay · 1/8.", 2]]);
    const out = (name: string) => m.links.find((l) => l.id === `o:${id(p, name)}`)!;
    expect(out("Kick").to).toBe(id(p, "Drums"));
    expect(out("Drums").to).toBe(MIX);
    expect(out("Lead Vox").to).toBe(MIX);
    expect(out("Plate").to).toBe(MIX);
    // A louder fader is a thicker line.
    expect(thickness(out("Lead Vox").level)).toBeGreaterThan(thickness(out("Strings").level));
    // The marks on the boxes: who wrote the automation, the lock, the assistant's note.
    const row = (name: string) => m.families.flatMap((f) => f.rows).find((r) => r.name === name)!;
    expect(row("Bass DI").auto).toBe("user");
    expect(row("BVs").lock).toBe(true);
  });

  // req: UI-15
  it("draws a send for every track that feeds a return and a sidechain for every block that listens", () => {
    const p = demoProject();
    const m = buildMap(p);
    const sends = m.links.filter((l) => l.kind === "send" && l.to === id(p, "Plate"));
    expect(sends.map((l) => [p.tracks.find((t) => t.id === l.from)!.name, l.db]).sort()).toEqual([["Acoustic", -20], ["BVs", -12], ["Lead Vox", -14], ["Snare", -18]]);
    const keys = m.links.filter((l) => l.kind === "key").map((l) => `${p.tracks.find((t) => t.id === l.from)!.name}>${p.tracks.find((t) => t.id === l.to)!.name}`);
    expect(keys.sort()).toEqual(["Kick>Bass DI", "Lead Vox>Gtr Left", "Lead Vox>Gtr Right"]);
    expect(m.links.find((l) => l.kind === "key" && l.to === id(p, "Bass DI"))!.tint).toBe(m.families.flatMap((f) => f.rows).find((r) => r.name === "Kick")!.tint);
  });

  // req: UI-15
  it("lights what the selection is connected to: its way to the mix bus, its sends, who feeds it, who listens", () => {
    const p = demoProject();
    const m = buildMap(p);
    const vox = related(m, id(p, "Lead Vox"));
    expect([...vox.nodes].sort()).toEqual([id(p, "Lead Vox"), id(p, "Plate"), id(p, "Echo"), id(p, "Gtr Left"), id(p, "Gtr Right"), MIX].sort());
    expect(vox.links.has(`o:${id(p, "Lead Vox")}`) && vox.links.has(`s:${id(p, "Lead Vox")}>${id(p, "Plate")}`) && vox.links.has(`o:${id(p, "Plate")}`)).toBe(true);
    expect(vox.links.has(`o:${id(p, "Kick")}`)).toBe(false);
    // A return: whoever sends to it. A group: its members and its own way down. A track in a group: the group too.
    const plate = related(m, id(p, "Plate"));
    expect(["Snare", "Acoustic", "Lead Vox", "BVs"].every((n) => plate.nodes.has(id(p, n)))).toBe(true);
    expect(plate.nodes.has(id(p, "Kick"))).toBe(false);
    const drums = related(m, id(p, "Drums"));
    expect(["Kick", "Snare", "Overheads", "Room"].every((n) => drums.nodes.has(id(p, n)) && drums.links.has(`o:${id(p, n)}`))).toBe(true);
    const kick = related(m, id(p, "Kick"));
    expect(kick.nodes.has(id(p, "Drums")) && kick.links.has(`o:${id(p, "Drums")}`) && kick.nodes.has(id(p, "Bass DI"))).toBe(true);
    expect(related(m, null).nodes.size).toBe(0);
  });

  // req: UI-15
  it("folds a group into one row that carries its tracks' sends and sidechains", () => {
    const p = demoProject();
    const drums = id(p, "Drums");
    const m = buildMap(p, [drums]);
    const rows = m.families.flatMap((f) => f.rows);
    const stub = rows.find((r) => r.fold)!;
    expect([stub.name, stub.fold!.group, stub.to, stub.family]).toEqual(["4 tracks", drums, drums, "drums"]);
    expect(rows.some((r) => r.name === "Kick")).toBe(false);
    expect(m.groups.find((g) => g.id === drums)!.folded).toBe(true);
    // Snare's send and Kick's sidechain now start from the row.
    expect(m.links.some((l) => l.id === `s:${stub.id}>${id(p, "Plate")}` && l.db === -18)).toBe(true);
    expect(m.links.some((l) => l.id === `k:${stub.id}>${id(p, "Bass DI")}`)).toBe(true);
    expect(m.links.filter((l) => l.kind === "out" && l.to === drums).map((l) => l.from)).toEqual([stub.id]);
  });

  // req: UI-15
  it("gives nested groups a column each", () => {
    const p = demoProject();
    const all = { ...p.tracks[0], id: "g9", name: "Band" };
    p.tracks.unshift(all);
    for (const n of ["Drums", "Guitars"]) p.tracks.find((t) => t.name === n)!.output = { kind: "group", id: "g9" };
    const m = buildMap(p);
    expect(m.depth).toBe(2);
    expect(m.groups.find((g) => g.name === "Band")!.depth).toBe(2);
    const lay = layoutMap(m, 920, "normal");
    expect(lay.boxes.get("g9")!.x).toBeGreaterThan(lay.boxes.get(id(p, "Drums"))!.x + lay.boxes.get(id(p, "Drums"))!.w);
    expect(lay.boxes.get(MIX)!.x).toBeGreaterThan(lay.boxes.get("g9")!.x + lay.boxes.get("g9")!.w);
  });

  // req: UI-15
  it("lays the map out left to right without overlaps, and tightens its rows before it scrolls", () => {
    const p = demoProject();
    const m = buildMap(p);
    const lay = layoutMap(m, 920, "normal");
    const boxes = [...lay.boxes.entries()];
    for (const [a, x] of boxes) for (const [b, y] of boxes) if (a < b) expect(x.x + x.w <= y.x || y.x + y.w <= x.x || x.y + x.h <= y.y || y.y + y.h <= x.y, `${a} over ${b}`).toBe(true);
    const kick = lay.boxes.get(id(p, "Kick"))!;
    const drums = lay.boxes.get(id(p, "Drums"))!;
    expect(kick.x + kick.w).toBeLessThan(drums.x);
    expect(drums.x + drums.w).toBeLessThan(lay.boxes.get(MIX)!.x);
    // Returns sit under every track, so no line to the mix bus crosses them.
    expect(lay.boxes.get(id(p, "Plate"))!.y).toBeGreaterThan(Math.max(...m.families.flatMap((f) => f.rows).map((r) => lay.boxes.get(r.id)!.y)));
    expect(lay.links.length).toBe(m.links.length);
    // Same map, same drawing.
    expect(layoutMap(buildMap(demoProject()), 920, "normal")).toEqual(lay);
    expect(modeFor(m, false, 920, lay.height)).toBe("normal");
    expect(modeFor(m, false, 920, lay.height - 1)).toBe("dense");
    expect(modeFor(m, true, 920, 2000)).toBe("compact");
    expect(layoutMap(m, 920, "dense").height).toBeLessThan(lay.height);
  });
});

describe("sections (UI-15, UI-16)", () => {
  // req: UI-15, UI-16
  it("counts sections of the same name as one and says what plays only there, what is silent, what changes", () => {
    const p = demoProject();
    expect(sectionGroups(p).map((g) => [g.name, g.spans.length])).toEqual([["Intro", 1], ["Verse 1", 1], ["Chorus", 3], ["Verse 2", 1], ["Bridge", 1], ["Outro", 1]]);
    const f = sectionFacts(p, "Chorus")!;
    const names = (ids: string[]) => ids.map((x) => p.tracks.find((t) => t.id === x)!.name);
    expect([f.total, f.playing.length]).toEqual([12, 11]);
    expect(names(f.onlyHere)).toEqual(["Gtr Left", "Gtr Right", "BVs"]);
    expect(names(f.silent)).toEqual(["Rhodes"]);
    // The doubler the assistant switches on in the choruses only (a bypass lane).
    expect(f.blocks.map((x) => [names([x.track])[0], x.block])).toEqual([["Lead Vox", "Doubler"]]);
    expect(sectionFacts(p, "Nowhere")).toBeNull();
    // A fader drawn 3 dB up in the choruses is a change of that section, signed and attributed.
    const vox = p.tracks.find((t) => t.name === "Lead Vox")!;
    const S = p.sample_rate!;
    vox.automation = vox.automation.filter((l) => l.param !== "fader");
    vox.automation.push({ param: "fader", author: "ai", visible: false, points: [[0, -6], [47, -6], [48, -3], [80, -3], [81, -6], [111, -6], [112, -3], [144, -3], [145, -6], [167, -6], [168, -3], [208, -3], [209, -6]].map(([s, v]) => ({ frame: s * S, value: v })) });
    expect(sectionFacts(p, "Chorus")!.changes).toEqual([{ track: vox.id, delta: 3, by: "ai" }]);
    expect(sectionFacts(p, "Intro")!.changes.filter((c) => c.track === vox.id)).toEqual([{ track: vox.id, delta: -1.3, by: "ai" }]);
    expect(laneAt(vox.automation.at(-1)!, 47.5 * S)).toBeCloseTo(-4.5, 6);
  });
});

describe("the mix in words (UI-16)", () => {
  // req: UI-16, UI-14
  it("tells the whole mix in fixed parts, in order, from the project alone: same project, same text", () => {
    const p = demoProject();
    const l = mockLevels(p);
    const d = explainMix(p, l);
    expect(d.sections.map((s) => s.id)).toEqual(["short", "structure", "balance", "pan", "space", "react", "time", "reasons"]);
    expect(text(d, "short")).toContain("12 tracks reach the mix bus through 2 groups and share 2 effects.");
    expect(text(d, "short")).toContain("Bass DI steps back each time Kick plays.");
    expect(text(d, "structure")).toContain("Drums takes Kick, Snare, Overheads and Room");
    expect(text(d, "structure")).toContain("Plate is a plate reverb with a 1.2 s tail");
    expect(text(d, "structure")).toContain("4 tracks send it a copy of themselves");
    expect(text(d, "structure")).toContain("5 tracks go straight to the mix bus");
    expect(text(d, "pan")).toContain("Gtr Left 80 % left, Gtr Right 80 % right");
    expect(text(d, "space")).toContain("BVs is the wettest in Plate (−12.0 dB), then Lead Vox (−14.0 dB)");
    expect(text(d, "react")).toContain("The compressor of Bass DI listens to Kick");
    expect(text(d, "time")).toContain("Gtr Left, Gtr Right and BVs play only here.");
    // Deterministic: no clock, no random, no model.
    expect(explainMix(demoProject(), mockLevels(demoProject()))).toEqual(d);
    // Every name is a chip of an element of the project, in its colour.
    for (const c of chips(d)) expect(p.tracks.some((t) => t.id === c.id && t.name === c.name), c.name).toBe(true);
    expect(chips(d).find((c) => c.name === "Kick")!.tint).toBe("t2");
    // Every term the text uses is in the glossary, with one sentence.
    const terms = d.sections.flatMap((s) => s.blocks).flatMap((b) => (b.kind === "p" ? b.parts : b.kind === "item" ? [...b.head, ...b.body] : [])).filter((x): x is { term: string; text: string } => typeof x !== "string" && "term" in x);
    expect(terms.length).toBeGreaterThan(2);
    for (const t of terms) expect(GLOSSARY[t.term], t.term).toMatch(/\.$/);
  });

  // req: UI-16
  it("shows who is in front from the measured levels, says so while they are measured, and leaves the part out without them", () => {
    const p = demoProject();
    const l = mockLevels(p);
    const bars = sec(explainMix(p, l), "balance")!.blocks.find((b) => b.kind === "bars")!;
    if (bars.kind !== "bars") throw new Error("bars");
    // Loudest first, in LUFS (the mix plus the family's distance from it), the loudest bar full.
    const top = [...l.families].sort((a, b) => b.level_lu! - a.level_lu!)[0];
    expect(bars.bars[0].value).toBe(1);
    expect(bars.bars[0].text).toBe(`${(l.mix_lufs! + top.level_lu!).toFixed(1).replace("-", "−")} LUFS`);
    expect(bars.bars.map((b) => b.value)).toEqual([...bars.bars.map((b) => b.value)].sort((a, b) => b - a).slice(0, bars.bars.length));
    expect(bars.bars.at(-1)!.label).toBe("Shared effects");
    expect(text(explainMix(p, null), "balance")).toContain("Measuring how loud each family");
    const none = explainMix(p, { mix_lufs: null, sections: [], families: [], returns: [] });
    expect(sec(none, "balance")).toBeUndefined();
    expect(text(none, "short")).not.toContain("in front");
  });

  // req: UI-16
  it("quotes the assistant's summary and notes as reasons, and invents none", () => {
    const p = demoProject();
    p.ai.summary = null;
    p.ai.notes = [];
    expect(sec(explainMix(p, null), "reasons")).toBeUndefined();
    expect(sec(explainElement(p, id(p, "Lead Vox"), null)!, "reasons")).toBeUndefined();
    p.ai.summary = "Voce davanti, batteria asciutta.";
    p.ai.notes = [{ number: 1, track: id(p, "Lead Vox"), text: "Alzata di 1,8 dB." }, { number: 2, track: null, text: "Meno riverbero ovunque." }];
    const r = sec(explainMix(p, null), "reasons")!;
    const notes = r.blocks.filter((b) => b.kind === "note");
    expect(notes.map((b) => (b.kind === "note" ? [b.number, b.title, b.text] : []))).toEqual([[null, "The assistant’s summary", "Voce davanti, batteria asciutta."], [1, "Lead Vox", "Alzata di 1,8 dB."], [2, "The whole mix", "Meno riverbero ovunque."]]);
    const vox = explainElement(p, id(p, "Lead Vox"), null)!;
    expect(text(vox, "reasons")).toBe("Alzata di 1,8 dB.");
    expect(sec(explainElement(p, id(p, "Kick"), null)!, "reasons")).toBeUndefined();
  });

  // req: UI-16
  it("tells one track: where it goes, where it sits, its working blocks in signal order with what each does", () => {
    const p = demoProject();
    const d = explainElement(p, id(p, "Bass DI"), mockLevels(p))!;
    expect(d.title).toBe("Bass DI");
    expect(d.sections.map((s) => s.id)).toEqual(["where", "place", "react", "chain", "reasons"]);
    expect(text(d, "where")).toBe("Bass DI goes straight to the mix bus: it is not part of a group. It sends nothing to the shared effects: it stays dry.");
    expect(text(d, "place")).toContain("Fader at −7.2 dB, in the centre.");
    expect(text(d, "place")).toContain("It plays in 6 of the 8 sections: not in the intro or the bridge.");
    expect(text(d, "react")).toContain("Its compressor listens to Kick through a sidechain");
    const steps = sec(d, "chain")!.blocks[0];
    if (steps.kind !== "steps") throw new Error("steps");
    // Chain order, bypassed blocks left out and named under the list, the fader last.
    expect(steps.steps.map((s) => s.sig)).toEqual(["HP", "EQ", "CMP", "BAS", "LVL"]);
    expect(steps.steps[2].keyTint).toBe("t2");
    expect(steps.steps[1].body.map(partText).join("")).toBe("More weight (+1.5 dB at 80 Hz), less mud (−2.0 dB at 250 Hz).");
    expect(steps.steps.at(-1)!.body.map(partText).join("")).toBe("Its level in the mix. Automated by hand: it moves along the song.");
    expect(text(d, "chain")).toContain("Off on this track: Saturation.");
    // A track in a group, with sends; a block that plays in some sections only.
    const snare = explainElement(p, id(p, "Snare"), null)!;
    expect(text(snare, "where")).toBe("Snare goes into Drums, which goes to the mix bus. It also sends a copy of itself to Plate (−18.0 dB).");
    expect(text(explainElement(p, id(p, "Lead Vox"), null)!, "chain")).toContain("On only in: Choruses.");
    expect(explainElement(p, "nope", null)).toBeNull();
  });

  // req: UI-16
  it("tells a group and a return: what they take, who feeds them, what that does", () => {
    const p = demoProject();
    const g = explainElement(p, id(p, "Drums"), null)!;
    expect(text(g, "where")).toContain("Drums takes Kick, Snare, Overheads and Room and sends them on straight to the mix bus as one.");
    expect(text(g, "place")).toBe("Group fader at −4.0 dB: it moves 4 tracks together.");
    const r = explainElement(p, id(p, "Plate"), null)!;
    expect(r.sections.map((s) => s.id)).toEqual(["what", "senders", "does"]);
    expect(text(r, "what")).toContain("A plate reverb with a 1.2 s tail");
    expect(text(r, "what")).toContain("only the effect comes back, into the mix bus");
    expect(text(r, "senders")).toContain("BVs −12.0 dB\nLead Vox −14.0 dB\nSnare −18.0 dB\nAcoustic −20.0 dB");
    expect(text(r, "senders")).toContain("BVs is the wettest, Acoustic the driest.");
    expect(text(explainElement(p, id(p, "Echo"), null)!, "what")).toContain("it repeats what it receives every dotted 1/8 note");
  });

  // req: UI-16
  it("tells a section: what plays, what is set differently, how loud it is next to the others", () => {
    const p = demoProject();
    const l = mockLevels(p);
    const d = explainSection(p, "Chorus", l)!;
    expect(d.sections.map((s) => s.id)).toEqual(["plays", "diff", "compare"]);
    expect(text(d, "plays")).toBe("11 of the 12 tracks. Gtr Left, Gtr Right and BVs play only here. Rhodes is silent here.");
    expect(text(d, "diff")).toContain("Lead Vox Doubler on");
    const bars = sec(d, "compare")!.blocks[0];
    if (bars.kind !== "bars") throw new Error("bars");
    expect(bars.bars.filter((b) => b.accent).map((b) => b.label)).toEqual(["Chorus"]);
    expect(bars.bars.map((b) => b.label).sort()).toEqual(["Bridge", "Chorus", "Intro", "Outro", "Verse 1", "Verse 2"]);
    expect(text(explainSection(p, "Intro", null)!, "compare")).toContain("Measuring");
    expect(explainSection(p, "Nowhere", l)).toBeNull();
  });

  // req: UI-16
  it("says so when nothing has been shaped, and gives no advice of its own", () => {
    const p = demoProject();
    p.tracks = p.tracks.filter((t) => t.kind === "audio");
    p.ai.summary = null;
    p.ai.notes = [];
    for (const t of p.tracks) {
      t.output = { kind: "mix_bus" };
      t.sends = [];
      t.automation = [];
      t.chain.pan = 0;
      for (const b of t.chain.blocks) b.fx.bypass = true;
    }
    const d = explainMix(p, null);
    expect(text(d, "short")).toContain("Nothing has been shaped yet. All 12 tracks go straight to the mix bus at the level of their files.");
    expect(sec(d, "structure")).toBeUndefined();
    expect(text(d, "pan")).toContain("Every track is in the centre.");
    expect(sec(d, "space")).toBeUndefined();
    expect(text(d, "reasons")).toContain("This page describes the mix; it does not judge it.");
    expect(buildMap(p).links.every((l) => l.kind === "out" && l.to === MIX)).toBe(true);
  });

  // req: UI-16
  it("has a sentence for every kind of block, and copies the whole text as plain text", () => {
    const p = demoProject();
    const kinds = new Set<string>();
    for (const t of p.tracks) {
      for (const b of t.chain.blocks) {
        kinds.add(b.fx.type);
        const words = blockWords({ ...b.fx, bypass: false }, p).map(partText).join("");
        expect(words.length, b.fx.type).toBeGreaterThan(20);
        expect(words, b.fx.type).toMatch(/\.$/);
      }
    }
    expect(kinds.size).toBeGreaterThan(9);
    const plain = toPlainText(explainMix(p, mockLevels(p)));
    expect(plain.startsWith("Explain this mix\n\nIN SHORT\n12 tracks reach the mix bus")).toBe(true);
    expect(plain).toContain("HOW IT IS BUILT");
    expect(plain).toContain("- Lead vocal: ");
    expect(plain).not.toMatch(/\[object|undefined|NaN/);
    expect(toPlainText(explainElement(p, id(p, "Bass DI"), null)!)).toContain("1. High-pass (40 Hz): Removes what is below 40 Hz");
  });
});
