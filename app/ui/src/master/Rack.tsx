// The master chain as a rack (design 15a, 15b, 15i, 15j; MAS-1, MAS-15): one row per block, top to bottom, beside the
// editor. Input first, limiter and dither last and locked; the free blocks between them can be dragged, moved from
// the keyboard, switched off, removed and added.
import { useRef, useState } from "react";
import type { Block } from "../bindings/Block";
import type { BlockKind } from "../bindings/BlockKind";
import type { BlockView } from "../bindings/BlockView";
import type { ChainOp } from "../bindings/ChainOp";
import type { MasterState } from "../bindings/MasterState";
import type { MasterStatus } from "../bindings/MasterStatus";
import { Icon } from "../shared/Icon";
import { useMenu, type MenuItem } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import { useMasterEdit } from "./bind";

export const n1 = (v: number | null | undefined) => (v === null || v === undefined || !Number.isFinite(v) ? "—" : v.toFixed(1).replace("-", "−"));
export const hz = (f: number) => (f >= 1000 ? `${n1(f / 1000)} kHz` : `${Math.round(f)} Hz`);
const signed = (v: number) => `${v > 0 ? "+" : ""}${n1(v)}`;

export const KIND_NAME: Record<BlockKind, string> = {
  eq: "EQ", match_eq: "Match EQ", compressor: "Compressor", multiband: "Multiband", width: "Width", saturation: "Saturation", clipper: "Clipper",
};
const KINDS = Object.keys(KIND_NAME) as BlockKind[];

/** What a block is set to, in a few words (the row's value). */
export function blockValue(b: Block): string {
  switch (b.type) {
    case "eq": {
      const n = b.bands.filter((x) => x.enabled).length;
      return n ? `${n} band${n > 1 ? "s" : ""}` : "No band set";
    }
    case "match_eq":
      return `${Math.round(b.amount * 100)} %`;
    case "compressor":
      return b.mix < 1 ? `${n1(b.ratio)}:1 parallel` : `${n1(b.ratio)}:1`;
    case "multiband":
      return `${b.bands.length} bands`;
    case "width":
      return `${signed(b.side_gain_db)} dB side`;
    case "saturation":
      return `${n1(b.drive_db)} dB`;
    default:
      return `Up to ${n1(b.take_off_db)} dB`;
  }
}

/** Whether a block does something: it is on, and an EQ has a band in use. */
export const working = (b: Block) => !b.bypass && !(b.type === "eq" && b.bands.every((x) => !x.enabled));

function GrBar({ db }: { db: number }) {
  return (
    <span className="hbar gr rgr" aria-hidden="true">
      <i style={{ width: `${Math.min(100, Math.max(0, -db) * 20)}%` }} />
    </span>
  );
}

/**
 * The rack. With `preview` (a proposal in review, 15f) it lists the proposed chain and nothing can be changed.
 */
export function Rack({ ms, preview }: { ms: MasterStatus; preview?: MasterState }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const meters = useApp((s) => s.meters);
  const open = useApp((s) => s.masterModule);
  const section = useApp((s) => s.masterSection);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const edit = useMasterEdit();
  const menu = useMenu();
  const rows = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ id: string; to: number; y: number } | null>(null);
  const m = preview ?? project.master;
  const ro = !!preview;
  const fail = (e: unknown) => setError(String(e));
  // The proposed chain names its blocks the way the core would: by kind, numbered when there are two.
  const views: BlockView[] = preview
    ? preview.chain.blocks.map((b) => {
        const same = preview.chain.blocks.filter((x) => x.block.type === b.block.type);
        return { id: b.id, kind: b.block.type, inst: 0, name: same.length < 2 ? KIND_NAME[b.block.type] : `${KIND_NAME[b.block.type]} ${same.indexOf(b) + 1}` };
      })
    : ms.chain;
  const sel = section ? null : open;
  const select = (id: string) => setView({ masterModule: id, masterSection: null });
  const op = (o: ChainOp) => backend.masterChain(o).catch(fail);
  const add = (kind: BlockKind, at: number | null) =>
    void backend
      .masterChain({ op: "add", kind, at })
      .then((id) => id && select(id))
      .catch(fail);
  const addItems = (at: number | null): MenuItem[] => {
    const can = KINDS.filter((k) => ms.addable.includes(k));
    const full = views.length >= 9;
    const twice = KINDS.filter((k) => !ms.addable.includes(k) && k !== "match_eq" && k !== "width");
    const once = (["match_eq", "width"] as BlockKind[]).filter((k) => !ms.addable.includes(k));
    const why = full
      ? "The chain is full: twelve blocks with input, limiter and dither. Remove one to add another."
      : [twice.length ? `${twice.map((k) => KIND_NAME[k]).join(", ")}: two already` : "", once.length ? `${once.map((k) => KIND_NAME[k]).join(" and ")}: only one` : ""].filter(Boolean).join(" · ");
    return [
      { header: at === null ? "Add before the limiter" : "Add here" },
      ...can.map((k): MenuItem => ({ label: KIND_NAME[k], icon: "plus", onSelect: () => add(k, at) })),
      ...(why ? (["-", { header: why }] as MenuItem[]) : []),
    ];
  };
  const toggle = (id: string, name: string, on: boolean) =>
    edit(`Master · ${name} ${on ? "on" : "off"}`, (x) => {
      const b = x.chain.blocks.find((y) => y.id === id);
      if (b) b.block.bypass = !on;
    });
  const move = (id: string, by: number) => {
    const i = views.findIndex((v) => v.id === id);
    const to = i + by;
    if (i >= 0 && to >= 0 && to < views.length) void op({ op: "move", id, to });
  };
  const remove = (id: string) => {
    if (open === id) select("limiter");
    void op({ op: "remove", id });
  };
  const rowMenu = (v: BlockView, on: boolean): MenuItem[] => {
    const i = views.findIndex((x) => x.id === v.id);
    return [
      { header: v.name },
      { label: "Move up", kbd: "⌥↑", disabled: i === 0, onSelect: () => move(v.id, -1) },
      { label: "Move down", kbd: "⌥↓", disabled: i === views.length - 1, onSelect: () => move(v.id, 1) },
      "-",
      { label: on ? "Switch off" : "Switch on", onSelect: () => toggle(v.id, v.name, !on) },
      { label: "Reset this module", onSelect: () => void op({ op: "reset", id: v.id }) },
      { label: "Remove", icon: "trash", kbd: "⌫", danger: true, onSelect: () => remove(v.id) },
    ];
  };
  // Dragging a row by its grip: the place it would land follows the pointer; a click without moving opens the menu.
  const onGrip = (e: React.PointerEvent, v: BlockView, on: boolean) => {
    if (ro || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const grip = e.currentTarget as HTMLElement;
    const y0 = e.clientY;
    let moved = false;
    // Where the block would land, kept here: the move is sent once, outside React's state updates.
    let landing: number | null = null;
    const place = (y: number) => {
      const els = Array.from(rows.current?.querySelectorAll<HTMLElement>("[data-free]") ?? []);
      let to = els.length;
      for (let i = 0; i < els.length; i++) {
        const r = els[i].getBoundingClientRect();
        if (y < r.top + r.height / 2) {
          to = i;
          break;
        }
      }
      const top = rows.current?.getBoundingClientRect().top ?? 0;
      landing = to;
      setDrag({ id: v.id, to, y: y - top });
    };
    const onMove = (ev: PointerEvent) => {
      if (!moved && Math.abs(ev.clientY - y0) < 4) return;
      moved = true;
      place(ev.clientY);
    };
    const end = (cancel: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey);
      setDrag(null);
      if (landing !== null && !cancel) {
        const from = views.findIndex((x) => x.id === v.id);
        // The place counts the rows as they are: past its own, the block has left one free.
        const to = landing > from ? landing - 1 : landing;
        if (to !== from) void op({ op: "move", id: v.id, to });
      }
      if (!moved && !cancel) menu.open(grip, rowMenu(v, on), v.name);
    };
    const onUp = () => end(false);
    const onKey = (ev: KeyboardEvent) => ev.key === "Escape" && end(true);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
  };
  const onRowKey = (e: React.KeyboardEvent, v: BlockView) => {
    if (ro) return;
    if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      move(v.id, e.key === "ArrowUp" ? -1 : 1);
    } else if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      remove(v.id);
    }
  };
  const fixed = (id: string, n: number, name: string, value: string, led: React.ReactNode, gr?: number) => (
    <div className={`rrow is-fixed ${sel === id ? "on" : ""} ${drag ? "is-dim" : ""}`} onClick={() => !ro && select(id)}>
      <Icon name="lock" size="sm" className="faint" />
      <span className="faint s1 rn" aria-hidden="true">{n}</span>
      <button type="button" className="tabname b ell rname" aria-pressed={sel === id} disabled={ro} onClick={(e) => { e.stopPropagation(); select(id); }}>{name}</button>
      <span className="dim s1 ell grow">{value}</span>
      {gr !== undefined && <GrBar db={gr} />}
      {led}
    </div>
  );
  const limGr = meters?.masterGr?.[1] ?? 0;
  const count = views.length + 3;
  return (
    <section className="card rack" aria-label="Master chain">
      <div className="ph">
        <span className="h">Chain</span>
        <span className="faint s1 grow">{ro ? "as proposed" : drag ? "moving a block" : "top to bottom"}</span>
        {!ro && <button type="button" className="btn quiet sm" onClick={() => void op({ op: "reset_order" })}>Reset</button>}
      </div>
      <div className="sect grow rrows" ref={rows} role="group" aria-label="Blocks of the chain">
        {fixed("input", 1, "Input", `${n1(m.chain.input_gain_db)} dB`, <i className="led" aria-hidden="true" />)}
        {views.map((v, i) => {
          const b = m.chain.blocks.find((x) => x.id === v.id)?.block;
          if (!b) return null;
          const on = !b.bypass;
          const gr = !ro && on && (v.kind === "compressor" || v.kind === "multiband" || v.kind === "clipper") ? (meters?.masterBlocks?.[i] ?? 0) : undefined;
          return (
            <div key={v.id} className="rwrap">
              {drag && drag.to === i && drag.id !== v.id && <div className="rslot" aria-hidden="true" />}
              {!ro && !drag && ms.addable.length > 0 && (
                <button type="button" className="rins" aria-label={`Add a block before ${v.name}`} onClick={(e) => menu.open(e.currentTarget, addItems(i), "Add a block")}>+</button>
              )}
              <div
                data-free
                data-testid={`block-${v.id}`}
                className={`rrow ${on ? "" : "is-off"} ${sel === v.id ? "on" : ""} ${drag?.id === v.id ? "is-moving" : ""}`}
                onClick={() => !ro && select(v.id)}
                onContextMenu={(e) => { if (ro) return; e.preventDefault(); menu.open(e, rowMenu(v, on), v.name); }}
              >
                {ro ? (
                  <Icon name="dots" size="sm" className="faint" />
                ) : (
                  <button type="button" className="rgrip" aria-label={`${v.name}: move or more`} onPointerDown={(e) => onGrip(e, v, on)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); menu.open(e.currentTarget, rowMenu(v, on), v.name); } }}>
                    <Icon name="dots" size="sm" />
                  </button>
                )}
                <span className="faint s1 rn" aria-hidden="true">{i + 2}</span>
                <button type="button" className="tabname b ell rname" aria-pressed={sel === v.id} disabled={ro} onKeyDown={(e) => onRowKey(e, v)} onClick={(e) => { e.stopPropagation(); select(v.id); }}>{v.name}</button>
                <span className="dim s1 ell grow">{on ? blockValue(b) : "Off"}</span>
                {gr !== undefined && <GrBar db={gr} />}
                {ro ? (
                  <i className={`led ${on ? "" : "off"}`} aria-hidden="true" />
                ) : (
                  <button type="button" className={`led ${on ? "" : "off"}`} aria-label={`${v.name} ${on ? "on" : "off"}`} aria-pressed={on} onClick={(e) => { e.stopPropagation(); toggle(v.id, v.name, !on); }} />
                )}
              </div>
            </div>
          );
        })}
        {drag && drag.to >= views.length && <div className="rslot" aria-hidden="true" />}
        {!ro && (
          <button type="button" className={`radd ${ms.addable.length ? "" : "is-full"}`} data-testid="add-block" onClick={(e) => menu.open(e.currentTarget, addItems(null), "Add a block")}>
            <Icon name="plus" size="m" />
            Add module
            <span className="faint s1 ml-auto">{count} of 12</span>
          </button>
        )}
        {fixed("limiter", views.length + 2, "Limiter", `${n1(m.target.true_peak_db)} dBTP`, <i className="led" aria-hidden="true" />, ro ? undefined : limGr)}
        {fixed(
          "dither",
          views.length + 3,
          "Dither",
          "16-bit only",
          ro ? (
            <i className={`led ${m.chain.dither.bypass ? "off" : ""}`} aria-hidden="true" />
          ) : (
            <button type="button" className={`led ${m.chain.dither.bypass ? "off" : ""}`} aria-label={`Dither ${m.chain.dither.bypass ? "off" : "on"}`} aria-pressed={!m.chain.dither.bypass} onClick={(e) => { e.stopPropagation(); edit(`Master · Dither ${m.chain.dither.bypass ? "on" : "off"}`, (x) => (x.chain.dither.bypass = !x.chain.dither.bypass)); }} />
          ),
        )}
        {drag && <div className="rrow is-drag" aria-hidden="true" style={{ top: drag.y }}><Icon name="dots" size="sm" /><span className="b ell">{views.find((v) => v.id === drag.id)?.name}</span></div>}
        <span className="faint s1 rhint">
          {ro ? "The chain as the proposal sets it. Apply or discard it to work on the chain." : drag ? "Input, limiter and dither stay in place · Esc cancels" : <><Icon name="dots" size="sm" /> drag to reorder · <Icon name="lock" size="sm" /> stays in place</>}
        </span>
      </div>
      {menu.node}
    </section>
  );
}
