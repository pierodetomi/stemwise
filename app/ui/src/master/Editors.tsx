// The editors of the master chain (design 15a, 15c–15e, 15h; MAS-1, MAS-11 … MAS-14): the block selected in the rack,
// or the offsets of a section, beside the rack.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { keysInText } from "../shared/keys";
import type { BlockView } from "../bindings/BlockView";
import type { MasterStatus } from "../bindings/MasterStatus";
import { CompCurve, biquad, magDb } from "../mix/EqGraph";
import { Field, IconButton, Knob, Seg, Toggle } from "../shared/controls";
import { Icon } from "../shared/Icon";
import { useMenu } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import { MS, blockOf, useBlockCtl, useMasterCtl, useMasterEdit, withOffsets } from "./bind";
import { hz, n1 } from "./Rack";

export function useWidth(ref: React.RefObject<HTMLElement | null>, fallback = 600) {
  const [w, setW] = useState(fallback);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setW(Math.max(120, Math.floor(el.clientWidth))));
    ro.observe(el);
    setW(Math.max(120, Math.floor(el.clientWidth)));
    return () => ro.disconnect();
  }, [ref]);
  return w;
}

const KIND: Record<string, string> = { low_shelf: "Low shelf", peak: "Peak", high_shelf: "High shelf" };
export const fx = (f: number, w: number) => (Math.log(f / 20) / Math.log(1000)) * w;
const xf = (x: number, w: number) => 20 * Math.pow(1000, Math.min(1, Math.max(0, x / w)));
const TICKS = [50, 100, 200, 500, 1000, 2000, 5000, 10000];

/** The references' 8-band profile as a dashed line, aligned with the live spectrum's level (drawing only). */
export function profileLine(bands: number[] | null | undefined, spectrum: number[] | null, w: number, h: number): string {
  if (!bands) return "";
  const centres = [35, 85, 175, 350, 1000, 2800, 5600, 12600];
  const mean = spectrum ? spectrum.reduce((a, v) => a + v, 0) / spectrum.length : -40;
  const tmean = bands.reduce((a, v) => a + v, 0) / 8;
  return "M" + centres.map((f, i) => `${fx(f, w).toFixed(1)} ${(h - Math.max(0, Math.min(1, (bands[i] - tmean + mean + 90) / 84)) * h).toFixed(1)}`).join("L");
}

export function spectrumPath(spectrum: number[] | null, w: number, h: number): string {
  if (!spectrum) return "";
  const n = spectrum.length;
  return `M0 ${h}L${spectrum.map((db, k) => `${(((k + 0.5) / n) * w).toFixed(1)} ${(h - Math.max(0, Math.min(1, (db + 90) / 84)) * h).toFixed(1)}`).join("L")}L${w} ${h}Z`;
}

function GrRow({ label, db }: { label: string; db: number }) {
  return (
    <div className="fx g2">
      {label && <span className="faint s1 grl">{label}</span>}
      <div className="hbar gr grow" aria-hidden="true"><i style={{ width: `${Math.min(100, Math.max(0, -db) * 10)}%` }} /></div>
      <span className="b s1 grv">{n1(db)} dB</span>
    </div>
  );
}

function Info({ children }: { children: React.ReactNode }) {
  return (
    <div className="tile row">
      <Icon name="info" />
      <span className="dim s1">{children}</span>
    </div>
  );
}

/** The header every block editor shares: its switch and its name. */
function Head({ v, on, sub, children }: { v: BlockView; on: boolean; sub?: string; children?: React.ReactNode }) {
  const edit = useMasterEdit();
  return (
    <div className="ph">
      <Toggle kind="by" on={on} label={`${v.name} on`} onClick={() => edit(`Master · ${v.name} ${on ? "off" : "on"}`, (x) => { const b = x.chain.blocks.find((y) => y.id === v.id); if (b) b.block.bypass = on; })} />
      <span className="h">{v.name}</span>
      {sub && <span className="faint s1 grow ell">{sub}</span>}
      {!sub && <div className="grow" />}
      {children}
    </div>
  );
}

// ---------- EQ ----------
function EqEditor({ v, ms }: { v: BlockView; ms: MasterStatus }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const spectrum = useApp((s) => s.meters?.spectrum ?? null);
  const setError = useApp((s) => s.setError);
  const ref = useRef<HTMLDivElement>(null);
  const w = useWidth(ref, 440);
  const [band, setBand] = useState(0);
  const [range, setRange] = useState(6);
  const [live, setLive] = useState<{ i: number; f: number; g: number } | null>(null);
  const m = project.master;
  const eq = blockOf(m, v.id, "eq")!;
  const rate = project.sample_rate ?? 48000;
  const h = 190;
  const mid = h / 2;
  const bands = eq.bands.map((b, i) => (live && live.i === i ? { ...b, enabled: true, freq_hz: live.f, gain_db: live.g } : b));
  const y = (db: number) => Math.max(1, Math.min(h - 1, mid - (db / range) * mid));
  const pts: string[] = [];
  for (let x = 0; x <= w; x += 3) {
    const f = xf(x, w);
    let d = 0;
    if (!eq.bypass) for (const b of bands) if (b.enabled) d += magDb(biquad(b.kind, b.freq_hz, b.gain_db, b.q, rate), f, rate);
    pts.push(`${x} ${y(d).toFixed(1)}`);
  }
  const line = "M" + pts.join("L");
  const spec = spectrumPath(spectrum, w, h);
  const refLine = profileLine(ms.target_bands, spectrum, w, h);
  const sel = eq.bands[band];
  const name = v.name;
  const fCtl = useBlockCtl(MS.eqFreq, `${name} ${band + 1} frequency`, v.id, "eq", (b) => b.bands[band].freq_hz, (b, x) => (b.bands[band].freq_hz = x));
  const gCtl = useBlockCtl(MS.eqGain, `${name} ${band + 1} gain`, v.id, "eq", (b) => b.bands[band].gain_db, (b, x) => { b.bands[band].gain_db = x; b.bands[band].enabled = true; });
  const qCtl = useBlockCtl(MS.eqQ, `${name} ${band + 1} Q`, v.id, "eq", (b) => b.bands[band].q, (b, x) => (b.bands[band].q = x));
  const edit = useMasterEdit();
  const menu = useMenu();
  const onBand = (label: string, f: (b: NonNullable<typeof eq>) => void) =>
    edit(label, (x) => {
      const b = blockOf(x, v.id, "eq");
      if (b) f(b);
    });
  const onHandle = (e: React.PointerEvent, i: number) => {
    e.stopPropagation();
    setBand(i);
    const r = ref.current!.getBoundingClientRect();
    const start = eq.bands[i];
    const x0 = e.clientX;
    const y0 = e.clientY;
    let began = false;
    let q: Promise<unknown> = Promise.resolve();
    const send = (f: () => Promise<unknown>) => (q = q.then(f).catch((x) => setError(String(x))));
    const move = (ev: PointerEvent) => {
      if (!began) {
        if (Math.abs(ev.clientX - x0) < 3 && Math.abs(ev.clientY - y0) < 3) return;
        began = true;
        send(() => backend.beginGesture(`Master · ${name} ${i + 1}`, "master"));
      }
      const fine = ev.shiftKey ? 0.1 : 1;
      const f = Math.round(xf(fx(start.freq_hz, r.width) + (ev.clientX - x0) * fine, r.width));
      const g = Math.round(Math.max(-12, Math.min(12, start.gain_db - ((ev.clientY - y0) * fine * range) / mid)) * 10) / 10;
      setLive({ i, f, g });
      const next = structuredClone(m);
      const nb = blockOf(next, v.id, "eq");
      if (!nb) return;
      nb.bypass = false;
      nb.bands[i] = { ...start, enabled: true, freq_hz: f, gain_db: g };
      send(() => backend.gestureEdit({ op: "set_master", master: next }));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setLive(null);
      if (began) send(() => backend.endGesture(false));
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  return (
    <section className="card grow" aria-label={name}>
      <Head v={v} on={!eq.bypass} sub="dashed: the references · grey: the live spectrum">
        <Seg className="quiet sm" label="Range" value={range} items={[{ value: 6, label: "±6 dB" }, { value: 12, label: "±12 dB" }]} onChange={setRange} />
      </Head>
      <div className="sect grow">
        <div className="eq" ref={ref} style={{ height: h }}>
          <svg width={w} height={h} aria-hidden="true">
            <path className="gd" d={TICKS.map((f) => `M${fx(f, w).toFixed(1)} 0V${h}`).join("")} />
            {spec && <path className="sp" d={spec} />}
            {refLine && <path className="rf" d={refLine} />}
            <path className="zr" d={`M0 ${mid}H${w}`} />
            <path className="ar" d={`${line}L${w} ${mid}L0 ${mid}Z`} />
            <path className="cv" d={line} />
          </svg>
          {bands.map((b, i) => (
            <i key={i} role="slider" tabIndex={0} aria-label={`Master EQ band ${i + 1}`} aria-valuenow={b.gain_db} aria-valuemin={-12} aria-valuemax={12} className={`dot ${band === i ? "is-sel" : ""} ${b.enabled ? "" : "is-dis"}`} style={{ left: fx(b.freq_hz, w), top: y(b.enabled ? b.gain_db : 0) }} onPointerDown={(e) => onHandle(e, i)} onDoubleClick={() => onBand(`Master · ${name} ${i + 1} reset`, (x) => (x.bands[i].gain_db = 0))} />
          ))}
          {TICKS.map((f) => (
            <span key={f} className="ft" style={{ left: fx(f, w) }}>{f >= 1000 ? `${f / 1000}k` : f}</span>
          ))}
        </div>
        <div className="fx g1">
          {eq.bands.map((b, i) => (
            <button type="button" key={i} className={`pill ${band === i ? "on" : ""} ${b.enabled ? "" : "is-off"}`} onClick={() => setBand(i)} aria-pressed={band === i} aria-label={`Band ${i + 1}`}>
              <span className="faint s1 ell">{b.enabled ? hz(b.freq_hz) : `Band ${i + 1}`}</span>
              <span className="b ell">{b.enabled ? `${b.gain_db > 0 ? "+" : ""}${n1(b.gain_db)} dB` : "Off"}</span>
            </button>
          ))}
        </div>
        <div className="fx g2 end-al wrap">
          <label className="flabel">
            <span className="l">Type</span>
            <button type="button" className="select" onClick={(e) => menu.open(e.currentTarget, (["low_shelf", "peak", "high_shelf"] as const).map((k) => ({ label: KIND[k], checked: sel.kind === k, onSelect: () => onBand(`Master · ${name} ${band + 1} type`, (x) => { x.bands[band].kind = k; x.bands[band].enabled = true; }) })), "Type")}>
              <span className="ell">{KIND[sel.kind]}</span>
              <Icon name="chev" size="m" />
            </button>
          </label>
          <label className="flabel"><span className="l">Frequency</span><Field ctl={fCtl} /></label>
          <label className="flabel"><span className="l">Gain</span><Field ctl={gCtl} /></label>
          <label className="flabel"><span className="l">Q</span><Field ctl={qCtl} /></label>
          <Toggle kind="by" on={sel.enabled} label={`Band ${band + 1} on`} onClick={() => onBand(`Master · ${name} ${band + 1} ${sel.enabled ? "off" : "on"}`, (x) => (x.bands[band].enabled = !sel.enabled))} />
        </div>
        <span className="faint s1">Drag a handle, double-click to reset. {keysInText("Hold ⇧ for fine steps.")}</span>
      </div>
      {menu.node}
    </section>
  );
}

// ---------- compressor ----------
function CompEditor({ v, gr }: { v: BlockView; gr: number }) {
  const m = useApp((s) => s.project)!.master;
  const c = blockOf(m, v.id, "compressor")!;
  const k = (s: (typeof MS)[keyof typeof MS], label: string, get: (b: typeof c) => number, put: (b: typeof c, x: number) => void) =>
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useBlockCtl(s, `${v.name} ${label}`, v.id, "compressor", get, put);
  const thr = k(MS.threshold, "threshold", (b) => b.threshold_db, (b, x) => (b.threshold_db = x));
  const ratio = k(MS.ratio, "ratio", (b) => b.ratio, (b, x) => (b.ratio = x));
  const att = k(MS.attack, "attack", (b) => b.attack_ms, (b, x) => (b.attack_ms = x));
  const rel = k(MS.release, "release", (b) => b.release_ms, (b, x) => (b.release_ms = x));
  const mk = k(MS.makeup, "makeup", (b) => b.makeup_db, (b, x) => (b.makeup_db = x));
  const mix = k(MS.compMix, "mix", (b) => b.mix, (b, x) => (b.mix = x));
  const knee = k(MS.knee, "knee", (b) => b.knee_db, (b, x) => (b.knee_db = x));
  const listen = k(MS.listenAbove, "listens above", (b) => b.listen_above_hz, (b, x) => (b.listen_above_hz = x));
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!c.bypass} sub="stereo-linked">
        {c.mix < 1 && <span className="tag acc"><Icon name="level" size="sm" />Parallel · {Math.round(c.mix * 100)} %</span>}
      </Head>
      <div className="sect grow">
        <div className="fx g3">
          <CompCurve threshold={c.threshold_db} ratio={c.ratio} knee={c.knee_db} />
          <div className="col g3 grow">
            <GrRow label="Reduction" db={gr} />
            <div className="fx g3 end-al">
              <label className="flabel"><span className="l">Knee</span><Field ctl={knee} /></label>
              <label className="flabel"><span className="l">Listens above</span><Field ctl={listen} /></label>
            </div>
            <span className="faint s1">
              {c.listen_above_hz > 0
                ? `It does not react to what is below ${Math.round(c.listen_above_hz)} Hz, so the low end does not make the whole mix duck.`
                : "It hears everything, the low end too. Set a frequency to keep what is below it out of the detector."}
            </span>
          </div>
        </div>
        <div className="knobs">
          <Knob ctl={thr} name="Threshold" bare />
          <Knob ctl={ratio} name="Ratio" bare />
          <Knob ctl={att} name="Attack" bare />
          <Knob ctl={rel} name="Release" bare />
          <Knob ctl={mk} name="Makeup" bare />
          <Knob ctl={mix} name="Mix" bare />
        </div>
        <Info>
          {c.mix < 1
            ? `Mix at ${Math.round(c.mix * 100)} %: the compressed signal is blended with ${Math.round((1 - c.mix) * 100)} % of the untouched one. The peaks keep their attack and the quiet details come up.`
            : "Mix under 100 % blends the compressed signal with the untouched one (parallel compression): the peaks keep their attack and the quiet details come up."}
        </Info>
      </div>
    </section>
  );
}

// ---------- clipper ----------
function ClipCurve({ softness }: { softness: number }) {
  const t = 72;
  const a = t * (1 - 0.5 * softness);
  const pts: string[] = [];
  for (let x = 0; x <= 100; x += 2) {
    const yv = x <= a || softness === 0 ? Math.min(x, t) : t - (t - a) * Math.exp(-(x - a) / (t - a));
    pts.push(`${x} ${(100 - yv).toFixed(1)}`);
  }
  return (
    <div className="tc-curve lg" aria-hidden="true">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        <path className="un" d="M0 100L100 0" />
        <path className="th" d={`M0 ${100 - t}H100`} />
        <path className="cv" d={`M${pts.join("L")}`} />
      </svg>
    </div>
  );
}

function ClipperEditor({ v, ms, gr }: { v: BlockView; ms: MasterStatus; gr: number }) {
  const backend = useBackend();
  const m = useApp((s) => s.project)!.master;
  const limGr = useApp((s) => s.meters?.masterGr?.[1] ?? 0);
  const c = blockOf(m, v.id, "clipper")!;
  const take = useBlockCtl(MS.takeOff, `${v.name} take off`, v.id, "clipper", (b) => b.take_off_db, (b, x) => (b.take_off_db = x));
  const soft = useBlockCtl(MS.softness, `${v.name} softness`, v.id, "clipper", (b) => b.softness, (b, x) => (b.softness = x));
  // Where the peaks of the mixdown sit, column by column (a picture for the eye: the clipper works on the peak the
  // app measures at its input).
  const [hist, setHist] = useState<number[]>([]);
  const md = m.mixdown?.number;
  useEffect(() => {
    if (!ms.mixdown_measures) return;
    backend
      .mixdownPeaks(1024)
      .then((peaks) => {
        const top = Math.max(1e-9, ...peaks.map(([lo, hi]) => Math.max(-lo, hi)));
        const bins = new Array<number>(26).fill(0);
        for (const [lo, hi] of peaks) {
          const db = 20 * Math.log10(Math.max(1e-6, Math.max(-lo, hi) / top));
          if (db > -26) bins[Math.min(25, Math.floor(26 + db))]++;
        }
        const max = Math.max(1, ...bins);
        setHist(bins.map((n) => (n / max) * 100));
      })
      .catch(() => {});
  }, [backend, md, ms.mixdown_measures]);
  const cut = Math.max(0, Math.min(26, 26 - c.take_off_db));
  const work = ms.measures?.blocks.find((b) => b.id === v.id)?.work;
  const lim = ms.measures?.limiter;
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!c.bypass} sub="takes the top off the tallest peaks" />
      <div className="sect grow">
        <div className="fx g3 top-al">
          <ClipCurve softness={c.softness} />
          <div className="col g2 grow">
            <span className="faint s1">Where the peaks of the mixdown sit · right of the line is clipped</span>
            <div className="hist">
              <div className="bars">{hist.map((h, i) => <i key={i} className={i >= Math.floor(cut) ? "cut" : "dimd"} style={{ height: `${Math.max(1, h)}%` }} />)}</div>
              <i className="cutline" style={{ left: `${(cut / 26) * 100}%` }} />
              <span className="cutlbl" style={{ left: `${(cut / 26) * 100}%` }}>{n1(c.take_off_db)} dB under the tallest peak</span>
            </div>
            <div className="fx between faint s1"><span>quiet</span><span>body of the mix</span><span>tallest peaks</span></div>
          </div>
        </div>
        <div className="fx g4 end-al">
          <div className="knobs clipk">
            <Knob ctl={take} name="Take off up to" bare />
            <Knob ctl={soft} name="Softness" bare />
          </div>
          <div className="col g1 grow">
            <GrRow label="Clipper" db={gr} />
            <GrRow label="Limiter" db={limGr} />
          </div>
        </div>
        {work && lim && !c.bypass && (
          <span className="faint s1" data-testid="clipper-work">
            It works on {n1(work.active_pct)} % of the song, up to {n1(work.max_db)} dB. The limiter is left with {n1(lim.max_db)} dB at most.
          </span>
        )}
        <Info>
          “Take off up to” is how many dB come off the tallest peaks that reach the clipper, wherever it sits in the chain and whatever the target. Keep it to peaks that are few and short: when it works on more than a few per cent of the song it has reached the body of the mix, and that is heard as distortion.
        </Info>
      </div>
    </section>
  );
}

// ---------- multiband ----------
const BAND_NAMES: Record<number, string[]> = {
  2: ["Low", "High"],
  3: ["Low", "Mid", "High"],
  4: ["Low", "Low mid", "High mid", "High"],
  5: ["Sub", "Low", "Mid", "Presence", "Air"],
  6: ["Sub", "Low", "Low mid", "High mid", "Presence", "Air"],
};

function MultibandEditor({ v, ms }: { v: BlockView; ms: MasterStatus }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const spectrum = useApp((s) => s.meters?.spectrum ?? null);
  const bandsGr = useApp((s) => s.meters?.masterBands ?? null);
  const setError = useApp((s) => s.setError);
  const edit = useMasterEdit();
  const ref = useRef<HTMLDivElement>(null);
  const w = useWidth(ref, 440);
  const [selRaw, setSel] = useState(0);
  const [live, setLive] = useState<{ i: number; f: number } | null>(null);
  const m = project.master;
  const mb = blockOf(m, v.id, "multiband")!;
  const n = mb.bands.length;
  const sel = Math.min(selRaw, n - 1);
  const h = n > 3 ? 120 : 140;
  const xos = mb.crossovers_hz.map((f, i) => (live && live.i === i ? live.f : f));
  const edges = [20, ...xos, 20000];
  const names = BAND_NAMES[n] ?? mb.bands.map((_, i) => `Band ${i + 1}`);
  const gr = (i: number) => bandsGr?.[v.inst * 6 + i] ?? 0;
  const fail = (e: unknown) => setError(String(e));
  const op = (o: Parameters<typeof backend.masterChain>[0]) => void backend.masterChain(o).catch(fail);
  const band = (label: string, get: (b: (typeof mb.bands)[number]) => number, put: (b: (typeof mb.bands)[number], x: number) => void, s: (typeof MS)[keyof typeof MS]) =>
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useBlockCtl(s, `${v.name} ${names[sel]} ${label}`, v.id, "multiband", (b) => (b.bands[sel] ? get(b.bands[sel]) : s.default), (b, x) => { if (b.bands[sel]) put(b.bands[sel], x); });
  const thr = band("threshold", (b) => b.threshold_db, (b, x) => (b.threshold_db = x), MS.threshold);
  const ratio = band("ratio", (b) => b.ratio, (b, x) => (b.ratio = x), MS.ratio);
  const att = band("attack", (b) => b.attack_ms, (b, x) => (b.attack_ms = x), MS.attack);
  const rel = band("release", (b) => b.release_ms, (b, x) => (b.release_ms = x), MS.release);
  const gain = band("gain", (b) => b.gain_db, (b, x) => (b.gain_db = x), MS.bandGain);
  const solo = ms.solo && ms.solo[0] === v.id ? ms.solo[1] : 0;
  // Dragging a crossover: it stays a third of an octave from its neighbours.
  const onXo = (e: React.PointerEvent, i: number) => {
    e.stopPropagation();
    const r = ref.current!.getBoundingClientRect();
    const lo = edges[i] * 1.26;
    const hi = edges[i + 2] / 1.26;
    let began = false;
    let q: Promise<unknown> = Promise.resolve();
    const send = (f: () => Promise<unknown>) => (q = q.then(f).catch(fail));
    const move = (ev: PointerEvent) => {
      if (!began) {
        began = true;
        send(() => backend.beginGesture(`Master · ${v.name} crossover`, "master"));
      }
      const raw = xf(ev.clientX - r.left, r.width);
      const f = Math.round(Math.max(Math.max(40, lo), Math.min(Math.min(12000, hi), raw)));
      setLive({ i, f });
      const next = structuredClone(m);
      const nb = blockOf(next, v.id, "multiband");
      if (!nb) return;
      nb.crossovers_hz[i] = f;
      send(() => backend.gestureEdit({ op: "set_master", master: next }));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setLive(null);
      if (began) send(() => backend.endGesture(false));
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  const range = (i: number) => (i === 0 ? `below ${hz(edges[1])}` : i === n - 1 ? `above ${hz(edges[n - 1])}` : `${hz(edges[i])} – ${hz(edges[i + 1])}`);
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!mb.bypass} sub="two to six bands">
        <div className="fx g1">
          <IconButton icon="minus" label="One band less" disabled={n <= 2} onClick={() => op({ op: "bands", id: v.id, bands: n - 1 })} />
          <span className="b s1 stepv" data-testid="band-count">{n} bands</span>
          <IconButton icon="plus" label="One band more" disabled={n >= 6} onClick={() => op({ op: "bands", id: v.id, bands: n + 1 })} />
        </div>
      </Head>
      <div className="sect grow">
        <div className="eq mb" ref={ref} style={{ height: h }} onDoubleClick={(e) => { const r = ref.current!.getBoundingClientRect(); op({ op: "split", id: v.id, hz: Math.round(xf(e.clientX - r.left, r.width)) }); }}>
          <svg width={w} height={h} aria-hidden="true">
            <path className="sp" d={spectrumPath(spectrum, w, h)} />
            {mb.bands.map((_, i) => {
              const x0 = i === 0 ? 0 : fx(edges[i], w);
              const x1 = i === n - 1 ? w : fx(edges[i + 1], w);
              return (
                <g key={i}>
                  <rect className={`bd ${i === sel ? "is-sel" : ""}`} x={x0} y={0} width={Math.max(0, x1 - x0)} height={h} onClick={() => setSel(i)} />
                  <rect className="grb" x={x0 + 6} y={0} width={Math.max(0, x1 - x0 - 12)} height={Math.min(h / 2, Math.max(0, -gr(i)) * 8)} rx={2} />
                </g>
              );
            })}
            <path className="xo" d={xos.map((f) => `M${fx(f, w).toFixed(1)} 0V${h}`).join("")} />
          </svg>
          {xos.map((f, i) => (
            <span key={i}>
              <i role="slider" tabIndex={0} aria-label={`Crossover ${i + 1}`} aria-valuenow={Math.round(f)} aria-valuemin={40} aria-valuemax={12000} className="dot" style={{ left: fx(f, w), top: h / 2 }} onPointerDown={(e) => onXo(e, i)} onDoubleClick={(e) => { e.stopPropagation(); op({ op: "merge", id: v.id, crossover: i }); }} />
              <span className="xl" style={{ left: fx(f, w) }}>{hz(f)}</span>
            </span>
          ))}
          {TICKS.map((f) => (
            <span key={f} className="ft" style={{ left: fx(f, w) }}>{f >= 1000 ? `${f / 1000}k` : f}</span>
          ))}
        </div>
        <div className="col brows" role="group" aria-label="Bands">
          {mb.bands.map((b, i) => (
            <div key={i} className={`brow ${i === sel ? "on" : ""} ${b.bypass ? "is-off" : ""}`} onClick={() => setSel(i)}>
              <Toggle kind="by" small on={!b.bypass} label={`${names[i]} band on`} onClick={() => edit(`Master · ${v.name} ${names[i]} ${b.bypass ? "on" : "off"}`, (x) => { const nb = blockOf(x, v.id, "multiband"); if (nb) nb.bands[i].bypass = !b.bypass; })} />
              <button type="button" className="tabname b ell bname" aria-pressed={i === sel} onClick={(e) => { e.stopPropagation(); setSel(i); }}>{names[i]}</button>
              <span className="faint s1 ell brange">{range(i)}</span>
              <div className="hbar gr grow" aria-hidden="true"><i style={{ width: `${Math.min(100, Math.max(0, -gr(i)) * 10)}%` }} /></div>
              <span className="b s1 grv">{n1(gr(i))} dB</span>
              <Toggle kind="so" small on={(solo & (1 << i)) !== 0} label={`Solo ${names[i]}`} onClick={() => void backend.masterSolo(v.id, solo ^ (1 << i)).catch(fail)} />
            </div>
          ))}
        </div>
        <div className="knobs">
          <Knob ctl={thr} name="Threshold" bare />
          <Knob ctl={ratio} name="Ratio" bare />
          <Knob ctl={att} name="Attack" bare />
          <Knob ctl={rel} name="Release" bare />
          <Knob ctl={gain} name="Gain" bare />
        </div>
        <span className="faint s1">
          {names[sel]} selected. Drag a line to move a crossover; double-click the graph to split a band there, a line to join the bands beside it. S solos a band while you listen.
        </span>
      </div>
    </section>
  );
}

// ---------- the simpler blocks ----------
function MatchEditor({ v, ms }: { v: BlockView; ms: MasterStatus }) {
  const m = useApp((s) => s.project)!.master;
  const b = blockOf(m, v.id, "match_eq")!;
  const amount = useBlockCtl(MS.match, "Match amount", v.id, "match_eq", (x) => x.amount, (x, a) => (x.amount = a));
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!b.bypass} />
      <div className="sect grow">
        {ms.references.length ? (
          <>
            <div className="knobs"><Knob ctl={amount} name="Amount" size="l" /></div>
            <span className="faint s1">Moves the mixdown's 8-band profile toward the references' (at most ±6 dB per band).</span>
          </>
        ) : (
          <div className="empty">
            <Icon name="music" />
            <span className="b">No reference track</span>
            <span className="s1">Match EQ and the reference line in the spectrum need one. Mastering works without it: add one from the brief in Mix.</span>
          </div>
        )}
      </div>
    </section>
  );
}

function WidthEditor({ v }: { v: BlockView }) {
  const m = useApp((s) => s.project)!.master;
  const b = blockOf(m, v.id, "width")!;
  const side = useBlockCtl(MS.side, "Side gain", v.id, "width", (x) => x.side_gain_db, (x, a) => (x.side_gain_db = a));
  const mono = useBlockCtl(MS.mono, "Mono below", v.id, "width", (x) => x.mono_below_hz, (x, a) => (x.mono_below_hz = a));
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!b.bypass} />
      <div className="sect grow">
        <div className="knobs"><Knob ctl={side} name="Side gain" size="l" /><Knob ctl={mono} name="Mono below" size="l" /></div>
      </div>
    </section>
  );
}

function SatEditor({ v }: { v: BlockView }) {
  const m = useApp((s) => s.project)!.master;
  const b = blockOf(m, v.id, "saturation")!;
  const drive = useBlockCtl(MS.drive, `${v.name} drive`, v.id, "saturation", (x) => x.drive_db, (x, a) => (x.drive_db = a));
  const mix = useBlockCtl(MS.mix, `${v.name} mix`, v.id, "saturation", (x) => x.mix, (x, a) => (x.mix = a));
  return (
    <section className="card grow" aria-label={v.name}>
      <Head v={v} on={!b.bypass} />
      <div className="sect grow">
        <div className="knobs"><Knob ctl={drive} name="Drive" size="l" /><Knob ctl={mix} name="Mix" size="l" /></div>
        <span className="faint s1">A soft shaper that rounds what is above −18 dBFS and adds harmonics. A colour first, a peak tool second: for peaks alone the clipper is cleaner.</span>
      </div>
    </section>
  );
}

/** Input, limiter and dither: the three blocks that stay in place. */
function FixedEditor({ id }: { id: string }) {
  const m = useApp((s) => s.project)!.master;
  const edit = useMasterEdit();
  const input = useMasterCtl(MS.input, "Input gain", (x) => x.chain.input_gain_db, (x, v) => (x.chain.input_gain_db = v));
  const lrel = useMasterCtl(MS.limRelease, "Limiter release", (x) => x.chain.limiter.release_ms, (x, v) => (x.chain.limiter.release_ms = v));
  const name = id === "input" ? "Input" : id === "dither" ? "Dither" : "Limiter";
  return (
    <section className="card grow" aria-label={name}>
      <div className="ph">
        {id === "dither" && <Toggle kind="by" on={!m.chain.dither.bypass} label="Dither on" onClick={() => edit(`Master · Dither ${m.chain.dither.bypass ? "on" : "off"}`, (x) => (x.chain.dither.bypass = !x.chain.dither.bypass))} />}
        <span className="h">{name}</span>
        <span className="faint s1 grow">{id === "input" ? "first in the chain" : id === "dither" ? "last in the chain" : "always after the free blocks"}</span>
      </div>
      <div className="sect grow">
        {id === "input" && <div className="knobs"><Knob ctl={input} name="Input gain" size="l" /></div>}
        {id === "limiter" && (
          <>
            <div className="knobs"><Knob ctl={lrel} name="Release" size="l" /></div>
            <span className="faint s1">Ceiling {n1(m.target.true_peak_db)} dBTP from the target, with 2 ms look-ahead on the 4x true peak. It always runs in the export, so the ceiling is guaranteed on the file: that is why nothing can follow it.</span>
          </>
        )}
        {id === "dither" && <span className="dim">TPDF dither is added when a 16-bit file is written (WAV or FLAC), with a fixed seed so the same master gives the same file. Turn it off only if a later step adds its own.</span>}
      </div>
    </section>
  );
}

/** The editor of what the rack has selected. */
export function BlockEditor({ ms }: { ms: MasterStatus }) {
  const open = useApp((s) => s.masterModule);
  const blocksGr = useApp((s) => s.meters?.masterBlocks ?? null);
  const i = ms.chain.findIndex((b) => b.id === open);
  const v = ms.chain[i];
  if (!v) return <FixedEditor id={open === "input" || open === "dither" ? open : "limiter"} />;
  const gr = blocksGr?.[i] ?? 0;
  switch (v.kind) {
    case "eq":
      return <EqEditor key={v.id} v={v} ms={ms} />;
    case "match_eq":
      return <MatchEditor v={v} ms={ms} />;
    case "compressor":
      return <CompEditor key={v.id} v={v} gr={gr} />;
    case "multiband":
      return <MultibandEditor key={v.id} v={v} ms={ms} />;
    case "width":
      return <WidthEditor v={v} />;
    case "saturation":
      return <SatEditor key={v.id} v={v} />;
    default:
      return <ClipperEditor key={v.id} v={v} ms={ms} gr={gr} />;
  }
}

// ---------- section offsets ----------
export function SectionEditor({ ms, id }: { ms: MasterStatus; id: string }) {
  const project = useApp((s) => s.project)!;
  const setView = useApp((s) => s.setView);
  const edit = useMasterEdit();
  const [all, setAll] = useState(true);
  const [eqId, setEqId] = useState<string | null>(null);
  const m = project.master;
  const sec = project.sections.find((s) => s.id === id);
  const rate = project.sample_rate ?? 48000;
  const same = sec ? project.sections.filter((s) => s.name === sec.name).map((s) => s.id) : [];
  const scope = all && same.length > 1 ? same : [id];
  const eqs = ms.chain.filter((b) => b.kind === "eq");
  const eqView = eqs.find((b) => b.id === eqId) ?? eqs[0];
  const eq = eqView ? blockOf(m, eqView.id, "eq") : undefined;
  const cur = m.sections.find((o) => o.section === id);
  const has = !!m.chain.blocks.find((b) => b.block.type === "width" && !b.block.bypass);
  const put = (f: (o: NonNullable<typeof cur>) => void) => (x: typeof m) => withOffsets(x, scope, f);
  const level = useMasterCtl(MS.secLevel, "Section level", (x) => x.sections.find((o) => o.section === id)?.level_db ?? 0, (x, v) => put((o) => (o.level_db = v))(x));
  const side = useMasterCtl(MS.secSide, "Section width", (x) => x.sections.find((o) => o.section === id)?.side_gain_db ?? 0, (x, v) => put((o) => (o.side_gain_db = v))(x));
  const release = useMasterCtl(MS.secRelease, "Section limiter release", (x) => x.sections.find((o) => o.section === id)?.limiter_release_ms ?? 0, (x, v) => put((o) => (o.limiter_release_ms = v))(x));
  const [band, setBand] = useState(0);
  const eqKey = eqView?.id ?? "";
  const bandCtl = useMasterCtl(
    MS.secEq,
    "Section EQ gain",
    (x) => x.sections.find((o) => o.section === id)?.eq.find((e) => e.block === eqKey)?.gains_db[band] ?? 0,
    (x, v) =>
      put((o) => {
        const n = blockOf(x, eqKey, "eq")?.bands.length ?? 0;
        let e = o.eq.find((y) => y.block === eqKey);
        if (!e) {
          e = { block: eqKey, gains_db: new Array<number>(n).fill(0) };
          o.eq.push(e);
        }
        e.gains_db[band] = v;
      })(x),
  );
  if (!sec) return null;
  const t = (f: number) => `${Math.floor(f / rate / 60)}:${String(Math.floor((f / rate) % 60)).padStart(2, "0")}`;
  const offs = cur?.eq.find((e) => e.block === eqKey)?.gains_db ?? [];
  const inUse = eq ? eq.bands.map((b, i) => ({ b, i })).filter((x) => x.b.enabled) : [];
  return (
    <section className="card grow" aria-label={`Offsets of ${sec.name}`} data-testid="section-editor">
      <div className="ph">
        <Icon name="cut" />
        <span className="h">{sec.name}</span>
        <span className="faint s1 grow ell">{t(sec.start)} – {t(sec.end)}</span>
        {same.length > 1 && <Seg className="quiet sm" label="Scope" value={all ? 1 : 0} items={[{ value: 0, label: "This one" }, { value: 1, label: `All ${same.length}` }]} onChange={(x) => setAll(x === 1)} />}
        <button type="button" className="btn quiet sm" onClick={() => setView({ masterSection: null })}>Done</button>
      </div>
      <div className="sect grow">
        <span className="faint s1">Offsets added to the song-wide master, only while {same.length > 1 && all ? `the ${same.length} sections named ${sec.name} play` : "this section plays"}.</span>
        <div className="fx g3"><span className="b offl">Level</span><Field ctl={level} /><span className="faint s1 grow">More or less level into the limiter.</span></div>
        <div className="fx g3"><span className="b offl">Width</span><Field ctl={side} /><span className="faint s1 grow">{has ? "More or less side than the rest of the song." : "Needs the Width block on in the chain."}</span></div>
        <div className="fx g3"><span className="b offl">Limiter release</span><Field ctl={release} /><span className="faint s1 grow">Longer or shorter than the rest of the song.</span></div>
        {eq && inUse.length > 0 ? (
          <>
            <div className="fx g2">
              <span className="faint s1 grow">{eqView!.name} in this section: gain added to its bands</span>
              {eqs.length > 1 && <Seg className="quiet sm" label="EQ" value={eqView!.id} items={eqs.map((b) => ({ value: b.id, label: b.name }))} onChange={(x) => { setEqId(x); setBand(0); }} />}
            </div>
            <div className="fx g1">
              {inUse.map(({ b, i }) => (
                <button type="button" key={i} className={`pill ${band === i ? "on" : ""}`} aria-pressed={band === i} onClick={() => setBand(i)}>
                  <span className="faint s1 ell">{hz(b.freq_hz)}</span>
                  <span className="b ell">{offs[i] ? `${offs[i] > 0 ? "+" : ""}${n1(offs[i])} dB` : "—"}</span>
                </button>
              ))}
            </div>
            {inUse.some((x) => x.i === band) && <div className="fx g3"><span className="b offl">Band {band + 1}</span><Field ctl={bandCtl} /><span className="faint s1 grow">On top of its {n1(eq.bands[band].gain_db)} dB.</span></div>}
          </>
        ) : (
          <span className="faint s1">An EQ with a band in use can have its gains offset here.</span>
        )}
        <div className="tile row">
          <Icon name="info" />
          <span className="dim s1">Offsets glide over a tenth of a second at the edges of the section. The loudness of the target is still measured on the whole song.</span>
          <span className="grow" />
          <button type="button" className="btn quiet sm" disabled={!cur} onClick={() => edit(`Master · ${sec.name} offsets cleared`, (x) => (x.sections = x.sections.filter((o) => !scope.includes(o.section))))}>Clear</button>
        </div>
      </div>
    </section>
  );
}
