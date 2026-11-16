// Top bar of the Mix workspace (design 3a): project, phase selector (UI-1), transport (TRN-1), position, section,
// undo/redo (HIS-1), mix bus meters (true peak, loudness) and engine load.
import { Icon, Mark } from "../shared/Icon";
import { keysInText } from "../shared/keys";
import { ProjectMenuButton } from "../project/ProjectMenu";
import { IconButton, Seg } from "../shared/controls";
import { BusMeters } from "../shared/BusMeters";
import { PrepRing } from "../master/Prep";
import { formatBars, formatClock, formatTime } from "../shared/format";
import { useApp, useBackend } from "../state/context";
import { songFrames } from "./geometry";
import { keysOf } from "../shared/shortcuts";
import type { IconName } from "../shared/Icon";
import { TOUR_TEXT } from "../tour/text";

function savedText(dirty: boolean, savedAt: number | null) {
  if (dirty) return "Unsaved changes · autosaved";
  if (!savedAt) return "Saved";
  const d = new Date(savedAt);
  return `Saved ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/**
 * One switch per panel (UI-4): each shows or hides its own panel, whatever the others do. The name stays the same
 * and the pressed state says whether the panel is there; the shortcut is in the tooltip. While a proposal takes the
 * side columns, their two switches have nothing to act on and leave the bar, which is short of room then.
 */
function PanelSwitches({ sideTaken }: { sideTaken: boolean }) {
  const hideMixer = useApp((s) => s.hideMixer);
  const hideInspector = useApp((s) => s.hideInspector);
  const hideAssistant = useApp((s) => s.hideAssistant);
  const setView = useApp((s) => s.setView);
  const one = (name: string, icon: IconName, shortcut: string, hidden: boolean, toggle: () => void) => (
    <button
      type="button"
      className={`ib ${hidden ? "" : "on"}`}
      aria-label={`${name} panel`}
      aria-pressed={!hidden}
      title={`${hidden ? "Show" : "Hide"} ${name.toLowerCase()} (${keysOf(shortcut)})`}
      onClick={toggle}
    >
      <Icon name={icon} />
    </button>
  );
  return (
    <div className="fx g1" role="group" aria-label="Panels">
      {one("Mixer", "faders", "mixer", hideMixer, () => setView({ hideMixer: !hideMixer }))}
      {!sideTaken && one("Inspector", "eye", "inspector", hideInspector, () => setView({ hideInspector: !hideInspector }))}
      {!sideTaken && one("Assistant", "spark", "assistant", hideAssistant, () => setView({ hideAssistant: !hideAssistant }))}
    </div>
  );
}

export function TopBar({ onMaster, mid, sideTaken = false }: { onMaster: () => void; mid?: React.ReactNode; sideTaken?: boolean }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const meters = useApp((s) => s.meters);
  const dirty = useApp((s) => s.dirty);
  const demo = useApp((s) => s.demo);
  const savedAt = useApp((s) => s.savedAt);
  const canUndo = useApp((s) => s.canUndo);
  const canRedo = useApp((s) => s.canRedo);
  const loopOn = useApp((s) => s.loopOn);
  const loop = useApp((s) => s.loop);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const prep = useApp((s) => s.masterStatus?.prep ?? null);
  const retarget = useApp((s) => s.masterStatus?.retarget ?? null);
  const rate = project.sample_rate ?? 48000;
  const frame = meters?.frame ?? 0;
  const playing = meters?.playing ?? false;
  const section = project.sections.find((s) => frame >= s.start && frame < s.end);
  const fail = (e: unknown) => setError(String(e));
  const openSettings = useApp((s) => s.openSettings);
  const bars = useApp((s) => s.prefs.bars);
  const explain = useApp((s) => s.explain);
  const toggleLoop = () => {
    const on = !loopOn;
    if (on && !loop) {
      toast("warn", "No loop region", "Drag on the ruler to set the region to loop.");
      return;
    }
    setView({ loopOn: on });
    void backend.setLoop(on && loop ? loop[0] : null, on && loop ? loop[1] : null).catch(fail);
  };
  const undo = async () => {
    try {
      const label = await backend.undo("mix");
      toast("ok", "Undone", label);
    } catch (e) {
      fail(e);
    }
  };
  const redo = async () => {
    try {
      const label = await backend.redo("mix");
      toast("ok", "Redone", label);
    } catch (e) {
      fail(e);
    }
  };
  return (
    <header className="top" aria-label="Top bar">
      <div className="fx g2 top-id">
        <Mark />
        <ProjectMenuButton />
        <span className="faint s1 ell" title={demo ? TOUR_TEXT.status : savedText(dirty, savedAt)}>{demo ? TOUR_TEXT.status : savedText(dirty, savedAt)}</span>
      </div>
      <Seg
        className="phase"
        label="Phase"
        value="mix"
        items={[
          { value: "mix", label: "Mix" },
          // 9f: the master is being prepared while you mix; a ring shows how far.
          prep && !prep.error
            ? { value: "master", label: <>Master<PrepRing fraction={prep.fraction} /></>, title: `Preparing the master · ${Math.round(prep.fraction * 100)} % · keeps going while you mix` }
            : retarget
              ? // 9v: a new target is being reached while you mix.
                { value: "master", label: <>Master<PrepRing fraction={retarget.fraction} /></>, title: `Bringing the master to the new target · ${Math.round(retarget.fraction * 100)} % · keeps going while you mix` }
              : { value: "master", label: "Master" },
        ]}
        onChange={(v) => v === "master" && onMaster()}
      />
      <div className="fx g2" role="group" aria-label="Transport">
        <IconButton icon="rew" label="Return to start" onClick={() => void backend.locate(0).catch(fail)} />
        <button
          type="button"
          className={`playb ${playing ? "" : "idle"}`}
          aria-label={playing ? "Pause" : "Play"}
          onClick={() => void (playing ? backend.pause() : backend.play()).catch(fail)}
        >
          <Icon name={playing ? "pause" : "play"} />
        </button>
        <IconButton icon="stop" label="Stop" onClick={() => void backend.stop().catch(fail)} />
        <IconButton icon="loop" label="Loop" on={loopOn} onClick={toggleLoop} />
      </div>
      <div className="fx g2">
        <span className="pos" data-testid="position">{bars && project.tempo ? formatBars(frame, rate, project.tempo) : formatTime(frame, rate)}</span>
        <span className="faint s1 nowrap">/ {formatClock(songFrames(project) / rate)}</span>
        {section && <span className="tag">{section.name}</span>}
      </div>
      <div className="fx g1">
        <IconButton icon="undo" label="Undo" disabled={!canUndo} onClick={() => void undo()} className={canUndo ? "" : "is-dis"} />
        <IconButton icon="redo" label="Redo" disabled={!canRedo} onClick={() => void redo()} className={canRedo ? "" : "is-dis"} />
      </div>
      {/* UI-14: the mix explained, in place of the workspace (design 17a). */}
      <button type="button" className={`btn xbtn ${explain ? "on" : "quiet"}`} aria-pressed={explain} title={`Explain this mix (${keysOf("explain")})`} onClick={() => setView({ explain: !explain })}>
        <Icon name="map" size="m" />
        Explain
      </button>
      {mid}
      <div className="grow" />
      {!explain && <PanelSwitches sideTaken={sideTaken} />}
      <IconButton icon="gear" label={keysInText("Settings (⌘,)")} onClick={() => openSettings()} />
      <BusMeters label="Mix bus" />
    </header>
  );
}
