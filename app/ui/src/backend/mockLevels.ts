// Levels of the demo mix for the Explain view (ANA-20). The real backend measures renders; the mock has no audio,
// so it derives stand-in numbers from the document: the same shape, stable for the same project, moving the right
// way when a fader moves or a track stops playing in a section.
import type { MixLevels } from "../bindings/MixLevels";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import families from "../bindings/role_families.json";

const FAMILY_OF = families.of as Record<string, string>;
const pow = (db: number) => 10 ** (db / 10);
const db = (p: number) => 10 * Math.log10(p);
const r1 = (v: number) => Math.round(v * 10) / 10;

/** Gain of a track down to the mix bus: its fader and the faders of the groups it goes through. */
function gainDb(p: Project, t: Track): number {
  let g = t.chain.fader_db;
  let out = t.output;
  for (let i = 0; i < 8 && out.kind === "group"; i++) {
    const id = out.id;
    const parent = p.tracks.find((x) => x.id === id);
    if (!parent) break;
    g += parent.chain.fader_db;
    out = parent.output;
  }
  return g;
}

const plays = (t: Track, a: number, b: number) => t.clips.some((c) => !c.mute && c.start < b && c.start + c.length > a);

export function mockLevels(p: Project): MixLevels {
  const audio = p.tracks.filter((t) => t.kind === "audio" && !t.mute && t.clips.length > 0);
  if (!audio.length) return { mix_lufs: null, sections: [], families: [], returns: [] };
  const sections = [...p.sections].sort((a, b) => a.start - b.start);
  const sum = (ts: Track[]) => ts.reduce((a, t) => a + pow(gainDb(p, t) - 12), 0);
  const level = (ts: Track[]) => (ts.length ? db(sum(ts)) - 6 : null);
  const mix = level(audio)!;
  const secMix = sections.map((s) => level(audio.filter((t) => plays(t, s.start, s.end))));
  const rel = (v: number | null, m: number | null) => (v === null || m === null ? null : r1(v - m));
  return {
    mix_lufs: r1(mix),
    sections: sections.map((s, i) => ({ id: s.id, lufs: secMix[i] === null ? null : r1(secMix[i]) })),
    families: families.families
      .map((name) => {
        const ts = audio.filter((t) => (FAMILY_OF[t.role ?? "other"] ?? "other") === name);
        return {
          name,
          tracks: ts.map((t) => t.id),
          level_lu: rel(level(ts), mix),
          section_lu: sections.map((s, i) => rel(level(ts.filter((t) => plays(t, s.start, s.end))), secMix[i])),
        };
      })
      .filter((f) => f.tracks.length > 0),
    returns: p.tracks
      .filter((t) => t.kind === "return" && !t.mute)
      .map((r) => {
        const fed = audio.flatMap((t) => t.sends.filter((s) => s.target === r.id).map((s) => pow(gainDb(p, t) + s.level_db - 12)));
        return { id: r.id, level_lu: fed.length ? r1(db(fed.reduce((a, b) => a + b, 0)) - 6 + r.chain.fader_db - mix) : null };
      }),
  };
}
