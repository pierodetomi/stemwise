// The Mix workspace (design 3a–3d, UI-1, UI-4): top bar, timeline over the mixer, inspector and assistant columns.
// Panels collapse in the design's order (assistant ⌘J, inspector ⌘I, mixer ⌘M; the timeline never closes); the
// mixer is resized from its top edge. Global shortcuts for transport, history, tools and saving (UI-3).
import { useEffect, useRef, useState, type ReactNode } from "react";
import { keysInText } from "../shared/keys";
import { Icon } from "../shared/Icon";
import { useApp, useBackend, useStore } from "../state/context";
import { shortcutOf } from "../shared/shortcuts";
import { DeviceBanner } from "../shared/StateBanners";
import { Assistant } from "./Assistant";
import { Explain } from "./explain/Explain";
import { Inspector } from "./Inspector";
import { groupMembers, groupTracks } from "./menus";
import { useProjectCommands } from "../project/commands";
import { Mixer } from "./Mixer";
import { Timeline } from "./Timeline";
import { TopBar } from "./TopBar";

function Rail({ label, icon, onOpen, keys }: { label: string; icon: "spark" | "eye"; onOpen: () => void; keys: string }) {
  return (
    <button type="button" className="card rail" aria-label={`Show ${label}`} title={`Show ${label} (${keys})`} onClick={onOpen}>
      <Icon name={icon} />
      <span className="rail-l">{label}</span>
    </button>
  );
}

export function Workspace({ onMaster, assistantFooter, snapshots, proposals, overlay, side, mid, working }: { onMaster: () => void; assistantFooter?: ReactNode; snapshots?: ReactNode; proposals?: ReactNode; overlay?: ReactNode; side?: ReactNode; mid?: ReactNode; working?: ReactNode }) {
  const backend = useBackend();
  const store = useStore();
  const hideAssistant = useApp((s) => s.hideAssistant);
  const hideInspector = useApp((s) => s.hideInspector);
  const hideMixer = useApp((s) => s.hideMixer);
  const mixerH = useApp((s) => s.mixerH);
  const explain = useApp((s) => s.explain);
  const setView = useApp((s) => s.setView);
  const bodyRef = useRef<HTMLDivElement>(null);
  const cmd = useProjectCommands();

  // Global shortcuts (UI-3), from the shared table.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tgt = e.target as HTMLElement;
      if (tgt.closest("input, textarea, select, [contenteditable], .menu, [role=dialog]")) return;
      const id = shortcutOf(e);
      if (!id) return;
      const s = store.getState();
      const fail = (x: unknown) => s.setError(String(x));
      const p = s.project;
      const sel = p?.tracks.find((t) => t.id === s.selTrack) ?? null;
      const handled = () => e.preventDefault();
      switch (id) {
        case "play":
          handled();
          void (s.meters?.playing ? backend.pause() : backend.play()).catch(fail);
          return;
        case "stop":
          void backend.stop().catch(fail);
          return;
        case "start":
          void backend.locate(0).catch(fail);
          return;
        case "loop": {
          const on = !s.loopOn && !!s.loop;
          setView({ loopOn: on });
          void backend.setLoop(on && s.loop ? s.loop[0] : null, on && s.loop ? s.loop[1] : null).catch(fail);
          return;
        }
        case "loopSel": {
          handled();
          const spans = s.selClips.map((k) => k.split("/")).map(([t, c]) => p?.tracks.find((x) => x.id === t)?.clips.find((x) => x.id === c)).filter((c) => !!c);
          if (!spans.length) return;
          const a = Math.min(...spans.map((c) => c!.start));
          const b = Math.max(...spans.map((c) => c!.start + c!.length));
          setView({ loop: [a, b], loopOn: true });
          void backend.setLoop(a, b).catch(fail);
          return;
        }
        case "follow":
          setView({ follow: !s.follow });
          s.toast("ok", s.follow ? "Follow playhead off" : "Follow playhead on", "");
          return;
        case "snap":
          setView({ snap: !s.snap });
          return;
        case "toolSelect":
          setView({ tool: "select" });
          return;
        case "toolSplit":
          setView({ tool: "split" });
          return;
        case "toolAuto":
          setView({ tool: "auto" });
          return;
        case "undo":
        case "redo":
          handled();
          void (id === "redo" ? backend.redo("mix") : backend.undo("mix")).catch(fail);
          return;
        case "save":
          handled();
          void cmd.save();
          return;
        case "saveAs":
          handled();
          void cmd.saveAs();
          return;
        case "snapshot":
          handled();
          void backend.snapshotSave(`Snapshot ${(p?.snapshots.length ?? 0) + 1}`).then(() => s.toast("ok", "Snapshot saved", "")).catch(fail);
          return;
        case "fit":
          handled();
          setView({ pps: 0, scrollX: 0 });
          return;
        case "taller":
        case "shorter": {
          handled();
          const order = ["s", "m", "l"] as const;
          const i = order.indexOf(s.lane) + (id === "taller" ? 1 : -1);
          setView({ lane: order[Math.max(0, Math.min(2, i))] });
          return;
        }
        case "explain":
          handled();
          setView({ explain: !s.explain });
          return;
        case "mixer":
          handled();
          setView({ hideMixer: !s.hideMixer });
          return;
        case "inspector":
          handled();
          setView({ hideInspector: !s.hideInspector });
          return;
        case "assistant":
          handled();
          setView({ hideAssistant: !s.hideAssistant });
          return;
        case "muteTrack":
          if (sel) void backend.setView(sel.id, "mute", !sel.mute).catch(fail);
          return;
        case "soloTrack":
          if (sel) void backend.setView(sel.id, "solo", !sel.solo).catch(fail);
          return;
        case "lockTrack":
          if (sel && sel.kind !== "return") void backend.edit({ op: "set_locked", track: sel.id, locked: !sel.locked }, `${sel.name} · ${sel.locked ? "unlocked" : "locked"} for the assistant`).catch(fail);
          return;
        case "group":
          handled();
          if (p) {
            const members = groupMembers(p, s.selTracks, s.selTrack);
            if (members.length) void groupTracks(p, members, (e, label) => backend.edit(e, label)).catch(fail);
          }
          return;
        case "automation":
          if (sel) void backend.setLaneVisible(sel.id, "fader", !sel.automation.some((l) => l.param === "fader" && l.visible)).catch(fail);
          return;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [backend, store, setView, cmd]);

  // The mixer's top edge: the same thin accented line as the track headers' grip, on hover and while dragged.
  const [gripOn, setGripOn] = useState(false);
  const resize = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const y0 = e.clientY;
    const h0 = mixerH;
    setGripOn(true);
    const move = (ev: PointerEvent) => setView({ mixerH: Math.max(168, Math.min(420, h0 - (ev.clientY - y0))) });
    const up = () => {
      setGripOn(false);
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  return (
    <>
      <TopBar onMaster={onMaster} mid={mid} sideTaken={!!side} />
      <DeviceBanner />
      {explain ? (
        <Explain />
      ) : (
      <div className="body" ref={bodyRef}>
        <div className="left">
          <Timeline />
          {hideMixer ? (
            <button type="button" className="card bridge" aria-label="Show mixer" title={keysInText("Show mixer (⌘M)")} onClick={() => setView({ hideMixer: false })}>
              <Icon name="up" size="m" />
              <span className="s1 dim">Mixer</span>
            </button>
          ) : (
            <div className="mixer-wrap" style={{ height: mixerH }}>
              <div className={`mixer-grip ${gripOn ? "on" : ""}`} role="separator" aria-orientation="horizontal" aria-label="Resize mixer" title="Drag to resize the mixer" data-testid="mixer-grip" onPointerDown={resize} />
              <Mixer />
            </div>
          )}
        </div>
        {side ?? (
          <>
            {hideInspector ? <Rail label="Inspector" icon="eye" keys={keysInText("⌘I")} onOpen={() => setView({ hideInspector: false })} /> : <Inspector />}
            {hideAssistant ? <Rail label="Assistant" icon="spark" keys={keysInText("⌘J")} onOpen={() => setView({ hideAssistant: false })} /> : <Assistant footer={assistantFooter} snapshots={snapshots} proposals={proposals} working={working} />}
          </>
        )}
      </div>
      )}
      {overlay}
    </>
  );
}
