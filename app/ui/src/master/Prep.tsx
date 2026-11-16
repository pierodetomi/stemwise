// Preparing the master (design 9d–9k): while the mixdown and the references are read and measured and the master is
// brought to the target loudness, the workspace is not usable, so an overlay with the steps covers it and the full
// workspace appears at once when every measure is ready. Going back to Mix keeps the work going; there is no cancel.
// The steps and their progress come from the backend (`MasterStatus.prep`); the words are written here from `kind`.
import { useEffect, useRef, useState } from "react";
import type { MasterPrep } from "../bindings/MasterPrep";
import type { MasterRetarget } from "../bindings/MasterRetarget";
import type { MasterStatus } from "../bindings/MasterStatus";
import type { PrepStep } from "../bindings/PrepStep";
import { Icon } from "../shared/Icon";
import { formatClock } from "../shared/format";
import { useApp, useBackend } from "../state/context";
import { targetName } from "./bind";

/** `--wait-show`: a background wait shows its overlay only past this. */
export const WAIT_SHOW_MS = 250;
/** `--wait-min`: once shown, it stays at least this long, so it never flickers. */
export const WAIT_MIN_MS = 600;
/** `--dur-3`: the overlay fades out over this. */
const FADE_MS = 220;

/** Whether a wait is shown: only after `WAIT_SHOW_MS`, then for at least `WAIT_MIN_MS` plus the fade. `leaving` is true
 * while it fades out after the work ended. */
export function useWaitShown(active: boolean) {
  const [shown, setShown] = useState(false);
  const since = useRef(0);
  useEffect(() => {
    if (active && !shown) {
      const t = window.setTimeout(() => {
        since.current = Date.now();
        setShown(true);
      }, WAIT_SHOW_MS);
      return () => window.clearTimeout(t);
    }
    if (!active && shown) {
      const t = window.setTimeout(() => setShown(false), Math.max(FADE_MS, WAIT_MIN_MS - (Date.now() - since.current)));
      return () => window.clearTimeout(t);
    }
    return undefined;
  }, [active, shown]);
  return { shown, leaving: shown && !active };
}

const kHz = (r: number) => `${(r / 1000).toFixed(r % 1000 ? 1 : 0)} kHz`;
const refsText = (n: number) => (n === 0 ? "no references" : n === 1 ? "1 reference" : `${n} references`);

/** What a step says: its label, its second line, and what shows on the right. */
function words(s: PrepStep, prep: MasterPrep, target: string) {
  const name = s.references[0] ?? "the reference";
  const label =
    s.kind === "read_mixdown"
      ? `Reading Mixdown ${prep.mixdown}`
      : s.kind === "measure_mixdown"
        ? "Measuring the mixdown"
        : s.kind === "convert_reference"
          ? s.state === "skipped"
            ? `${name} can’t be read`
            : `Converting ${name} to ${kHz(prep.rate)}`
          : s.kind === "measure_references"
            ? "Measuring the references"
            : "Bringing the master to the target loudness";
  const sub =
    s.state === "failed" || (s.state === "skipped" && s.detail)
      ? `${s.detail ?? ""}${s.state === "skipped" ? " · skipped" : ""}`
      : s.kind === "measure_mixdown"
        ? "Loudness, peaks, spectrum, waveform"
        : s.kind === "convert_reference"
          ? `From ${kHz(s.from_rate ?? prep.rate)} · the copy stays in the project`
          : s.kind === "measure_references"
            ? s.references.join(", ")
            : s.kind === "loudness"
              ? target
              : "";
  const right =
    s.state === "waiting"
      ? "Waiting"
      : s.state === "running"
        ? s.fraction > 0
          ? `${Math.round(s.fraction * 100)} %`
          : "Working…"
        : s.state === "done"
          ? s.source === "bounce"
            ? "From the bounce"
            : s.source === "saved"
              ? "Saved in the project"
              : s.ms !== null
                ? `${(s.ms / 1000).toFixed(1)} s`
                : "Done"
          : s.state === "skipped"
            ? "Skipped"
            : s.state === "failed"
              ? "Failed"
              : "Not started";
  return { label, sub, right };
}

const CLS: Record<string, string> = { done: "done", running: "now", skipped: "warn", failed: "fail", not_started: "later", waiting: "" };

function Steps({ prep, target }: { prep: MasterPrep; target: string }) {
  return (
    <div className="steps phase" data-testid="prep-steps">
      {prep.steps.map((s, i) => {
        const w = words(s, prep, target);
        return (
          <div key={`${s.kind}-${i}`} className={`step ${CLS[s.state] ?? ""}`} data-state={s.state}>
            <span className="n">
              {s.state === "done" ? <Icon name="check" size="sm" /> : s.state === "skipped" ? <Icon name="warn" size="sm" /> : s.state === "failed" ? <Icon name="x" size="sm" /> : i + 1}
            </span>
            <span className="col grow">
              <span>{w.label}</span>
              {w.sub && <span className="sub s1">{w.sub}</span>}
            </span>
            <span className={`s1 ${s.state === "failed" ? "dangerc" : s.state === "running" ? "" : "faint"}`}>{w.right}</span>
          </div>
        );
      })}
    </div>
  );
}

/** The workspace's shells behind the overlay: names only, no values (9d). */
export function PrepGhost({ mixdown }: { mixdown: number | null }) {
  const mods = ["Input", "EQ", "Match EQ", "Compressor", "Width", "Saturation", "Limiter", "Dither"];
  return (
    <div className="body ghost" aria-hidden="true" data-testid="prep-ghost">
      <div className="left">
        <section className="card">
          <div className="tools"><span className="b faint">{mixdown ? `Mixdown ${mixdown}` : "Mixdown"}</span></div>
          <div className="mwave" />
        </section>
        <div className="mods">
          {mods.map((m, i) => (
            <div key={m} className="mod">
              <span className="fx g2"><span className="faint s1">{i + 1}</span><span className="h s faint">{m}</span></span>
              <span className="faint s1">&nbsp;</span>
              <span className="faint s1">&nbsp;</span>
            </div>
          ))}
        </div>
        <section className="card grow"><div className="ph"><span className="h">EQ</span></div></section>
      </div>
      <aside className="card side"><div className="ph"><span className="h">Loudness</span></div></aside>
      <aside className="card side"><div className="ph"><span className="h">Assistant</span></div></aside>
    </div>
  );
}

/** The overlay itself (9d, 9e, 9h; 9j when it failed). */
export function PrepOverlay({ ms, prep, leaving }: { ms: MasterStatus; prep: MasterPrep; leaving: boolean }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const back = useRef<HTMLButtonElement>(null);
  const [, tick] = useState(0);
  // The elapsed time moves on its own between reports.
  const [t0] = useState(() => Date.now() - prep.elapsed_ms);
  useEffect(() => {
    const id = window.setInterval(() => tick((n) => n + 1), 500);
    return () => window.clearInterval(id);
  }, []);
  useEffect(() => back.current?.focus(), []);
  const fail = (e: unknown) => setError(String(e));
  const toMix = () => void backend.setPhase(false).catch(fail);
  const target = targetName(project.master);
  const err = prep.error;
  const pct = Math.round(Math.min(1, leaving ? 1 : prep.fraction) * 100);
  const elapsed = formatClock((Date.now() - t0) / 1000);
  const eta = leaving ? "almost done" : prep.eta_ms === null ? null : prep.eta_ms < 1000 ? "almost done" : `about ${Math.max(1, Math.round(prep.eta_ms / 1000))} s left`;
  void ms;
  if (err) {
    const stepName = words(prep.steps.find((s) => s.kind === err.step) ?? prep.steps[0], prep, target).label;
    const unreadable = err.kind === "mixdown_unreadable";
    return (
      <div className="scrim below-top">
        <div className="dialog" role="status" aria-live="polite" aria-label="The master can’t be prepared" data-testid="prep-error">
          <div className="dlg-h">
            <span className="h l">The master can’t be prepared</span>
            <span className="dim text">
              {unreadable
                ? `Mixdown ${prep.mixdown} could not be read: the file is missing from the project or damaged.`
                : `Something went wrong while ${stepName.charAt(0).toLowerCase()}${stepName.slice(1)}.`}
            </span>
          </div>
          <div className="dlg-b">
            <Steps prep={prep} target={target} />
            <div className="tile row">
              <Icon name="info" />
              <span className="dim s1">
                {unreadable
                  ? "Your mix and your master settings are safe. Bouncing again writes a new mixdown from the current mix and keeps every master setting."
                  : "Your mix and your master settings are safe. Nothing in the project changed."}
              </span>
            </div>
          </div>
          <div className="dlg-f">
            <button
              type="button"
              className="btn quiet"
              onClick={() =>
                void navigator.clipboard
                  ?.writeText(`Mixdown ${prep.mixdown} · ${err.kind} · ${err.step}: ${err.detail}`)
                  .then(() => toast("ok", "Details copied", ""))
              }
            >
              <Icon name="copy" size="m" />
              Copy details
            </button>
            <span className="grow" />
            <button type="button" className="btn" ref={back} onClick={toMix}>Back to Mix</button>
            {unreadable ? (
              <button type="button" className="btn primary" onClick={() => setView({ bounceOpen: true })}>
                <Icon name="wave" size="m" />
                Bounce again
              </button>
            ) : (
              <button type="button" className="btn primary" onClick={() => void backend.masterPrepareRetry().catch(fail)}>
                Try again
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }
  const running = prep.steps.find((s) => s.state === "running");
  return (
    <div className={`scrim below-top prep ${leaving ? "is-leaving" : ""}`}>
      <div className="dialog" role="status" aria-live="off" aria-label="Preparing the master" data-testid="prep-overlay">
        <div className="dlg-h">
          <span className="h l">Preparing the master</span>
          <span className="dim text">
            Mixdown {prep.mixdown} · {formatClock(prep.frames / Math.max(1, prep.rate))} · {kHz(prep.rate)} · {refsText(prep.references)}
          </span>
        </div>
        <div className="dlg-b">
          <div className="col g1">
            <div className="progress" role="progressbar" aria-label="Preparing the master" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
              <i style={{ width: `${pct}%` }} />
            </div>
            <div className="fx between">
              <span className="b" data-testid="prep-percent">{pct} %</span>
              <span className="faint s1">{elapsed} elapsed{eta ? ` · ${eta}` : ""}</span>
            </div>
          </div>
          {/* Announced once per step, not at every percent (UI-7). */}
          <span className="sr-only" aria-live="polite">{running ? words(running, prep, target).label : ""}</span>
          <Steps prep={prep} target={target} />
          <div className="tile row">
            <Icon name="info" />
            <span className="dim s1">
              {prep.references
                ? "The master opens when every measure is ready, so the first thing you hear is already at the target loudness. Saved in the project, so this runs once per mixdown and reference."
                : "No references in this project, so there is nothing to convert or compare. Add one in Project settings."}
            </span>
          </div>
        </div>
        <div className="dlg-f">
          <span className="faint s1 grow">Going back to Mix keeps this going.</span>
          <button type="button" className="btn" ref={back} onClick={toMix}>
            Back to Mix <span className="kbd">Esc</span>
          </button>
        </div>
      </div>
    </div>
  );
}

const lv = (v: number) => v.toFixed(1).replace(/\.0$/, "").replace("-", "−");

/** The share of a retarget's work spent searching the gain; the measure of the result is the rest. */
const SEARCH_SHARE = 0.85;

/** Bringing the master to a new destination (9v): the loudness or the ceiling changed, and until the gain is found
 * again the change is not heard. The same overlay as the preparation, over the workspace as it is; it goes when the
 * new level is in place. Going back to Mix keeps the work going; undoing is one step once it is done. */
export function RetargetOverlay({ retarget, leaving }: { retarget: MasterRetarget; leaving: boolean }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const setError = useApp((s) => s.setError);
  const back = useRef<HTMLButtonElement>(null);
  const [, tick] = useState(0);
  const [t0] = useState(() => Date.now() - retarget.elapsed_ms);
  useEffect(() => {
    const id = window.setInterval(() => tick((n) => n + 1), 500);
    return () => window.clearInterval(id);
  }, []);
  useEffect(() => back.current?.focus(), []);
  const t = project.master.target;
  const f = leaving ? 1 : Math.min(1, retarget.fraction);
  const pct = Math.round(f * 100);
  const measuring = f >= SEARCH_SHARE;
  const search = Math.min(1, f / SEARCH_SHARE);
  const sameLoudness = Math.abs(t.lufs - retarget.from_lufs) < 1e-9;
  const steps: [string, string, string, string][] = [
    [
      measuring ? "done" : "now",
      sameLoudness ? `Finding the gain under the ${lv(t.true_peak_db)} dBTP ceiling` : `Finding the gain for ${lv(t.lufs)} LUFS`,
      "The whole master is rendered and measured until it is on target",
      measuring ? "Done" : search > 0 ? `${Math.round(search * 100)} %` : "Working…",
    ],
    [leaving ? "done" : measuring ? "now" : "", "Measuring the result", "Loudness, true peak, range", leaving ? "Done" : measuring ? "Working…" : "Waiting"],
  ];
  return (
    <div className={`scrim below-top prep ${leaving ? "is-leaving" : ""}`}>
      <div className="dialog" role="status" aria-live="off" aria-label="Bringing the master to the new target" data-testid="retarget-overlay">
        <div className="dlg-h">
          <span className="h l">Bringing the master to the new target</span>
          <span className="dim text" data-testid="retarget-sub">
            {targetName(project.master)} · was {lv(retarget.from_lufs)} LUFS, {lv(retarget.from_true_peak_db)} dBTP
          </span>
        </div>
        <div className="dlg-b">
          <div className="col g1">
            <div className="progress" role="progressbar" aria-label="Bringing the master to the new target" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
              <i style={{ width: `${pct}%` }} />
            </div>
            <div className="fx between">
              <span className="b">{pct} %</span>
              <span className="faint s1">{formatClock((Date.now() - t0) / 1000)} elapsed</span>
            </div>
          </div>
          <span className="sr-only" aria-live="polite">{steps[measuring ? 1 : 0][1]}</span>
          <div className="steps phase" data-testid="retarget-steps">
            {steps.map(([cls, label, sub, right], i) => (
              <div key={i} className={`step ${cls}`}>
                <span className="n">{cls === "done" ? <Icon name="check" size="sm" /> : i + 1}</span>
                <span className="col grow">
                  <span>{label}</span>
                  <span className="sub s1">{sub}</span>
                </span>
                <span className={`s1 ${cls === "now" ? "" : "faint"}`}>{right}</span>
              </div>
            ))}
          </div>
          <div className="tile row">
            <Icon name="info" />
            <span className="dim s1">
              Until this is done the master still plays at the level it had. The workspace comes back when the new level is in place, so what you hear and what you read are the new target.
            </span>
          </div>
        </div>
        <div className="dlg-f">
          <span className="faint s1 grow">Going back to Mix keeps this going.</span>
          <button type="button" className="btn" ref={back} onClick={() => void backend.setPhase(false).catch((e) => setError(String(e)))}>
            Back to Mix <span className="kbd">Esc</span>
          </button>
        </div>
      </div>
    </div>
  );
}

/** The ring on the Master segment while the preparation runs in Mix (9f). */
export function PrepRing({ fraction }: { fraction: number }) {
  const c = 2 * Math.PI * 6;
  return (
    <svg className="ring" viewBox="0 0 16 16" aria-hidden="true" data-testid="prep-ring">
      <circle className="bgc" cx="8" cy="8" r="6" />
      <circle className="fgc" cx="8" cy="8" r="6" strokeDasharray={`${(fraction * c).toFixed(1)} ${c.toFixed(1)}`} />
    </svg>
  );
}

/** In Mix: a toast when the preparation ends (9g) or fails, with the way to Master. */
export function usePrepToasts(ms: MasterStatus | null) {
  const backend = useBackend();
  const toast = useApp((s) => s.toast);
  const setError = useApp((s) => s.setError);
  const project = useApp((s) => s.project);
  const bounceOpen = useApp((s) => s.bounceOpen);
  const prev = useRef<MasterPrep | null>(null);
  useEffect(() => {
    const was = prev.current;
    const now = ms?.prep ?? null;
    prev.current = now;
    if (!ms || ms.phase !== "mix" || bounceOpen || !project) return;
    const open = { label: "Open Master", run: () => void backend.setPhase(true).catch((e) => setError(String(e))) };
    if (was && !was.error && !now && ms.measures) {
      const l = ms.measures.lufs;
      toast("ok", "Master is ready", `Mixdown ${was.mixdown} is measured and at ${l === null ? "the target" : `${l.toFixed(0).replace("-", "−")} LUFS`} for ${targetName(project.master).split(" · ")[0]}.`, open);
    } else if (now?.error && !was?.error) {
      toast("err", "The master can’t be prepared", now.error.detail, { ...open, label: "Show" });
    }
  }, [ms, backend, toast, setError, project, bounceOpen]);
}
