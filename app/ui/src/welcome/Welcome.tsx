// Welcome (design 1a, 1b; PRJ-1, UI-2): recent projects, new project, open; on first launch (no recent projects)
// the right pane invites to set up the assistant, which can be skipped.
import { useEffect, useState } from "react";
import { THIS_COMPUTER } from "../shared/platform";
import type { AppInfo, RecentProject } from "../backend/types";
import { Icon, Mark } from "../shared/Icon";
import { useApp, useBackend } from "../state/context";
import { setPendingDrop } from "./NewProject";
import { TOUR_TEXT } from "../tour/text";
import { useTakeTour } from "../tour/useTour";

function when(ms: number) {
  const d = new Date(ms);
  const now = new Date();
  const day = 86_400_000;
  const t = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  if (d.toDateString() === now.toDateString()) return `Today ${t}`;
  if (new Date(now.getTime() - day).toDateString() === d.toDateString()) return "Yesterday";
  return d.toLocaleDateString("en-GB", { day: "numeric", month: "short" });
}

function Thumb({ seed }: { seed: number }) {
  return (
    <span className={`thumb t${(seed % 12) + 1}`} aria-hidden="true">
      {Array.from({ length: 9 }, (_, i) => (
        <i key={i} style={{ height: `${20 + ((seed * 7 + i * 37) % 70)}%` }} />
      ))}
    </span>
  );
}

/** 1b: the first launch invites to set up the assistant; it can be skipped, and the offline demo needs nothing. */
function FirstRun({ onSkip }: { onSkip: () => void }) {
  const backend = useBackend();
  const ai = useApp((s) => s.ai);
  const setView = useApp((s) => s.setView);
  const toast = useApp((s) => s.toast);
  return (
    <>
      <div className="ph">
        <h2 className="h">Set up the assistant</h2>
        <span className="tag">Optional</span>
      </div>
      <div className="sect g4">
        <p className="text dim">
          Stemwise asks a language model for mix and master proposals through an OpenRouter account that is yours. Setting it up takes about two minutes; you can also do it later in Settings.
        </p>
        <ol className="steps" style={{ margin: 0, padding: 0, listStyle: "none" }}>
          {[["Connect your OpenRouter account", "in the browser"], ["Add a little credit", "$5 is plenty to start"], ["Ask for your first mix", ""]].map(([t, hint], i) => (
            <li key={t} className="step" style={{ color: "var(--tx-2)" }}>
              <span className="n">{i + 1}</span>
              <span className="grow">{t}</span>
              {hint && <span className="s1 faint">{hint}</span>}
            </li>
          ))}
        </ol>
        <div className="fx g2">
          <button type="button" className="btn primary lg" onClick={() => setView({ setup: "connect" })}>
            <Icon name="spark" size="m" />
            Set up the assistant
          </button>
          <button type="button" className="btn lg" onClick={onSkip}>Skip for now</button>
        </div>
        <div className="tile row" style={{ height: "auto" }}>
          <Icon name="lock" className="aic" />
          <div className="col">
            <span className="b">Your audio never leaves {THIS_COMPUTER}</span>
            <span className="dim s1">The model receives measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span>
          </div>
        </div>
        <div className="hr" />
        <div className="tile row" style={{ height: "auto" }}>
          <Icon name="spark" />
          <div className="col grow">
            <span className="b">Try the offline demo</span>
            <span className="dim s1">A built-in rule-based assistant that needs no account and no network. Good for a first look.</span>
          </div>
          <button
            type="button"
            className="btn sm"
            onClick={() => {
              if (!ai) return;
              void backend.aiSetConfig({ ...ai.config, offline_demo: true }).then(() => toast("ai", "Offline demo on", "The assistant answers from built-in rules: no account, no network, no cost."));
              onSkip();
            }}
          >
            Use offline demo
          </button>
        </div>
      </div>
      <div className="foot">
        <span className="faint s1">No recent projects yet.</span>
      </div>
    </>
  );
}

export function Welcome() {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const setView = useApp((s) => s.setView);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [recent, setRecent] = useState<RecentProject[] | null>(null);
  const [filter, setFilter] = useState("");
  const [over, setOver] = useState(false);
  const [skipped, setSkipped] = useState(false);
  const ai = useApp((s) => s.ai);
  useEffect(() => {
    backend.appInfo().then(setInfo).catch(() => {});
    backend.recentProjects().then(setRecent).catch(() => setRecent([]));
  }, [backend]);
  useEffect(
    () =>
      backend.onFileDrop((e) => {
        setOver(e.type === "over");
        if (e.type === "drop" && e.paths.length) {
          // A folder of stems dropped on the window starts a new project from it.
          setPendingDrop(e.paths);
          setView({ screen: "new" });
        }
      }),
    [backend, setView],
  );
  const open = async (path: string | null) => {
    if (!path) return;
    try {
      const recover = await backend.openProject(path);
      if (recover) {
        setView({
          dialog: {
            title: "Recover unsaved changes?",
            sub: "Stemwise closed while this project had changes that were not saved. They were kept in an automatic copy.",
            confirm: "Recover changes",
            onConfirm: () => void backend.recoverProject(true).catch((e) => setError(String(e))),
          },
        });
      }
    } catch (e) {
      setError(String(e));
    }
  };
  const openSettings = useApp((s) => s.openSettings);
  const tour = useTakeTour();
  const list = (recent ?? []).filter((r) => r.name.toLowerCase().includes(filter.toLowerCase()));
  const first = recent !== null && recent.length === 0;
  return (
    <>
    <div className={`page ${over ? "is-drop-over" : ""}`}>
      <div className="grow fx">
        <div className="hero">
          <div className="col g2">
            <Mark large brand />
            <h1 className="h xl">Stemwise</h1>
            <p className="text dim" style={{ maxWidth: "var(--dlg-s)" }}>
              Mix and master the songs you have already recorded. An assistant proposes, you listen and decide.
            </p>
          </div>
          <div className="fx g2">
            <button type="button" className="btn primary lg" onClick={() => setView({ screen: "new" })}>
              <Icon name="plus" size="m" />
              New project
            </button>
            <button type="button" className="btn lg" onClick={async () => open(await backend.pickProject())}>
              <Icon name="folder" size="m" />
              Open project…
            </button>
          </div>
          {/* UI-17: the tour again, from here (design 18s). */}
          <button type="button" className="tourlink" title={TOUR_TEXT.welcomeTip} disabled={tour.busy} onClick={tour.take}>
            <Icon name="play" size="sm" />
            <span>{TOUR_TEXT.welcomeLink}</span>
            <span className="faint">{TOUR_TEXT.welcomeLength}</span>
          </button>
          <span className="faint s1">Drop a folder of stems anywhere on this window to start a project from it.</span>
          <button type="button" className="btn quiet sm" style={{ alignSelf: "flex-start" }} onClick={() => openSettings()}>
            <Icon name="gear" size="m" />
            Settings
          </button>
        </div>
      </div>
      <div className="pane welcome-pane">
        {first && !skipped && ai && !ai.has_key && !ai.config.offline_demo ? (
          <FirstRun onSkip={() => setSkipped(true)} />
        ) : (
          <>
            <div className="ph">
              <h2 className="h">Recent projects</h2>
              <div className="grow" />
              <label className="input sm">
                <Icon name="filter" size="m" />
                <input className="bare" placeholder="Filter" aria-label="Filter recent projects" value={filter} onChange={(e) => setFilter(e.target.value)} />
              </label>
            </div>
            <div className="sect list scroll grow" role="group" aria-label="Recent projects">
              {list.map((p, i) => (
                <button type="button" key={p.path} className="li" onClick={() => open(p.path)} title={p.path}>
                  <Thumb seed={[...p.name].reduce((a, c) => a + c.charCodeAt(0), i)} />
                  <div className="col grow">
                    <span className="b ell">{p.name}</span>
                    <span className="faint s1 ell">{p.path.replace(/\/[^/]+$/, "")}</span>
                  </div>
                  <span className="faint s1 nowrap" style={{ width: "var(--sp-8)", textAlign: "right" }}>
                    {when(p.opened_ms)}
                  </span>
                </button>
              ))}
              {recent !== null && !list.length && <span className="dim">{recent.length ? "No project matches." : "No recent projects yet."}</span>}
            </div>
            <div className="foot">
              <span className="faint s1">Projects are folders ending in .stemwise. Imported audio is copied inside; your originals are never touched.</span>
            </div>
          </>
        )}
      </div>
    </div>
    <div className="welcome-foot fx g4">
      <span className="faint s1" data-testid="backend-kind">
        {info ? `Version ${info.version} · ${info.backend} backend` : "…"}
      </span>
    </div>
    </>
  );
}
