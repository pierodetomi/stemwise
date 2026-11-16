// The assistant's controls in the Mix workspace (design 3a foot, 5a, 12e, 13e): the request field with scope and
// cost before sending (AI-3, AI-5, AI-16), Auto mix (AI-1), progress per phase and pass with Cancel (AI-12), the
// missing-key state (AI-11), the spending-cap confirmation, the brief and references (PRJ-7).
import { useEffect, useState, type ReactNode } from "react";
import type { AiStatus, CostEstimate } from "../backend/types";
import { Icon } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useApp, useBackend } from "../state/context";

/** Where this build keeps the key, in words (A-072). */
export const keyHome = (place: "keychain" | "file" | undefined) => (place === "file" ? "a private file of this app on your Mac" : "the macOS keychain");

export const usd = (v: number) => (v < 0.995 ? `$${v.toFixed(2)}` : `$${v.toFixed(2)}`);

/** Starts a call after the estimate; asks first when it would take the project over its cap (AI-16). */
export function useStartCall() {
  const backend = useBackend();
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const [cap, setCap] = useState<{ est: CostEstimate; go: () => void } | null>(null);
  const start = async (request: string | null, scope: string[] | null) => {
    try {
      const est = await backend.aiEstimate(request !== null);
      const go = () =>
        void backend.aiStart(request, scope).catch((e) => {
          if (String(e) === "no-key" || String(e).includes("no-key")) setView({ dialog: null });
          setError(String(e) === "no-key" ? "Set up the assistant, or use the offline demo." : String(e));
        });
      if (est.over_cap) setCap({ est, go });
      else go();
    } catch (e) {
      setError(String(e));
    }
  };
  const dialog = cap ? <CapDialog est={cap.est} onCancel={() => setCap(null)} onRun={(raise) => {
    const c = cap;
    setCap(null);
    const run = raise ? backend.setSpendCap(Math.ceil((c.est.spent_usd + c.est.cost_usd) * 2) / 2 + 1) : Promise.resolve();
    void run.then(c.go);
  }} /> : null;
  return { start, dialog };
}

function CapDialog({ est, onCancel, onRun }: { est: CostEstimate; onCancel: () => void; onRun: (raise: boolean) => void }) {
  const [raise, setRaise] = useState(false);
  const setView = useApp((s) => s.setView);
  const next = Math.ceil((est.spent_usd + est.cost_usd) * 2) / 2 + 1;
  return (
    <div className="scrim">
      <div className="dialog" role="dialog" aria-modal="true" aria-label="Spending cap" onKeyDown={(e) => e.key === "Escape" && onCancel()}>
        <div className="dlg-h">
          <span className="h l">This request goes over your spending cap</span>
          <span className="dim text">You asked to be warned above {usd(est.cap_usd ?? 0)} per project.</span>
        </div>
        <div className="dlg-b">
          <div className="fx g2 stretch">
            <div className="stat"><span className="faint s1">Spent so far</span><span className="v">{usd(est.spent_usd)}</span></div>
            <div className="stat"><span className="faint s1">This request, estimated</span><span className="v">{usd(est.cost_usd)}</span><span className="faint s1">about {Math.round((est.tokens_in + est.tokens_out) / 1000)}k tokens</span></div>
            <div className="stat"><span className="faint s1">Would reach</span><span className="v warnc">{usd(est.spent_usd + est.cost_usd)}</span></div>
          </div>
          <button type="button" className="fx g2 dim" role="checkbox" aria-checked={raise} onClick={() => setRaise(!raise)}>
            <span className={`check ${raise ? "on" : ""}`}><Icon name="check" /></span>Raise the cap to {usd(next)} for this project
          </button>
        </div>
        <div className="dlg-f">
          <button type="button" className="btn quiet" onClick={onCancel}>Cancel</button>
          <button type="button" className="btn" onClick={() => { onCancel(); setView({ screen: "usage" }); }}>Change cap…</button>
          <button type="button" className="btn primary" autoFocus onClick={() => onRun(raise)}>Run anyway</button>
        </div>
      </div>
    </div>
  );
}

const PHASES = { analysis: "Analysis", decision: "Deciding", render: "Rendering", evaluation: "Evaluating" } as const;

/** 5a: progress per phase and per pass; the workspace stays usable meanwhile. */
export function Working({ ai }: { ai: AiStatus }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const [, tick] = useState(0);
  useEffect(() => {
    const t = setInterval(() => tick((x) => x + 1), 1000);
    return () => clearInterval(t);
  }, []);
  const r = ai.running!;
  const p = r.progress;
  const of = p?.of ?? (r.kind === "revision" ? 1 : 3);
  const it = p?.iteration ?? 0;
  const order = ["decision", "render", "evaluation"];
  const frac = !p ? 0 : p.phase === "analysis" ? 0.1 * p.fraction : 0.1 + (0.9 * (it - 1 + (order.indexOf(p.phase) + p.fraction) / 3)) / of;
  const secs = Math.max(0, Math.round((Date.now() - r.started_ms) / 1000));
  const elapsed = `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, "0")}`;
  const title = r.kind === "revision" ? "Revision in progress" : "Auto mix in progress";
  const tracks = project.tracks.filter((t) => t.kind === "audio").length;
  return (
    <>
      <div className="sect" aria-live="polite">
        <div className="fx between">
          <span className="b">{title}</span>
          <span className="faint s1">{elapsed} elapsed</span>
        </div>
        <div className="progress ai" role="progressbar" aria-valuenow={Math.round(frac * 100)} aria-label="Progress">
          <i style={{ width: `${Math.round(frac * 100)}%` }} />
        </div>
        <div className="steps">
          <div className={`step ${p && p.phase !== "analysis" ? "done" : "now"}`}>
            <span className="n">{p && p.phase !== "analysis" ? <Icon name="check" size="sm" /> : 1}</span>
            <span className="grow">Analysis · {tracks} tracks, {project.sections.length} sections</span>
          </div>
          {Array.from({ length: of }, (_, i) => i + 1).map((k) => {
            const done = it > k || (it === k && p?.phase === "evaluation" && p.fraction >= 1);
            const now = it === k && !done;
            return (
              <div key={k} className={`step ${done ? "done" : now ? "now" : ""}`}>
                <span className="n">{done ? <Icon name="check" size="sm" /> : k + 1}</span>
                <span className="grow">
                  {r.kind === "revision" ? "Decide, render, evaluate" : `Pass ${k} · ${now && p ? PHASES[p.phase].toLowerCase() : k === 1 ? "decide, render, evaluate" : "only if it still improves"}`}
                </span>
                {now && p && <span className="s1">{Math.round(p.fraction * 100)} %</span>}
              </div>
            );
          })}
        </div>
      </div>
      <div className="sect">
        <div className="note colm">
          <span className="b">{r.kind === "revision" ? "One pass from your current mix" : `Pass ${Math.max(1, it)} of up to ${of}`}</span>
          <div className="fx g2">
            {(["decision", "render", "evaluation"] as const).map((ph) => (
              <span key={ph} className={`tag ${p?.phase === ph ? "ai" : order.indexOf(p?.phase ?? "") > order.indexOf(ph) ? "ok" : ""}`}>{PHASES[ph]}</span>
            ))}
          </div>
          <span className="dim">The best pass is kept even if a later one is worse.</span>
        </div>
        <span className="faint s1">Nothing changes in your mix until you review and apply the proposal. You can keep working meanwhile.</span>
      </div>
      <div className="foot">
        <div className="fx between">
          <span className="faint s1">Estimated for this run {usd(r.estimate_usd)}</span>
          <button type="button" className="btn" onClick={() => void backend.aiCancel()}>
            <Icon name="x" size="m" />
            Cancel
          </button>
        </div>
        <CostLine ai={ai} />
      </div>
    </>
  );
}

function CostLine({ ai }: { ai: AiStatus }) {
  const project = useApp((s) => s.project)!;
  const setView = useApp((s) => s.setView);
  const spent = project.ai.calls.reduce((a, c) => a + c.cost_usd, 0);
  void ai;
  return (
    <div className="fx between">
      <span className="faint s1">AI cost so far {usd(spent)} · {project.ai.calls.length} calls</span>
      <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: null })}>AI usage</button>
    </div>
  );
}

/** 13e: no key, and not on the offline demo. */
function KeyBanner({ onKey }: { onKey: () => void }) {
  const backend = useBackend();
  const ai = useApp((s) => s.ai)!;
  return (
    <div className="sect">
      <div className="banner" style={{ alignItems: "flex-start" }} data-testid="key-banner">
        <Icon name="key" />
        <div className="col g2 grow">
          <span className="b">The assistant is not set up</span>
          <span className="dim s1">Connect an OpenRouter account to ask the assistant, or switch to the offline demo.</span>
          <div className="fx g2 wrap">
            <button type="button" className="btn primary" onClick={onKey}>Set up the assistant…</button>
            <button type="button" className="btn" onClick={() => void backend.aiSetConfig({ ...ai.config, offline_demo: true })}>Use offline demo</button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** 13f: the last call could not reach OpenRouter. Same place and shape as the key banner. */
function OfflineBanner({ onRetry }: { onRetry: () => void }) {
  const backend = useBackend();
  const ai = useApp((s) => s.ai)!;
  return (
    <div className="sect">
      <div className="banner" role="alert" style={{ alignItems: "flex-start" }} data-testid="offline-banner">
        <Icon name="offline" />
        <div className="col g2 grow">
          <span className="b">No connection to OpenRouter</span>
          <span className="dim s1">Your mix is not affected. Requests will work again when the network is back.</span>
          <div className="fx g2 wrap">
            <button type="button" className="btn primary" onClick={onRetry}>Try again</button>
            <button type="button" className="btn" onClick={() => void backend.aiSetConfig({ ...ai.config, offline_demo: true })}>Use offline demo</button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** 14i–14k: the provider refused the last call. Same place and shape as the other two banners. */
function RefusalBanner({ kind }: { kind: "credit" | "cap" | "revoked" }) {
  const backend = useBackend();
  const ai = useApp((s) => s.ai)!;
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const page = (which: "keys" | "credits") => void backend.aiOpenPage(which).catch((e) => setError(String(e)));
  const demo = <button type="button" className="btn" onClick={() => void backend.aiSetConfig({ ...ai.config, offline_demo: true })}>Use offline demo</button>;
  const connect = (label: string, primary: boolean) => (
    <button type="button" className={`btn ${primary ? "primary" : ""}`} onClick={() => setView({ setup: "connect" })}>
      {primary && <Icon name="link" size="m" />}
      {label}
    </button>
  );
  const [title, text, actions] = {
    credit: [
      "Your OpenRouter credit has run out",
      "The request was not run and nothing in your mix changed. Add credit, then ask again.",
      <><button type="button" className="btn primary" onClick={() => page("credits")}><Icon name="link" size="m" />Add credit…</button>{demo}</>,
    ],
    cap: [
      "The key has reached its own limit",
      "A credit limit is set on this key in your OpenRouter account. Raise it there, or connect again for a new key.",
      <><button type="button" className="btn primary" onClick={() => page("keys")}><Icon name="link" size="m" />Open keys page</button>{connect("Connect again", false)}</>,
    ],
    revoked: [
      "OpenRouter no longer accepts this key",
      "It was deleted or has expired. Connect again to get a new one.",
      <>{connect("Connect with OpenRouter", true)}{demo}</>,
    ],
  }[kind] as [string, string, ReactNode];
  return (
    <div className="sect">
      <div className={`banner ${kind === "revoked" ? "err" : ""}`} role="alert" style={{ alignItems: "flex-start" }} data-testid="refusal-banner">
        <Icon name={kind === "revoked" ? "key" : "warn"} />
        <div className="col g2 grow">
          <span className="b">{title}</span>
          <span className="dim s1">{text}</span>
          <div className="fx g2 wrap">{actions}</div>
        </div>
      </div>
    </div>
  );
}

/**
 * The foot of the Notes tab: request, scope, one button, estimate. The button is the only way to start a call
 * (A-063): with a request it revises the current mix, without one it is the auto mix. Enter is a new line.
 */
export function AskFoot({ onKey }: { onKey: () => void }) {
  const backend = useBackend();
  const ai = useApp((s) => s.ai);
  const project = useApp((s) => s.project)!;
  const selTrack = useApp((s) => s.selTrack);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const [text, setText] = useState("");
  // A request another screen wrote for the author (15g): it lands in the field, nothing is sent.
  const draft = useApp((s) => s.assistDraft);
  const setViewDraft = useApp((s) => s.setView);
  useEffect(() => {
    if (draft === null) return;
    setText(draft);
    setViewDraft({ assistDraft: null });
  }, [draft, setViewDraft]);
  const [scopeOne, setScopeOne] = useState(false);
  const aiOffline = useApp((s) => s.aiOffline);
  const refusal = useApp((s) => s.aiRefusal);
  const missing = useApp((s) => s.missing.length);
  const [est, setEst] = useState<CostEstimate | null>(null);
  const { start, dialog } = useStartCall();
  const sel = project.tracks.find((t) => t.id === selTrack && t.kind !== "return");
  const revision = text.trim().length > 0;
  useEffect(() => {
    let alive = true;
    backend.aiEstimate(revision).then((e) => alive && setEst(e)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [backend, revision, project.ai.calls.length, project.tracks.length, ai?.config.offline_demo]);
  if (!ai) return null;
  const blocked = !ai.has_key && !ai.config.offline_demo;
  // UI-6: a missing file would reach the assistant as a silent track; the timeline's banner says how to fix it.
  const hasAudio = project.tracks.some((t) => t.kind === "audio" && t.clips.length > 0) && missing === 0;
  const scope = scopeOne && sel ? [sel.id] : null;
  const send = () => {
    if (blocked || !hasAudio) return;
    void start(revision ? text.trim() : null, scope);
    if (revision) setText("");
  };
  const offline = aiOffline && !ai.config.offline_demo && !blocked;
  return (
    <>
      {blocked && <KeyBanner onKey={onKey} />}
      {offline && <OfflineBanner onRetry={() => void start(null, scope)} />}
      {refusal && !blocked && !ai.config.offline_demo && <RefusalBanner kind={refusal} />}
      <div className="foot">
        <label className={`ask ${text ? "is-edit" : ""}`}>
          <textarea
            className="bare grow"
            rows={2}
            aria-label="Request to the assistant"
            placeholder="Tell the assistant what to change: “vocal a touch warmer in the verses”"
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
        </label>
        <span className="faint s1" data-testid="ask-hint">
          {revision
            ? "The assistant will revise the current mix as you ask."
            : "A request is optional: without one the assistant mixes on its own, from scratch."}
        </span>
        <div className="fx between g2">
          <Seg
            className="quiet sm scope"
            label="Scope"
            value={scopeOne && sel ? "one" : "all"}
            items={[
              // The name is the user's: it gives way to the button, which never leaves the panel (issue #6).
              { value: "one", label: <span className="ell">{sel?.name ?? "Selected"}</span>, aria: `Only ${sel?.name ?? "the selected track"}`, title: sel?.name },
              { value: "all", label: "All tracks" },
            ]}
            onChange={(v) => setScopeOne(v === "one")}
          />
          <button type="button" className={`btn primary none ${blocked || !hasAudio ? "is-dis" : ""}`} disabled={blocked || !hasAudio} onClick={send}>
            <Icon name="spark" size="m" />
            {revision ? "Revise mix" : "Auto mix"}
          </button>
        </div>
        {/* Design 3a: what the assistant may add besides settings (AI-17, AI-27). */}
        <div className="fx g3 s1">
          <span className="dim">Allow</span>
          <span className="fx g1" title="New returns and groups">
            <button
              type="button"
              role="checkbox"
              aria-checked={project.ai.allow_structure}
              aria-label="Allow structural changes"
              className={`check ${project.ai.allow_structure ? "on" : ""}`}
              onClick={() => void backend.setAllowStructure(!project.ai.allow_structure).catch((e) => setError(String(e)))}
            >
              <Icon name="check" />
            </button>
            <span className="dim">structural changes</span>
          </span>
          <span className="fx g1" title="Up to three optional ideas, off until you tick them">
            <button
              type="button"
              role="checkbox"
              aria-checked={project.ai.allow_ideas}
              aria-label="Allow creative ideas"
              className={`check ${project.ai.allow_ideas ? "on" : ""}`}
              onClick={() => void backend.setAllowIdeas(!project.ai.allow_ideas).catch((e) => setError(String(e)))}
            >
              <Icon name="check" />
            </button>
            <span className="dim">creative ideas</span>
          </span>
        </div>
        <div className="fx between">
          <span className="faint s1" data-testid="ai-estimate">
            This request about {usd(est?.cost_usd ?? 0)} · {usd(est?.spent_usd ?? 0)} so far{ai.config.offline_demo ? " · offline demo" : ""}
          </span>
          <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: null })}>AI usage</button>
        </div>
      </div>
      {dialog}
    </>
  );
}
