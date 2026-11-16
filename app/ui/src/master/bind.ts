// Controls of the master chain bound to the master history (HIS-5): every change is a `set_master` edit, a drag is
// one gesture in the master phase. The ranges are the master's (MAS-1, §5.5 limits widened for the author), the
// same the core checks on every `set_master` (`MasterState::check`).
import { useMemo } from "react";
import type { Block } from "../bindings/Block";
import type { MasterState } from "../bindings/MasterState";
import type { SectionOffsets } from "../bindings/SectionOffsets";
import type { ParamSpec } from "../bindings/ParamSpec";
import type { Ctl } from "../shared/controls";
import { useApp, useBackend } from "../state/context";

type Unit = ParamSpec["unit"];
const spec = (id: string, label: string, min: number, max: number, def: number, unit: Unit, log = false): ParamSpec => ({ id, label, min, max, default: def, unit, log, stepped: false });

export const MS = {
  input: spec("master.input", "Input gain", -12, 12, 0, "db"),
  eqFreq: spec("master.eq.freq", "Frequency", 20, 20000, 1000, "hz", true),
  eqGain: spec("master.eq.gain", "Gain", -12, 12, 0, "db"),
  eqQ: spec("master.eq.q", "Q", 0.3, 8, 0.7, "ratio", true),
  match: spec("master.match", "Amount", 0, 1, 0.5, "percent"),
  threshold: spec("comp.threshold", "Threshold", -40, 0, -14, "db"),
  ratio: spec("comp.ratio", "Ratio", 1, 10, 1.8, "ratio"),
  attack: spec("comp.attack", "Attack", 0.1, 100, 30, "ms", true),
  release: spec("comp.release", "Release", 20, 1000, 200, "ms", true),
  knee: spec("comp.knee", "Knee", 0, 12, 6, "db"),
  makeup: spec("comp.makeup", "Makeup", 0, 12, 0, "db"),
  side: spec("master.side", "Side", -6, 6, 0, "db"),
  mono: spec("master.mono", "Mono below", 0, 250, 120, "hz"),
  drive: spec("sat.drive", "Drive", 0, 18, 3, "db"),
  mix: spec("sat.mix", "Mix", 0, 1, 1, "percent"),
  limRelease: spec("master.lim", "Release", 20, 500, 80, "ms", true),
  compMix: spec("comp.mix", "Mix", 0, 1, 1, "percent"),
  listenAbove: spec("comp.listen", "Listens above", 0, 500, 0, "hz"),
  takeOff: spec("clip.take", "Take off up to", 0, 12, 2, "db"),
  softness: spec("clip.soft", "Softness", 0, 1, 0.4, "percent"),
  bandGain: spec("mb.gain", "Gain", -12, 12, 0, "db"),
  secLevel: spec("sec.level", "Level", -3, 3, 0, "db"),
  secSide: spec("sec.side", "Width", -3, 3, 0, "db"),
  secRelease: spec("sec.release", "Limiter release", -200, 200, 0, "ms"),
  secEq: spec("sec.eq", "Gain", -3, 3, 0, "db"),
  lufs: spec("master.lufs", "Loudness", -30, -5, -14, "db"),
  ceiling: spec("master.tp", "Ceiling", -6, 0, -1, "db"),
};

/** A control on one value of the master state: `get` reads it, `put` writes it into a copy. */
export function useMasterCtl(s: ParamSpec, label: string, get: (m: MasterState) => number, put: (m: MasterState, v: number) => void): Ctl {
  const backend = useBackend();
  const master = useApp((st) => st.project?.master);
  const setError = useApp((st) => st.setError);
  return useMemo(() => {
    const fail = (e: unknown) => setError(String(e));
    const m = master!;
    const make = (v: number) => {
      const next = structuredClone(m);
      put(next, v);
      return { op: "set_master" as const, master: next };
    };
    return {
      spec: s, label, value: m ? get(m) : s.default,
      begin: () => backend.beginGesture(`Master · ${label}`, "master").catch(fail),
      change: (v) => backend.gestureEdit(make(v)).catch(fail),
      end: (cancel) => backend.endGesture(cancel).catch(fail),
      set: (v) => backend.edit(make(v), `Master · ${label}`).catch(fail),
    };
    // `get`/`put` are stable per call site.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend, master, setError, s, label]);
}

/** The block of the chain with this id, narrowed to its kind (`undefined` if it is gone or of another kind). */
export function blockOf<K extends Block["type"]>(m: MasterState | undefined, id: string, kind: K): Extract<Block, { type: K }> | undefined {
  const b = m?.chain.blocks.find((x) => x.id === id)?.block;
  return b && b.type === kind ? (b as Extract<Block, { type: K }>) : undefined;
}

/** A control on one value of a block of the chain. */
export function useBlockCtl<K extends Block["type"]>(s: ParamSpec, label: string, id: string, kind: K, get: (b: Extract<Block, { type: K }>) => number, put: (b: Extract<Block, { type: K }>, v: number) => void): Ctl {
  return useMasterCtl(
    s,
    label,
    (m) => {
      const b = blockOf(m, id, kind);
      return b ? get(b) : s.default;
    },
    (m, v) => {
      const b = blockOf(m, id, kind);
      if (b) put(b, v);
    },
  );
}

/** The offsets of some sections in a master state, made where they are missing; entries left empty are dropped. */
export function withOffsets(m: MasterState, sections: string[], f: (o: SectionOffsets) => void) {
  for (const id of sections) {
    let o = m.sections.find((x) => x.section === id);
    if (!o) {
      o = { section: id, level_db: 0, side_gain_db: 0, limiter_release_ms: 0, eq: [] };
      m.sections.push(o);
    }
    f(o);
  }
  const empty = (o: SectionOffsets) => o.level_db === 0 && o.side_gain_db === 0 && o.limiter_release_ms === 0 && o.eq.every((e) => e.gains_db.every((g) => g === 0));
  m.sections = m.sections.filter((o) => !empty(o));
}

/** One step of the master history changing the master state. */
export function useMasterEdit() {
  const backend = useBackend();
  const master = useApp((st) => st.project?.master);
  const setError = useApp((st) => st.setError);
  return (label: string, f: (m: MasterState) => void) => {
    if (!master) return;
    const next = structuredClone(master);
    f(next);
    void backend.edit({ op: "set_master", master: next }, label).catch((e) => setError(String(e)));
  };
}

export const PRESETS: [string, string, number, number][] = [
  ["streaming", "Streaming", -14, -1],
  ["loud", "Loud", -9, -1],
  ["cd", "CD", -10, -0.3],
];

export function targetName(m: MasterState): string {
  const p = PRESETS.find(([k]) => k === m.target.preset);
  const fmt = (v: number) => v.toFixed(1).replace(/\.0$/, "").replace("-", "−");
  return `${p ? p[1] : "Custom"} · ${fmt(m.target.lufs)} LUFS · ${fmt(m.target.true_peak_db)} dBTP`;
}
