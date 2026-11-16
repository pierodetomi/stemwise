// Display formatting and control mapping only (FOUNDATION §5.6: no domain logic in the UI). Ranges come from the
// core's parameter specs; nothing here decides what a value means for the sound.
import type { ParamSpec } from "../bindings/ParamSpec";

/** m:ss.cc from frames. */
export function formatTime(frames: number, rate: number): string {
  const t = Math.max(0, frames / rate);
  const m = Math.floor(t / 60);
  const s = t - m * 60;
  return `${m}:${s.toFixed(2).padStart(5, "0")}`;
}

/** m:ss from seconds (ruler ticks, durations). */
export function formatClock(seconds: number): string {
  const t = Math.max(0, Math.round(seconds));
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
}

const minus = (s: string) => s.replace("-", "−");

/** dB with sign and one decimal; −∞ under −120. */
export function formatDb(db: number): string {
  if (db <= -120) return "−∞";
  const v = Math.abs(db) < 0.05 ? 0 : db;
  return `${v > 0 ? "+" : v < 0 ? "−" : ""}${Math.abs(v).toFixed(1)}`;
}

export function formatHz(hz: number): string {
  return hz >= 1000 ? `${(hz / 1000).toFixed(hz >= 10000 ? 1 : 1).replace(/\.0$/, "")} kHz` : `${Math.round(hz)} Hz`;
}

export function formatPan(p: number): string {
  if (Math.abs(p) < 0.005) return "C";
  return `${p < 0 ? "L" : "R"}${Math.round(Math.abs(p) * 100)}`;
}

/** A parameter value with its unit, as the controls show it. */
export function formatParam(spec: Pick<ParamSpec, "id" | "unit">, v: number): string {
  switch (spec.unit) {
    case "db":
      return `${formatDb(v)} dB`;
    case "hz":
      return formatHz(v);
    case "ms":
      return `${v < 10 ? v.toFixed(1) : Math.round(v)} ms`;
    case "s":
      return `${v < 1 ? v.toFixed(2) : v.toFixed(1)} s`;
    case "ratio":
      return spec.id === "comp.ratio" ? `${v.toFixed(1)}:1` : v.toFixed(2);
    case "percent":
      return `${Math.round(v * 100)} %`;
    case "pan":
      return formatPan(v);
    case "toggle":
      return v >= 0.5 ? "On" : "Off";
    default:
      return minus(v.toFixed(1));
  }
}

/** The number a user types for a value (same units as shown, without the unit). */
export function editText(spec: Pick<ParamSpec, "id" | "unit">, v: number): string {
  switch (spec.unit) {
    case "percent":
      return String(Math.round(v * 1000) / 10);
    case "pan":
      return String(Math.round(v * 100));
    case "db":
      return v <= -120 ? "-inf" : v.toFixed(1);
    case "hz":
      return String(Math.round(v));
    default:
      return String(Math.round(v * 100) / 100);
  }
}

/** Parses typed text into a value; `null` when it is not a number in range (UI-5: out of range is not applied). */
export function parseTyped(spec: ParamSpec, text: string): number | null {
  const t = text.trim().toLowerCase().replace("−", "-").replace(",", ".");
  if (spec.unit === "db" && (t === "-inf" || t === "-∞")) return spec.min;
  let v: number;
  if (spec.unit === "pan") {
    const m = /^([lrc])?\s*(-?\d+(\.\d+)?)?$/.exec(t);
    if (!m) return null;
    if (m[1] === "c" || (!m[1] && !m[2])) v = 0;
    else {
      const n = Number(m[2] ?? "0") / 100;
      v = m[1] === "l" ? -n : n;
    }
  } else {
    const k = /^(-?\d+(\.\d+)?)\s*(k)?/.exec(t);
    if (!k) return null;
    v = Number(k[1]) * (k[3] ? 1000 : 1);
    if (spec.unit === "percent") v /= 100;
  }
  if (!Number.isFinite(v) || v < spec.min - 1e-9 || v > spec.max + 1e-9) return null;
  return v;
}

/** Control position 0…1 of a value (log for frequencies and times). */
export function toNorm(spec: ParamSpec, v: number): number {
  if (spec.id === "fader" || spec.id.startsWith("send.") || spec.id === "bus") return faderPos(v);
  const c = Math.min(spec.max, Math.max(spec.min, v));
  if (spec.log && spec.min > 0) return Math.log(c / spec.min) / Math.log(spec.max / spec.min);
  return (c - spec.min) / (spec.max - spec.min);
}

export function fromNorm(spec: ParamSpec, n: number): number {
  const c = Math.min(1, Math.max(0, n));
  if (spec.id === "fader" || spec.id.startsWith("send.") || spec.id === "bus") return Math.min(spec.max, faderDb(c));
  if (spec.log && spec.min > 0) return spec.min * Math.pow(spec.max / spec.min, c);
  return spec.min + c * (spec.max - spec.min);
}

// Fader taper (drawing only): most of the travel covers −48…+12 dB, the bottom 15 % reaches −∞.
const TAPER_KNEE = 0.15;
export function faderPos(db: number): number {
  if (db >= -48) return Math.min(1, TAPER_KNEE + ((1 - TAPER_KNEE) * (db + 48)) / 60);
  if (db <= -144) return 0;
  return TAPER_KNEE * Math.pow((db + 144) / 96, 3);
}
export function faderDb(pos: number): number {
  if (pos >= TAPER_KNEE) return -48 + ((pos - TAPER_KNEE) / (1 - TAPER_KNEE)) * 60;
  if (pos <= 0) return -144;
  return -144 + 96 * Math.cbrt(pos / TAPER_KNEE);
}

/** Meter height 0…1 for a level in dBFS (−60 dB floor). */
export function meterPos(db: number): number {
  return Math.min(1, Math.max(0, (db + 60) / 60));
}

/** Position as bars · beats · ticks (1-based), from the project's tempo (SET-1, appearance). */
export function formatBars(frames: number, rate: number, tempo: { bpm: number; beats_per_bar: number; beat_unit: number }): string {
  const beatSec = (60 / tempo.bpm) * (4 / tempo.beat_unit);
  const beats = frames / rate / beatSec;
  const bar = Math.floor(beats / tempo.beats_per_bar) + 1;
  const beat = Math.floor(beats % tempo.beats_per_bar) + 1;
  const tick = Math.floor((beats % 1) * 100);
  return `${bar}.${beat}.${String(tick).padStart(2, "0")}`;
}
