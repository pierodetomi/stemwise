// The chain of a track, group or return in the inspector (design 16a–16c, 16p; FX-10, FX-11, UI-12): one row per
// block, top to bottom, with its label, value, where it plays and its switch. Trim first and Fader · Pan last are
// locked; the blocks between them are dragged or moved from the keyboard, duplicated, reset, removed. "Add block"
// opens a menu beside the inspector, one column per family. Every change is the core's own edit: one undo step.
import { useEffect, useRef, useState } from "react";
import { keysInText } from "../shared/keys";
import { createPortal } from "react-dom";
import type { FxKind } from "../bindings/FxKind";
import type { FxOp } from "../bindings/FxOp";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { formatDb } from "../shared/format";
import { Icon } from "../shared/Icon";
import { useMenu, type MenuItem } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import { useEdit } from "./bind";
import { INFO, MENU_COLUMNS, PURPOSE, RETURN_BLOCKS, TRACK_BLOCKS, addable, blockName, blockSummary, freeId, playsIn } from "./fx";
import { marksOf } from "./glance";
import { tintOf } from "./Marks";

const panText = (p: number) => (Math.abs(p) < 0.005 ? "C" : `${p < 0 ? "L" : "R"}${Math.round(Math.abs(p) * 100)}`);

/** The "Add block" menu (16b): beside the inspector, one column per family; what cannot be added says why. */
function AddMenu({ t, anchor, onPick, onClose }: { t: Track; anchor: HTMLElement; onPick: (k: FxKind) => void; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const ret = t.kind === "return";
  const can = addable(t.chain, ret);
  const aside = anchor.closest("aside") ?? anchor;
  const r = aside.getBoundingClientRect();
  const b = anchor.getBoundingClientRect();
  useEffect(() => {
    const down = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node) && !anchor.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("pointerdown", down);
    window.addEventListener("keydown", key);
    ref.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    return () => {
      window.removeEventListener("pointerdown", down);
      window.removeEventListener("keydown", key);
    };
  }, [anchor, onClose]);
  const width = Math.min(760, r.left - 16);
  const left = Math.max(8, r.left - width - 8);
  const top = Math.max(8, Math.min(b.top - 120, window.innerHeight - 440));
  return createPortal(
    <div ref={ref} className="sw pop addmenu" role="dialog" aria-label="Add a block" style={{ position: "fixed", left, top, width }} data-testid="add-block-menu">
      <div className="fx g2 am-head">
        <span className="b">Add a block</span>
        <span className="faint s1">lands before the fader</span>
      </div>
      <div className="am-cols">
        {MENU_COLUMNS.map((col) => (
          <div key={col.title} className="col g1 am-col" role="group" aria-label={col.title}>
            <span className="faint s1 am-fam">{col.title}</span>
            {col.kinds.map((k) => {
              const why = can.find((x) => x.kind === k)?.why ?? null;
              return (
                <button type="button" key={k} className="am-item" disabled={!!why} onClick={() => onPick(k)} aria-label={`${INFO[k].label}: ${why ?? PURPOSE[k]}`}>
                  <span className={`sg ${why ? "" : "on"}`}>{INFO[k].mark}</span>
                  <span className="col am-txt">
                    <span className="b ell">{INFO[k].label}</span>
                    <span className="faint s1 ell">{why ?? PURPOSE[k]}</span>
                  </span>
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </div>,
    document.querySelector(".sw.app") ?? document.body,
  );
}

/**
 * The rows of a chain. `sel` is the open block (its editor shows below the list); `onSel` opens one. On a return the
 * list follows the processor and has no trim (FX-11).
 */
export function ChainRack({ t, project, sel, onSel }: { t: Track; project: Project; sel: string; onSel: (id: string) => void }) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const edit = useEdit();
  const menu = useMenu();
  const rows = useRef<HTMLDivElement>(null);
  const addBtn = useRef<HTMLButtonElement>(null);
  const [adding, setAdding] = useState(false);
  const [drag, setDrag] = useState<{ id: string; to: number } | null>(null);
  const ret = t.kind === "return";
  const blocks = t.chain.blocks;
  const marks = marksOf(t.chain, project);
  const cap = ret ? RETURN_BLOCKS : TRACK_BLOCKS;
  const fail = (e: unknown) => setError(String(e));
  const op = (change: FxOp, label: string) => edit({ op: "chain", track: t.id, change }, `${t.name} · ${label}`);
  const add = (k: FxKind) => {
    setAdding(false);
    const id = freeId(t.chain, k);
    void op({ op: "add", kind: k, at: null }, `add ${INFO[k].label}`).then(() => onSel(id));
  };
  const toggle = (id: string) => {
    const b = blocks.find((x) => x.id === id);
    if (!b) return;
    const name = blockName(t.chain, id);
    void backend.setParam(t.id, `bypass.${id}`, b.fx.bypass ? 0 : 1, `${t.name} · ${name} ${b.fx.bypass ? "on" : "off"}`).catch(fail);
  };
  const move = (id: string, by: number) => {
    const i = blocks.findIndex((b) => b.id === id);
    const to = i + by;
    if (i >= 0 && to >= 0 && to < blocks.length) void op({ op: "move", id, to }, `move ${blockName(t.chain, id)}`);
  };
  const remove = (id: string) => {
    const i = blocks.findIndex((b) => b.id === id);
    const next = blocks[i + 1]?.id ?? blocks[i - 1]?.id ?? (ret ? "fx" : "trim");
    void op({ op: "remove", id }, `remove ${blockName(t.chain, id)}`).then(() => onSel(next));
  };
  const rowMenu = (id: string): MenuItem[] => {
    const i = blocks.findIndex((b) => b.id === id);
    const b = blocks[i];
    const name = blockName(t.chain, id);
    const full = blocks.length >= cap || blocks.filter((x) => x.fx.type === b.fx.type).length >= 2;
    return [
      { header: name },
      { label: "Move up", kbd: "⌥↑", disabled: i === 0, onSelect: () => move(id, -1) },
      { label: "Move down", kbd: "⌥↓", disabled: i === blocks.length - 1, onSelect: () => move(id, 1) },
      "-",
      { label: b.fx.bypass ? "Switch on" : "Switch off", onSelect: () => toggle(id) },
      { label: "Duplicate", disabled: full, onSelect: () => void op({ op: "duplicate", id }, `duplicate ${name}`).then(() => onSel(freeId(t.chain, b.fx.type))) },
      { label: "Reset to its starting values", onSelect: () => void op({ op: "reset", id }, `reset ${name}`) },
      { label: "Remove", icon: "trash", kbd: "⌫", danger: true, onSelect: () => remove(id) },
    ];
  };
  // Dragging a row by its grip (16c): a line shows where it lands; a click without moving opens the row's menu.
  const onGrip = (e: React.PointerEvent, id: string) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const grip = e.currentTarget as HTMLElement;
    const y0 = e.clientY;
    let moved = false;
    // Where the row would land, kept here: the move is sent once, outside React's state updates.
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
      landing = to;
      setDrag({ id, to });
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
        const from = blocks.findIndex((x) => x.id === id);
        const to = landing > from ? landing - 1 : landing;
        if (to !== from) void op({ op: "move", id, to }, `move ${blockName(t.chain, id)}`);
      }
      if (!moved && !cancel) menu.open(grip, rowMenu(id), blockName(t.chain, id));
    };
    const onUp = () => end(false);
    const onKey = (ev: KeyboardEvent) => ev.key === "Escape" && end(true);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
  };
  const onRowKey = (e: React.KeyboardEvent, id: string) => {
    if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      move(id, e.key === "ArrowUp" ? -1 : 1);
    } else if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      remove(id);
    }
  };
  const fixed = (key: string, name: string, value: string) => (
    <div className={`rrow tr is-fixed ${sel === key ? "on" : ""}`} onClick={() => onSel(key)} data-testid={`row-${key}`}>
      <Icon name="lock" size="sm" className="faint" />
      <button type="button" className="tabname b ell rname" aria-pressed={sel === key} onClick={(e) => { e.stopPropagation(); onSel(key); }}>{name}</button>
      <span className="dim s1 ell grow">{value}</span>
    </div>
  );
  const line = <div className="rline" aria-hidden="true" />;
  return (
    <div className="sect tch" aria-label={`${t.name} · chain`}>
      <div className="fx g2">
        <span className="b grow">{ret ? `After the ${t.ret?.type ?? "effect"}` : "Chain"}</span>
        <span className="faint s1">{drag ? `moving ${blockName(t.chain, drag.id)}` : "top to bottom"}</span>
      </div>
      <div className="col rrows" ref={rows} role="group" aria-label={`Blocks of ${t.name}`}>
        {!ret && fixed("trim", "Trim", `${formatDb(t.chain.trim_db)} dB`)}
        {blocks.map((b, i) => {
          const m = marks[i];
          const keyT = m.key ? project.tracks.find((x) => x.id === m.key) : undefined;
          const where = playsIn(t, b.id, project.sections);
          const name = blockName(t.chain, b.id);
          return (
            <div key={b.id}>
              {drag && drag.to === i && line}
              <div
                className={`rrow tr ${b.fx.bypass ? "is-off" : ""} ${sel === b.id ? "on" : ""} ${drag?.id === b.id ? "is-ghost" : ""}`}
                data-free
                data-testid={`row-${b.id}`}
                onClick={() => onSel(b.id)}
              >
                <button type="button" className="rgrip" aria-label={`${name}: move or more`} title="Drag to move · click for the menu" onPointerDown={(e) => onGrip(e, b.id)} onClick={(e) => e.stopPropagation()} onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); menu.open(e.currentTarget, rowMenu(b.id), name); } }}>
                  <Icon name="dots" size="sm" />
                </button>
                <span className={`sg ${m.state === "off" ? "" : m.state === "neutral" ? "nu" : "on"} ${keyT ? `key ${tintOf(keyT)}` : ""}`} title={m.title}>{m.sig}</span>
                <button
                  type="button"
                  className="tabname b ell rname"
                  aria-pressed={sel === b.id}
                  aria-label={`${name}, ${blockSummary(b.fx)}${b.fx.bypass ? ", off" : ""}`}
                  title={keysInText("⌥↑ ⌥↓ to move · ⌫ to remove")}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSel(b.id);
                  }}
                  onKeyDown={(e) => onRowKey(e, b.id)}
                >
                  {name}
                </button>
                <span className="dim s1 ell grow">{blockSummary(b.fx)}</span>
                {where && <span className="tag" title={`Plays in: ${where}`}>{where}</span>}
                <button
                  type="button"
                  className={`led ${b.fx.bypass ? "off" : ""}`}
                  aria-label={`${name} ${b.fx.bypass ? "off" : "on"}`}
                  aria-pressed={!b.fx.bypass}
                  title={b.fx.bypass ? "Switch on" : "Switch off"}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggle(b.id);
                  }}
                />
              </div>
            </div>
          );
        })}
        {drag && drag.to >= blocks.length && line}
        <button
          type="button"
          ref={addBtn}
          className={`radd tr ${adding ? "on" : ""}`}
          aria-expanded={adding}
          disabled={blocks.length >= cap && !adding}
          onClick={() => setAdding((a) => !a)}
          data-testid="add-block"
        >
          <Icon name="plus" size="sm" />
          Add block
          <span className="faint s1 ml-auto">
            {blocks.length} of {cap}
          </span>
        </button>
        {fixed("fader", "Fader · Pan", `${formatDb(t.chain.fader_db)} dB · ${panText(t.chain.pan)}`)}
      </div>
      {adding && addBtn.current && <AddMenu t={t} anchor={addBtn.current} onPick={add} onClose={() => setAdding(false)} />}
      {menu.node}
    </div>
  );
}
