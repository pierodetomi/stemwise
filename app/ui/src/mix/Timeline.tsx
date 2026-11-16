// Timeline (design 3a, 6a–6f, 7c): tools, ruler with loop, sections lane, track rows with clips and waveforms,
// automation lanes, playhead. Every gesture is previewed locally and becomes one edit when it ends (HIS-2); Esc
// cancels it. Snap (CLP-3) is computed here for the preview (A-017); the core decides what the edit does.
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent as RPE } from "react";
import { keysInText } from "../shared/keys";
import type { AutomationPoint } from "../bindings/AutomationPoint";
import type { Clip } from "../bindings/Clip";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { Icon } from "../shared/Icon";
import { MiniFader, Toggle } from "../shared/controls";
import { formatClock, formatParam, formatTime, fromNorm, toNorm } from "../shared/format";
import { useMenu, type MenuItem } from "../shared/Menu";
import { clipKey, specOf, splitKey, type ClipKey, type Tool } from "../state/store";
import { useApp, useAppShallow, useBackend, useStore } from "../state/context";
import { laneValue, useEdit, useTrackCtl } from "./bind";
import { gridStep, headWidth, snapFrame, snapTargets, songFrames, treeRows, type Row } from "./geometry";
import { AddMenu, paramMenu, paramLabel, trackMenu } from "./menus";
import { HeaderMarks, ReturnFeeds } from "./Marks";
import { useRelations } from "./Mixer";
import { keysOf, shortcutOf } from "../shared/shortcuts";
import { MissingBanner } from "../shared/StateBanners";
import { Waveform } from "./Waveform";
import { useScrub } from "../shared/scrub";

const LANE_PX = { s: 26, m: 38, l: 60 } as const;
const AUTO_H = 56;
const EDGE = 6;
/** How near the playhead (px) a press on the ruler grabs it to scrub. */
const HEAD_GRAB = 7;

type Drag =
  | { kind: "move"; keys: ClipKey[]; x0: number; y0: number; delta: number; toTrack: string | null; dup: boolean; snap: string | null; snapAt: number | null; moved: boolean }
  | { kind: "trim"; key: ClipKey; left: boolean; to: number; snap: string | null }
  | { kind: "fade"; key: ClipKey; fadeIn: boolean; length: number; linear: boolean }
  | { kind: "marquee"; x0: number; y0: number; x1: number; y1: number; add: boolean }
  | { kind: "loop"; a: number; b: number }
  | { kind: "section-new"; a: number; b: number }
  | { kind: "section-edge"; id: string; end: boolean; to: number }
  | { kind: "point"; track: string; param: string; index: number; points: AutomationPoint[] }
  | { kind: "draw"; track: string; param: string; stroke: AutomationPoint[] }
  | { kind: "range"; track: string; param: string; a: number; b: number };

/** Where dragged tracks would land: before `before` (the end if none), routed to `output`; the drop line is drawn at
 * `y` (inside the rows), indented by `depth`, or the group `into` is highlighted. */
type Place = { output: Track["output"]; before: string | null; into: string | null; y: number; depth: number };

function useSize(ref: React.RefObject<HTMLElement | null>) {
  const [w, setW] = useState(800);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setW(el.clientWidth));
    ro.observe(el);
    setW(el.clientWidth);
    return () => ro.disconnect();
  }, [ref]);
  return w;
}

export function Timeline() {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  // The playhead position is read when a gesture needs it, never as state of the timeline (NFR-3, A-065): only the
  // playhead's line and the lanes' current values follow it, so playing does not render rows and clips again.
  const store = useStore();
  const headFrame = useCallback(() => store.getState().meters?.frame ?? 0, [store]);
  const playing = useApp((s) => s.meters?.playing ?? false);
  const view = useAppShallow((s) => ({ tool: s.tool, snap: s.snap, pps: s.pps, lane: s.lane, scrollX: s.scrollX, follow: s.follow, loop: s.loop, loopOn: s.loopOn, selClips: s.selClips, selTrack: s.selTrack, selSection: s.selSection, folded: s.folded }));
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const specs = useApp((s) => s.specs);
  const edit = useEdit();
  const menu = useMenu();
  const tlRef = useRef<HTMLDivElement>(null);
  const rowsRef = useRef<HTMLDivElement>(null);
  const width = useSize(tlRef);
  const compact = useApp((s) => s.compact);
  // Track header width (tokens --head-w, --head-w-min): the marks' cell sits between the name and the toggles (P-013).
  // The design's width is the minimum; dragging the header's edge widens it, a per-user preference (issue #9, A-084).
  const headPref = useApp((s) => s.prefs.headW);
  const setPrefs = useApp((s) => s.setPrefs);
  const [headDrag, setHeadDrag] = useState<number | null>(null);
  const head = headWidth(headDrag ?? headPref, compact, width);
  const headMin = headWidth(0, compact, width);
  const headMax = headWidth(Infinity, compact, width);
  const rel = useRelations(project);
  const laneW = Math.max(100, width - head);
  const rate = project.sample_rate ?? 48000;
  const song = songFrames(project) / rate;
  const fit = laneW / song;
  const pps = view.pps > 0 ? view.pps : fit;
  const scrollX = Math.max(0, Math.min(view.scrollX, song * pps - laneW * 0.5));
  const [drag, setDrag] = useState<Drag | null>(null);
  const [hoverCut, setHoverCut] = useState<{ track: string; frame: number } | null>(null);
  const [range, setRange] = useState<{ track: string; param: string; a: number; b: number } | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const clipboard = useRef<{ points: AutomationPoint[] } | null>(null);
  const laneH = LANE_PX[view.lane];
  const rows = useMemo(() => treeRows(project, view.folded), [project, view.folded]);
  const notes = useMemo(() => new Map(project.ai.notes.filter((n) => n.track).map((n) => [n.track!, n.number])), [project.ai.notes]);
  const fail = useCallback((e: unknown) => setError(String(e)), [setError]);

  const xToFrame = (clientX: number) => {
    const r = tlRef.current!.getBoundingClientRect();
    return Math.max(0, Math.round(((clientX - r.left - head + scrollX) / pps) * rate));
  };
  const frameToX = (f: number) => (f / rate) * pps;
  const snapIt = (f: number, exclude: Set<string>, meta: boolean) => {
    if (!view.snap || meta) return { frame: f, what: null as string | null };
    const s = snapFrame(f, snapTargets(project, rate, headFrame(), pps, exclude), pps, rate);
    return s ? { frame: s.frame, what: s.what } : { frame: f, what: null };
  };

  // Follow the playhead while playing (TRN-1).
  useEffect(() => {
    if (!playing || !view.follow) return;
    const check = (f: number) => {
      const x = (f / rate) * pps;
      const sx = store.getState().scrollX;
      if (x > sx + laneW * 0.92 || x < sx) setView({ scrollX: Math.max(0, x - laneW * 0.1) });
    };
    check(store.getState().meters?.frame ?? 0);
    return store.subscribe((st, prev) => {
      if (st.meters?.frame !== prev.meters?.frame) check(st.meters?.frame ?? 0);
    });
  }, [playing, view.follow, rate, pps, laneW, store, setView]);

  // Wheel: horizontal scroll (trackpad or Shift), zoom with ⌘/Ctrl around the pointer (TRN-3).
  useEffect(() => {
    const el = tlRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if ((e.target as HTMLElement).closest(".kb-hit, .fader, .hbar.mini, [role=slider]")) return;
      if (e.metaKey || e.ctrlKey) {
        e.preventDefault();
        const r = el.getBoundingClientRect();
        const px = e.clientX - r.left - head;
        const t = (px + scrollX) / pps;
        const next = Math.min(400, Math.max(fit * 0.5, pps * (e.deltaY < 0 ? 1.15 : 1 / 1.15)));
        setView({ pps: next, scrollX: Math.max(0, t * next - px) });
      } else if (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) {
        e.preventDefault();
        setView({ scrollX: Math.max(0, scrollX + (e.shiftKey ? e.deltaY : e.deltaX)) });
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [head, pps, scrollX, fit, setView]);

  const zoom = useCallback(
    (k: number) => {
      const centre = scrollX + laneW / 2;
      const next = Math.min(400, Math.max(fit * 0.5, pps * k));
      setView({ pps: next, scrollX: Math.max(0, (centre / pps) * next - laneW / 2) });
    },
    [scrollX, laneW, fit, pps, setView],
  );

  // ---------- track headers: selection and drag (TRK-1, TRK-3) ----------
  // ⌘-click adds or removes a track, ⇧-click selects the run between the last one and this one. Dragging a header
  // moves the track (or the selection) between rows, or into a group when dropped on the middle of its header: one
  // `place_tracks` edit, one undo step. The core checks the routing; the drop line here only previews.
  const [tdrag, setTdrag] = useState<{ ids: string[]; place: Place | null } | null>(null);
  const dragged = useRef(false);
  const placeAt = (clientY: number, ids: string[]): Place | null => {
    const box = rowsRef.current;
    if (!box) return null;
    const els = [...box.children].slice(0, rows.length) as HTMLElement[];
    const top = box.getBoundingClientRect().top - box.scrollTop;
    const inMoving = (out: Track["output"]) => {
      // The target is one of the moved tracks or sits under one: the routing would loop.
      let o = out;
      for (let i = 0; i <= project.tracks.length && o.kind === "group"; i++) {
        const gid: string = o.id;
        if (ids.includes(gid)) return true;
        const g = project.tracks.find((x) => x.id === gid);
        if (!g) return false;
        o = g.output;
      }
      return false;
    };
    const firstReturn = rows.findIndex((r) => r.kind === "return");
    const end = (): Place => {
      const last = els[(firstReturn < 0 ? rows.length : firstReturn) - 1];
      return { output: { kind: "mix_bus" }, before: null, into: null, y: last ? last.getBoundingClientRect().bottom - top : 0, depth: 0 };
    };
    let i = els.findIndex((el) => {
      const r = el.getBoundingClientRect();
      return clientY >= r.top && clientY < r.bottom;
    });
    if (i < 0) return clientY < (els[0]?.getBoundingClientRect().top ?? 0) ? null : end();
    let after = false;
    // An automation lane belongs to the track above it: dropping there means "after that track".
    if (rows[i].kind === "lane") {
      const owner = rows[i].t.id;
      while (i > 0 && (rows[i].kind === "lane" || rows[i].t.id !== owner)) i--;
      after = true;
    }
    const row = rows[i];
    if (row.kind === "return") return end();
    if (ids.includes(row.t.id)) return null;
    const rect = els[i].getBoundingClientRect();
    const f = (clientY - rect.top) / rect.height;
    const hovered = els[i + (after ? 1 : 0)] ?? els[i];
    const next = (t: Track) => {
      const k = project.tracks.findIndex((x) => x.id === t.id);
      return project.tracks.slice(k + 1).find((x) => x.kind !== "return" && JSON.stringify(x.output) === JSON.stringify(t.output) && !ids.includes(x.id))?.id ?? null;
    };
    let place: Place;
    if (row.kind === "group" && !after && f >= 0.3 && f <= 0.7) {
      place = { output: { kind: "group", id: row.t.id }, before: null, into: row.t.id, y: 0, depth: 0 };
    } else if (!after && f < 0.5) {
      place = { output: row.t.output, before: row.t.id, into: null, y: rect.top - top, depth: row.depth };
    } else if (row.kind === "group" && row.open && !after) {
      const first = project.tracks.find((x) => x.output.kind === "group" && x.output.id === row.t.id && !ids.includes(x.id));
      place = { output: { kind: "group", id: row.t.id }, before: first?.id ?? null, into: null, y: rect.bottom - top, depth: row.depth + 1 };
    } else {
      place = { output: row.t.output, before: next(row.t), into: null, y: hovered.getBoundingClientRect().bottom - top, depth: row.depth };
    }
    if (inMoving(place.output)) return null;
    return place;
  };
  const onHeadDown = (e: RPE, t: Track) => {
    if (e.button !== 0 || t.kind === "return" || e.metaKey || e.ctrlKey || e.shiftKey) return;
    if ((e.target as HTMLElement).closest("button, input, .kb-hit, .fader, .hbar, [role=slider]")) return;
    const sel = store.getState().selTracks;
    const ids = sel.includes(t.id) ? project.tracks.filter((x) => sel.includes(x.id) && x.kind !== "return").map((x) => x.id) : [t.id];
    const x0 = e.clientX;
    const y0 = e.clientY;
    let started = false;
    const move = (ev: PointerEvent) => {
      if (!started && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 4) return;
      started = true;
      setTdrag({ ids, place: placeAt(ev.clientY, ids) });
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("keydown", esc, true);
      setTdrag(null);
    };
    const up = (ev: PointerEvent) => {
      stop();
      if (!started) return;
      dragged.current = true;
      setTimeout(() => (dragged.current = false), 0);
      const place = placeAt(ev.clientY, ids);
      if (!place) return;
      const name = (id: string) => project.tracks.find((x) => x.id === id)?.name ?? id;
      const what = ids.length === 1 ? name(ids[0]) : `${ids.length} tracks`;
      const where = place.output.kind === "group" ? ` into ${name(place.output.id)}` : "";
      void edit({ op: "place_tracks", tracks: ids, before: place.before, output: place.output }, `Move ${what}${where}`);
    };
    const esc = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.stopPropagation();
      started = false;
      stop();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("keydown", esc, true);
  };
  const onHeadClick = (e: React.MouseEvent, t: Track) => {
    if (dragged.current) return;
    const s = store.getState();
    if (e.metaKey || e.ctrlKey) {
      const has = s.selTracks.includes(t.id);
      const next = has ? s.selTracks.filter((x) => x !== t.id) : [...s.selTracks, t.id];
      setView({ selTracks: next, selTrack: has ? (next[next.length - 1] ?? null) : t.id });
    } else if (e.shiftKey && s.selTrack) {
      const order = rows.filter((r) => r.kind !== "lane").map((r) => r.t.id);
      const a = order.indexOf(s.selTrack);
      const b = order.indexOf(t.id);
      if (a < 0 || b < 0) return setView({ selTrack: t.id });
      setView({ selTracks: order.slice(Math.min(a, b), Math.max(a, b) + 1), selTrack: t.id });
    } else {
      setView({ selTrack: t.id });
    }
  };

  // ---------- drag machine ----------
  const dragRef = useRef<Drag | null>(null);
  dragRef.current = drag;
  useEffect(() => {
    if (!drag) return;
    const move = (e: PointerEvent) => {
      const d = dragRef.current;
      if (!d) return;
      const f = xToFrame(e.clientX);
      switch (d.kind) {
        case "move": {
          const dx = e.clientX - d.x0;
          if (!d.moved && Math.abs(dx) < 3 && Math.abs(e.clientY - d.y0) < 3) return;
          let delta = Math.round((dx / pps) * rate);
          const moving = d.keys.map((k) => findClip(project, k)).filter((x): x is [Track, Clip] => !!x);
          let snap: string | null = null;
          let snapAt: number | null = null;
          if (view.snap && !(e.metaKey || e.ctrlKey)) {
            const targets = snapTargets(project, rate, headFrame(), pps, new Set(d.keys));
            let best: { d: number; delta: number; what: string; at: number } | null = null;
            for (const [, c] of moving) {
              for (const edge of [c.start, c.start + c.length]) {
                const s = snapFrame(edge + delta, targets, pps, rate);
                if (s) {
                  const dd = Math.abs(s.frame - (edge + delta));
                  if (!best || dd < best.d) best = { d: dd, delta: s.frame - edge, what: s.what, at: s.frame };
                }
              }
            }
            if (best) {
              delta = best.delta;
              snap = best.what;
              snapAt = best.at;
            }
          }
          const minStart = Math.min(...moving.map(([, c]) => c.start));
          delta = Math.max(delta, -minStart);
          const over = (document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null)?.closest("[data-audio-track]") as HTMLElement | null;
          const src = moving[0]?.[0];
          let toTrack: string | null = null;
          if (over && src && over.dataset.audioTrack !== src.id && moving.every(([t]) => t.id === src.id)) {
            const dest = project.tracks.find((t) => t.id === over.dataset.audioTrack);
            if (dest && dest.channels === src.channels) toTrack = dest.id;
          }
          setDrag({ ...d, delta, toTrack, dup: e.altKey, snap, snapAt, moved: true });
          return;
        }
        case "trim": {
          const [t, c] = findClip(project, d.key) ?? [];
          if (!t || !c) return;
          const s = snapIt(f, new Set([d.key]), e.metaKey || e.ctrlKey);
          setDrag({ ...d, to: s.frame, snap: s.what });
          return;
        }
        case "fade": {
          const [, c] = findClip(project, d.key) ?? [];
          if (!c) return;
          const len = d.fadeIn ? f - c.start : c.start + c.length - f;
          setDrag({ ...d, length: Math.max(0, Math.min(c.length, len)), linear: e.altKey });
          return;
        }
        case "marquee": {
          const r = rowsRef.current!.getBoundingClientRect();
          setDrag({ ...d, x1: e.clientX - r.left, y1: e.clientY - r.top + rowsRef.current!.scrollTop });
          return;
        }
        case "loop":
          setDrag({ ...d, b: snapIt(f, new Set(), e.metaKey || e.ctrlKey).frame });
          return;
        case "section-new":
          setDrag({ ...d, b: snapIt(f, new Set(), e.metaKey || e.ctrlKey).frame });
          return;
        case "section-edge":
          setDrag({ ...d, to: snapIt(f, new Set(), e.metaKey || e.ctrlKey).frame });
          return;
        case "point": {
          const el = document.querySelector(`[data-lane="${d.track}:${d.param}"]`) as HTMLElement | null;
          if (!el) return;
          const r = el.getBoundingClientRect();
          const spec = specOf(specs, d.param);
          const n = 1 - (e.clientY - r.top - 6) / (r.height - 12);
          const pts = d.points.slice();
          const lo = d.index > 0 ? pts[d.index - 1].frame : 0;
          const hi = d.index < pts.length - 1 ? pts[d.index + 1].frame : Number.MAX_SAFE_INTEGER;
          pts[d.index] = { frame: Math.min(hi, Math.max(lo, f)), value: fromNorm(spec, n) };
          setDrag({ ...d, points: pts });
          void backend.gestureEdit({ op: "set_lane", track: d.track, param: d.param, points: pts }).catch(fail);
          return;
        }
        case "draw": {
          const el = document.querySelector(`[data-lane="${d.track}:${d.param}"]`) as HTMLElement | null;
          if (!el) return;
          const r = el.getBoundingClientRect();
          const spec = specOf(specs, d.param);
          const n = 1 - (e.clientY - r.top - 6) / (r.height - 12);
          setDrag({ ...d, stroke: [...d.stroke, { frame: f, value: fromNorm(spec, n) }] });
          return;
        }
        case "range":
          setDrag({ ...d, b: f });
          return;
      }
    };
    const up = (e: PointerEvent) => {
      const d = dragRef.current;
      setDrag(null);
      if (!d) return;
      finish(d, e);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const d = dragRef.current;
      setDrag(null);
      if (d?.kind === "point") void backend.endGesture(true).catch(fail);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("keydown", key);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag !== null]);

  const finish = (d: Drag, e: PointerEvent) => {
    switch (d.kind) {
      case "move": {
        if (!d.moved) return;
        if (d.delta === 0 && !d.toTrack && !d.dup) return;
        const clips = d.keys.map(splitKey);
        const n = clips.length;
        void edit(
          { op: "move_clips", clips, delta: d.delta, to_track: d.toTrack, duplicate: d.dup },
          `${d.dup ? "Duplicate" : "Move"} ${n === 1 ? "clip" : `${n} clips`}`,
        );
        return;
      }
      case "trim": {
        const [t, c] = findClip(project, d.key) ?? [];
        if (!t || !c) return;
        if (d.to === (d.left ? c.start : c.start + c.length)) return;
        void edit({ op: "trim_clip", clip: splitKey(d.key), left: d.left, to: d.to }, `${t.name} · trim clip`);
        return;
      }
      case "fade": {
        const [t] = findClip(project, d.key) ?? [];
        void edit(
          { op: "fade", clip: splitKey(d.key), fade_in: d.fadeIn, length: d.length, shape: d.linear ? "linear" : "equal_power" },
          `${t?.name ?? "Clip"} · fade ${d.fadeIn ? "in" : "out"}`,
        );
        return;
      }
      case "marquee": {
        const box = { l: Math.min(d.x0, d.x1), r: Math.max(d.x0, d.x1), t: Math.min(d.y0, d.y1), b: Math.max(d.y0, d.y1) };
        if (box.r - box.l < 3 && box.b - box.t < 3) return;
        const hits: ClipKey[] = [];
        rowsRef.current?.querySelectorAll<HTMLElement>("[data-clip]").forEach((el) => {
          const r = el.getBoundingClientRect();
          const o = rowsRef.current!.getBoundingClientRect();
          const l = r.left - o.left;
          const t = r.top - o.top + rowsRef.current!.scrollTop;
          if (l < box.r && l + r.width > box.l && t < box.b && t + r.height > box.t) hits.push(el.dataset.clip!);
        });
        setView({ selClips: d.add ? [...new Set([...view.selClips, ...hits])] : hits });
        return;
      }
      case "loop": {
        const a = Math.min(d.a, d.b);
        const b = Math.max(d.a, d.b);
        if ((b - a) / rate < 0.1) {
          void backend.locate(a).catch(fail);
          return;
        }
        setView({ loop: [a, b], loopOn: true });
        void backend.setLoop(a, b).catch(fail);
        return;
      }
      case "section-new": {
        const a = Math.min(d.a, d.b);
        const b = Math.max(d.a, d.b);
        if ((b - a) / rate < 0.5) return;
        void edit({ op: "add_section", name: `Section ${project.sections.length + 1}`, start: a, end: b }, "Add section");
        return;
      }
      case "section-edge": {
        const s = project.sections.find((x) => x.id === d.id);
        if (!s || d.to === (d.end ? s.end : s.start)) return;
        void edit(
          d.end ? { op: "move_boundary", section: d.id, to: d.to } : { op: "move_section_start", section: d.id, to: d.to },
          `Section “${s.name}” resized`,
        );
        return;
      }
      case "point":
        void backend.endGesture(false).catch(fail);
        return;
      case "draw": {
        const spec = specOf(specs, d.param);
        if (d.stroke.length < 2) {
          // A click: one point on the curve.
          const pt = d.stroke[0];
          const t = project.tracks.find((x) => x.id === d.track);
          const lane = t?.automation.find((l) => l.param === d.param);
          const pts = [...(lane?.points ?? []), pt].sort((a, b) => a.frame - b.frame);
          void edit({ op: "set_lane", track: d.track, param: d.param, points: pts }, `${t?.name} · ${paramLabel(project, d.param)} point`);
          return;
        }
        void edit(
          { op: "draw_lane", track: d.track, param: d.param, stroke: d.stroke, tolerance: (spec.max - spec.min) * 0.01 },
          `${project.tracks.find((x) => x.id === d.track)?.name} · ${paramLabel(project, d.param)} drawn`,
        );
        return;
      }
      case "range": {
        const a = Math.min(d.a, d.b);
        const b = Math.max(d.a, d.b);
        setRange(b - a > 0 ? { track: d.track, param: d.param, a, b } : null);
        return;
      }
    }
    void e;
  };

  // ---------- keyboard: clips, sections, lanes ----------
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tgt = e.target as HTMLElement;
      if (tgt.closest("input, textarea, [contenteditable]")) return;
      const id = shortcutOf(e);
      const clips = view.selClips.map(splitKey);
      if (id === "zoomIn" || id === "zoomOut") {
        e.preventDefault();
        zoom(id === "zoomIn" ? 1.5 : 1 / 1.5);
      } else if (id === "del" && range) {
        e.preventDefault();
        void edit({ op: "delete_lane_range", track: range.track, param: range.param, from: range.a, to: range.b }, "Delete automation range");
        setRange(null);
      } else if (id === "del" && clips.length) {
        e.preventDefault();
        void edit({ op: "delete_clips", clips }, clips.length === 1 ? "Delete clip" : `Delete ${clips.length} clips`);
      } else if (id === "del" && view.selSection) {
        e.preventDefault();
        const s = project.sections.find((x) => x.id === view.selSection);
        void edit({ op: "delete_section", section: view.selSection }, `Delete section “${s?.name}”`);
        setView({ selSection: null });
      } else if (id === "dup" && clips.length) {
        e.preventDefault();
        void edit({ op: "duplicate_clips", clips }, "Duplicate clips");
      } else if (id === "split") {
        e.preventDefault();
        const tracks = [...new Set(clips.map(([t]) => t))];
        const frame = headFrame();
        void edit({ op: "split_at", tracks, frame }, `Split at ${formatTime(frame, rate)}`);
      } else if (id === "muteClip" && clips.length) {
        const all = clips.every(([t, c]) => project.tracks.find((x) => x.id === t)?.clips.find((x) => x.id === c)?.mute);
        void edit({ op: "clip_mute", clips, mute: !all }, all ? "Unmute clips" : "Mute clips");
      } else if (id === "copy" && range) {
        const t = project.tracks.find((x) => x.id === range.track);
        const lane = t?.automation.find((l) => l.param === range.param);
        if (!lane) return;
        const spec = specOf(specs, range.param);
        const v0 = laneValue(lane.points, range.a, spec.stepped);
        const v1 = laneValue(lane.points, range.b, spec.stepped);
        const pts: AutomationPoint[] = [];
        if (v0 !== null) pts.push({ frame: 0, value: v0 });
        for (const p of lane.points) if (p.frame > range.a && p.frame < range.b) pts.push({ frame: p.frame - range.a, value: p.value });
        if (v1 !== null) pts.push({ frame: range.b - range.a, value: v1 });
        clipboard.current = { points: pts };
        toast("ok", "Automation copied", `${pts.length} points`);
      } else if (id === "paste" && clipboard.current && range) {
        void edit({ op: "paste_lane", track: range.track, param: range.param, at: headFrame(), points: clipboard.current.points }, "Paste automation");
      } else if (e.key === "Escape") {
        setRange(null);
        setView({ selClips: [], selSection: null });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view.selClips, view.selSection, range, headFrame, project, rate, edit, setView, specs, toast, zoom]);

  // ---------- pointer starts ----------
  const onClipDown = (e: RPE, t: Track, c: Clip) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const k = clipKey(t.id, c.id);
    if (view.tool === "split") {
      const f = snapIt(xToFrame(e.clientX), new Set(), e.metaKey || e.ctrlKey).frame;
      void edit({ op: "split_at", tracks: [t.id], frame: f }, `${t.name} · split at ${formatTime(f, rate)}`);
      return;
    }
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = e.clientX - r.left;
    let sel = view.selClips;
    if (e.shiftKey) sel = sel.includes(k) ? sel.filter((s) => s !== k) : [...sel, k];
    else if (!sel.includes(k)) sel = [k];
    setView({ selClips: sel, selTrack: t.id });
    if (x < EDGE || x > r.width - EDGE) {
      setDrag({ kind: "trim", key: k, left: x < EDGE, to: x < EDGE ? c.start : c.start + c.length, snap: null });
      return;
    }
    if (e.shiftKey && !sel.includes(k)) return;
    setDrag({ kind: "move", keys: sel.length ? sel : [k], x0: e.clientX, y0: e.clientY, delta: 0, toTrack: null, dup: e.altKey, snap: null, snapAt: null, moved: false });
  };

  const onFadeDown = (e: RPE, t: Track, c: Clip, fadeIn: boolean) => {
    e.stopPropagation();
    if (e.button !== 0) return;
    setDrag({ kind: "fade", key: clipKey(t.id, c.id), fadeIn, length: fadeIn ? c.fade_in.length : c.fade_out.length, linear: e.altKey });
  };

  const onLaneDown = (e: RPE, t: Track) => {
    if (e.button !== 0) return;
    setView({ selTrack: t.id });
    if (view.tool === "split") {
      const f = snapIt(xToFrame(e.clientX), new Set(), e.metaKey || e.ctrlKey).frame;
      void edit({ op: "split_at", tracks: [t.id], frame: f }, `${t.name} · split at ${formatTime(f, rate)}`);
      return;
    }
    const r = rowsRef.current!.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top + rowsRef.current!.scrollTop;
    if (!e.shiftKey) setView({ selClips: [] });
    setDrag({ kind: "marquee", x0: x, y0: y, x1: x, y1: y, add: e.shiftKey });
  };

  const onAutoDown = (e: RPE, t: Track, param: string) => {
    if (e.button !== 0) return;
    const f = xToFrame(e.clientX);
    if (e.shiftKey) {
      setDrag({ kind: "range", track: t.id, param, a: f, b: f });
      return;
    }
    setRange(null);
    if (view.tool !== "auto") {
      toast("ok", "Automation tool", "Pick the pen tool (A) to add or draw points.");
      return;
    }
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    const spec = specOf(specs, param);
    const n = 1 - (e.clientY - r.top - 6) / (r.height - 12);
    setDrag({ kind: "draw", track: t.id, param, stroke: [{ frame: f, value: fromNorm(spec, n) }] });
  };

  const onPointDown = (e: RPE, t: Track, param: string, index: number) => {
    e.stopPropagation();
    if (e.button !== 0) return;
    const lane = t.automation.find((l) => l.param === param);
    if (!lane) return;
    setRange(null);
    void backend.beginGesture(`${t.name} · ${paramLabel(project, param)} point`).catch(fail);
    setDrag({ kind: "point", track: t.id, param, index, points: lane.points });
  };

  // The ruler: grabbing the playhead there (or ⌥ anywhere on it) scrubs; elsewhere a click locates and a drag sets the
  // loop region. Below the ruler the playhead's line lets clicks through to clips and lanes.
  const scrub = useScrub(xToFrame);
  const nearHead = (e: { clientX: number }) => {
    const r = (e as unknown as RPE).currentTarget.getBoundingClientRect();
    return Math.abs(e.clientX - r.left - (frameToX(headFrame()) - scrollX)) <= HEAD_GRAB;
  };
  const [overHead, setOverHead] = useState(false);
  const onRulerDown = (e: RPE) => {
    if (e.button !== 0) return;
    if (e.altKey || nearHead(e)) {
      scrub.start(e);
      return;
    }
    const f = xToFrame(e.clientX);
    setDrag({ kind: "loop", a: f, b: f });
  };

  const onSectionsDown = (e: RPE) => {
    if (e.button !== 0 || e.target !== e.currentTarget) return;
    const f = snapIt(xToFrame(e.clientX), new Set(), e.metaKey || e.ctrlKey).frame;
    setView({ selSection: null });
    setDrag({ kind: "section-new", a: f, b: f });
  };

  // ---------- render ----------
  const ticks = useMemo(() => {
    // The ruler is in minutes:seconds (TRN-2); bars, when there is a tempo, only shape the snap grid.
    const g = gridStep(pps, { ...project, tempo: null });
    const out: { x: number; label: string }[] = [];
    const t0 = Math.floor(scrollX / pps / g.step) * g.step;
    for (let t = t0; t * pps < scrollX + laneW + 1; t += g.step) {
      if (t < 0) continue;
      out.push({ x: t * pps - scrollX, label: formatClock(t) });
    }
    return out;
  }, [pps, project, scrollX, laneW]);

  const loop = drag?.kind === "loop" ? [Math.min(drag.a, drag.b), Math.max(drag.a, drag.b)] : view.loop;
  const audioCount = project.tracks.filter((t) => t.kind === "audio").length;

  return (
    <section className="card timeline" aria-label="Timeline" data-tool={view.tool}>
      <div className="tools">
        <div className="seg quiet sm" role="radiogroup" aria-label="Tool">
          {(["select", "split", "auto"] as Tool[]).map((tool, i) => (
            <button
              type="button"
              key={tool}
              role="radio"
              aria-checked={view.tool === tool}
              aria-label={["Select", "Split", "Automation"][i]}
              title={`${["Select", "Split", "Automation"][i]} (${keysOf((["toolSelect", "toolSplit", "toolAuto"] as const)[i])})`}
              className={view.tool === tool ? "on" : ""}
              onClick={() => setView({ tool })}
            >
              <Icon name={(["cursor", "cut", "pen"] as const)[i]} />
            </button>
          ))}
        </div>
        <button type="button" className={`tag ${view.snap ? "acc" : ""}`} aria-pressed={view.snap} onClick={() => setView({ snap: !view.snap })}>
          <Icon name="snap" size="sm" />
          {view.snap ? `Snap · ${project.tempo ? "bars" : "grid"}, edges, sections` : "Snap off"}
        </button>
        <div className="fx gh">
          <button type="button" className="ib" aria-label="Zoom out" title="Zoom out (−)" onClick={() => zoom(1 / 1.5)}>
            <Icon name="minus" />
          </button>
          <button type="button" className="ib" aria-label="Zoom in" title="Zoom in (+)" onClick={() => zoom(1.5)}>
            <Icon name="plus" />
          </button>
          <button
            type="button"
            className="ib"
            aria-label="Track height"
            title={`Track height: ${{ s: "small", m: "medium", l: "large" }[view.lane]}`}
            onClick={() => setView({ lane: view.lane === "s" ? "m" : view.lane === "m" ? "l" : "s" })}
          >
            <Icon name="up" />
          </button>
        </div>
        <div className="grow" />
        <span className="faint s1">
          {audioCount ? `${audioCount} track${audioCount === 1 ? "" : "s"}` : "No tracks"} · {project.sample_rate ? `${Math.round(project.sample_rate / 100) / 10} kHz` : "rate not set"}
        </span>
        <AddMenu />
      </div>
      <MissingBanner />
      {audioCount === 0 && (
        // 13a: an empty project says what the timeline is for and how to fill it.
        <div className="drop grow" style={{ margin: "var(--sp-3)" }} data-testid="empty-timeline">
          <Icon name="file" />
          <span className="h">Drop your stems here</span>
          <span className="dim">WAV, FLAC or AIFF · one file per track</span>
          <button type="button" className="btn lg" onClick={() => setView({ screen: "new" })}>
            <Icon name="folder" size="m" />
            Choose files…
          </button>
        </div>
      )}
      <div className="tl" ref={tlRef} style={{ ["--head-w" as string]: `${head}px`, ...(audioCount === 0 ? { display: "none" } : {}) }}>
        <div className="row ruler">
          <div className="hd" />
          <div
            className={`ln ${overHead || scrub.scrubbing ? "on-head" : ""}`}
            onPointerDown={onRulerDown}
            onPointerMove={(e) => setOverHead(e.altKey || nearHead(e))}
            onPointerLeave={() => setOverHead(false)}
            data-testid="ruler"
            title={keysInText("Drag the playhead to scrub (⌥-drag anywhere on the ruler) · click to locate · drag to set the loop")}
          >
            {loop && (view.loopOn || drag?.kind === "loop") && (
              <div className="loop" style={{ left: frameToX(loop[0]) - scrollX, width: frameToX(loop[1] - loop[0]) }} />
            )}
            {ticks.map((t) => (
              <span key={t.x} className="tick" style={{ left: t.x }}>
                {t.label}
              </span>
            ))}
          </div>
        </div>
        <div className="row secs">
          <div className="hd">
            <span className="faint s1">Song sections</span>
          </div>
          <div className="ln" onPointerDown={onSectionsDown} data-testid="sections-lane">
            {project.sections.map((s) => {
              const edgeDrag = drag?.kind === "section-edge" && drag.id === s.id ? drag : null;
              const a = edgeDrag && !edgeDrag.end ? edgeDrag.to : s.start;
              const b = edgeDrag && edgeDrag.end ? edgeDrag.to : s.end;
              const sel = view.selSection === s.id;
              return (
                <span
                  key={s.id}
                  className={`sec ${/chorus|ritornello/i.test(s.name) ? "hot" : ""} ${sel ? "is-sel" : ""} ${renaming === s.id ? "is-edit" : ""}`}
                  style={{ left: frameToX(a) - scrollX, width: Math.max(4, frameToX(b - a) - 1) }}
                  onPointerDown={(e) => {
                    e.stopPropagation();
                    if (e.button !== 0) return;
                    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
                    const x = e.clientX - r.left;
                    setView({ selSection: s.id });
                    if (x > r.width - EDGE) setDrag({ kind: "section-edge", id: s.id, end: true, to: s.end });
                    else if (x < EDGE) setDrag({ kind: "section-edge", id: s.id, end: false, to: s.start });
                  }}
                  onDoubleClick={() => setRenaming(s.id)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    menu.open(e, [
                      { label: "Rename…", onSelect: () => setRenaming(s.id) },
                      { label: "Play from here", icon: "play", onSelect: () => void backend.locate(s.start).then(() => backend.play()).catch(fail) },
                      { label: "Loop this section", icon: "loop", onSelect: () => { setView({ loop: [s.start, s.end], loopOn: true }); void backend.setLoop(s.start, s.end).catch(fail); } },
                      "-",
                      { label: "Delete section", icon: "trash", danger: true, kbd: "⌫", onSelect: () => void edit({ op: "delete_section", section: s.id }, `Delete section “${s.name}”`) },
                    ], "Section");
                  }}
                  title={`${s.name} · ${formatClock(s.start / rate)}–${formatClock(s.end / rate)}`}
                >
                  {renaming === s.id ? (
                    <input
                      className="vt-edit"
                      autoFocus
                      aria-label="Section name"
                      defaultValue={s.name}
                      onPointerDown={(e) => e.stopPropagation()}
                      onBlur={() => setRenaming(null)}
                      onKeyDown={(e) => {
                        e.stopPropagation();
                        if (e.key === "Escape") setRenaming(null);
                        if (e.key === "Enter") {
                          const name = (e.target as HTMLInputElement).value.trim();
                          setRenaming(null);
                          if (name && name !== s.name) void edit({ op: "rename_section", section: s.id, name }, `Section renamed “${name}”`);
                        }
                      }}
                    />
                  ) : (
                    s.name
                  )}
                  {sel && <i className="grip" />}
                </span>
              );
            })}
            {!project.sections.length && drag?.kind !== "section-new" && (
              <span className="lane-note" title="With sections the assistant can treat verse and chorus differently.">
                <Icon name="pen" size="sm" /> Drag here to mark intro, verse, chorus…
              </span>
            )}
            {drag?.kind === "section-new" && (
              <span className="sec is-edit" style={{ left: frameToX(Math.min(drag.a, drag.b)) - scrollX, width: frameToX(Math.abs(drag.b - drag.a)) }} />
            )}
          </div>
        </div>
        <div className="tl-rows" ref={rowsRef}>
          {rows.map((row) => (
            <RowView
              key={row.kind === "lane" ? `${row.t.id}:${row.param}` : row.t.id}
              row={row}
              project={project}
              rate={rate}
              pps={pps}
              scrollX={scrollX}
              laneW={laneW}
              laneH={laneH}
              size={view.lane}
              note={notes.get(row.t.id)}
              rel={rel.reasons.get(row.t.id)}
              relRet={rel.ret}
              drag={drag}
              range={range}
              hoverCut={view.tool === "split" ? hoverCut : null}
              onHoverCut={(t, e) => view.tool === "split" && setHoverCut(e ? { track: t, frame: snapIt(xToFrame(e.clientX), new Set(), e.metaKey || e.ctrlKey).frame } : null)}
              onClipDown={onClipDown}
              onFadeDown={onFadeDown}
              onLaneDown={onLaneDown}
              onAutoDown={onAutoDown}
              onPointDown={onPointDown}
              onClipMenu={(e, t, c) => {
                e.preventDefault();
                const k = clipKey(t.id, c.id);
                if (!view.selClips.includes(k)) setView({ selClips: [k], selTrack: t.id });
                const keys = view.selClips.includes(k) ? view.selClips.map(splitKey) : [splitKey(k)];
                menu.open(e, clipMenu(project, t, c, keys, headFrame(), rate, edit), "Clip");
              }}
              onTrackMenu={(e, t) => {
                e.preventDefault();
                // On a track of the selection the menu acts on all of it (grouping); elsewhere it selects that track.
                const sel = store.getState().selTracks;
                const selTracks = sel.includes(t.id) ? sel : [t.id];
                setView({ selTrack: t.id, selTracks });
                menu.open(e, trackMenu(project, t, { edit, backend, setView, folded: view.folded, setError, selTracks }), "Track");
              }}
              onParamPick={(el, t, param) => menu.open(el, paramMenu(project, t, param, specs, edit, backend, fail), "Automate")}
              onHeadDown={onHeadDown}
              onHeadClick={onHeadClick}
              moving={!!tdrag && tdrag.ids.includes(row.t.id)}
              dropInto={tdrag?.place?.into === row.t.id}
            />
          ))}
          {tdrag?.place && !tdrag.place.into && (
            <div className="track-drop" style={{ top: tdrag.place.y, left: `calc(var(--sp-3) * ${tdrag.place.depth})` }} data-testid="track-drop" />
          )}
          {drag?.kind === "marquee" && (
            <div
              className="marquee"
              style={{ left: Math.min(drag.x0, drag.x1), top: Math.min(drag.y0, drag.y1), width: Math.abs(drag.x1 - drag.x0), height: Math.abs(drag.y1 - drag.y0) }}
            />
          )}
          <div className="tl-fill" onPointerDown={(e) => e.button === 0 && setView({ selClips: [] })} />
        </div>
        <HeadResizer
          head={head}
          active={headDrag !== null}
          min={headMin}
          max={headMax}
          onDrag={setHeadDrag}
          onCommit={(w) => {
            setHeadDrag(null);
            setPrefs({ headW: w });
          }}
        />
        <div className="tl-over" style={{ left: head }}>
          <Playhead playing={playing} scrubFrame={scrub.frame} rate={rate} pps={pps} scrollX={scrollX} />
          {drag?.kind === "move" && drag.snapAt !== null && (
            <>
              <div className="snapline" style={{ left: frameToX(drag.snapAt) - scrollX }} />
              <span className="callout" style={{ left: frameToX(drag.snapAt) - scrollX + 8, top: 44 }}>
                {drag.delta >= 0 ? "+" : "−"}
                {formatTime(Math.abs(drag.delta), rate)} · snapped to {drag.snap}
              </span>
            </>
          )}
          {drag?.kind === "move" && drag.snapAt === null && drag.moved && (
            <span className="callout" style={{ left: 8, top: 44 }}>
              {drag.delta >= 0 ? "+" : "−"}
              {formatTime(Math.abs(drag.delta), rate)}
              {drag.dup ? " · copy" : ""}
            </span>
          )}
          {drag?.kind === "trim" && drag.snap && <div className="snapline" style={{ left: frameToX(drag.to) - scrollX }} />}
          {drag?.kind === "section-edge" && (
            <span className="callout" style={{ left: frameToX(drag.to) - scrollX + 8, top: 24 }}>
              {project.sections.find((s) => s.id === drag.id)?.name} {drag.end ? "ends" : "starts"} {formatTime(drag.to, rate)}
            </span>
          )}
          {hoverCut && view.tool === "split" && (
            <span className="callout" style={{ left: frameToX(hoverCut.frame) - scrollX + 8, top: 44 }}>
              <Icon name="cut" size="sm" /> Split at {formatTime(hoverCut.frame, rate)}
            </span>
          )}
        </div>
      </div>
      {menu.node}
    </section>
  );
}

/** The track headers' right edge: drag (or arrows) to widen them, double-click to return to the layout's width. */
function HeadResizer({ head, active, min, max, onDrag, onCommit }: { head: number; active: boolean; min: number; max: number; onDrag(w: number | null): void; onCommit(w: number): void }) {
  const start = useRef<{ x: number; w: number; last: number } | null>(null);
  const clamp = (w: number) => Math.round(Math.max(min, Math.min(max, w)));
  return (
    <div
      className={`head-resize ${active ? "on" : ""}`}
      style={{ left: head }}
      role="separator"
      aria-orientation="vertical"
      aria-label="Track header width"
      aria-valuenow={head}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      title="Drag to widen the track headers · double-click for the default width"
      data-testid="head-resize"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        start.current = { x: e.clientX, w: head, last: head };
        onDrag(head);
      }}
      onPointerMove={(e) => {
        const s = start.current;
        if (!s) return;
        s.last = clamp(s.w + e.clientX - s.x);
        onDrag(s.last);
      }}
      onPointerUp={() => {
        const s = start.current;
        start.current = null;
        if (s) onCommit(s.last);
      }}
      onPointerCancel={() => {
        start.current = null;
        onDrag(null);
      }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        onCommit(0);
      }}
      onKeyDown={(e) => {
        const step = e.shiftKey ? 64 : 16;
        const to = e.key === "ArrowLeft" ? head - step : e.key === "ArrowRight" ? head + step : e.key === "Home" ? min : e.key === "End" ? max : null;
        if (to === null) return;
        e.preventDefault();
        e.stopPropagation();
        onCommit(to <= min ? 0 : clamp(to));
      }}
    />
  );
}

/** The playhead's line: the only part of the timeline that renders at every position update. */
function Playhead({ playing, scrubFrame, rate, pps, scrollX }: { playing: boolean; scrubFrame: number | null; rate: number; pps: number; scrollX: number }) {
  const frame = useApp((s) => s.meters?.frame ?? 0);
  const x = ((scrubFrame ?? frame) / rate) * pps - scrollX;
  return <div className={`play ${playing ? "" : "idle"}`} style={{ transform: `translateX(${x}px) translateX(-50%)` }} data-testid="playhead" />;
}

function findClip(p: Project, k: ClipKey): [Track, Clip] | null {
  const [tid, cid] = splitKey(k);
  const t = p.tracks.find((x) => x.id === tid);
  const c = t?.clips.find((x) => x.id === cid);
  return t && c ? [t, c] : null;
}

function clipMenu(_p: Project, t: Track, c: Clip, keys: [string, string][], frame: number, rate: number, edit: ReturnType<typeof useEdit>): MenuItem[] {
  const n = keys.length;
  return [
    { label: "Split at playhead", icon: "cut", kbd: "⌘T", onSelect: () => void edit({ op: "split_at", tracks: [...new Set(keys.map(([x]) => x))], frame }, `Split at ${formatTime(frame, rate)}`) },
    { label: "Duplicate", icon: "copy", kbd: "⌘D", onSelect: () => void edit({ op: "duplicate_clips", clips: keys }, n === 1 ? "Duplicate clip" : `Duplicate ${n} clips`) },
    { label: c.mute ? "Unmute clip" : "Mute clip", kbd: "M", onSelect: () => void edit({ op: "clip_mute", clips: keys, mute: !c.mute }, c.mute ? "Unmute clip" : "Mute clip") },
    "-",
    { label: "Fade in", sub: [0, 0.01, 0.5, 1, 2].map((s) => ({ label: s === 0 ? "None" : `${s} s`, onSelect: () => void edit({ op: "fade", clip: [t.id, c.id], fade_in: true, length: Math.round(s * rate), shape: c.fade_in.shape }, `${t.name} · fade in`) })) },
    { label: "Fade out", sub: [0, 0.01, 0.5, 1, 2, 4].map((s) => ({ label: s === 0 ? "None" : `${s} s`, onSelect: () => void edit({ op: "fade", clip: [t.id, c.id], fade_in: false, length: Math.round(s * rate), shape: c.fade_out.shape }, `${t.name} · fade out`) })) },
    { label: "Fade shape", sub: (["equal_power", "linear"] as const).map((shape) => ({ label: shape === "linear" ? "Linear" : "Equal power", checked: c.fade_in.shape === shape, onSelect: () => { void edit({ op: "fade", clip: [t.id, c.id], fade_in: true, length: c.fade_in.length, shape }, `${t.name} · fade shape`); } })) },
    { label: "Clip gain", kbd: "⌘G", sub: [-6, -3, -1, 0, 1, 3, 6].map((g) => ({ label: `${g > 0 ? "+" : g < 0 ? "−" : ""}${Math.abs(g).toFixed(1)} dB`, checked: Math.abs(c.gain_db - g) < 0.05, onSelect: () => void edit({ op: "clip_gain", clip: [t.id, c.id], gain_db: g }, `${t.name} · clip gain`) })) },
    "-",
    { label: "Delete", icon: "trash", kbd: "⌫", danger: true, onSelect: () => void edit({ op: "delete_clips", clips: keys }, n === 1 ? "Delete clip" : `Delete ${n} clips`) },
  ];
}

// ---------- rows ----------
interface RowProps {
  row: Row;
  project: Project;
  rate: number;
  pps: number;
  scrollX: number;
  laneW: number;
  laneH: number;
  size: "s" | "m" | "l";
  note?: number;
  /** Why this row is connected to the selection (UI-10). */
  rel?: string;
  relRet: string | null;
  drag: Drag | null;
  range: { track: string; param: string; a: number; b: number } | null;
  hoverCut: { track: string; frame: number } | null;
  onHoverCut(track: string, e: RPE | null): void;
  onClipDown(e: RPE, t: Track, c: Clip): void;
  onFadeDown(e: RPE, t: Track, c: Clip, fadeIn: boolean): void;
  onLaneDown(e: RPE, t: Track): void;
  onAutoDown(e: RPE, t: Track, param: string): void;
  onPointDown(e: RPE, t: Track, param: string, index: number): void;
  onClipMenu(e: RPE | React.MouseEvent, t: Track, c: Clip): void;
  onTrackMenu(e: React.MouseEvent, t: Track): void;
  onParamPick(el: HTMLElement, t: Track, param: string): void;
  onHeadDown(e: RPE, t: Track): void;
  onHeadClick(e: React.MouseEvent, t: Track): void;
  /** Being dragged to another place. */
  moving: boolean;
  /** Dragged tracks would go into this group. */
  dropInto: boolean;
}

function HeaderControls({ t, implied }: { t: Track; implied: boolean }) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const edit = useEdit();
  const fader = useTrackCtl(t, "fader", "level");
  const fail = (e: unknown) => setError(String(e));
  return (
    <>
      <Toggle kind="mu" on={t.mute} label={`Mute ${t.name}`} onClick={() => void backend.setView(t.id, "mute", !t.mute).catch(fail)} />
      <Toggle kind="so" on={t.solo} implied={implied} label={`Solo ${t.name}`} onClick={() => void backend.setView(t.id, "solo", !t.solo).catch(fail)} />
      {t.kind !== "return" && (
        <Toggle kind="lk" on={t.locked} label={`Lock ${t.name} for the assistant`} onClick={() => void edit({ op: "set_locked", track: t.id, locked: !t.locked }, `${t.name} · ${t.locked ? "unlocked" : "locked"} for the assistant`)} />
      )}
      <MiniFader ctl={fader} />
    </>
  );
}

function RowView(p: RowProps) {
  const { row, project, rate, pps, scrollX, laneH, size } = p;
  const selTrack = useApp((s) => s.selTrack);
  const selTracks = useApp((s) => s.selTracks);
  const selClips = useApp((s) => s.selClips);
  const setView = useApp((s) => s.setView);
  const folded = useApp((s) => s.folded);
  const renameTrack = useApp((s) => s.renameTrack);
  const edit = useEdit();
  const anySolo = project.tracks.some((t) => t.solo);
  const x = (f: number) => (f / rate) * pps - scrollX;
  const t = row.t;
  const indent = `d${Math.min(3, row.depth)}`;
  const tint = t.kind === "audio" ? `t${t.color || 1}` : "tb";

  if (row.kind === "lane") {
    const lane = t.automation.find((l) => l.param === row.param);
    return <AutoRow {...p} t={t} param={row.param} lane={lane} indent={indent} tint={tint} x={x} />;
  }

  const sub =
    row.kind === "group"
      ? `group · ${row.members} ${row.members === 1 ? "track" : "tracks"}${row.open ? "" : " · folded"}`
      : row.kind === "return"
        ? `${t.ret?.type ?? ""} return`
        : (t.role ?? "").replace(/_/g, " ");
  const cls = `row ${tint} ${size === "s" ? "s" : size === "l" ? "l" : ""} ${selTrack === t.id || selTracks.includes(t.id) ? "is-sel" : ""} ${p.moving ? "is-moving" : ""} ${p.dropInto ? "is-into" : ""} ${t.locked ? "is-lock" : ""} ${row.kind === "group" ? "grp" : ""} ${p.rel ? "is-rel" : ""} ${p.drag?.kind === "move" && p.drag.toTrack === t.id ? "is-drop" : ""}`;
  const clipH = laneH - 2 * 2;
  const waveH = size === "s" ? clipH - 4 : clipH - 13 - 3;
  return (
    <div className={cls} style={{ height: laneH }} data-audio-track={t.kind === "audio" ? t.id : undefined} data-track-row={t.id}>
      <div className={`hd ${indent} ${t.kind === "return" ? "" : "grab"}`} onPointerDown={(e) => p.onHeadDown(e, t)} onClick={(e) => p.onHeadClick(e, t)} onContextMenu={(e) => p.onTrackMenu(e, t)} aria-label={`${t.name} header`} title={t.kind === "return" ? undefined : "Drag to move · drop on a group to put it inside · ⌘-click to select more"}>
        {row.kind === "group" ? (
          <button
            type="button"
            className="ib fold"
            aria-label={row.open ? `Fold ${t.name}` : `Unfold ${t.name}`}
            aria-expanded={row.open}
            onClick={(e) => {
              e.stopPropagation();
              setView({ folded: row.open ? [...folded, t.id] : folded.filter((f) => f !== t.id) });
            }}
          >
            <Icon name={row.open ? "chev" : "right"} size="m" />
          </button>
        ) : row.kind === "return" ? (
          <Icon name="wave" size="m" />
        ) : (
          <i className="tdot" />
        )}
        <div className="names" onDoubleClick={(e) => { e.stopPropagation(); setView({ renameTrack: t.id }); }}>
          {renameTrack === t.id ? (
            <input
              className="vt-edit nm"
              autoFocus
              aria-label="Track name"
              defaultValue={t.name}
              onClick={(e) => e.stopPropagation()}
              onBlur={() => setView({ renameTrack: null })}
              onKeyDown={(e) => {
                e.stopPropagation();
                if (e.key === "Escape") setView({ renameTrack: null });
                if (e.key === "Enter") {
                  const name = (e.target as HTMLInputElement).value.trim();
                  setView({ renameTrack: null });
                  if (name && name !== t.name) void edit({ op: "rename_track", track: t.id, name, color: null }, `Rename ${t.name} to ${name}`);
                }
              }}
            />
          ) : (
            <span className="nm ell">{t.name}</span>
          )}
          <span className="rl ell">{p.rel ?? sub}</span>
        </div>
        {p.note !== undefined && (
          <button type="button" className="badge" aria-label={`Assistant note ${p.note}`} onClick={(e) => { e.stopPropagation(); setView({ selTrack: t.id, asstTab: 0, hideAssistant: false }); }}>
            {p.note}
          </button>
        )}
        <HeaderMarks t={t} project={project} relRet={p.relRet} />
        <HeaderControls t={t} implied={anySolo && !t.solo} />
      </div>
      <div
        className="ln"
        onPointerDown={(e) => (t.kind === "audio" ? p.onLaneDown(e, t) : setView({ selTrack: t.id }))}
        onPointerMove={(e) => p.onHoverCut(t.id, e)}
        onPointerLeave={() => p.onHoverCut(t.id, null)}
      >
        {row.kind === "group" &&
          groupSpans(project, t.id).map(([a, b], i) => <i key={i} className="gsum" style={{ left: x(a), width: Math.max(2, x(b) - x(a)) }} />)}
        {t.kind === "audio" &&
          t.clips.map((c) => {
            const k = clipKey(t.id, c.id);
            const d = p.drag;
            const moving = d?.kind === "move" && d.moved && d.keys.includes(k);
            const trimming = d?.kind === "trim" && d.key === k ? d : null;
            const fading = d?.kind === "fade" && d.key === k ? d : null;
            let start = c.start;
            let len = c.length;
            if (trimming) {
              if (trimming.left) {
                const to = Math.min(trimming.to, c.start + c.length - 1);
                len = c.start + c.length - Math.max(to, c.start - c.offset);
                start = c.start + c.length - len;
              } else len = Math.max(1, trimming.to - c.start);
            }
            if (x(start + len) < -50 || x(start) > p.laneW + 50) return null;
            const w = Math.max(3, (len / rate) * pps - 1);
            const fi = fading?.fadeIn ? fading.length : c.fade_in.length;
            const fo = fading && !fading.fadeIn ? fading.length : c.fade_out.length;
            const ghost = moving && !(d.kind === "move" && d.toTrack === null && d.dup === false && d.delta === 0);
            return (
              <div key={c.id}>
                {ghost && <div className="clip is-ghost" style={{ left: x(c.start), width: w }} />}
                {!(moving && d.kind === "move" && d.toTrack) && (
                  <ClipView
                    t={t}
                    c={c}
                    left={x(start + (moving && d.kind === "move" ? d.delta : 0))}
                    width={w}
                    waveH={waveH}
                    rate={rate}
                    pps={pps}
                    fi={fi}
                    fo={fo}
                    sel={selClips.includes(k)}
                    muted={t.mute || c.mute}
                    trim={trimming ? (trimming.left ? "l" : "r") : null}
                    offsetShift={trimming?.left ? start - c.start : 0}
                    len={len}
                    onDown={(e) => p.onClipDown(e, t, c)}
                    onFade={(e, fin) => p.onFadeDown(e, t, c, fin)}
                    onMenu={(e) => p.onClipMenu(e, t, c)}
                  />
                )}
              </div>
            );
          })}
        {t.kind === "audio" && p.drag?.kind === "move" && p.drag.moved && p.drag.toTrack === t.id &&
          p.drag.keys.map((k) => {
            const fc = findClip(project, k);
            if (!fc) return null;
            const [st, c] = fc;
            const w = Math.max(3, (c.length / rate) * pps - 1);
            return (
              <ClipView key={`drop-${k}`} t={{ ...t, color: st.color }} c={c} left={x(c.start + (p.drag as { delta: number }).delta)} width={w} waveH={waveH} rate={rate} pps={pps} fi={c.fade_in.length} fo={c.fade_out.length} sel muted={false} trim={null} offsetShift={0} len={c.length} onDown={() => {}} onFade={() => {}} onMenu={() => {}} />
            );
          })}
        {p.hoverCut && p.hoverCut.track === t.id && <div className="cutline" style={{ left: x(p.hoverCut.frame) }} />}
        {row.kind === "return" && <ReturnFeeds r={t} project={project} />}
      </div>
    </div>
  );
}

function groupSpans(p: Project, gid: string): [number, number][] {
  const members = (id: string): Track[] => p.tracks.filter((t) => t.output.kind === "group" && t.output.id === id).flatMap((t) => (t.kind === "group" ? members(t.id) : [t]));
  const spans = members(gid).flatMap((t) => t.clips.map((c): [number, number] => [c.start, c.start + c.length])).sort((a, b) => a[0] - b[0]);
  const out: [number, number][] = [];
  for (const s of spans) {
    const last = out[out.length - 1];
    if (last && s[0] <= last[1]) last[1] = Math.max(last[1], s[1]);
    else out.push([s[0], s[1]]);
  }
  return out;
}

function ClipView(p: {
  t: Track; c: Clip; left: number; width: number; waveH: number; rate: number; pps: number; fi: number; fo: number; sel: boolean; muted: boolean;
  trim: "l" | "r" | null; offsetShift: number; len: number;
  onDown(e: RPE): void; onFade(e: RPE, fadeIn: boolean): void; onMenu(e: React.MouseEvent): void;
}) {
  const { t, c, rate, pps } = p;
  const fiw = (p.fi / rate) * pps;
  const fow = (p.fo / rate) * pps;
  const bodyH = p.waveH + 3;
  const k = clipKey(t.id, c.id);
  const prefs = useApp((s) => s.prefs);
  const missing = useApp((s) => s.missing.includes(c.media));
  const [loading, setLoading] = useState(false);
  const setView = useApp((s) => s.setView);
  return (
    <div
      className={`clip t${t.color || 1} ${p.sel ? "is-sel" : ""} ${p.muted ? "is-muted" : ""} ${missing ? "is-missing" : ""} ${loading && !missing && prefs.waveforms ? "is-loading" : ""}`}
      style={{ left: p.left, width: p.width }}
      data-clip={k}
      onPointerDown={p.onDown}
      onContextMenu={p.onMenu}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        // UI-7: a clip is reached with Tab and selected with Enter or Space (then ⌫, ⌘D, M… act on it).
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          e.stopPropagation();
          setView({ selClips: [k], selTrack: t.id });
        }
      }}
      aria-label={`Clip ${t.name} at ${formatClock(c.start / rate)}${missing ? ", audio file missing" : ""}`}
      aria-pressed={p.sel}
    >
      {prefs.clipNames && <span className="cn">{t.name}</span>}
      {prefs.waveforms && !missing && <Waveform media={c.media} offset={c.offset + p.offsetShift} length={p.len} width={p.width} height={p.waveH} onLoading={setLoading} />}
      {fiw > 1 && (
        <svg className="fade" style={{ left: 0 }} width={fiw} height={bodyH} aria-hidden="true">
          <path d={`M0 0H${fiw}Q${fiw * 0.3} ${bodyH * 0.2} 0 ${bodyH}Z`} />
          <path className="ln2" d={`M0 ${bodyH}Q${fiw * 0.3} ${bodyH * 0.2} ${fiw} 0`} />
        </svg>
      )}
      {fow > 1 && (
        <svg className="fade" style={{ right: 0 }} width={fow} height={bodyH} aria-hidden="true">
          <path d={`M${fow} 0H0Q${fow * 0.7} ${bodyH * 0.2} ${fow} ${bodyH}Z`} />
          <path className="ln2" d={`M0 0Q${fow * 0.7} ${bodyH * 0.2} ${fow} ${bodyH}`} />
        </svg>
      )}
      <i className="fh fh-in" style={{ left: Math.max(4, fiw) }} onPointerDown={(e) => p.onFade(e, true)} title={keysInText("Fade in (⌥ for linear)")} />
      <i className="fh fh-out" style={{ right: Math.max(4, fow), transform: "translate(50%,-25%)" }} onPointerDown={(e) => p.onFade(e, false)} title={keysInText("Fade out (⌥ for linear)")} />
      {(prefs.gainBadge || Math.abs(c.gain_db) >= 0.05) && <span className="gain">{c.gain_db > 0.05 ? "+" : c.gain_db < -0.05 ? "−" : ""}{Math.abs(c.gain_db).toFixed(1)} dB</span>}
      {p.trim && <i className={`trim ${p.trim}`} />}
    </div>
  );
}

function AutoRow(p: RowProps & { t: Track; param: string; lane: Track["automation"][number] | undefined; indent: string; tint: string; x: (f: number) => number }) {
  const backend = useBackend();
  const specs = useApp((s) => s.specs);
  const setError = useApp((s) => s.setError);
  const { t, param, lane, x } = p;
  const spec = specOf(specs, param);
  const pts = p.drag?.kind === "point" && p.drag.track === t.id && p.drag.param === param ? p.drag.points : (lane?.points ?? []);
  const h = AUTO_H;
  const y = (v: number) => h - 6 - toNorm(spec, v) * (h - 12);
  const ai = lane?.author === "ai";
  // The value under the playhead: this row renders again only when it changes.
  const cur = useApp((s) => laneValue(pts, s.meters?.frame ?? 0, spec.stepped));
  let d = "";
  if (pts.length) {
    const xy: [number, number][] = [];
    xy.push([Math.min(0, x(pts[0].frame)), y(pts[0].value)]);
    pts.forEach((q, i) => {
      if (spec.stepped && i > 0) xy.push([x(q.frame), y(pts[i - 1].value)]);
      xy.push([x(q.frame), y(q.value)]);
    });
    xy.push([Math.max(p.laneW, x(pts[pts.length - 1].frame)), y(pts[pts.length - 1].value)]);
    d = "M" + xy.map(([a, b]) => `${a.toFixed(1)} ${b.toFixed(1)}`).join("L");
  }
  const draw = p.drag?.kind === "draw" && p.drag.track === t.id && p.drag.param === param ? p.drag.stroke : null;
  const rg = p.drag?.kind === "range" && p.drag.track === t.id && p.drag.param === param ? { a: Math.min(p.drag.a, p.drag.b), b: Math.max(p.drag.a, p.drag.b) } : p.range && p.range.track === t.id && p.range.param === param ? p.range : null;
  return (
    <div className={`row auto ${p.tint} ${ai ? "by-ai" : ""}`} style={{ height: h }}>
      <div className={`hd ${p.indent}`}>
        <button type="button" className="select grow" aria-label={`Automated parameter of ${t.name}`} onClick={(e) => p.onParamPick(e.currentTarget, t, param)}>
          <span className={`ell b ${ai ? "aic" : "acc"}`}>{paramLabel(p.project, param)}</span>
          <Icon name="chev" size="m" />
        </button>
        <span className="s1 b nowrap">{cur === null ? "—" : formatParam(spec, cur)}</span>
        <button type="button" className="ib" aria-label="Hide lane" title={ai ? "Written by the assistant · hide lane" : "Hide lane"} onClick={() => void backend.setLaneVisible(t.id, param, false).catch((e) => setError(String(e)))}>
          <Icon name="x" size="m" />
        </button>
      </div>
      <div className="ln" data-lane={`${t.id}:${param}`} onPointerDown={(e) => p.onAutoDown(e, t, param)}>
        <svg width={p.laneW} height={h} aria-hidden="true">
          {d && <path className="ar" d={`${d}L${p.laneW} ${h}L0 ${h}Z`} />}
          {d && <path className="al" d={d} />}
          {draw && draw.length > 1 && <path className="al" d={"M" + draw.map((q) => `${x(q.frame).toFixed(1)} ${y(q.value).toFixed(1)}`).join("L")} />}
        </svg>
        {rg && <div className="marquee" style={{ left: x(rg.a), width: Math.max(1, x(rg.b) - x(rg.a)), top: 4, height: h - 8 }} />}
        {pts.map((q, i) =>
          x(q.frame) < -10 || x(q.frame) > p.laneW + 10 ? null : (
            <i
              key={i}
              className={`dot ${ai ? "ai" : ""} ${rg && q.frame >= rg.a && q.frame <= rg.b ? "is-sel" : ""}`}
              style={{ left: x(q.frame), top: y(q.value) }}
              onPointerDown={(e) => p.onPointDown(e, t, param, i)}
              title={`${formatTime(q.frame, p.rate)} · ${formatParam(spec, q.value)}`}
            />
          ),
        )}
        {!pts.length && <span className="lane-note">Pick the pen tool and click or draw to add points</span>}
      </div>
    </div>
  );
}
