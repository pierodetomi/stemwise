// The chain of blocks in the mock backend: what the core does in Rust (`Chain::edit`, `Chain::ensure`, `Fx::new`)
// redone only as far as the screens need it in the browser. New blocks come from the catalog the core generates.
// Never used by the real app.
import type { Chain } from "../bindings/Chain";
import type { Compressor } from "../bindings/Compressor";
import type { Equalizer } from "../bindings/Equalizer";
import type { Filter } from "../bindings/Filter";
import type { Fx } from "../bindings/Fx";
import type { FxBlock } from "../bindings/FxBlock";
import type { FxKind } from "../bindings/FxKind";
import type { FxOp } from "../bindings/FxOp";
import type { Saturation } from "../bindings/Saturation";
import { CLASSIC, INFO, PER_KIND, RETURN_BLOCKS, TRACK_BLOCKS, freeId } from "../mix/fx";

export const newFx = (kind: FxKind): Fx => structuredClone(INFO[kind].new);

export const classicBlocks = (): FxBlock[] => CLASSIC.map((k) => ({ id: INFO[k].key, fx: newFx(k) }));

export { freeId };

/** The first block of a classic kind, put back at its starting place if missing (`Chain::ensure`). */
export function ensure(c: Chain, kind: FxKind): Fx {
  let b = c.blocks.find((x) => x.fx.type === kind);
  if (!b) {
    const rank = (k: FxKind) => (CLASSIC.includes(k) ? CLASSIC.indexOf(k) : CLASSIC.length);
    let at = c.blocks.findIndex((x) => rank(x.fx.type) > rank(kind) && rank(x.fx.type) < CLASSIC.length);
    if (at < 0) {
      const before = c.blocks.map((x) => rank(x.fx.type) < rank(kind));
      const last = before.lastIndexOf(true);
      at = last < 0 ? 0 : last + 1;
    }
    b = { id: freeId(c, kind), fx: newFx(kind) };
    c.blocks.splice(at, 0, b);
  }
  return b.fx;
}

export const hpfOf = (c: Chain) => ensure(c, "hpf") as Filter & { type: "hpf" };
export const lpfOf = (c: Chain) => ensure(c, "lpf") as Filter & { type: "lpf" };
export const eqOf = (c: Chain) => ensure(c, "eq") as Equalizer & { type: "eq" };
export const compOf = (c: Chain) => ensure(c, "compressor") as Compressor & { type: "compressor" };
export const satOf = (c: Chain) => ensure(c, "saturation") as Saturation & { type: "saturation" };

/** Applies a structural change in place; returns the id of a block it added. Throws what the core would refuse. */
export function chainEdit(c: Chain, op: FxOp, ret: boolean): string | null {
  const cap = ret ? RETURN_BLOCKS : TRACK_BLOCKS;
  const at = (id: string) => {
    const i = c.blocks.findIndex((b) => b.id === id);
    if (i < 0) throw new Error("no such block");
    return i;
  };
  const canAdd = (kind: FxKind) => {
    if (c.blocks.length >= cap) throw new Error(`the chain holds ${cap} blocks at most: remove one first`);
    if (c.blocks.filter((b) => b.fx.type === kind).length >= PER_KIND) throw new Error(`the chain already has ${PER_KIND} ${INFO[kind].label}`);
  };
  switch (op.op) {
    case "add": {
      canAdd(op.kind);
      const id = freeId(c, op.kind);
      c.blocks.splice(Math.min(op.at ?? c.blocks.length, c.blocks.length), 0, { id, fx: newFx(op.kind) });
      return id;
    }
    case "remove":
      c.blocks.splice(at(op.id), 1);
      return null;
    case "move": {
      const [b] = c.blocks.splice(at(op.id), 1);
      c.blocks.splice(Math.min(op.to, c.blocks.length), 0, b);
      return null;
    }
    case "duplicate": {
      const i = at(op.id);
      canAdd(c.blocks[i].fx.type);
      const id = freeId(c, c.blocks[i].fx.type);
      c.blocks.splice(i + 1, 0, { id, fx: structuredClone(c.blocks[i].fx) });
      return id;
    }
    case "reset": {
      const b = c.blocks[at(op.id)];
      const off = b.fx.bypass;
      b.fx = { ...newFx(b.fx.type), bypass: off } as Fx;
      return null;
    }
    case "bands": {
      const b = c.blocks[at(op.id)];
      const fx = b.fx;
      const n = op.bands;
      if (fx.type === "multiband") {
        if (n < 2 || n > 6) throw new Error("a multiband has 2 to 6 bands");
        while (fx.bands.length > n) {
          fx.bands.pop();
          fx.crossovers_hz.pop();
        }
        while (fx.bands.length < n) {
          const last = fx.crossovers_hz[fx.crossovers_hz.length - 1] ?? 200;
          fx.crossovers_hz.push(Math.min(16000, Math.round(Math.sqrt(last * 20000))));
          fx.bands.push({ ...fx.bands[fx.bands.length - 1] });
        }
      } else if (fx.type === "dynamic_eq" || fx.type === "harmonizer") {
        if (n < 1 || n > 2) throw new Error("one or two");
        const fresh = newFx(fx.type) as typeof fx;
        if (fx.type === "dynamic_eq" && fresh.type === "dynamic_eq") {
          fx.bands = fx.bands.slice(0, n);
          while (fx.bands.length < n) fx.bands.push({ ...fresh.bands[fx.bands.length], enabled: true });
        } else if (fx.type === "harmonizer" && fresh.type === "harmonizer") {
          fx.voices = fx.voices.slice(0, n);
          while (fx.voices.length < n) fx.voices.push({ ...fresh.voices[fx.voices.length], enabled: true });
        }
      } else if (fx.type === "doubler") {
        fx.voices = n;
      } else throw new Error("this block has no bands");
      return null;
    }
  }
}
