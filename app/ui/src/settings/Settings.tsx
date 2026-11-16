// Settings (design 11a–11d; SET-1): audio device and buffer, the assistant (decider, key, model, effort,
// temperature, spending cap, what is sent, total across projects), appearance, and the shortcuts (view only, UI-3).
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { KEYCHAIN_SHORT, THIS_COMPUTER } from "../shared/platform";
import { keysInText } from "../shared/keys";
import type { AudioPrefs, DeviceInfo, TransportState } from "../backend/types";
import { Seg } from "../shared/controls";
import { keyHome } from "../ai/Assist";
import { Icon, Mark } from "../shared/Icon";
import { useMenu } from "../shared/Menu";
import { SHORTCUTS, type Shortcut } from "../shared/shortcuts";
import { useApp, useBackend } from "../state/context";
import type { ViewState } from "../state/store";

type Tab = ViewState["settingsTab"];
const TABS: [Tab, string, Parameters<typeof Icon>[0]["name"]][] = [
  ["audio", "Audio", "speaker"],
  ["ai", "AI", "spark"],
  ["appearance", "Appearance", "eye"],
  ["shortcuts", "Shortcuts", "key"],
];

function Row({ title, sub, children }: { title: string; sub: string; children: ReactNode }) {
  return (
    <div className="frow">
      <div className="fl">
        <span className="b">{title}</span>
        <span className="faint s1">{sub}</span>
      </div>
      <div className="col g2">{children}</div>
    </div>
  );
}

export function Switch({ on, label, onChange, disabled }: { on: boolean; label: string; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <label className={`fx g2 switch-row ${disabled ? "is-dis" : ""}`}>
      <button type="button" role="switch" aria-checked={on} aria-label={label} disabled={disabled} className={`switch ${on ? "on" : ""}`} onClick={() => onChange(!on)} />
      <span>{label}</span>
    </label>
  );
}

export function Settings() {
  const tab = useApp((s) => s.settingsTab);
  const back = useApp((s) => s.settingsBack);
  const project = useApp((s) => s.project);
  const setView = useApp((s) => s.setView);
  const done = () => setView({ screen: back === "workspace" || back === "usage" ? (project ? back : "welcome") : back });
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || (e.target as HTMLElement).closest("input, textarea, .menu, [role=dialog]")) return;
      done();
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  });
  return (
    <>
      <header className="top" aria-label="Top bar">
        <Mark />
        <span className="h">Settings</span>
        <div className="grow" />
        <button type="button" className="btn primary" onClick={done}>Done</button>
      </header>
      <div className="body settings">
        <nav className="card nav" aria-label="Settings sections">
          {TABS.map(([id, name, icon]) => (
            <button type="button" key={id} className={`mi ${tab === id ? "on" : ""}`} aria-current={tab === id ? "page" : undefined} onClick={() => setView({ settingsTab: id })}>
              <Icon name={icon} size="m" />
              <span>{name}</span>
            </button>
          ))}
        </nav>
        <section className="card grow scroll" aria-label={TABS.find(([id]) => id === tab)?.[1]}>
          <div className="ph"><span className="h">{TABS.find(([id]) => id === tab)?.[1]}</span></div>
          {tab === "audio" ? <AudioPane /> : tab === "ai" ? <AiPane /> : tab === "appearance" ? <AppearancePane /> : <ShortcutsPane />}
        </section>
      </div>
    </>
  );
}

// ---------- 11a Audio ----------

function AudioPane() {
  const backend = useBackend();
  const project = useApp((s) => s.project);
  const setError = useApp((s) => s.setError);
  const menu = useMenu();
  const [devices, setDevices] = useState<DeviceInfo[] | null>(null);
  const [prefs, setPrefs] = useState<AudioPrefs | null>(null);
  const [ts, setTs] = useState<TransportState | null>(null);
  const fail = (e: unknown) => setError(String(e));
  useEffect(() => {
    let live = true;
    backend.audioDevices().then((d) => live && setDevices(d)).catch(fail);
    backend.audioPrefs().then((p) => live && setPrefs(p)).catch(fail);
    const poll = () => backend.transportState().then((t) => live && setTs(t)).catch(() => {});
    void poll();
    const id = window.setInterval(poll, 500);
    return () => {
      live = false;
      window.clearInterval(id);
    };
    // fail is stable enough: setError is a store action.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend]);
  const put = (p: AudioPrefs) => {
    setPrefs(p);
    void backend.setAudioPrefs(p).catch(fail);
  };
  const def = devices?.find((d) => d.default);
  const current = prefs?.device ?? def?.name ?? null;
  const devRate = ts?.deviceRate ?? null;
  const pRate = project?.sample_rate ?? null;
  const buffer = prefs?.buffer ?? 512;
  const rate = devRate ?? pRate ?? 48000;
  const load = Math.round((ts?.load ?? 0) * 100);
  return (
    <div className="form">
      <Row title="Output device" sub="Where you hear the project.">
        <button
          type="button"
          className="select lg"
          aria-label="Output device"
          onClick={(e) =>
            prefs &&
            menu.open(
              e.currentTarget,
              [
                { label: `System default${def ? ` (${def.name})` : ""}`, checked: prefs.device === null, onSelect: () => put({ ...prefs, device: null }) },
                ...(devices ?? []).map((d) => ({ label: d.name, checked: prefs.device === d.name, onSelect: () => put({ ...prefs, device: d.name }) })),
              ],
              "Output device",
            )
          }
        >
          <span className="ell">{current ?? "No output device"}</span>
          <Icon name="chev" size="m" />
        </button>
        {ts?.disconnected || ts?.deviceError ? (
          <span className="dangerc s1" data-testid="device-state"><Icon name="warn" size="sm" /> {ts.deviceError ?? "The device is not connected."} Playback is stopped; the project is safe.</span>
        ) : devRate && pRate ? (
          <span className="faint s1" data-testid="device-state">
            <Icon name="info" size="sm" />{" "}
            {devRate === pRate ? `Device runs at ${devRate / 1000} kHz, same as the project: no conversion on output.` : `Device runs at ${devRate / 1000} kHz, the project at ${pRate / 1000} kHz: converted on output only.`}
          </span>
        ) : (
          <span className="faint s1" data-testid="device-state">{devRate ? `Device runs at ${devRate / 1000} kHz.` : "The device starts when a project is open."}</span>
        )}
      </Row>
      <Row title="Buffer size" sub="Smaller is more responsive, larger is safer.">
        <Seg className="quiet" label="Buffer" value={buffer} items={[128, 256, 512, 1024].map((b) => ({ value: b, label: String(b) }))} onChange={(b) => prefs && put({ ...prefs, buffer: b })} />
        <span className="faint s1">{buffer} samples · {((buffer / rate) * 1000).toFixed(1)} ms at {rate / 1000} kHz</span>
      </Row>
      <Row title="Engine load" sub="Share of each buffer the engine uses.">
        <div className="fx g3">
          <div className="hbar grow" role="meter" aria-label="Engine load" aria-valuenow={load} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${Math.min(100, load)}%` }} /></div>
          <span className={`b ${load > 80 ? "warnc" : ""}`}>{load} %</span>
        </div>
        <span className="faint s1">Above 80 % you may hear dropouts: raise the buffer size.{ts && ts.xruns > 0 ? ` ${ts.xruns} dropout${ts.xruns === 1 ? "" : "s"} so far.` : ""}</span>
      </Row>
      {menu.node}
    </div>
  );
}

// ---------- 11b AI ----------

function AiPane() {
  const backend = useBackend();
  const ai = useApp((s) => s.ai);
  const project = useApp((s) => s.project);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const setView = useApp((s) => s.setView);
  const setAiRef = useApp((s) => s.setAi);
  const [model, setModel] = useState(ai?.config.model ?? "");
  const [temp, setTemp] = useState(ai?.config.temperature.toFixed(1) ?? "0.2");
  const cap = project?.ai.spend_cap_usd ?? null;
  const [capText, setCapText] = useState((cap ?? 2).toFixed(2));
  const fail = (e: unknown) => setError(String(e));
  if (!ai) return <div className="form"><span className="dim">Loading…</span></div>;
  const c = ai.config;
  // Read the status back: the pane stays in step even when no status event follows the change.
  const set = (patch: Partial<typeof c>) => void backend.aiSetConfig({ ...c, ...patch }).then(() => backend.aiStatus()).then(setAiRef).catch(fail);
  return (
    <div className="form">
      <Row title="Assistant" sub="Who answers mix and master requests.">
        <Seg
          className="quiet"
          label="Decider"
          value={c.offline_demo ? "demo" : "openrouter"}
          items={[
            { value: "openrouter", label: "OpenRouter (your key)" },
            { value: "demo", label: "Offline demo" },
          ]}
          onChange={(v) => set({ offline_demo: v === "demo" })}
        />
        <span className="faint s1">The offline demo is rule-based: no key, no network, no cost.</span>
      </Row>
      <Row title="OpenRouter account" sub={`The key is kept in ${keyHome(ai.key_place)}, never in the project.`}>
        <div className="fx g2">
          <span className="input sm grow"><span className="grow ell" data-testid="key-state">{ai.has_key ? (ai.key_place === "file" ? "Stored in a private file of this app" : `Stored in ${KEYCHAIN_SHORT}`) : "No key"}</span></span>
          {ai.has_key ? (
            <>
              <button type="button" className="btn" onClick={() => setView({ setup: "connect" })}>
                <Icon name="link" size="m" />
                Connect again
              </button>
              <button
                type="button"
                className="btn quiet"
                onClick={() =>
                  setView({
                    dialog: {
                      title: "Disconnect OpenRouter?",
                      sub: `The key is deleted from ${keyHome(ai.key_place)}. It stays valid in your OpenRouter account until you delete it there. The offline demo keeps working.`,
                      confirm: "Disconnect",
                      danger: true,
                      onConfirm: () => void backend.aiSetKey(null).then(() => backend.aiStatus()).then(setAiRef).catch(fail),
                    },
                  })
                }
              >
                Disconnect
              </button>
            </>
          ) : (
            <button type="button" className="btn primary" onClick={() => setView({ setup: "connect" })}>Set up the assistant…</button>
          )}
        </div>
        <span className={`s1 ${ai.has_key ? "okc" : "faint"}`}>
          <Icon name={ai.has_key ? "check" : "info"} size="sm" /> {ai.has_key ? "A key is stored. It is checked on the first request." : "Without a key, use the offline demo."}
        </span>
        <div className="fx g4">
          {ai.has_key && <button type="button" className="link s1" onClick={() => void backend.aiOpenPage("credits").catch(fail)}>Add credit on OpenRouter</button>}
          <button type="button" className="link s1" onClick={() => setView({ setup: "manual" })}>Paste a key instead</button>
        </div>
      </Row>
      <Row title="Model" sub="Any model id offered by OpenRouter.">
        <input className="input sm" aria-label="Model" value={model} onChange={(e) => setModel(e.target.value)} onBlur={() => model.trim() && model.trim() !== c.model && set({ model: model.trim() })} />
        <div className="fx g4">
          <label className="flabel">
            <span className="l">Reasoning effort</span>
            <Seg className="quiet" label="Effort" value={c.effort} items={["low", "medium", "high"].map((e) => ({ value: e, label: e[0].toUpperCase() + e.slice(1) }))} onChange={(e) => set({ effort: e })} />
          </label>
          <label className="flabel">
            <span className="l">Temperature</span>
            <input
              className="input sm num"
              aria-label="Temperature"
              inputMode="decimal"
              value={temp}
              onChange={(e) => setTemp(e.target.value)}
              onBlur={() => {
                const v = Number(temp.replace(",", "."));
                if (!Number.isFinite(v) || v < 0 || v > 2) {
                  setTemp(c.temperature.toFixed(1));
                  toast("warn", "Temperature not changed", "Use a value from 0 to 2.");
                } else if (v !== c.temperature) set({ temperature: v });
              }}
            />
          </label>
        </div>
      </Row>
      <Row title="Spending cap" sub="Ask before a project goes over this.">
        {project ? (
          <div className="fx g3">
            <Switch on={cap !== null} label="Ask me above" onChange={(on) => void backend.setSpendCap(on ? Number(capText) || 2 : null).catch(fail)} />
            <span className="field">
              <input
                className="bare num"
                aria-label="Spending cap in dollars"
                inputMode="decimal"
                value={capText}
                onChange={(e) => setCapText(e.target.value)}
                onBlur={() => {
                  const v = Number(capText.replace(",", "."));
                  if (!Number.isFinite(v) || v <= 0) setCapText((cap ?? 2).toFixed(2));
                  else if (cap !== null && v !== cap) void backend.setSpendCap(v).catch(fail);
                }}
              />
              <span className="u">$</span>
            </span>
            <span className="faint s1">per project</span>
          </div>
        ) : (
          <span className="faint s1">Open a project to set its cap.</span>
        )}
      </Row>
      <Row title="What is sent" sub="Always visible, per call, in AI usage.">
        <div className="tile row">
          <Icon name="info" />
          <span className="dim s1">Measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span>
        </div>
      </Row>
      <Row title="All projects" sub={`Total recorded on ${THIS_COMPUTER}.`}>
        <div className="fx g2 stretch">
          <div className="stat"><span className="faint s1">Cost</span><span className="v" data-testid="total-cost">${ai.total_usd.toFixed(2)}</span></div>
          <div className="stat"><span className="faint s1">Calls</span><span className="v">{ai.total_calls}</span></div>
          <div className="stat"><span className="faint s1">Projects</span><span className="v">{ai.total_projects}</span></div>
        </div>
      </Row>
    </div>
  );
}

// ---------- 11c Appearance ----------

function AppearancePane() {
  const prefs = useApp((s) => s.prefs);
  const setPrefs = useApp((s) => s.setPrefs);
  const setView = useApp((s) => s.setView);
  const project = useApp((s) => s.project);
  return (
    <div className="form">
      <Row title="Track height" sub="Default for new projects.">
        <Seg
          className="quiet"
          label="Track height"
          value={prefs.lane}
          items={[
            { value: "s", label: "Small" },
            { value: "m", label: "Default" },
            { value: "l", label: "Tall" },
          ]}
          onChange={(lane) => {
            setPrefs({ lane });
            setView({ lane });
          }}
        />
      </Row>
      <Row title="Clips" sub="What a clip shows.">
        <Switch on={prefs.clipNames} label="Name strip on clips" onChange={(clipNames) => setPrefs({ clipNames })} />
        <Switch on={prefs.waveforms} label="Waveforms" onChange={(waveforms) => setPrefs({ waveforms })} />
        <Switch on={prefs.gainBadge} label="Clip gain badge always visible" onChange={(gainBadge) => setPrefs({ gainBadge })} />
      </Row>
      <Row title="Time" sub="Ruler and position display.">
        <Seg
          className="quiet"
          label="Time format"
          value={prefs.bars ? "bars" : "time"}
          items={[
            { value: "time", label: "Minutes : seconds" },
            { value: "bars", label: "Bars · beats" },
          ]}
          onChange={(v) => setPrefs({ bars: v === "bars" })}
        />
        <span className="faint s1">{project && !project.tempo ? "This project has no tempo: positions stay in minutes and seconds." : "Bars need a tempo and meter in the project. The ruler stays in minutes and seconds."}</span>
      </Row>
      <Row title="Meters" sub="How meters behave.">
        <Switch on={prefs.holdPeaks} label="Hold peaks for 2 seconds" onChange={(holdPeaks) => setPrefs({ holdPeaks })} />
        <Switch on={prefs.peakNumbers} label="Show numeric peak above each strip" onChange={(peakNumbers) => setPrefs({ peakNumbers })} />
      </Row>
      <Row title="Motion" sub="Panel and phase transitions.">
        <Switch on={prefs.reduceMotion} label="Reduce motion" onChange={(reduceMotion) => setPrefs({ reduceMotion })} />
      </Row>
      <Row title="Track colours" sub="Assigned in order; change any from the track menu.">
        <div className="fx g1" role="list" aria-label="Track colours">
          {Array.from({ length: 12 }, (_, i) => (
            <span key={i} role="listitem" aria-label={`Colour ${i + 1}`} className={`swt t${i + 1}`} style={{ background: "var(--tc)" }} />
          ))}
        </div>
      </Row>
    </div>
  );
}

// ---------- 11d Shortcuts ----------

function ShortcutsPane() {
  const [q, setQ] = useState("");
  const groups = useMemo(() => {
    const f = q.trim().toLowerCase();
    const hit = (s: Shortcut) => !f || s.label.toLowerCase().includes(f) || keysInText(s.keys).toLowerCase().includes(f) || s.group.toLowerCase().includes(f);
    const out = new Map<string, Shortcut[]>();
    for (const s of SHORTCUTS.filter(hit)) out.set(s.group, [...(out.get(s.group) ?? []), s]);
    return [...out];
  }, [q]);
  return (
    <div className="form">
      <div className="fx between">
        <span className="input sm">
          <Icon name="filter" size="m" />
          <input className="bare" aria-label="Filter shortcuts" placeholder="Filter shortcuts" value={q} onChange={(e) => setQ(e.target.value)} />
        </span>
        <span className="faint s1">Shortcuts can be viewed here; changing them is not part of this version.</span>
      </div>
      <div className="sheet c3" style={{ padding: 0 }} data-testid="shortcuts">
        {groups.map(([g, list]) => (
          <div key={g} className="col g1" role="group" aria-label={g}>
            <span className="b">{g}</span>
            {list.map((s) => (
              <div key={s.id} className="fx between" style={{ height: "var(--ctl-h)" }}>
                <span className="dim">{s.label}</span>
                <span className="kbd" aria-label={`Shortcut ${keysInText(s.keys)}`}>{keysInText(s.keys)}</span>
              </div>
            ))}
          </div>
        ))}
        {!groups.length && <span className="dim">No shortcut matches “{q}”.</span>}
      </div>
    </div>
  );
}
