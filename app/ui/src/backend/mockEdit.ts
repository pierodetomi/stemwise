// Coarse in-memory version of the core's edits and parameters for the mock backend (FOUNDATION §5.6). It exists so
// the browser build, Playwright and the visual checks can exercise every screen; the real rules live in Rust
// (`stemwise_core::edit`, `params`) and are tested there. Never imported by the UI itself.
import type { Edit } from "../bindings/Edit";
import type { Clip } from "../bindings/Clip";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { chainEdit } from "./mockFx";
import { fxGet, fxSet } from "../mix/fx";
import { paramValue } from "../mix/bind";
import type { ReturnFx } from "../bindings/ReturnFx";
import type { ReverbAlgorithm } from "../bindings/ReverbAlgorithm";

const end = (c: Clip) => c.start + c.length;
const trackOf = (p: Project, id: string): Track => {
  const t = p.tracks.find((x) => x.id === id);
  if (!t) throw new Error(`track \`${id}\` does not exist`);
  return t;
};
const newId = (p: Project, prefix: string) => `${prefix}${p.next_id++}`;

/** CLP-4: clips under `incoming` are trimmed, split or removed. */
function place(p: Project, existing: Clip[], incoming: Clip[]): Clip[] {
  let out = existing.slice();
  for (const n of incoming) {
    const a = n.start;
    const b = end(n);
    const next: Clip[] = [];
    for (const c of out) {
      const s = c.start;
      const e = end(c);
      if (e <= a || s >= b) next.push(c);
      else if (s >= a && e <= b) continue;
      else if (s < a && e > b) {
        next.push({ ...c, length: a - s, fade_out: { ...c.fade_out, length: 0 } });
        next.push({ ...c, id: newId(p, "c"), start: b, offset: c.offset + (b - s), length: e - b, fade_in: { ...c.fade_in, length: 0 } });
      } else if (s < a) next.push({ ...c, length: a - s });
      else next.push({ ...c, start: b, offset: c.offset + (b - s), length: e - b });
    }
    next.push(n);
    out = next;
  }
  return out.sort((x, y) => x.start - y.start);
}

export function setParam(p: Project, track: string, param: string, value: number) {
  const t = trackOf(p, track);
  const c = t.chain;
  const on = value >= 0.5;
  const dot = param.indexOf(".");
  const [a, b] = dot < 0 ? [param, ""] : [param.slice(0, dot), param.slice(dot + 1)];
  if (param === "fader") c.fader_db = value;
  else if (param === "pan") c.pan = value;
  else if (param === "trim") c.trim_db = value;
  else if (param === "mute") t.mute = on;
  else if (a === "send") {
    const s = t.sends.find((x) => x.target === param.slice(5));
    if (s) s.level_db = value;
  } else if (a === "bypass") {
    if (b === "fx" && t.ret) t.ret.bypass = on;
    else {
      const blk = c.blocks.find((x) => x.id === b);
      if (!blk) throw new Error(`parameter \`${param}\` does not exist on this track`);
      blk.fx.bypass = on;
    }
  } else if ((a === "rev" || a === "dly") && t.ret) {
    const r = t.ret as Record<string, unknown> & ReturnFx;
    const m: Record<string, string> = {
      "rev.decay": "decay_s", "rev.size": "size", "rev.damping": "damping", "rev.modulation": "modulation", "rev.predelay": "predelay_ms", "rev.hpf": "hpf_hz", "rev.lpf": "lpf_hz", "rev.shimmer": "shimmer_amount",
      "dly.time": "time_ms", "dly.feedback": "feedback", "dly.fb_hpf": "feedback_hpf_hz", "dly.fb_lpf": "feedback_lpf_hz", "dly.wow": "wow", "dly.drive": "drive_db",
    };
    if (m[param]) r[m[param]] = value;
    if (param === "dly.time" && r.type === "delay") r.sync = null;
  } else {
    const blk = c.blocks.find((x) => x.id === a);
    if (!blk || !fxSet(blk.fx, b, value)) throw new Error(`parameter \`${param}\` does not exist on this track`);
  }
}

/** A reverb with the starting values of its algorithm (the core's `ReturnFx::reverb`). */
export function reverbOf(algorithm: ReverbAlgorithm): ReturnFx {
  const [decay_s, predelay_ms, hpf_hz, lpf_hz] = { plate: [1.2, 30, 300, 12000], room: [0.6, 5, 200, 10000], hall: [2.0, 20, 250, 9000], shimmer: [3.5, 30, 300, 10000] }[algorithm];
  return { type: "reverb", bypass: false, algorithm, decay_s, size: 0.5, damping: 0.5, modulation: 0.2, predelay_ms, hpf_hz, lpf_hz, shimmer_semitones: 12, shimmer_amount: algorithm === "shimmer" ? 0.4 : 0 };
}

export function getParam(t: Track, param: string): number | null {
  if (param.startsWith("send.")) return t.sends.find((x) => x.target === param.slice(5))?.level_db ?? null;
  if (param.startsWith("bypass.") && param !== "bypass.fx" && !t.chain.blocks.some((b) => b.id === param.slice(7))) return null;
  const dot = param.indexOf(".");
  const a = dot < 0 ? param : param.slice(0, dot);
  if (!["fader", "pan", "trim", "mute", "bypass", "rev", "dly"].includes(a)) {
    const blk = t.chain.blocks.find((x) => x.id === a);
    return blk ? fxGet(blk.fx, param.slice(dot + 1)) : null;
  }
  return paramValue(t, param);
}

export function runEdit(p: Project, e: Edit): void {
  switch (e.op) {
    case "move_clips": {
      const tracks = new Set(e.clips.map(([t]) => t));
      if (e.to_track) {
        const d = trackOf(p, e.to_track);
        for (const [t] of e.clips) if (trackOf(p, t).channels !== d.channels) throw new Error("clips can only move between tracks with the same channel count");
        tracks.add(e.to_track);
      }
      const moving = e.clips.map(([t, c]) => {
        const clip = trackOf(p, t).clips.find((x) => x.id === c);
        if (!clip) throw new Error(`clip \`${c}\` does not exist`);
        return { dest: e.to_track ?? t, clip: { ...clip, id: e.duplicate ? newId(p, "c") : clip.id, start: Math.max(0, clip.start + e.delta) } };
      });
      for (const id of tracks) {
        const t = trackOf(p, id);
        const base = e.duplicate ? t.clips : t.clips.filter((c) => !e.clips.some(([x, y]) => x === id && y === c.id));
        t.clips = place(p, base, moving.filter((m) => m.dest === id).map((m) => m.clip));
      }
      return;
    }
    case "split_at": {
      const ids = e.tracks.length ? e.tracks : p.tracks.filter((t) => t.kind === "audio").map((t) => t.id);
      for (const id of ids) {
        const t = trackOf(p, id);
        t.clips = t.clips.flatMap((c) =>
          c.start < e.frame && end(c) > e.frame
            ? [
                { ...c, length: e.frame - c.start, fade_out: { ...c.fade_out, length: 0 } },
                { ...c, id: newId(p, "c"), start: e.frame, offset: c.offset + (e.frame - c.start), length: end(c) - e.frame, fade_in: { ...c.fade_in, length: 0 } },
              ]
            : [c],
        );
      }
      return;
    }
    case "trim_clip": {
      const t = trackOf(p, e.clip[0]);
      const c = t.clips.find((x) => x.id === e.clip[1]);
      if (!c) throw new Error("no clip");
      const media = p.media.find((m) => m.id === c.media)?.frames ?? c.offset + c.length;
      const n = { ...c };
      if (e.left) {
        const to = Math.min(Math.max(e.to, Math.max(0, c.start - c.offset)), end(c) - 1);
        n.offset = c.offset + (to - c.start);
        n.length = end(c) - to;
        n.start = to;
      } else {
        const to = Math.min(Math.max(e.to, c.start + 1), c.start + (media - c.offset));
        n.length = to - c.start;
      }
      t.clips = place(p, t.clips.filter((x) => x.id !== c.id), [n]);
      return;
    }
    case "duplicate_clips": {
      if (!e.clips.length) return;
      const all = e.clips.map(([t, c]) => trackOf(p, t).clips.find((x) => x.id === c)!);
      const lo = Math.min(...all.map((c) => c.start));
      const hi = Math.max(...all.map(end));
      runEdit(p, { op: "move_clips", clips: e.clips, delta: hi - lo, to_track: null, duplicate: true });
      return;
    }
    case "delete_clips":
      for (const t of p.tracks) t.clips = t.clips.filter((c) => !e.clips.some(([x, y]) => x === t.id && y === c.id));
      return;
    case "clip_gain": {
      if (e.gain_db < -24 || e.gain_db > 24) throw new Error("clip gain must be within −24 … +24 dB");
      const c = trackOf(p, e.clip[0]).clips.find((x) => x.id === e.clip[1]);
      if (c) c.gain_db = e.gain_db;
      return;
    }
    case "clip_mute":
      for (const [t, id] of e.clips) {
        const c = trackOf(p, t).clips.find((x) => x.id === id);
        if (c) c.mute = e.mute;
      }
      return;
    case "fade": {
      const c = trackOf(p, e.clip[0]).clips.find((x) => x.id === e.clip[1]);
      if (!c) return;
      const f = { length: Math.min(e.length, c.length), shape: e.shape };
      if (e.fade_in) c.fade_in = f;
      else c.fade_out = f;
      return;
    }
    case "add_group": {
      if (e.members.some((m) => trackOf(p, m).kind === "return")) throw new Error("a return cannot go into a group");
      const g: Track = {
        ...structuredClone(trackOf(p, p.tracks[0].id)),
        id: newId(p, "g"), kind: "group", name: e.name, color: 0, role: null, channels: 2, clips: [], ret: null, sends: [], automation: [], locked: false, mute: false, solo: false,
        output: e.members.length ? trackOf(p, e.members[0]).output : { kind: "mix_bus" },
      };
      const idx = e.members.length ? Math.min(...e.members.map((m) => p.tracks.findIndex((t) => t.id === m))) : p.tracks.length;
      p.tracks.splice(idx, 0, g);
      for (const m of e.members) trackOf(p, m).output = { kind: "group", id: g.id };
      return;
    }
    case "add_return": {
      const returns = p.tracks.filter((t) => t.kind === "return");
      if (returns.length >= 8) throw new Error("a project holds at most 8 returns");
      const proto = returns.find((t) => (t.ret?.type === "reverb") === e.reverb);
      const n = returns.filter((t) => (t.ret?.type === "reverb") === e.reverb).length + 1;
      const ret: ReturnFx = e.reverb
        ? reverbOf(e.algorithm ?? "hall")
        : { type: "delay", bypass: false, time_ms: 375, sync: null, feedback: 0.25, feedback_hpf_hz: 200, feedback_lpf_hz: 6000, mode: "stereo", character: "clean", wow: 0, drive_db: 0 };
      p.tracks.push({
        ...structuredClone(proto ?? p.tracks[0]),
        id: newId(p, "r"), kind: "return", name: `${e.reverb ? "Reverb" : "Delay"} ${n}`, color: 0, role: null, channels: 2, clips: [], ret, sends: [], automation: [], chain: { ...structuredClone((proto ?? p.tracks[0]).chain), blocks: [] },
        output: { kind: "mix_bus" }, locked: false, mute: false, solo: false,
      });
      return;
    }
    case "delete_track": {
      const v = trackOf(p, e.track);
      for (const t of p.tracks) {
        if (t.output.kind === "group" && t.output.id === e.track) t.output = v.output;
        t.sends = t.sends.filter((s) => s.target !== e.track);
        for (const b of t.chain.blocks) {
          const fx = b.fx;
          if (fx.type === "compressor" && fx.sidechain?.source === e.track) fx.sidechain = null;
          if (fx.type === "gate" && fx.sidechain === e.track) fx.sidechain = null;
          if (fx.type === "dynamic_eq") fx.bands.forEach((x) => x.sidechain === e.track && (x.sidechain = null));
        }
      }
      p.tracks = p.tracks.filter((t) => t.id !== e.track);
      return;
    }
    case "duplicate_track": {
      const t = trackOf(p, e.track);
      const i = p.tracks.indexOf(t);
      const n = structuredClone(t);
      n.id = newId(p, t.kind === "audio" ? "t" : t.kind === "group" ? "g" : "r");
      n.name = `${t.name} copy`;
      n.locked = false;
      n.clips.forEach((c) => (c.id = newId(p, "c")));
      p.tracks.splice(i + 1, 0, n);
      return;
    }
    case "rename_project":
      if (!e.name.trim()) throw new Error("the project needs a name");
      p.name = e.name.trim();
      return;
    case "place_tracks": {
      // Mirrors stemwise-core edit::place_tracks: same checks, same result.
      const moving = p.tracks.filter((t) => e.tracks.includes(t.id));
      if (moving.length !== e.tracks.length) throw new Error(`no track ${e.tracks.find((id) => !p.tracks.some((t) => t.id === id))}`);
      if (moving.some((t) => t.kind === "return")) throw new Error("a return cannot be moved into a group");
      if (e.output.kind === "group") {
        let o: Track["output"] = e.output;
        for (let i = 0; i <= p.tracks.length && o.kind === "group"; i++) {
          const gid: string = o.id;
          if (moving.some((t) => t.id === gid)) throw new Error("a group cannot go inside itself");
          o = trackOf(p, gid).output;
        }
      }
      const rest = p.tracks.filter((t) => !moving.includes(t));
      const at = e.before ? rest.findIndex((t) => t.id === e.before) : -1;
      rest.splice(at < 0 ? rest.length : at, 0, ...moving);
      p.tracks = rest;
      for (const t of moving) t.output = structuredClone(e.output);
      return;
    }
    case "move_track": {
      const t = trackOf(p, e.track);
      p.tracks = p.tracks.filter((x) => x !== t);
      p.tracks.splice(Math.min(e.index, p.tracks.length), 0, t);
      return;
    }
    case "rename_track": {
      const t = trackOf(p, e.track);
      t.name = e.name;
      if (e.color !== null) t.color = e.color;
      return;
    }
    case "set_locked":
      trackOf(p, e.track).locked = e.locked;
      return;
    case "set_output": {
      if (e.output.kind === "group") {
        // Refuse cycles like the core does (TRK-3).
        let g: string | null = e.output.id;
        while (g) {
          if (g === e.track) throw new Error("this routing would create a loop");
          const o: Track["output"] = trackOf(p, g).output;
          g = o.kind === "group" ? o.id : null;
        }
      }
      trackOf(p, e.track).output = e.output;
      return;
    }
    case "set_send": {
      const t = trackOf(p, e.track);
      if (e.level_db === null) t.sends = t.sends.filter((s) => s.target !== e.target);
      else {
        const s = t.sends.find((x) => x.target === e.target);
        if (s) s.level_db = e.level_db;
        else t.sends.push({ target: e.target, level_db: e.level_db });
      }
      return;
    }
    case "set_sidechain": {
      const t = trackOf(p, e.track);
      const blk = e.block ? t.chain.blocks.find((b) => b.id === e.block) : t.chain.blocks.find((b) => b.fx.type === "compressor");
      if (!blk) throw new Error("no such block");
      const fx = blk.fx;
      if (fx.type === "compressor") fx.sidechain = e.source ? { source: e.source, hpf_hz: fx.sidechain?.hpf_hz ?? 100 } : null;
      else if (fx.type === "gate") fx.sidechain = e.source;
      else if (fx.type === "dynamic_eq") fx.bands[e.band ?? 0].sidechain = e.source;
      else throw new Error("this block has no sidechain");
      return;
    }
    case "set_eq_band": {
      const t = trackOf(p, e.track);
      const blk = e.block ? t.chain.blocks.find((b) => b.id === e.block) : t.chain.blocks.find((b) => b.fx.type === "eq");
      if (!blk || (blk.fx.type !== "eq" && blk.fx.type !== "dynamic_eq")) throw new Error("no such EQ");
      const b = blk.fx.bands[e.band];
      b.enabled = e.enabled;
      b.kind = e.kind;
      return;
    }
    case "chain": {
      const t = trackOf(p, e.track);
      const before = t.chain.blocks.map((b) => b.id);
      chainEdit(t.chain, e.change, t.kind === "return");
      const gone = before.filter((id) => !t.chain.blocks.some((b) => b.id === id));
      t.automation = t.automation.filter((l) => !gone.includes((l.param.startsWith("bypass.") ? l.param.slice(7) : l.param).split(".")[0]));
      return;
    }
    case "set_block": {
      const t = trackOf(p, e.track);
      const i = t.chain.blocks.findIndex((b) => b.id === e.block.id);
      if (i < 0) throw new Error("no such block");
      if (t.chain.blocks[i].fx.type !== e.block.fx.type) throw new Error("a block keeps its kind");
      t.chain.blocks[i] = structuredClone(e.block);
      return;
    }
    case "set_return": {
      const t = trackOf(p, e.track);
      if (t.ret?.type !== e.ret.type) throw new Error("a return keeps its type");
      t.ret = structuredClone(e.ret);
      return;
    }
    case "set_reverb_algorithm": {
      const r = trackOf(p, e.track).ret;
      if (r?.type !== "reverb") throw new Error("not a reverb return");
      r.algorithm = e.algorithm;
      return;
    }
    case "set_delay_sync": {
      const r = trackOf(p, e.track).ret;
      if (r?.type !== "delay") throw new Error("only a delay return has a musical time");
      r.sync = e.sync;
      if (e.sync && p.tempo) {
        const m = /^1\/(\d+)([.t])?$/.exec(e.sync);
        if (m) r.time_ms = ((60000 / p.tempo.bpm) * 4) / Number(m[1]) * (m[2] === "." ? 1.5 : m[2] === "t" ? 2 / 3 : 1);
      }
      return;
    }
    case "add_section": {
      if (e.end <= e.start) throw new Error("a section needs a length");
      if (p.sections.some((s) => s.start < e.end && e.start < s.end)) throw new Error("sections cannot overlap");
      p.sections.push({ id: newId(p, "s"), name: e.name, color: (p.sections.length % 12) + 1, start: e.start, end: e.end });
      p.sections.sort((a, b) => a.start - b.start);
      return;
    }
    case "move_boundary":
    case "move_section_start": {
      const ss = p.sections.slice().sort((a, b) => a.start - b.start);
      let i = ss.findIndex((s) => s.id === e.section);
      if (i < 0) throw new Error("no section");
      if (e.op === "move_section_start") {
        if (i > 0 && ss[i - 1].end === ss[i].start) i -= 1;
        else {
          ss[i].start = Math.min(Math.max(e.to, i > 0 ? ss[i - 1].end : 0), ss[i].end - 1);
          return;
        }
      }
      const to = Math.max(e.to, ss[i].start + 1);
      const next = ss[i + 1];
      if (next && next.start === ss[i].end) {
        if (to >= next.end) throw new Error("the boundary would cross the next section");
        next.start = to;
      } else if (next && to > next.start) throw new Error("sections cannot overlap");
      ss[i].end = to;
      return;
    }
    case "rename_section": {
      const s = p.sections.find((x) => x.id === e.section);
      if (s) s.name = e.name;
      return;
    }
    case "delete_section":
      p.sections = p.sections.filter((s) => s.id !== e.section);
      return;
    case "set_lane": {
      const t = trackOf(p, e.track);
      const pts = e.points.slice().sort((a, b) => a.frame - b.frame);
      const l = t.automation.find((x) => x.param === e.param);
      if (l) {
        l.points = pts;
        l.author = "user";
      } else t.automation.push({ param: e.param, points: pts, author: "user", visible: true });
      return;
    }
    case "draw_lane": {
      if (!e.stroke.length) return;
      const s = e.stroke.slice().sort((a, b) => a.frame - b.frame);
      const a = s[0].frame;
      const b = s[s.length - 1].frame;
      const t = trackOf(p, e.track);
      const old = t.automation.find((l) => l.param === e.param)?.points ?? [];
      // Coarse decimation stands in for the core's simplification.
      const kept = s.filter((_, i) => i === 0 || i === s.length - 1 || i % 8 === 0);
      runEdit(p, { op: "set_lane", track: e.track, param: e.param, points: [...old.filter((q) => q.frame < a || q.frame > b), ...kept] });
      return;
    }
    case "paste_lane": {
      if (!e.points.length) return;
      const len = e.points[e.points.length - 1].frame;
      const old = trackOf(p, e.track).automation.find((l) => l.param === e.param)?.points ?? [];
      const pts = [...old.filter((q) => q.frame < e.at || q.frame > e.at + len), ...e.points.map((q) => ({ frame: q.frame + e.at, value: q.value }))];
      runEdit(p, { op: "set_lane", track: e.track, param: e.param, points: pts });
      return;
    }
    case "delete_lane_range": {
      const old = trackOf(p, e.track).automation.find((l) => l.param === e.param)?.points ?? [];
      runEdit(p, { op: "set_lane", track: e.track, param: e.param, points: old.filter((q) => q.frame < e.from || q.frame > e.to) });
      return;
    }
    case "set_role":
      trackOf(p, e.track).role = e.role;
      return;
    case "remove_lane": {
      const t = trackOf(p, e.track);
      t.automation = t.automation.filter((l) => l.param !== e.param);
      return;
    }
    case "offset_lane": {
      const l = trackOf(p, e.track).automation.find((x) => x.param === e.param);
      if (!l) throw new Error("no lane");
      l.points = l.points.map((q) => ({ ...q, value: q.value + e.delta }));
      l.author = "user";
      return;
    }
    case "set_tempo":
      p.tempo = e.tempo;
      return;
    case "set_mix_bus":
      p.mix_bus.fader_db = e.fader_db;
      return;
    case "set_brief":
      p.brief = e.brief;
      return;
    case "set_master": {
      // As the core: an invalid ISRC is refused, the mixdown is not the edit's to change (MasterState::check).
      const isrc = e.master.metadata.isrc.trim();
      if (isrc && !/^([A-Za-z]{2}[A-Za-z0-9]{3}\d{7}|[A-Za-z]{2}-[A-Za-z0-9]{3}-\d{2}-\d{5})$/.test(isrc)) throw new Error("not an ISRC: use CC-XXX-YY-NNNNN or CCXXXYYNNNNN");
      p.master = { ...e.master, mixdown: p.master.mixdown };
      return;
    }
    default: {
      // Every edit of the core has a mock counterpart: a new variant fails to compile here.
      const never: never = e;
      throw new Error(`unknown edit ${JSON.stringify(never)}`);
    }
  }
}
