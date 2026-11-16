// Editors of the blocks that have one of their own (design 16d–16o, UI-12): what the block does to the sound, drawn
// from its settings, then its controls. Curves are computed for drawing only (FOUNDATION §5.6).
import { useEffect, useState, type FC, type ReactNode } from "react";
import type { FxBlock } from "../bindings/FxBlock";
import type { FxKind } from "../bindings/FxKind";
import type { LfoShape } from "../bindings/LfoShape";
import type { Project } from "../bindings/Project";
import type { SatCharacter } from "../bindings/SatCharacter";
import type { Track } from "../bindings/Track";
import { Seg } from "../shared/controls";
import { Icon } from "../shared/Icon";
import { useEdit } from "./bind";
import { useBackend } from "../state/context";
import { BKnob, SidechainButton, useSetBlock } from "./BlockEditors";
import { biquad, magDb } from "./EqGraph";
import { hzText } from "./fx";

export type EditorProps = { t: Track; project: Project; b: FxBlock };

const W = 280;
const n1 = (v: number) => Number(v.toFixed(1)).toString().replace("-", "−");

/** A small drawing of what a block does (the boards' `.fxg`), with an optional legend below. */
function Graph({ h = 76, children, legend, label }: { h?: number; children: ReactNode; legend?: [string, string][]; label: string }) {
  return (
    <>
      <div className="eq fxg" style={{ height: h }} role="img" aria-label={label}>
        <svg width="100%" height={h} viewBox={`0 0 ${W} ${h}`} preserveAspectRatio="none" aria-hidden="true">
          {children}
        </svg>
      </div>
      {legend && (
        <div className="fx g3 s1 faint lgd">
          {legend.map(([cls, txt]) => (
            <span key={txt} className="fx g1">
              <i className={`lg ${cls}`} />
              {txt}
            </span>
          ))}
        </div>
      )}
    </>
  );
}

function Info({ children }: { children: ReactNode }) {
  return (
    <div className="tile row tinfo">
      <Icon name="info" size="sm" />
      <span className="dim s1">{children}</span>
    </div>
  );
}

const path = (pts: [number, number][]) => "M" + pts.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join("L");
const fxX = (f: number) => (Math.log(f / 20) / Math.log(1000)) * W;
const freqs = (n = 140) => Array.from({ length: n + 1 }, (_, i) => 20 * Math.pow(1000, i / n));

/** Transfer curve of a gate: input −80…0 dB against output (16d). */
function GateEditor({ t, project, b }: EditorProps) {
  if (b.fx.type !== "gate") return null;
  const g = b.fx;
  const h = 76;
  const X = (d: number) => ((d + 80) / 80) * W;
  const Y = (d: number) => h - ((d + 80) / 80) * h;
  const out = (d: number) => (d >= g.threshold_db ? d : d - Math.min(g.range_db, (g.ratio - 1) * (g.threshold_db - d)));
  const pts: [number, number][] = [];
  for (let d = -80; d <= 0; d += 1) pts.push([X(d), Y(Math.max(-80, out(d)))]);
  return (
    <>
      <Graph label={`Gate curve: closes ${n1(g.range_db)} dB under ${n1(g.threshold_db)} dB`} legend={[["", "output against input"], ["rf", "unchanged"]]}>
        <path className="zr" d={`M0 ${h}L${W} 0`} />
        <path className="gd" d={`M${X(g.threshold_db).toFixed(1)} 0V${h}`} />
        <path className="cv" d={path(pts)} />
      </Graph>
      <div className="fx g2">
        <SidechainButton t={t} project={project} b={b} />
        <span className="grow" />
        <span className="tag">{g.ratio >= 19.5 ? "gate" : `expander ${n1(g.ratio)}:1`}</span>
      </div>
      <div className="knobs wrap">
        <BKnob t={t} b={b} k="threshold" name="Threshold" />
        <BKnob t={t} b={b} k="range" name="Range" />
        <BKnob t={t} b={b} k="ratio" name="Ratio" />
        <BKnob t={t} b={b} k="attack" name="Attack" />
        <BKnob t={t} b={b} k="hold" name="Hold" />
        <BKnob t={t} b={b} k="release" name="Release" />
        <BKnob t={t} b={b} k="det_hpf" name="Listens above" />
        <BKnob t={t} b={b} k="det_lpf" name="Listens below" />
      </div>
      <Info>Ratio under 20 is a gentler expander; less range sounds more natural.</Info>
    </>
  );
}

/** Where a de-esser works: the band above its frequency, down by at most its reduction (16a). */
function DeesserEditor({ t, b }: EditorProps) {
  const set = useSetBlock(t);
  const backend = useBackend();
  const [listen, setListen] = useState(false);
  useEffect(() => {
    // Listening ends with the editor: it is never left on by mistake.
    return () => void backend.listenBlock(t.id, b.id, false).catch(() => {});
  }, [backend, t.id, b.id]);
  if (b.fx.type !== "deesser") return null;
  const d = b.fx;
  const h = 76;
  const Y = (db: number) => 8 + (-db / 12) * (h - 16);
  const curve = (depth: number) =>
    path(
      freqs().map((f) => {
        const k = d.mode === "wide" ? 1 : 1 - Math.pow(10, magDb(biquad("lpf", d.freq_hz, 0, Math.SQRT1_2, 48000), f, 48000) / 20);
        return [fxX(f), Y(-depth * Math.max(0, Math.min(1, k)))];
      }),
    );
  return (
    <>
      <Graph label={`De-esser: ${d.mode === "split" ? `above ${hzText(d.freq_hz)}` : "the whole signal"}, at most ${n1(d.max_reduction_db)} dB`} legend={[["cv", "taking off now"], ["rf", "as far as it goes"]]}>
        <path className="zr" d={`M0 ${Y(0)}H${W}`} />
        <path className="rf" d={curve(d.max_reduction_db)} />
        <path className="cv" d={curve(d.max_reduction_db * 0.6)} />
        <path className="gd" d={`M${fxX(d.freq_hz).toFixed(1)} 0V${h}`} />
      </Graph>
      <div className="fx g2">
        <Seg className="quiet sm" label="Mode" value={d.mode} items={[{ value: "split", label: "Split" }, { value: "wide", label: "Wide" }]} onChange={(v) => void set(b, { ...d, mode: v }, `mode ${v}`)} />
        <span className="grow" />
        <button
          type="button"
          className={`btn quiet sm ${listen ? "on" : ""}`}
          aria-pressed={listen}
          onClick={() => {
            setListen(!listen);
            void backend.listenBlock(t.id, b.id, !listen).catch(() => {});
          }}
        >
          <Icon name="ear" size="sm" />
          Listen to what it takes
        </button>
      </div>
      <div className="knobs">
        <BKnob t={t} b={b} k="freq" name="Frequency" />
        <BKnob t={t} b={b} k="threshold" name="Threshold" />
        <BKnob t={t} b={b} k="reduction" name="At most" />
      </div>
      <Info>Split lowers only what is above {hzText(d.freq_hz)}, and only while an s or a t is too loud. Wide lowers the whole voice.</Info>
    </>
  );
}

/** A dynamic EQ (16e): each band's curve at rest and as far as it goes, with what it listens to. */
function DynEqEditor({ t, project, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "dynamic_eq") return null;
  const d = b.fx;
  const h = 76;
  const Y = (db: number) => h / 2 - (db / 12) * (h / 2 - 4);
  const kind = (k: string) => (k === "low_shelf" ? "low_shelf" : k === "high_shelf" ? "high_shelf" : "peak");
  const curve = (amount: number) =>
    path(freqs().map((f) => [fxX(f), Y(d.bands.filter((x) => x.enabled).reduce((s, x) => s + magDb(biquad(kind(x.kind), x.freq_hz, x.range_db * amount, x.q, 48000), f, 48000), 0))]));
  return (
    <>
      <Graph label="Dynamic EQ: the bands at rest and as far as they go" legend={[["rf", "as far as it goes"], ["zr", "at rest, under the threshold"]]}>
        <path className="zr" d={`M0 ${Y(0)}H${W}`} />
        <path className="rf" d={curve(1)} />
        <path className="cv" d={curve(0.5)} />
      </Graph>
      {d.bands.map((x, i) => (
        <div key={i} className="col g2">
          <div className="fx g2">
            <button
              type="button"
              className={`tgl by ${x.enabled ? "on" : ""}`}
              aria-pressed={x.enabled}
              aria-label={`Band ${i + 1} on`}
              onClick={() => void set(b, { ...d, bands: d.bands.map((y, j) => (j === i ? { ...y, enabled: !y.enabled } : y)) }, `band ${i + 1} ${x.enabled ? "off" : "on"}`)}
            >
              <Icon name="power" />
            </button>
            <span className="b">Band {i + 1}</span>
            <Seg
              className="quiet sm"
              label={`Shape of band ${i + 1}`}
              value={x.kind}
              items={[{ value: "low_shelf", label: "Low" }, { value: "peak", label: "Peak" }, { value: "high_shelf", label: "High" }]}
              onChange={(v) => void set(b, { ...d, bands: d.bands.map((y, j) => (j === i ? { ...y, kind: v } : y)) }, `band ${i + 1} ${v}`)}
            />
            <span className="grow" />
            <SidechainButton t={t} project={project} b={b} band={i} />
          </div>
          {x.enabled && (
            <div className="knobs wrap">
              <BKnob t={t} b={b} k={`${i + 1}.freq`} name="Frequency" />
              <BKnob t={t} b={b} k={`${i + 1}.q`} name="Q" />
              <BKnob t={t} b={b} k={`${i + 1}.threshold`} name="Threshold" />
              <BKnob t={t} b={b} k={`${i + 1}.range`} name="Range" bip />
              <BKnob t={t} b={b} k={`${i + 1}.attack`} name="Attack" />
              <BKnob t={t} b={b} k={`${i + 1}.release`} name="Release" />
            </div>
          )}
        </div>
      ))}
      <Info>A band moves only while what it listens to passes the threshold: with a sidechain, only while that track plays there.</Info>
    </>
  );
}

/** The hit as recorded and after the transient shaper (16f). */
function TransientEditor({ t, b }: EditorProps) {
  if (b.fx.type !== "transient") return null;
  const x = b.fx;
  const h = 76;
  const env = (i: number, a: number, s: number) => {
    const tt = i / 140;
    const atk = Math.exp(-tt * (20 + 40 * x.speed));
    const raw = tt < 0.02 ? tt / 0.02 : Math.exp(-(tt - 0.02) * 4);
    const g = Math.pow(10, (a * atk + s * (1 - atk)) / 20);
    return raw * g;
  };
  const Y = (v: number) => h - 4 - Math.min(1.6, v) / 1.6 * (h - 8);
  const line = (a: number, s: number) => path(Array.from({ length: 141 }, (_, i) => [(i / 140) * W, Y(env(i, a, s))]));
  return (
    <>
      <Graph label="The hit as recorded and after" legend={[["cv", "after"], ["rf", "the hit as recorded"]]}>
        <path className="rf" d={line(0, 0)} />
        <path className="cv" d={line(x.attack_db * x.mix, x.sustain_db * x.mix)} />
      </Graph>
      <div className="knobs">
        <BKnob t={t} b={b} k="attack" name="Attack" bip />
        <BKnob t={t} b={b} k="sustain" name="Sustain" bip />
        <BKnob t={t} b={b} k="speed" name="Speed" />
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>No threshold: soft and loud hits are shaped alike. Speed: how long counts as attack.</Info>
    </>
  );
}

const CHARACTER: { value: SatCharacter; label: string; hint: string; harm: number[] }[] = [
  { value: "soft", label: "Soft", hint: "a gentle curve, odd harmonics", harm: [0.15, 0.6, 0.1, 0.3, 0.05, 0.15] },
  { value: "tape", label: "Tape", hint: "rounded peaks, the highs a little down", harm: [0.25, 0.55, 0.15, 0.2, 0.05, 0.08] },
  { value: "tube", label: "Tube", hint: "even harmonics", harm: [0.8, 0.3, 0.5, 0.12, 0.25, 0.05] },
  { value: "transistor", label: "Transistor", hint: "odd harmonics, harder", harm: [0.1, 0.85, 0.08, 0.6, 0.05, 0.4] },
];

/** Harmonics as bars, 2nd to 7th (16g, 16h). */
function Harmonics({ levels, lit, label }: { levels: number[]; lit: (i: number) => boolean; label: string }) {
  const h = 64;
  const bw = W / levels.length;
  return (
    <Graph h={h} label={label}>
      {levels.map((v, i) => (
        <rect key={i} className={lit(i) ? "pl" : "cz"} x={i * bw + bw * 0.2} width={bw * 0.6} y={h - 4 - v * (h - 12)} height={v * (h - 12)} />
      ))}
    </Graph>
  );
}

/** Saturation (FX-4, FX-18, design 16g): a character, drive and mix. */
function SaturationEditor({ t, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "saturation") return null;
  const fx = b.fx;
  const c = CHARACTER.find((x) => x.value === fx.character) ?? CHARACTER[0];
  const k = Math.min(1, fx.drive_db / 18) * fx.mix;
  return (
    <>
      <Seg className="quiet sm" label="Character" value={fx.character} items={CHARACTER.map((x) => ({ value: x.value, label: x.label }))} onChange={(v) => void set(b, { ...fx, character: v }, `character ${v}`)} />
      <span className="faint s1">Harmonics it adds · 2nd to 7th · {c.hint}</span>
      <Harmonics label={`Harmonics of the ${c.label} character`} levels={c.harm.map((v) => v * (0.25 + 0.75 * k))} lit={(i) => (fx.character === "tube" ? i % 2 === 0 : i % 2 === 1)} />
      <div className="knobs">
        <BKnob t={t} b={b} k="drive" name="Drive" />
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>Level is matched: Drive changes the colour, not the loudness. Four times oversampled.</Info>
    </>
  );
}

/** Spectrum sketch: what is there and what the block adds above or below a frequency (16h, 16i). */
function AddedSpectrum({ f0, above, amount, label, legend }: { f0: number; above: boolean; amount: number; label: string; legend: [string, string][] }) {
  const h = 64;
  const x0 = fxX(f0);
  const base = path(freqs().map((f) => [fxX(f), h - 10 - 30 * Math.exp(-Math.pow(Math.log(f / 300), 2) / 6)]));
  const add = above
    ? `M${x0.toFixed(1)} ${h}` + freqs().filter((f) => f >= f0).map((f) => `L${fxX(f).toFixed(1)} ${(h - 6 - amount * 36 * Math.exp(-Math.log(f / f0) / 2)).toFixed(1)}`).join("") + `L${W} ${h}Z`
    : [2, 3, 4].map((k) => `M${fxX(f0 * k * 0.7).toFixed(1)} ${h}V${(h - 6 - amount * 40 / k).toFixed(1)}`).join("");
  return (
    <Graph h={h} label={label} legend={legend}>
      <path className="wv" d={base} />
      <path className="gd" d={`M${x0.toFixed(1)} 0V${h}`} />
      <path className={above ? "bz" : "hm"} d={add} />
    </Graph>
  );
}

function ExciterEditor({ t, b }: EditorProps) {
  if (b.fx.type !== "exciter") return null;
  const e = b.fx;
  return (
    <>
      <AddedSpectrum f0={e.freq_hz} above amount={e.amount} label={`Exciter: sparkle added above ${hzText(e.freq_hz)}`} legend={[["bz", "added sparkle"]]} />
      <div className="knobs">
        <BKnob t={t} b={b} k="freq" name="Frequency" />
        <BKnob t={t} b={b} k="drive" name="Drive" />
        <BKnob t={t} b={b} k="amount" name="Amount" />
      </div>
      <Info>Saturates only the top and blends it back: presence without more EQ.</Info>
    </>
  );
}

function BassEditor({ t, b }: EditorProps) {
  if (b.fx.type !== "bass_enhancer") return null;
  const e = b.fx;
  return (
    <>
      <AddedSpectrum f0={e.freq_hz} above={false} amount={e.harmonics * e.mix * 2} label={`Bass enhancer: harmonics of what is under ${hzText(e.freq_hz)}`} legend={[["", "fundamentals"], ["hm", "added harmonics"]]} />
      <div className="knobs">
        <BKnob t={t} b={b} k="freq" name="Frequency" />
        <BKnob t={t} b={b} k="harmonics" name="Harmonics" />
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>Small speakers cannot play the lowest notes, but they play their harmonics and the ear fills in the note. Nothing is added below the frequency.</Info>
    </>
  );
}

function WidthEditor({ t, b }: EditorProps) {
  if (b.fx.type !== "width") return null;
  const w = b.fx;
  const h = 76;
  const cx = W / 2;
  // The stereo image as an ellipse: wider with more side.
  const rx = Math.max(2, Math.min(W / 2 - 8, (W / 6) * w.width));
  const mono = t.kind === "audio" && t.channels === 1 && !t.chain.blocks.slice(0, t.chain.blocks.findIndex((x) => x.id === b.id)).some((x) => x.fx.type === "modulation" || x.fx.type === "doubler" || x.fx.type === "harmonizer" || (x.fx.type === "tremolo" && x.fx.mode === "autopan"));
  return (
    <>
      <Graph label={`Width ${Math.round(w.width * 100)} %`} legend={[["cv", "with Width"], ["rf", "as recorded"]]}>
        <ellipse className="rf" cx={cx} cy={h / 2} rx={W / 6} ry={h / 2 - 6} fill="none" />
        <ellipse className="cv" cx={cx} cy={h / 2} rx={rx} ry={h / 2 - 6} fill="none" />
        <path className="gd" d={`M${cx} 0V${h}`} />
      </Graph>
      <div className="knobs">
        <BKnob t={t} b={b} k="width" name="Width" />
        <BKnob t={t} b={b} k="mono" name="Mono below" />
      </div>
      {mono ? (
        <Info>On a mono track there is nothing to widen here: put Width after a Modulation or a Doubler.</Info>
      ) : (
        <Info>The side of the stereo image up or down; under the Mono frequency the lows sit in the centre.</Info>
      )}
    </>
  );
}

const SYNC = ["1/1", "1/2", "1/4", "1/4.", "1/8", "1/8.", "1/8t", "1/16"];

/** The LFO over four beats (16k, 16l): the shape the block follows, the right channel offset. */
function LfoGraph({ shape, rate, phase, label }: { shape: LfoShape; rate: string; phase: number; label: string }) {
  const h = 64;
  const v = (x: number) => {
    const p = ((x % 1) + 1) % 1;
    if (shape === "triangle") return 1 - 4 * Math.abs(p - 0.5) + 0;
    if (shape === "square") return Math.tanh(6 * Math.sin(2 * Math.PI * p));
    return Math.sin(2 * Math.PI * p);
  };
  const line = (off: number) => path(Array.from({ length: 161 }, (_, i) => [(i / 160) * W, h / 2 - v((i / 160) * 3 + off) * (h / 2 - 6)]));
  return (
    <Graph h={h} label={`${label} · ${rate}`} legend={[["cv", "left"], ["hm", `right · ${Math.round(phase)}° apart`]]}>
      <path className="zr" d={`M0 ${h / 2}H${W}`} />
      <path className="cv" d={line(0)} />
      <path className="hm" d={line(phase / 360)} />
    </Graph>
  );
}

function RateSeg({ t, b, sync, tempo }: { t: Track; b: FxBlock; sync: string | null; tempo: boolean }) {
  const set = useSetBlock(t);
  const fx = b.fx;
  if (fx.type !== "modulation" && fx.type !== "tremolo") return null;
  return (
    <Seg
      className="quiet sm"
      label="Rate unit"
      value={sync ?? "hz"}
      items={["hz", ...(tempo ? SYNC : [])].map((v) => ({ value: v, label: v === "hz" ? "Hz" : v }))}
      onChange={(v) => void set(b, { ...fx, sync: v === "hz" ? null : v }, `rate ${v === "hz" ? "free" : v}`)}
    />
  );
}

function ModulationEditor({ t, project, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "modulation") return null;
  const m = b.fx;
  return (
    <>
      <div className="fx g2 wrap">
        <Seg className="quiet sm" label="Mode" value={m.mode} items={[{ value: "chorus", label: "Chorus" }, { value: "flanger", label: "Flanger" }, { value: "phaser", label: "Phaser" }]} onChange={(v) => void set(b, { ...m, mode: v }, v)} />
        {m.mode === "chorus" && <Seg className="quiet sm" label="Voices" value={m.voices} items={[1, 2, 3].map((v) => ({ value: v, label: String(v) }))} onChange={(v) => void set(b, { ...m, voices: v }, `${v} voices`)} />}
        {m.mode === "phaser" && <Seg className="quiet sm" label="Stages" value={m.stages} items={[4, 6, 8, 12].map((v) => ({ value: v, label: String(v) }))} onChange={(v) => void set(b, { ...m, stages: v }, `${v} stages`)} />}
      </div>
      <LfoGraph shape="sine" rate={m.sync ?? `${n1(m.rate_hz)} Hz`} phase={m.spread_deg} label="The sweep" />
      <RateSeg t={t} b={b} sync={m.sync} tempo={!!project.tempo} />
      <div className="knobs wrap">
        <BKnob t={t} b={b} k="rate" name="Rate" />
        <BKnob t={t} b={b} k="depth" name="Depth" />
        {m.mode !== "chorus" && <BKnob t={t} b={b} k="feedback" name="Feedback" bip />}
        <BKnob t={t} b={b} k="spread" name="Spread" />
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>The sweep follows the song, not the play button: what you hear from the middle is what is exported.</Info>
    </>
  );
}

function TremoloEditor({ t, project, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "tremolo") return null;
  const m = b.fx;
  return (
    <>
      <div className="fx g2 wrap">
        <Seg className="quiet sm" label="Mode" value={m.mode} items={[{ value: "tremolo", label: "Tremolo" }, { value: "autopan", label: "Auto-pan" }]} onChange={(v) => void set(b, { ...m, mode: v }, v)} />
        <Seg className="quiet sm" label="Shape" value={m.shape} items={[{ value: "sine", label: "Sine" }, { value: "triangle", label: "Triangle" }, { value: "square", label: "Square" }]} onChange={(v) => void set(b, { ...m, shape: v }, `${v} shape`)} />
      </div>
      <LfoGraph shape={m.shape} rate={m.sync ?? `${n1(m.rate_hz)} Hz`} phase={m.mode === "autopan" ? 180 : m.phase_deg} label={m.mode === "autopan" ? "Pan over time" : "Level over time"} />
      <RateSeg t={t} b={b} sync={m.sync} tempo={!!project.tempo} />
      <div className="knobs">
        <BKnob t={t} b={b} k="rate" name="Rate" />
        <BKnob t={t} b={b} k="depth" name="Depth" />
        {m.mode === "tremolo" && <BKnob t={t} b={b} k="phase" name="Stereo phase" />}
      </div>
      <Info>Auto-pan moves the track left and right instead: a mono track becomes stereo from here on.</Info>
    </>
  );
}

/** Harmonizer (16m): the voices on a scale of −12 … +12 semitones. */
function HarmonizerEditor({ t, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "harmonizer") return null;
  const hz = b.fx;
  const h = 40;
  const X = (st: number) => ((st + 12) / 24) * (W - 16) + 8;
  return (
    <>
      <Seg className="quiet sm" label="Voices" value={hz.voices.length} items={[1, 2].map((v) => ({ value: v, label: `${v} voice${v > 1 ? "s" : ""}` }))} onChange={(v) => void set(b, { ...hz, voices: v === 1 ? hz.voices.slice(0, 1) : [...hz.voices, { ...hz.voices[0], semitones: -5, pan: -hz.voices[0].pan, enabled: true }].slice(0, 2) }, `${v} voices`)} />
      <Graph h={h} label="The voices from an octave down to an octave up">
        <path className="zr" d={`M8 ${h / 2}H${W - 8}`} />
        {Array.from({ length: 25 }, (_, i) => (
          <path key={i} className="gd" d={`M${X(i - 12).toFixed(1)} ${h / 2 - (i % 12 === 0 ? 8 : 3)}V${h / 2 + (i % 12 === 0 ? 8 : 3)}`} />
        ))}
        {hz.voices.map((v, i) => v.enabled && <circle key={i} className={i ? "pr" : "pl"} cx={X(v.semitones + v.cents / 100)} cy={h / 2} r={5} />)}
      </Graph>
      {hz.voices.map((_, i) => (
        <div key={i} className="col g1">
          <span className="b s1">Voice {i + 1}</span>
          <div className="knobs wrap">
            <BKnob t={t} b={b} k={`${i + 1}.semis`} name="Interval" bip />
            <BKnob t={t} b={b} k={`${i + 1}.cents`} name="Fine" bip />
            <BKnob t={t} b={b} k={`${i + 1}.level`} name="Level" />
            <BKnob t={t} b={b} k={`${i + 1}.pan`} name="Pan" bip />
          </div>
        </div>
      ))}
      <div className="knobs">
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>Fixed intervals: the voices do not follow the key, so a third may land out of the scale. Fifths and octaves always fit.</Info>
    </>
  );
}

function DoublerEditor({ t, b }: EditorProps) {
  const set = useSetBlock(t);
  if (b.fx.type !== "doubler") return null;
  const d = b.fx;
  const h = 56;
  const wave = (shift: number, amp: number) => path(Array.from({ length: 141 }, (_, i) => [(i / 140) * W, h / 2 - amp * Math.sin((i / 140) * 18 - shift) * Math.exp(-Math.pow(i / 140 - 0.45, 2) * 6) * (h / 2 - 4)]));
  return (
    <>
      <Seg className="quiet sm" label="Voices" value={d.voices} items={[1, 2].map((v) => ({ value: v, label: `${v} voice${v > 1 ? "s" : ""}` }))} onChange={(v) => void set(b, { ...d, voices: v }, `${v} voices`)} />
      <Graph h={h} label="The take and the double" legend={[["rf", "the take"], ["cv", "the double: a little later, a little out of tune, moving"]]}>
        <path className="rf" d={wave(0, 1)} />
        <path className="cv" d={wave(d.delay_ms / 6, 0.8)} />
      </Graph>
      <div className="knobs wrap">
        <BKnob t={t} b={b} k="detune" name="Detune" />
        <BKnob t={t} b={b} k="delay" name="Delay" />
        <BKnob t={t} b={b} k="spread" name="Width" />
        <BKnob t={t} b={b} k="mix" name="Mix" />
      </div>
      <Info>Inside this track: no copy of the clip, no new track.</Info>
    </>
  );
}

/** Multiband of a track (FX-17): its bands, two to six, each with its compressor and level. */
function MultibandEditor({ t, b }: EditorProps) {
  const edit = useEdit();
  if (b.fx.type !== "multiband") return null;
  const m = b.fx;
  return (
    <>
      <Seg
        className="quiet sm"
        label="Bands"
        value={m.bands.length}
        items={[2, 3, 4, 5, 6].map((v) => ({ value: v, label: String(v) }))}
        onChange={(v) => void edit({ op: "chain", track: t.id, change: { op: "bands", id: b.id, bands: v } }, `${t.name} · multiband ${v} bands`)}
      />
      <div className="knobs wrap">
        {m.crossovers_hz.map((_, i) => (
          <BKnob key={i} t={t} b={b} k={`xover.${i + 1}`} name={`Split ${i + 1}`} />
        ))}
      </div>
      {m.bands.map((_, i) => (
        <div key={i} className="col g1">
          <span className="b s1">Band {i + 1}</span>
          <div className="knobs wrap">
            <BKnob t={t} b={b} k={`${i + 1}.threshold`} name="Threshold" />
            <BKnob t={t} b={b} k={`${i + 1}.ratio`} name="Ratio" />
            <BKnob t={t} b={b} k={`${i + 1}.attack`} name="Attack" />
            <BKnob t={t} b={b} k={`${i + 1}.release`} name="Release" />
            <BKnob t={t} b={b} k={`${i + 1}.gain`} name="Gain" bip />
          </div>
        </div>
      ))}
      <Info>The master's multiband on this track: crossovers that sum flat, a compressor per range.</Info>
    </>
  );
}

function ClipperEditor({ t, b }: EditorProps) {
  if (b.fx.type !== "clipper") return null;
  return (
    <>
      <div className="knobs">
        <BKnob t={t} b={b} k="threshold" name="Threshold" />
        <BKnob t={t} b={b} k="softness" name="Softness" />
      </div>
      <Info>Shaves what goes above the threshold, four times oversampled. On a track the threshold is absolute (dBFS).</Info>
    </>
  );
}

export const FX_EDITORS: Partial<Record<FxKind, FC<EditorProps>>> = {
  gate: GateEditor,
  deesser: DeesserEditor,
  dynamic_eq: DynEqEditor,
  transient: TransientEditor,
  saturation: SaturationEditor,
  exciter: ExciterEditor,
  bass_enhancer: BassEditor,
  width: WidthEditor,
  modulation: ModulationEditor,
  tremolo: TremoloEditor,
  harmonizer: HarmonizerEditor,
  doubler: DoublerEditor,
  multiband: MultibandEditor,
  clipper: ClipperEditor,
};
