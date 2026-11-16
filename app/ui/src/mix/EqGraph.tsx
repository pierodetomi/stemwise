// Graphic EQ editor (FX-7, design 3a "drag a handle · scroll for Q"): the response curve of the track's filters and
// bands, its live spectrum, and one handle per band. Handles move in frequency and gain; the wheel changes Q; a
// double click resets the band's gain. The curve is computed for drawing only (RBJ magnitude, FOUNDATION §5.6).
import { useLayoutEffect, useRef, useState } from "react";
import type { EqBand } from "../bindings/EqBand";
import type { Track } from "../bindings/Track";
import { useApp, useBackend } from "../state/context";

const RANGE = 18;
const FMIN = 20;
const FMAX = 20000;

export function biquad(kind: string, f0: number, gainDb: number, q: number, rate: number): number[] {
  const A = Math.pow(10, gainDb / 40);
  const w0 = (2 * Math.PI * Math.min(f0, rate * 0.49)) / rate;
  const cs = Math.cos(w0);
  const sn = Math.sin(w0);
  const alpha = sn / (2 * Math.max(0.05, q));
  switch (kind) {
    case "low_shelf": {
      const k = 2 * Math.sqrt(A) * alpha;
      return [A * (A + 1 - (A - 1) * cs + k), 2 * A * (A - 1 - (A + 1) * cs), A * (A + 1 - (A - 1) * cs - k), A + 1 + (A - 1) * cs + k, -2 * (A - 1 + (A + 1) * cs), A + 1 + (A - 1) * cs - k];
    }
    case "high_shelf": {
      const k = 2 * Math.sqrt(A) * alpha;
      return [A * (A + 1 + (A - 1) * cs + k), -2 * A * (A - 1 + (A + 1) * cs), A * (A + 1 + (A - 1) * cs - k), A + 1 - (A - 1) * cs + k, 2 * (A - 1 - (A + 1) * cs), A + 1 - (A - 1) * cs - k];
    }
    case "hpf":
      return [(1 + cs) / 2, -(1 + cs), (1 + cs) / 2, 1 + alpha, -2 * cs, 1 - alpha];
    case "lpf":
      return [(1 - cs) / 2, 1 - cs, (1 - cs) / 2, 1 + alpha, -2 * cs, 1 - alpha];
    default:
      return [1 + alpha * A, -2 * cs, 1 - alpha * A, 1 + alpha / A, -2 * cs, 1 - alpha / A];
  }
}

export function magDb(c: number[], f: number, rate: number): number {
  const w = (2 * Math.PI * f) / rate;
  const re = (a: number, b: number, d: number) => a + b * Math.cos(w) + d * Math.cos(2 * w);
  const im = (b: number, d: number) => -(b * Math.sin(w) + d * Math.sin(2 * w));
  const nr = re(c[0], c[1], c[2]);
  const ni = im(c[1], c[2]);
  const dr = re(c[3], c[4], c[5]);
  const di = im(c[4], c[5]);
  return 10 * Math.log10((nr * nr + ni * ni) / (dr * dr + di * di) + 1e-30);
}

/** Total response of a chain's filters and the enabled bands of its EQs at `f` (dB). */
export function response(t: Track, f: number, rate: number): number {
  let d = 0;
  for (const { fx } of t.chain.blocks) {
    if (fx.bypass) continue;
    if (fx.type === "hpf" || fx.type === "lpf") d += magDb(biquad(fx.type, fx.freq_hz, 0, fx.resonance, rate), f, rate);
    else if (fx.type === "eq") for (const b of fx.bands) if (b.enabled) d += magDb(biquad(b.kind, b.freq_hz, b.gain_db, b.q, rate), f, rate);
  }
  return d;
}

/** The bands of an EQ block (none if the block is not an EQ). */
export function eqBands(t: Track, block: string): EqBand[] {
  const b = t.chain.blocks.find((x) => x.id === block);
  return b?.fx.type === "eq" ? b.fx.bands : [];
}

const fx = (f: number, w: number) => (Math.log(f / FMIN) / Math.log(FMAX / FMIN)) * w;
const xf = (x: number, w: number) => FMIN * Math.pow(FMAX / FMIN, Math.min(1, Math.max(0, x / w)));

export function EqGraph({ t, block, height, selBand, onSelBand }: { t: Track; block: string; height: number; selBand: number; onSelBand: (b: number) => void }) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const rate = useApp((s) => s.project?.sample_rate ?? 48000);
  const spectrum = useApp((s) => s.meters?.spectrum ?? null);
  const ref = useRef<HTMLDivElement>(null);
  const [w, setW] = useState(280);
  const [live, setLive] = useState<{ band: number; b: EqBand } | null>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setW(el.clientWidth));
    ro.observe(el);
    setW(el.clientWidth);
    return () => ro.disconnect();
  }, []);
  const h = height;
  const mid = h / 2;
  const y = (db: number) => Math.max(1, Math.min(h - 1, mid - (db / RANGE) * mid));
  const bands = eqBands(t, block);
  const off = t.chain.blocks.find((x) => x.id === block)?.fx.bypass ?? true;
  const shownBands = live ? bands.map((b, i) => (i === live.band ? live.b : b)) : bands;
  const shown: Track = live
    ? { ...t, chain: { ...t.chain, blocks: t.chain.blocks.map((x) => (x.id === block && x.fx.type === "eq" ? { ...x, fx: { ...x.fx, bands: shownBands } } : x)) } }
    : t;
  const pts: string[] = [];
  for (let x = 0; x <= w; x += 2) pts.push(`${x} ${y(response(shown, xf(x, w), rate)).toFixed(1)}`);
  const line = "M" + pts.join("L");
  const grid = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
  let spec = "";
  if (spectrum) {
    const n = spectrum.length;
    const sp = spectrum.map((db, k) => `${((k + 0.5) / n * w).toFixed(1)} ${(h - Math.max(0, Math.min(1, (db + 90) / 84)) * h).toFixed(1)}`);
    spec = `M0 ${h}L${sp.join("L")}L${w} ${h}Z`;
  }
  const fail = (e: unknown) => setError(String(e));
  const label = (i: number) => `${t.name} · EQ band ${i + 1}`;
  const pid = (i: number, k: string) => `${block}.${i + 1}.${k}`;

  const onHandle = (e: React.PointerEvent, i: number) => {
    e.stopPropagation();
    if (e.button !== 0) return;
    onSelBand(i);
    const el = ref.current!;
    const r = el.getBoundingClientRect();
    const start = bands[i];
    let began = false;
    // Calls of one gesture go out in order.
    let q: Promise<unknown> = Promise.resolve();
    const send = (f: () => Promise<unknown>) => {
      q = q.then(f).catch(fail);
    };
    const x0 = e.clientX;
    const y0 = e.clientY;
    const move = (ev: PointerEvent) => {
      if (!began) {
        if (Math.abs(ev.clientX - x0) < 3 && Math.abs(ev.clientY - y0) < 3) return;
        began = true;
        send(() => backend.beginGesture(label(i)));
        // Dragging a band that is off switches it on, inside the same step.
        if (!start.enabled) send(() => backend.gestureEdit({ op: "set_eq_band", track: t.id, band: i, enabled: true, kind: start.kind, block }));
      }
      const fine = ev.shiftKey ? 0.1 : 1;
      const fq = xf(fx(start.freq_hz, r.width) + (ev.clientX - x0) * fine, r.width);
      const g = Math.max(-RANGE, Math.min(RANGE, start.gain_db - ((ev.clientY - y0) * fine * RANGE) / mid));
      const b = { ...start, enabled: true, freq_hz: Math.round(fq * 10) / 10, gain_db: Math.round(g * 10) / 10 };
      setLive({ band: i, b });
      send(() => backend.gestureParam(t.id, pid(i, "freq"), b.freq_hz));
      send(() => backend.gestureParam(t.id, pid(i, "gain"), b.gain_db));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("keydown", esc);
      setLive(null);
      if (began) send(() => backend.endGesture(false));
    };
    const esc = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("keydown", esc);
      setLive(null);
      if (began) send(() => backend.endGesture(true));
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("keydown", esc);
  };

  const onWheel = (e: React.WheelEvent, i: number) => {
    const b = bands[i];
    const q = Math.max(0.1, Math.min(10, b.q * (e.deltaY < 0 ? 1.1 : 1 / 1.1)));
    void backend.setParam(t.id, pid(i, "q"), Math.round(q * 100) / 100, `${label(i)} width`).catch(fail);
  };

  return (
    <div className={`eq ${off ? "is-off" : ""}`} ref={ref} style={{ height: h }} aria-label={`EQ of ${t.name}`} role="group">
      <svg width={w} height={h} aria-hidden="true">
        <path className="gd" d={grid.map((f) => `M${fx(f, w).toFixed(1)} 0V${h}`).join("")} />
        {spec && <path className="sp" d={spec} />}
        <path className="zr" d={`M0 ${mid}H${w}`} />
        <path className="ar" d={`${line}L${w} ${mid}L0 ${mid}Z`} />
        <path className="cv" d={line} />
      </svg>
      {shownBands.map((b, i) => (
        <i
          key={i}
          role="slider"
          tabIndex={0}
          aria-label={`${label(i)} (${b.enabled ? "on" : "off"})`}
          aria-valuenow={Math.round(b.gain_db * 10) / 10}
          aria-valuemin={-18}
          aria-valuemax={18}
          aria-valuetext={`${Math.round(b.freq_hz)} Hz, ${b.gain_db.toFixed(1)} dB`}
          className={`dot ${selBand === i ? "is-sel" : ""} ${b.enabled ? "" : "is-dis"}`}
          style={{ left: fx(b.freq_hz, w), top: y(b.enabled ? b.gain_db : 0) }}
          onPointerDown={(e) => onHandle(e, i)}
          onWheel={(e) => onWheel(e, i)}
          onDoubleClick={() => void backend.setParam(t.id, pid(i, "gain"), 0, `${label(i)} reset`).catch(fail)}
          onKeyDown={(e) => {
            const step = e.shiftKey ? 0.1 : 0.5;
            if (e.key === "ArrowUp" || e.key === "ArrowDown") {
              e.preventDefault();
              void backend.setParam(t.id, pid(i, "gain"), Math.max(-18, Math.min(18, b.gain_db + (e.key === "ArrowUp" ? step : -step))), `${label(i)} gain`).catch(fail);
            } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
              e.preventDefault();
              void backend.setParam(t.id, pid(i, "freq"), Math.max(20, Math.min(20000, b.freq_hz * (e.key === "ArrowRight" ? 1.06 : 1 / 1.06))), `${label(i)} frequency`).catch(fail);
            }
          }}
        />
      ))}
      {grid.map((f) => (
        <span key={f} className="ft" style={{ left: fx(f, w) }}>
          {f >= 1000 ? `${f / 1000}k` : f}
        </span>
      ))}
    </div>
  );
}

/** Static transfer curve of the compressor (FX-7): input −48…0 dB against output. */
export function CompCurve({ threshold, ratio, knee }: { threshold: number; ratio: number; knee: number }) {
  const X = (d: number) => ((d + 48) / 48) * 100;
  const pts: string[] = [];
  for (let d = -48; d <= 0; d += 1) {
    let o = d;
    if (d > threshold + knee / 2) o = threshold + (d - threshold) / ratio;
    else if (knee > 0 && d > threshold - knee / 2) o = d + ((1 / ratio - 1) * Math.pow(d - threshold + knee / 2, 2)) / (2 * knee);
    pts.push(`${X(d).toFixed(1)} ${(100 - X(o)).toFixed(1)}`);
  }
  return (
    <div className="tc-curve" aria-hidden="true">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        <path className="un" d="M0 100L100 0" />
        <path className="th" d={`M${X(threshold).toFixed(1)} 0V100`} />
        <path className="cv" d={`M${pts.join("L")}`} />
      </svg>
    </div>
  );
}
