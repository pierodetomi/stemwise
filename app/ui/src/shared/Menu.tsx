// Menus of the design (C2): context menus, pickers. Opens at a point, closes on Esc, outside click or choice;
// arrows move between items, Enter chooses, → opens a submenu (UI-7).
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { keysInText } from "./keys";
import { createPortal } from "react-dom";
import { Icon, type IconName } from "./Icon";

export type MenuItem =
  | "-"
  | { header: string }
  | {
      label: string;
      icon?: IconName;
      kbd?: string;
      /** A note at the right (9s: where a destination sits); `warn` makes it amber with the triangle. */
      meta?: { text: string; warn?: boolean };
      danger?: boolean;
      checked?: boolean;
      disabled?: boolean;
      sub?: MenuItem[];
      onSelect?: () => void;
      custom?: ReactNode;
    };

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
  label: string;
}

export function Menu({ menu, onClose }: { menu: MenuState; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: menu.x, y: menu.y });
  const [sub, setSub] = useState<{ index: number; items: MenuItem[]; x: number; y: number } | null>(null);
  const [focus, setFocus] = useState(-1);
  const actionable = menu.items.map((it, i) => (typeof it === "object" && "label" in it && !it.disabled ? i : -1)).filter((i) => i >= 0);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const x = Math.min(menu.x, window.innerWidth - r.width - 8);
    const y = Math.min(menu.y, window.innerHeight - r.height - 8);
    setPos({ x: Math.max(8, x), y: Math.max(8, y) });
    el.focus();
  }, [menu.x, menu.y]);

  useEffect(() => {
    const down = (e: PointerEvent) => {
      if (!(e.target as HTMLElement).closest?.(".menu")) onClose();
    };
    window.addEventListener("pointerdown", down, true);
    return () => window.removeEventListener("pointerdown", down, true);
  }, [onClose]);

  const choose = (i: number, el?: HTMLElement) => {
    const it = menu.items[i];
    if (typeof it !== "object" || !("label" in it) || it.disabled) return;
    if (it.sub) {
      const r = (el ?? ref.current?.children[i] as HTMLElement | undefined)?.getBoundingClientRect();
      setSub({ index: i, items: it.sub, x: (r?.right ?? pos.x) + 2, y: r?.top ?? pos.y });
      return;
    }
    it.onSelect?.();
    onClose();
  };

  return createPortal(
    <>
      <div
        ref={ref}
        className="menu sw-pop"
        role="menu"
        aria-label={menu.label}
        tabIndex={-1}
        style={{ left: pos.x, top: pos.y, position: "fixed" }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            onClose();
          } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            const k = actionable.indexOf(focus);
            const n = e.key === "ArrowDown" ? (k + 1) % actionable.length : (k - 1 + actionable.length) % actionable.length;
            setFocus(actionable[n] ?? -1);
          } else if ((e.key === "Enter" || e.key === "ArrowRight") && focus >= 0) {
            e.preventDefault();
            choose(focus);
          }
        }}
      >
        {menu.items.map((it, i) => {
          if (it === "-") return <div key={i} className="ms" />;
          if ("header" in it) return <div key={i} className="mh">{it.header}</div>;
          if (it.custom) return <div key={i} className="mi tall">{it.custom}</div>;
          return (
            <button
              type="button"
              key={i}
              role="menuitem"
              className={`mi ${it.danger ? "danger" : ""} ${it.checked ? "on" : ""} ${focus === i || sub?.index === i ? "is-hover" : ""} ${it.disabled ? "is-dis" : ""}`}
              aria-checked={it.checked}
              aria-haspopup={it.sub ? "menu" : undefined}
              disabled={it.disabled}
              onPointerEnter={(e) => {
                setFocus(i);
                if (it.sub) choose(i, e.currentTarget);
                else setSub(null);
              }}
              onClick={(e) => choose(i, e.currentTarget)}
            >
              {it.checked !== undefined && <span className="chk">{it.checked && <Icon name="check" size="m" />}</span>}
              {it.icon && <Icon name={it.icon} size="m" />}
              <span className="ell">{it.label}</span>
              {it.kbd && <span className="kbd">{keysInText(it.kbd)}</span>}
              {it.meta && (
                <span className={`meta ${it.meta.warn ? "warnc" : ""}`}>
                  {it.meta.warn && <Icon name="warn" size="sm" />} {it.meta.text}
                </span>
              )}
              {it.sub && (
                <span className="ml-auto fx">
                  <Icon name="right" size="m" />
                </span>
              )}
            </button>
          );
        })}
      </div>
      {sub && <Menu menu={{ x: sub.x, y: sub.y, items: sub.items, label: menu.label }} onClose={onClose} />}
    </>,
    document.querySelector(".sw.app") ?? document.body,
  );
}

/** A menu state with an opener: `open(e, items)` at the pointer or under an element. */
export function useMenu() {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const open = (at: { clientX: number; clientY: number } | HTMLElement, items: MenuItem[], label = "Menu") => {
    if (at instanceof HTMLElement) {
      const r = at.getBoundingClientRect();
      setMenu({ x: r.left, y: r.bottom + 4, items, label });
    } else setMenu({ x: at.clientX, y: at.clientY, items, label });
  };
  const node = menu ? <Menu menu={menu} onClose={() => setMenu(null)} /> : null;
  return { open, node, close: () => setMenu(null) };
}
