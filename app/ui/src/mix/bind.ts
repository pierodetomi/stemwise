// Binds controls to the core: a track parameter (gesture protocol, HIS-2), an automated parameter (AUT-3: the
// control shows the curve's value and moving it offsets the whole curve), or an edit (mix bus, sends).
import { useMemo } from "react";
import type { AutomationPoint } from "../bindings/AutomationPoint";
import type { Edit } from "../bindings/Edit";
import type { Track } from "../bindings/Track";
import type { Ctl } from "../shared/controls";
import { specOf } from "../state/store";
import { blockOf, fxGet } from "./fx";
import { useApp, useBackend } from "../state/context";

/** The stored value of a parameter of a track (a read of the mirror, no logic). */
export function paramValue(t: Track, id: string): number {
  const c = t.chain;
  const dot = id.indexOf(".");
  const [a, b] = dot < 0 ? [id, ""] : [id.slice(0, dot), id.slice(dot + 1)];
  switch (a) {
    case "fader":
      return c.fader_db;
    case "pan":
      return c.pan;
    case "trim":
      return c.trim_db;
    case "mute":
      return t.mute ? 1 : 0;
    case "send":
      return t.sends.find((s) => s.target === id.slice(5))?.level_db ?? -144;
    case "bypass":
      if (b === "fx") return t.ret?.bypass ? 1 : 0;
      return (blockOf(t, b)?.fx.bypass ?? true) ? 1 : 0;
    case "rev":
      if (t.ret?.type !== "reverb") return 0;
      return (
        { decay: t.ret.decay_s, size: t.ret.size, damping: t.ret.damping, modulation: t.ret.modulation, predelay: t.ret.predelay_ms, hpf: t.ret.hpf_hz, lpf: t.ret.lpf_hz, shimmer: t.ret.shimmer_amount } as Record<string, number>
      )[b];
    case "dly":
      if (t.ret?.type !== "delay") return 0;
      return ({ time: t.ret.time_ms, feedback: t.ret.feedback, fb_hpf: t.ret.feedback_hpf_hz, fb_lpf: t.ret.feedback_lpf_hz, wow: t.ret.wow, drive: t.ret.drive_db } as Record<string, number>)[b];
    default: {
      const blk = blockOf(t, a);
      if (!blk) return 0;
      // A compressor without a sidechain still has its detector low cut at the default (the core's engine layout).
      return fxGet(blk.fx, b) ?? (b === "sc_hpf" ? 100 : 0);
    }
  }
}

/** Value of a lane at a frame, for display (the engine computes the sound). */
export function laneValue(points: AutomationPoint[], frame: number, stepped: boolean): number | null {
  if (!points.length) return null;
  if (frame <= points[0].frame) return points[0].value;
  for (let i = 1; i < points.length; i++) {
    const b = points[i];
    if (frame < b.frame) {
      const a = points[i - 1];
      if (stepped || b.frame === a.frame) return a.value;
      return a.value + ((b.value - a.value) * (frame - a.frame)) / (b.frame - a.frame);
    }
  }
  return points[points.length - 1].value;
}

/** A control on a track parameter; automated parameters show and offset their curve (AUT-3). */
export function useTrackCtl(t: Track, id: string, label?: string): Ctl {
  const backend = useBackend();
  const specs = useApp((s) => s.specs);
  const setError = useApp((s) => s.setError);
  const lane = t.automation.find((l) => l.param === id && l.points.length > 0);
  const frame = useApp((s) => (lane ? Math.round((s.meters?.frame ?? 0) / 256) * 256 : 0));
  return useMemo(() => {
    const spec = specOf(specs, id);
    const what = label ?? spec.label;
    const name = `${t.name} · ${what.charAt(0).toUpperCase()}${what.slice(1)}`;
    const fail = (e: unknown) => setError(String(e));
    if (lane) {
      const shown = laneValue(lane.points, frame, spec.stepped) ?? paramValue(t, id);
      let start = shown;
      return {
        spec, label: name, value: shown, auto: true, lock: t.locked,
        begin: () => {
          start = shown;
          backend.beginGesture(`${name} · automation offset`).catch(fail);
        },
        change: (v) => backend.gestureEdit({ op: "offset_lane", track: t.id, param: id, delta: v - start }).catch(fail),
        end: (cancel) => backend.endGesture(cancel).catch(fail),
        set: (v) => backend.edit({ op: "offset_lane", track: t.id, param: id, delta: v - shown }, `${name} · automation offset`).catch(fail),
      };
    }
    return {
      spec, label: name, value: paramValue(t, id), lock: t.locked,
      begin: () => backend.beginGesture(name).catch(fail),
      change: (v) => backend.gestureParam(t.id, id, v).catch(fail),
      end: (cancel) => backend.endGesture(cancel).catch(fail),
      set: (v) => backend.setParam(t.id, id, v, name).catch(fail),
    };
  }, [backend, specs, setError, t, id, label, lane, frame]);
}

/** A control driving an edit (the mix bus fader, a send level): the gesture replays the edit from its start. */
export function useEditCtl(specId: string, label: string, value: number, make: (v: number) => Edit): Ctl {
  const backend = useBackend();
  const specs = useApp((s) => s.specs);
  const setError = useApp((s) => s.setError);
  return useMemo(() => {
    const fail = (e: unknown) => setError(String(e));
    return {
      spec: specOf(specs, specId), label, value,
      begin: () => backend.beginGesture(label).catch(fail),
      change: (v) => backend.gestureEdit(make(v)).catch(fail),
      end: (cancel) => backend.endGesture(cancel).catch(fail),
      set: (v) => backend.edit(make(v), label).catch(fail),
    };
  }, [backend, specs, setError, specId, label, value, make]);
}

/** Runs an edit as one step, reporting errors as a toast (UI-6). */
export function useEdit() {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  return (e: Edit, label: string) => backend.edit(e, label).catch((x) => setError(String(x)));
}
