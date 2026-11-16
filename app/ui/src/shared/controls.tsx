// Controls of the design (C1): knob, fader, numeric field, toggles, segmented control, meters. Every continuous
// control follows UI-5: vertical drag over --knob-travel, Shift for fine, wheel steps, double click for the
// default, click on the value to type it (Enter applies, Esc cancels, out-of-range text is refused), Esc during a
// drag cancels the gesture without a history step (HIS-2). Keyboard: arrows step the value (UI-7).
import { useApp } from "../state/context";
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { ParamSpec } from "../bindings/ParamSpec";
import { editText, faderPos, formatParam, fromNorm, meterPos, parseTyped, toNorm } from "./format";
import { Icon } from "./Icon";

const TRAVEL = 200; // --knob-travel
const FINE = 0.1; // --fine
const THRESHOLD = 3; // --drag-threshold

/** What a control drives: a parameter with its gesture protocol. */
export interface Ctl {
  spec: ParamSpec;
  value: number;
  label: string;
  begin(): void;
  change(v: number): void;
  end(cancel: boolean): void;
  set(v: number): void;
  auto?: boolean;
  lock?: boolean;
  disabled?: boolean;
}

/** Drag, wheel, keys, double click: returns the value to show (live while dragging) and the handlers. */
export function useControl(ctl: Ctl, axis: "y" | "x" = "y") {
  const [live, setLive] = useState<number | null>(null);
  const st = useRef<{ id: number; last: number; norm: number; dragging: boolean } | null>(null);
  const wheel = useRef<{ norm: number; timer: ReturnType<typeof setTimeout> } | null>(null);
  const el = useRef<HTMLElement | null>(null);
  const ctlRef = useRef(ctl);
  ctlRef.current = ctl;

  const shown = live ?? ctl.value;

  useEffect(() => {
    const node = el.current;
    if (!node) return;
    const onWheel = (e: WheelEvent) => {
      const c = ctlRef.current;
      if (c.disabled) return;
      e.preventDefault();
      const step = (c.spec.stepped ? 1 : 0.01) * (e.shiftKey ? FINE : 1) * (e.deltaY > 0 || e.deltaX > 0 ? -1 : 1);
      if (!wheel.current) {
        c.begin();
        wheel.current = { norm: toNorm(c.spec, c.value), timer: setTimeout(() => {}, 0) };
      }
      const w = wheel.current;
      clearTimeout(w.timer);
      w.norm = Math.min(1, Math.max(0, w.norm + step));
      const v = fromNorm(c.spec, w.norm);
      setLive(v);
      c.change(v);
      w.timer = setTimeout(() => {
        wheel.current = null;
        setLive(null);
        ctlRef.current.end(false);
      }, 400);
    };
    node.addEventListener("wheel", onWheel, { passive: false });
    return () => node.removeEventListener("wheel", onWheel);
  }, []);

  const cancel = () => {
    if (st.current?.dragging) {
      st.current = null;
      setLive(null);
      ctlRef.current.end(true);
    }
  };

  const handlers = {
    ref: (n: HTMLElement | null) => {
      el.current = n;
    },
    onPointerDown: (e: React.PointerEvent) => {
      if (ctl.disabled || e.button !== 0) return;
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
      st.current = { id: e.pointerId, last: axis === "y" ? e.clientY : e.clientX, norm: toNorm(ctl.spec, ctl.value), dragging: false };
      e.stopPropagation();
    },
    onPointerMove: (e: React.PointerEvent) => {
      const s = st.current;
      if (!s || s.id !== e.pointerId) return;
      const pos = axis === "y" ? e.clientY : e.clientX;
      const d = axis === "y" ? s.last - pos : pos - s.last;
      if (!s.dragging) {
        if (Math.abs(d) < THRESHOLD) return;
        s.dragging = true;
        ctl.begin();
      }
      s.last = pos;
      s.norm = Math.min(1, Math.max(0, s.norm + (d / TRAVEL) * (e.shiftKey ? FINE : 1)));
      const v = fromNorm(ctl.spec, s.norm);
      setLive(v);
      ctl.change(v);
    },
    onPointerUp: (e: React.PointerEvent) => {
      const s = st.current;
      if (!s || s.id !== e.pointerId) return;
      st.current = null;
      if (s.dragging) {
        setLive(null);
        ctl.end(false);
      }
    },
    onLostPointerCapture: () => cancel(),
    onDoubleClick: () => {
      if (!ctl.disabled) ctl.set(ctl.spec.default);
    },
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancel();
        return;
      }
      const dir = e.key === "ArrowUp" || e.key === "ArrowRight" ? 1 : e.key === "ArrowDown" || e.key === "ArrowLeft" ? -1 : 0;
      if (!dir || ctl.disabled) return;
      e.preventDefault();
      const step = (ctl.spec.stepped ? 1 : 0.02) * (e.shiftKey ? FINE : 1) * dir;
      ctl.set(fromNorm(ctl.spec, toNorm(ctl.spec, ctl.value) + step));
    },
  };
  return { shown, handlers, dragging: live !== null };
}

/** The value text of a control; click to type it (UI-5). */
/** The number of a value without its unit (knobs and strips of the design: "−18.0", "120"). */
export function bareValue(spec: Ctl["spec"], v: number): string {
  const t = formatParam(spec, v);
  if (spec.unit === "ratio" || spec.unit === "pan") return t.replace(":1", "");
  const sp = t.lastIndexOf(" ");
  return sp > 0 && spec.unit !== "hz" ? t.slice(0, sp) : t;
}

export function ValueText({ ctl, shown, className, bare }: { ctl: Ctl; shown: number; className: string; bare?: boolean }) {
  const [edit, setEdit] = useState<string | null>(null);
  const [err, setErr] = useState(false);
  if (edit !== null) {
    return (
      <input
        className={`${className} vt-edit ${err ? "is-err" : ""}`}
        autoFocus
        aria-label={`${ctl.label} value`}
        value={edit}
        onChange={(e) => {
          setEdit(e.target.value);
          setErr(false);
        }}
        onBlur={() => setEdit(null)}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Escape") setEdit(null);
          if (e.key === "Enter") {
            const v = parseTyped(ctl.spec, edit);
            if (v === null) setErr(true);
            else {
              ctl.set(v);
              setEdit(null);
            }
          }
        }}
      />
    );
  }
  return (
    <span
      className={className}
      onClick={(e) => {
        e.stopPropagation();
        if (!ctl.disabled) setEdit(editText(ctl.spec, shown));
      }}
    >
      {bare ? bareValue(ctl.spec, shown) : formatParam(ctl.spec, shown)}
    </span>
  );
}

function sliderAria(ctl: Ctl, shown: number) {
  return {
    role: "slider",
    tabIndex: ctl.disabled ? -1 : 0,
    "aria-label": ctl.label,
    "aria-valuemin": ctl.spec.min,
    "aria-valuemax": ctl.spec.max,
    "aria-valuenow": Math.round(shown * 100) / 100,
    "aria-valuetext": formatParam(ctl.spec, shown),
    "aria-disabled": ctl.disabled || undefined,
  } as const;
}

// ---------- knob ----------
function arc(s: number, e: number) {
  const c = 20;
  const r = 17;
  if (e < s) [s, e] = [e, s];
  if (e - s < 0.03) e = s + 0.03;
  const p = [c + r * Math.cos(s), c + r * Math.sin(s)];
  const q = [c + r * Math.cos(e), c + r * Math.sin(e)];
  return `M${p[0].toFixed(1)} ${p[1].toFixed(1)}A${r} ${r} 0 ${e - s > Math.PI ? 1 : 0} 1 ${q[0].toFixed(1)} ${q[1].toFixed(1)}`;
}

export function KnobSvg({ v, size = "", bip = false }: { v: number; size?: "" | "s" | "l"; bip?: boolean }) {
  const a0 = Math.PI * 0.75;
  const a1 = Math.PI * 2.25;
  const av = a0 + (a1 - a0) * v;
  const am = (a0 + a1) / 2;
  const ri = 17 - 4 - 2;
  return (
    <svg className={`knob ${size}`} viewBox="0 0 40 40" aria-hidden="true">
      <circle className="k-body" cx="20" cy="20" r={ri} />
      <path className="k-bg" d={arc(a0, a1)} />
      <path className="k-val" d={arc(bip ? am : a0, av)} />
      <path
        className="k-ptr"
        d={`M${(20 + ri * 0.3 * Math.cos(av)).toFixed(1)} ${(20 + ri * 0.3 * Math.sin(av)).toFixed(1)}L${(20 + (ri - 1) * Math.cos(av)).toFixed(1)} ${(20 + (ri - 1) * Math.sin(av)).toFixed(1)}`}
      />
    </svg>
  );
}

export function Knob({ ctl, name, size = "", bip, valueless, bare }: { ctl: Ctl; name?: string; size?: "" | "s" | "l"; bip?: boolean; valueless?: boolean; bare?: boolean }) {
  const { shown, handlers, dragging } = useControl(ctl);
  const cls = `kb ${ctl.auto ? "is-auto" : ""} ${ctl.lock ? "is-lock" : ""} ${dragging ? "is-active" : ""} ${ctl.disabled ? "is-dis" : ""}`;
  return (
    <div className={cls}>
      <div className="kb-hit" {...handlers} {...sliderAria(ctl, shown)}>
        <KnobSvg v={toNorm(ctl.spec, shown)} size={size} bip={bip ?? (ctl.spec.unit === "pan" || ctl.spec.min < 0 && ctl.spec.max > 0 && ctl.spec.unit !== "db")} />
      </div>
      {!valueless && <ValueText ctl={ctl} shown={shown} className="kv" bare={bare} />}
      {name && <span className="kl">{name}</span>}
    </div>
  );
}

// ---------- fader ----------
export function Fader({ ctl, className = "" }: { ctl: Ctl; className?: string }) {
  const { shown, handlers, dragging } = useControl(ctl);
  const cls = `fader ${ctl.auto ? "is-auto" : ""} ${ctl.lock ? "is-lock" : ""} ${dragging ? "is-active" : ""} ${className}`;
  return (
    <div className={cls} {...handlers} {...sliderAria(ctl, shown)}>
      <div className="cap" style={{ bottom: `${faderPos(shown) * 100}%` }} />
    </div>
  );
}

/** Horizontal mini fader of a track header (drag left/right). */
export function MiniFader({ ctl }: { ctl: Ctl }) {
  const { shown, handlers } = useControl(ctl, "x");
  return (
    <div className="hbar mini" {...handlers} {...sliderAria(ctl, shown)}>
      <i style={{ width: `${faderPos(shown) * 100}%` }} />
    </div>
  );
}

/** Pan as a short horizontal bar (strip). */
export function PanKnob({ ctl }: { ctl: Ctl }) {
  const { shown, handlers } = useControl(ctl);
  return (
    <div className="kb-hit" {...handlers} {...sliderAria(ctl, shown)} title={formatParam(ctl.spec, shown)}>
      <KnobSvg v={toNorm(ctl.spec, shown)} size="s" bip />
    </div>
  );
}

// ---------- numeric field ----------
export function Field({ ctl, className = "" }: { ctl: Ctl; className?: string }) {
  const { shown, handlers } = useControl(ctl);
  const text = formatParam(ctl.spec, shown);
  const sp = text.lastIndexOf(" ");
  const unit = sp > 0 ? text.slice(sp + 1) : "";
  const num = sp > 0 ? text.slice(0, sp) : text;
  const [edit, setEdit] = useState<string | null>(null);
  const [err, setErr] = useState(false);
  return (
    <span
      className={`field ${ctl.auto ? "is-auto" : ""} ${ctl.lock ? "is-lock" : ""} ${edit !== null ? "is-edit" : ""} ${err ? "is-err" : ""} ${className}`}
      {...handlers}
      {...sliderAria(ctl, shown)}
      onClick={() => edit === null && !ctl.disabled && setEdit(editText(ctl.spec, shown))}
    >
      {edit === null ? (
        <span className="v">{num}</span>
      ) : (
        <input
          className="v vt-edit"
          autoFocus
          aria-label={`${ctl.label} value`}
          value={edit}
          onPointerDown={(e) => e.stopPropagation()}
          onChange={(e) => {
            setEdit(e.target.value);
            setErr(false);
          }}
          onBlur={() => {
            setEdit(null);
            setErr(false);
          }}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Escape") setEdit(null);
            if (e.key === "Enter") {
              const v = parseTyped(ctl.spec, edit);
              if (v === null) setErr(true);
              else {
                ctl.set(v);
                setEdit(null);
              }
            }
          }}
        />
      )}
      <span className="u">{unit}</span>
    </span>
  );
}

// ---------- toggles, segmented control, buttons ----------
const TGL = { mu: ["M", "Mute"], so: ["S", "Solo"], lk: [null, "Lock for AI"], by: [null, "Bypass"] } as const;

export function Toggle({
  kind, on, onClick, implied, small, label, disabled,
}: { kind: keyof typeof TGL; on: boolean; onClick: () => void; implied?: boolean; small?: boolean; label?: string; disabled?: boolean }) {
  const [txt, aria] = TGL[kind];
  return (
    <button
      type="button"
      className={`tgl ${kind} ${on ? "on" : ""} ${implied ? "implied" : ""} ${small ? "sm" : ""}`}
      aria-label={label ?? aria}
      aria-pressed={on}
      disabled={disabled}
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
    >
      {kind === "lk" ? <Icon name="lock" /> : kind === "by" ? <Icon name="power" /> : txt}
    </button>
  );
}

export function Seg<T extends string | number>({
  items, value, onChange, className = "", label,
}: { items: { value: T; label: ReactNode; aria?: string; title?: string; className?: string }[]; value: T; onChange: (v: T) => void; className?: string; label: string }) {
  return (
    <div className={`seg ${className}`} role="radiogroup" aria-label={label}>
      {items.map((it) => (
        <button
          type="button"
          key={String(it.value)}
          role="radio"
          aria-checked={it.value === value}
          aria-label={it.aria}
          title={it.title}
          className={`${it.value === value ? "on" : ""} ${it.className ?? ""}`.trim() || undefined}
          onClick={() => onChange(it.value)}
        >
          {it.label}
        </button>
      ))}
    </div>
  );
}

export function IconButton({ icon, label, onClick, on, disabled, className = "" }: { icon: Parameters<typeof Icon>[0]["name"]; label: string; onClick: () => void; on?: boolean; disabled?: boolean; className?: string }) {
  return (
    <button type="button" className={`ib ${on ? "on" : ""} ${className}`} aria-label={label} title={label} aria-pressed={on} disabled={disabled} onClick={onClick}>
      <Icon name={icon} />
    </button>
  );
}

// ---------- meters ----------
/** Ballistics of the design: rises at once, falls 20 dB/s, peak held 2 s (drawing only). */
export function useBallistics(level: number, peak: number, hold = true) {
  const st = useRef({ lv: -200, pk: -200, hold: 0, t: 0 });
  const now = performance.now();
  const s = st.current;
  const dt = s.t ? (now - s.t) / 1000 : 0;
  s.t = now;
  s.lv = Math.max(level, s.lv - 20 * dt);
  if (peak >= s.pk || now > s.hold) {
    s.pk = Math.max(peak, now > s.hold ? s.pk - 20 * dt : s.pk);
    if (peak >= s.pk) s.hold = now + (hold ? 2000 : 0);
  }
  return { level: s.lv, peak: Math.max(s.pk, s.lv) };
}

export function Meter({ rms, peak, className = "" }: { rms: number; peak: number; className?: string }) {
  const hold = useApp((s) => s.prefs.holdPeaks);
  const b = useBallistics(rms, peak, hold);
  return (
    <div className={`meter ${peak > -0.1 ? "over" : ""} ${className}`} aria-hidden="true">
      <i className="pk" style={{ height: `${meterPos(b.peak) * 100}%` }} />
      <i className="rms" style={{ height: `${meterPos(b.level) * 100}%` }} />
    </div>
  );
}

/** Gain reduction, top-down, 0…24 dB. */
export function GrMeter({ gr }: { gr: number }) {
  return (
    <div className="meter gr" aria-hidden="true">
      <i style={{ height: `${Math.min(100, (Math.abs(gr) / 24) * 100)}%` }} />
    </div>
  );
}
