// Recoverable states of a working project (design 13d; UI-6): the audio device that went away, the audio files
// missing from the project folder. Each says what happened, that the project is safe, and offers the way out.
import { useEffect, useState } from "react";
import type { DeviceInfo, TransportState } from "../backend/types";
import { useApp, useBackend } from "../state/context";
import { Icon } from "./Icon";
import { useMenu } from "./Menu";

/** Polls the output state while a project is open (the device can disappear while nothing plays). */
export function useTransportPoll(ms = 1000) {
  const backend = useBackend();
  const setTransport = useApp((s) => s.setTransport);
  useEffect(() => {
    let live = true;
    const poll = () => backend.transportState().then((t) => live && setTransport(t)).catch(() => {});
    void poll();
    const id = window.setInterval(poll, ms);
    return () => {
      live = false;
      window.clearInterval(id);
    };
  }, [backend, setTransport, ms]);
}

/** Under the top bar: the device was disconnected or could not start (13d). */
export function DeviceBanner() {
  const backend = useBackend();
  const t: TransportState | null = useApp((s) => s.transport);
  const openSettings = useApp((s) => s.openSettings);
  const setError = useApp((s) => s.setError);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const down = !!t && (t.disconnected || !!t.deviceError);
  useEffect(() => {
    if (!down) return;
    backend.audioDevices().then(setDevices).catch(() => {});
  }, [backend, down]);
  if (!down || !t) return null;
  const other = devices.find((d) => d.default && d.name !== t.device) ?? devices.find((d) => d.name !== t.device);
  const use = async (name: string | null) => {
    try {
      const p = await backend.audioPrefs();
      await backend.setAudioPrefs({ ...p, device: name });
    } catch (e) {
      setError(String(e));
    }
  };
  return (
    <div className="banner err" role="alert" style={{ margin: "0 var(--gap) var(--gap)" }} data-testid="device-banner">
      <Icon name="warn" />
      <div className="col grow">
        <span className="b">{t.disconnected ? `${t.device ?? "The audio device"} was disconnected` : "The audio output could not start"}</span>
        <span className="dim s1">Playback stopped. Nothing in the project changed.{t.deviceError && !t.disconnected ? ` ${t.deviceError}` : ""}</span>
      </div>
      {other && (
        <button type="button" className="btn sm" onClick={() => void use(other.default ? null : other.name)}>
          Use {other.name}
        </button>
      )}
      <button type="button" className="btn quiet sm" onClick={() => openSettings("audio")}>Audio settings</button>
    </div>
  );
}

/** In the timeline: audio files missing from the project folder; their clips are hatched and silent (13d). */
export function MissingBanner() {
  const backend = useBackend();
  const project = useApp((s) => s.project);
  const missing = useApp((s) => s.missing);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const menu = useMenu();
  if (!project || !missing.length) return null;
  // An id the project does not list (a clip left without its media) cannot be located, only reported.
  const media = missing.map((id) => project.media.find((m) => m.id === id) ?? { id, original_name: "Unknown file", orphan: true as const });
  const locate = async (id: string, name: string) => {
    try {
      const [path] = await backend.pickAudioFiles();
      if (!path) return;
      const relinked = await backend.relinkMedia(id, path);
      toast("ok", relinked ? `“${name}” now uses another file` : `“${name}” found`, relinked ? "One step in the history: undo goes back." : "");
    } catch (e) {
      setError(String(e));
    }
  };
  const n = media.length;
  const can = (m: (typeof media)[number]) => !("orphan" in m);
  return (
    <div className="banner" role="alert" style={{ margin: "0 var(--sp-3) var(--sp-2)" }} data-testid="missing-banner">
      <Icon name="warn" />
      <span className="grow">
        {n === 1 ? `“${media[0].original_name}” is missing from the project folder. Its clips are hatched and silent.` : `${n} audio files are missing from the project folder. Their clips are hatched and silent.`}
      </span>
      <button
        type="button"
        className="btn sm"
        onClick={(e) =>
          n === 1 && can(media[0])
            ? void locate(media[0].id, media[0].original_name)
            : menu.open(
                e.currentTarget,
                media.map((m) => ({ label: m.original_name, disabled: !can(m), onSelect: () => void locate(m.id, m.original_name) })),
                "Missing files",
              )
        }
      >
        Locate…
      </button>
      {menu.node}
    </div>
  );
}
