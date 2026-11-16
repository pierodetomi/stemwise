// The master chain in the mock backend: what the core does in Rust (`MasterState::chain_edit`, `MasterChain::name`,
// the default chain) redone only as far as the screens need it in the browser. Never used by the real app.
import type { Block } from "../bindings/Block";
import type { BlockKind } from "../bindings/BlockKind";
import type { BlockView } from "../bindings/BlockView";
import type { ChainOp } from "../bindings/ChainOp";
import type { MasterChain } from "../bindings/MasterChain";
import type { MasterState } from "../bindings/MasterState";

export const KINDS: BlockKind[] = ["eq", "match_eq", "compressor", "multiband", "width", "saturation", "clipper"];
export const KIND_LABEL: Record<BlockKind, string> = {
  eq: "EQ", match_eq: "Match EQ", compressor: "Compressor", multiband: "Multiband", width: "Width", saturation: "Saturation", clipper: "Clipper",
};
const FREE_BLOCKS = 9;
const maxOf = (k: BlockKind) => (k === "match_eq" || k === "width" ? 1 : 2);

export function newBlock(kind: BlockKind): Block {
  const band = (k: "low_shelf" | "peak" | "high_shelf", freq_hz: number) => ({ enabled: false, kind: k, freq_hz, gain_db: 0, q: 0.7 });
  const mb = () => ({ bypass: false, threshold_db: -18, ratio: 2, attack_ms: 20, release_ms: 160, gain_db: 0 });
  switch (kind) {
    case "eq":
      return { type: "eq", bypass: true, bands: [band("low_shelf", 60), band("peak", 200), band("peak", 800), band("peak", 2500), band("peak", 6000), band("high_shelf", 12000)] };
    case "match_eq":
      return { type: "match_eq", bypass: true, amount: 0.5 };
    case "compressor":
      return { type: "compressor", bypass: true, threshold_db: -14, ratio: 1.8, attack_ms: 30, release_ms: 200, knee_db: 6, makeup_db: 0, mix: 1, listen_above_hz: 0 };
    case "multiband":
      return { type: "multiband", bypass: true, crossovers_hz: [120, 4000], bands: [mb(), mb(), mb()] };
    case "width":
      return { type: "width", bypass: true, side_gain_db: 0, mono_below_hz: 120 };
    case "saturation":
      return { type: "saturation", bypass: true, drive_db: 3, mix: 1, character: "soft" };
    default:
      return { type: "clipper", bypass: true, take_off_db: 2, softness: 0.4 };
  }
}

/** A new project's chain: every kind once, off (the EQ on with no band in use). */
export function defaultChain(): MasterChain {
  return {
    input_gain_db: 0,
    blocks: KINDS.map((k, i) => ({ id: `b${i + 1}`, block: k === "eq" ? { ...newBlock(k), bypass: false } : newBlock(k) })),
    limiter: { bypass: false, release_ms: 80 },
    dither: { bypass: false },
  };
}

const count = (c: MasterChain, k: BlockKind) => c.blocks.filter((b) => b.block.type === k).length;

export function chainView(c: MasterChain): BlockView[] {
  return c.blocks.map((b) => {
    const k = b.block.type;
    const same = c.blocks.filter((x) => x.block.type === k);
    const inst = same.map((x) => x.id).sort().indexOf(b.id);
    return { id: b.id, kind: k, inst, name: same.length < 2 ? KIND_LABEL[k] : `${KIND_LABEL[k]} ${same.findIndex((x) => x.id === b.id) + 1}` };
  });
}

export function addable(c: MasterChain): BlockKind[] {
  return c.blocks.length >= FREE_BLOCKS ? [] : KINDS.filter((k) => count(c, k) < maxOf(k));
}

/** The master after a structural change, and the id of the block it added. */
export function chainEdit(m: MasterState, op: ChainOp): { master: MasterState; added: string | null } {
  const next = structuredClone(m);
  const c = next.chain;
  let added: string | null = null;
  const mbOf = (id: string) => {
    const b = c.blocks.find((x) => x.id === id)?.block;
    if (!b || b.type !== "multiband") throw new Error("no such multiband");
    return b;
  };
  const split = (mb: ReturnType<typeof mbOf>, i: number, hz: number) => {
    const lo = i === 0 ? 20 : mb.crossovers_hz[i - 1];
    const hi = mb.crossovers_hz[i] ?? 20000;
    if (mb.bands.length >= 6 || !(hz > lo * 1.26 && hz * 1.26 < hi)) return false;
    mb.crossovers_hz.splice(i, 0, hz);
    mb.bands.splice(i, 0, { ...mb.bands[i] });
    return true;
  };
  switch (op.op) {
    case "add": {
      if (!addable(c).includes(op.kind)) throw new Error(c.blocks.length >= FREE_BLOCKS ? "the chain is full: remove a block first" : `the chain already has ${maxOf(op.kind)} ${KIND_LABEL[op.kind]}`);
      let n = 1;
      while (c.blocks.some((b) => b.id === `b${n}`)) n++;
      added = `b${n}`;
      c.blocks.splice(Math.min(op.at ?? c.blocks.length, c.blocks.length), 0, { id: added, block: newBlock(op.kind) });
      break;
    }
    case "remove":
      c.blocks = c.blocks.filter((b) => b.id !== op.id);
      next.sections = next.sections.map((s) => ({ ...s, eq: s.eq.filter((e) => e.block !== op.id) }));
      break;
    case "move": {
      const i = c.blocks.findIndex((b) => b.id === op.id);
      if (i < 0) throw new Error("no such block");
      const [b] = c.blocks.splice(i, 1);
      c.blocks.splice(Math.min(op.to, c.blocks.length), 0, b);
      break;
    }
    case "bands": {
      const mb = mbOf(op.id);
      if (op.bands < 2 || op.bands > 6) throw new Error("a multiband has 2 to 6 bands");
      while (mb.bands.length > op.bands) {
        mb.crossovers_hz.pop();
        mb.bands.pop();
      }
      while (mb.bands.length < op.bands) {
        const edge = (k: number): [number, number] => [k === 0 ? 20 : mb.crossovers_hz[k - 1], mb.crossovers_hz[k] ?? 20000];
        let k = 0;
        for (let i = 1; i < mb.bands.length; i++) if (edge(i)[1] / edge(i)[0] > edge(k)[1] / edge(k)[0]) k = i;
        const [a, b] = edge(k);
        if (!split(mb, k, Math.round(Math.sqrt(a * b)))) throw new Error("the bands are too narrow to split further");
      }
      break;
    }
    case "split": {
      const mb = mbOf(op.id);
      if (!split(mb, mb.crossovers_hz.filter((x) => x < op.hz).length, op.hz)) throw new Error("a band cannot be split there: too close to a crossover, or six bands already");
      break;
    }
    case "reset": {
      const b = c.blocks.find((x) => x.id === op.id);
      if (!b) throw new Error("no such block");
      b.block = { ...newBlock(b.block.type), bypass: b.block.bypass };
      next.sections = next.sections.map((s) => ({ ...s, eq: s.eq.filter((e) => e.block !== op.id) }));
      break;
    }
    case "reset_order":
      c.blocks.sort((x, y) => KINDS.indexOf(x.block.type) - KINDS.indexOf(y.block.type));
      break;
    default: {
      const mb = mbOf(op.id);
      if (mb.bands.length <= 2 || op.crossover >= mb.crossovers_hz.length) throw new Error("a multiband keeps two bands at least");
      mb.crossovers_hz.splice(op.crossover, 1);
      mb.bands.splice(op.crossover + 1, 1);
    }
  }
  return { master: next, added };
}
