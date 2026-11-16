// AI usage (design 12a–12d; AI-10, AI-14, AI-15, AI-16): the project's calls summarised by type and model, the
// history with filters and export, the readable detail of a call next to its raw JSON, the spending cap.
import { Fragment, useEffect, useMemo, useState } from "react";
import type { CallDetail } from "../backend/types";
import type { AiCallRecord } from "../bindings/AiCallRecord";
import { Icon, Mark } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useMenu } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import { usd } from "./Assist";

const KIND = { auto_mix: "Auto mix", revision: "Revision", master: "Master", master_revision: "Master revision" } as const;
export const OUTCOME: Record<AiCallRecord["outcome"], [string, string]> = {
  pending: ["In review", "ai"], applied: ["Applied", "ok"], applied_partly: ["Applied in part", "acc"], discarded: ["Discarded", "warn"], failed: ["Failed", "err"], cancelled: ["Cancelled", ""],
};
const k = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n));
const dur = (ms: number) => {
  const s = Math.round(ms / 1000);
  return s >= 60 ? `${Math.floor(s / 60)} m ${String(s % 60).padStart(2, "0")} s` : `${s} s`;
};
const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};
const tries = (c: AiCallRecord) => (c.kind === "auto_mix" ? `${c.iterations} pass${c.iterations === 1 ? "" : "es"}` : c.attempts > 1 ? `${c.attempts} tries` : "1");
export const short = (m: string) => m.split("/").pop() ?? m;

function Breakdown({ title, rows, note }: { title: string; rows: [string, AiCallRecord[]][]; note?: string }) {
  const total = rows.reduce((a, [, cs]) => a + cs.reduce((x, c) => x + c.cost_usd, 0), 0) || 1;
  return (
    <div className="tile grow">
      <span className="b">{title}</span>
      <div className="split">
        {rows.map(([n, cs]) => (
          <i key={n} style={{ flex: Math.max(0.5, (cs.reduce((x, c) => x + c.cost_usd, 0) / total) * 100) }} />
        ))}
      </div>
      <div className="mtable" style={{ gridTemplateColumns: "1fr auto auto auto" }}>
        <span className="th">{title.replace("By ", "").replace(/^./, (c) => c.toUpperCase())}</span>
        <span className="th r">Calls</span>
        <span className="th r">Tokens</span>
        <span className="th r">Cost</span>
        {rows.map(([n, cs]) => (
          <Fragment key={n}>
            <span className="ell">{n}</span>
            <span className="r dim">{cs.length}</span>
            <span className="r dim">{k(cs.reduce((a, c) => a + c.tokens_in + c.tokens_out, 0))}</span>
            <span className="r b">{usd(cs.reduce((a, c) => a + c.cost_usd, 0))}</span>
          </Fragment>
        ))}
        {note && <span className="faint s1" style={{ gridColumn: "1/-1", border: 0 }}>{note}</span>}
      </div>
    </div>
  );
}

function group<T>(xs: T[], key: (x: T) => string): [string, T[]][] {
  const m = new Map<string, T[]>();
  for (const x of xs) m.set(key(x), [...(m.get(key(x)) ?? []), x]);
  return [...m.entries()];
}

function Detail({ id, calls }: { id: string; calls: AiCallRecord[] }) {
  const backend = useBackend();
  const setView = useApp((s) => s.setView);
  const project = useApp((s) => s.project)!;
  const toast = useApp((s) => s.toast);
  const [d, setD] = useState<CallDetail | null>(null);
  const [raw, setRaw] = useState(false);
  const [side, setSide] = useState<"request" | "response">("response");
  useEffect(() => {
    let alive = true;
    backend.aiDetail(id).then((x) => alive && setD(x)).catch(() => alive && setD(null));
    return () => {
      alive = false;
    };
  }, [backend, id]);
  const c = calls.find((x) => x.id === id);
  if (!c) return null;
  const file = (prefix: string) => d?.files.filter(([n]) => n.startsWith(prefix)).sort((a, b) => a[0].localeCompare(b[0]));
  const lastReq = file("request-")?.at(-1)?.[1] ?? "";
  const lastRes = file("response-")?.at(-1)?.[1] ?? "";
  const planText = file("plan-")?.at(-1)?.[1] ?? "";
  const analysisText = d?.files.find(([n]) => n === "analysis.json")?.[1] ?? "";
  const pretty = (t: string) => {
    try {
      return JSON.stringify(JSON.parse(t), null, 2);
    } catch {
      return t;
    }
  };
  let analysis: { tracks?: Record<string, { name?: string; role?: string; locked?: boolean; lufs?: number; peak_db?: number; crest_db?: number; active_pct?: number }> } = {};
  let plan: { summary?: string; tracks?: Record<string, { rationale?: string; gain_db?: number }> } = {};
  try {
    analysis = JSON.parse(analysisText);
  } catch {
    /* no analysis saved */
  }
  try {
    plan = JSON.parse(planText);
  } catch {
    /* no plan: the call failed */
  }
  const [label, cls] = OUTCOME[c.outcome];
  const rawText = side === "request" ? pretty(lastReq) : pretty(lastRes);
  return (
    <section className="card grow usage-detail" aria-label="Call detail">
      <div className="ph">
        <span className="h">{KIND[c.kind]} · {hhmm(c.started_ms)}</span>
        <span className={`tag ${cls}`}>{label}</span>
        <span className="faint s1 ell">
          {short(c.model)} · {c.effort} · {c.attempts} {c.attempts === 1 ? "try" : "tries"} · {k(c.tokens_in)} in, {k(c.tokens_out)} out · {usd(c.cost_usd)}{c.cost_estimated ? " est." : ""} · {dur(c.duration_ms)}
        </span>
        <div className="grow" />
        <Seg className="quiet sm" label="View" value={raw ? 1 : 0} items={[{ value: 0, label: "Readable" }, { value: 1, label: "Raw JSON" }]} onChange={(v) => setRaw(v === 1)} />
        {c.history_label && (
          <button type="button" className="btn quiet sm" onClick={() => setView({ screen: "workspace", asstTab: 1 })}>
            <Icon name="clock" size="m" />
            History step
          </button>
        )}
      </div>
      {raw ? (
        <div className="sect grow">
          <div className="fx between">
            <span className="b">{side === "request" ? "Request" : "Response"} · {c.kind.startsWith("master") ? "master_plan" : "mix_plan"}</span>
            <div className="fx g2">
              <Seg className="quiet sm" label="Request or response" value={side} items={[{ value: "request", label: "Request" }, { value: "response", label: "Response" }]} onChange={setSide} />
              <button type="button" className="btn sm" onClick={() => void navigator.clipboard?.writeText(rawText).then(() => toast("ok", "Copied", ""))}>
                <Icon name="copy" size="m" />
                Copy
              </button>
            </div>
          </div>
          <pre className="json grow scroll" data-testid="raw-json">{rawText || "Nothing was received."}</pre>
          <span className="faint s1">Exactly what was {side === "request" ? "sent: no audio, no file names, no paths, no song title" : "received"}.</span>
        </div>
      ) : (
        <div className="fx g4 top-al grow scroll" style={{ padding: "0 var(--sp-3) var(--sp-3)" }}>
          <div className="col g3 grow">
            <span className="b">What was sent</span>
            {c.request && (
              <div className="tile">
                <span className="faint s1">Your request</span>
                <span className="text">“{c.request}”</span>
              </div>
            )}
            <div className="tile">
              <span className="faint s1">Brief</span>
              <span>{[project.brief.genre, project.brief.mood].filter(Boolean).join(" · ") || "No brief"}</span>
              {project.brief.notes && <span className="dim">{project.brief.notes}</span>}
            </div>
            <div className="mtable" style={{ gridTemplateColumns: "1.4fr 1fr repeat(4,auto)" }}>
              <span className="th">Track</span>
              <span className="th">Role</span>
              <span className="th r">LUFS</span>
              <span className="th r">Peak</span>
              <span className="th r">Crest</span>
              <span className="th r">Active</span>
              {Object.entries(analysis.tracks ?? {}).map(([tid, t]) => (
                <Fragment key={tid}>
                  <span className="b ell">{tid} · {t.name}</span>
                  <span className="dim ell">{(t.role ?? "").replace(/_/g, " ")}{t.locked ? " · locked" : ""}</span>
                  <span className="r">{t.lufs?.toFixed(1)}</span>
                  <span className="r">{t.peak_db?.toFixed(1)}</span>
                  <span className="r">{t.crest_db?.toFixed(1)}</span>
                  <span className="r">{t.active_pct !== undefined ? `${Math.round(t.active_pct)} %` : ""}</span>
                </Fragment>
              ))}
              <span className="faint s1" style={{ gridColumn: "1/-1", border: 0 }}>Ids are neutral (t1, s1); names are the track names.</span>
            </div>
          </div>
          <div className="col g3" style={{ width: "var(--dlg-s)" }}>
            <span className="b">What came back</span>
            {c.error && <div className="banner err"><Icon name="warn" /><span className="grow">{c.error}</span></div>}
            {plan.summary && (
              <div className="tile">
                <span className="faint s1">Summary</span>
                <span className="text">{plan.summary}</span>
              </div>
            )}
            {Object.entries(plan.tracks ?? {}).map(([tid, t]) => (
              <div key={tid} className="pcard">
                <div className="fx g2">
                  <span className="b grow">{analysis.tracks?.[tid]?.name ?? tid}</span>
                  {t.gain_db !== undefined && <span className="faint s1">fader {t.gain_db.toFixed(1)} dB</span>}
                </div>
                {t.rationale && <p className="why dim">{t.rationale}</p>}
              </div>
            ))}
            <span className="faint s1">{c.outcome === "failed" ? "No valid plan came back." : `Validation: plan accepted after ${c.attempts} ${c.attempts === 1 ? "try" : "tries"}. Limits and locks respected.`}</span>
          </div>
        </div>
      )}
    </section>
  );
}

export function Usage() {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const sel = useApp((s) => s.usageCall);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const ai = useApp((s) => s.ai);
  const menu = useMenu();
  const [type, setType] = useState("all");
  const [outcome, setOutcome] = useState("all");
  const [model, setModel] = useState("all");
  const [q, setQ] = useState("");
  const [capText, setCapText] = useState(project.ai.spend_cap_usd?.toFixed(2) ?? "");
  const calls = useMemo(() => [...project.ai.calls].sort((a, b) => b.started_ms - a.started_ms), [project.ai.calls]);
  const shown = calls.filter(
    (c) => (type === "all" || c.kind === type) && (outcome === "all" || c.outcome === outcome) && (model === "all" || c.model === model) && (!q || (c.request ?? "").toLowerCase().includes(q.toLowerCase())),
  );
  const fail = (e: unknown) => setError(String(e));
  const cost = calls.reduce((a, c) => a + c.cost_usd, 0);
  const estimates = calls.filter((c) => c.cost_estimated).length;
  const back = (
    <button type="button" className="btn" onClick={() => setView({ screen: "workspace", usageCall: null })}>
      <Icon name="x" size="m" />
      Back to Mix
    </button>
  );
  const picker = (label: string, value: string, set: (v: string) => void, opts: [string, string][]) => (
    <button
      type="button"
      className="select"
      aria-label={label}
      onClick={(e) => menu.open(e.currentTarget, opts.map(([v, l]) => ({ label: l, checked: value === v, onSelect: () => set(v) })), label)}
    >
      <span className="ell">{opts.find(([v]) => v === value)?.[1]}</span>
      <Icon name="chev" size="m" />
    </button>
  );
  return (
    <>
      <header className="top">
        <Mark />
        <span className="h">AI usage</span>
        <span className="faint s1">{project.name}</span>
        <div className="grow" />
        {ai && <span className="faint s1">All projects: {usd(ai.total_usd)} · {ai.total_calls} calls</span>}
        {back}
      </header>
      <div className="body">
        {!calls.length ? (
          <section className="card grow">
            <div className="empty grow">
              <Icon name="spark" />
              <span className="h">No AI calls in this project yet</span>
              <span style={{ maxWidth: "var(--dlg-s)" }}>When you run Auto mix or ask for a change in words, every call is listed here with what was sent, what came back, tokens and cost.</span>
              <button type="button" className="btn lg" onClick={() => setView({ screen: "workspace" })}>Back to Mix</button>
            </div>
          </section>
        ) : sel ? (
          <>
            <section className="card" style={{ width: "var(--dlg-m)", flex: "none" }} aria-label="Calls">
              <div className="ph">
                <span className="h">Calls</span>
                <div className="grow" />
                <button type="button" className="btn quiet sm" onClick={() => setView({ usageCall: null })}>All calls</button>
              </div>
              <div className="sect scroll">
                {calls.map((c) => (
                  <button type="button" key={c.id} className={`li ${c.id === sel ? "is-sel" : ""}`} style={{ height: "var(--sp-7)" }} onClick={() => setView({ usageCall: c.id })}>
                    <div className="col grow">
                      <span className="b">{KIND[c.kind]} · {hhmm(c.started_ms)}</span>
                      <span className="faint s1 ell">{short(c.model)} · {c.effort} · {k(c.tokens_in + c.tokens_out)} tokens · {dur(c.duration_ms)}</span>
                    </div>
                    <span className="b">{usd(c.cost_usd)}</span>
                    <span className={`tag ${OUTCOME[c.outcome][1]}`}>{OUTCOME[c.outcome][0]}</span>
                  </button>
                ))}
              </div>
            </section>
            <Detail id={sel} calls={calls} />
          </>
        ) : (
          <section className="card grow">
            <div className="sect g3 scroll" style={{ paddingTop: "var(--sp-3)" }}>
              <div className="fx g2 stretch">
                <div className="stat"><span className="faint s1">Calls</span><span className="v" data-testid="usage-calls">{calls.length}</span><span className="faint s1">{calls.filter((c) => c.outcome.startsWith("applied")).length} applied · {calls.filter((c) => c.outcome === "failed").length} failed</span></div>
                <div className="stat"><span className="faint s1">Tokens in</span><span className="v">{k(calls.reduce((a, c) => a + c.tokens_in, 0))}</span></div>
                <div className="stat"><span className="faint s1">Tokens out</span><span className="v">{k(calls.reduce((a, c) => a + c.tokens_out, 0))}</span></div>
                <div className="stat">
                  <span className="faint s1">Cost</span>
                  <span className="v" data-testid="usage-cost">{usd(cost)}</span>
                  <span className="faint s1">{project.ai.spend_cap_usd !== null ? `cap ${usd(project.ai.spend_cap_usd)}` : "no cap"}{estimates ? ` · ${estimates} estimate${estimates === 1 ? "" : "s"}` : ""}</span>
                </div>
                <div className="stat"><span className="faint s1">Time</span><span className="v">{dur(calls.reduce((a, c) => a + c.duration_ms, 0))}</span></div>
              </div>
              <div className="fx g2 stretch">
                <Breakdown title="By type" rows={group(calls, (c) => KIND[c.kind])} />
                <Breakdown title="By model" rows={group(calls, (c) => c.model)} note="Costs are the ones the provider reports. An estimate from the price list is marked “est.”" />
              </div>
              <div className="fx g3">
                <span className="faint s1">Spending cap for this project</span>
                <input className="input sm" style={{ width: "var(--field-w)" }} aria-label="Spending cap in USD" placeholder="none" value={capText} onChange={(e) => setCapText(e.target.value)} />
                <button
                  type="button"
                  className="btn sm"
                  onClick={() => {
                    const v = capText.trim() === "" ? null : Number(capText.replace("$", ""));
                    if (v !== null && !(v > 0)) {
                      setError("The cap is an amount in USD, or empty for none.");
                      return;
                    }
                    void backend.setSpendCap(v).then(() => toast("ok", v === null ? "No spending cap" : `Cap set to ${usd(v)}`, "")).catch(fail);
                  }}
                >
                  Set cap
                </button>
                <span className="faint s1">You are asked before a request that would go over it.</span>
              </div>
              <div className="fx between">
                <span className="h">Calls</span>
                <span className="faint s1">Newest first · failed and cancelled calls count with what they spent</span>
              </div>
              <div className="fx g2">
                {picker("Type", type, setType, [["all", "All types"], ...Object.entries(KIND)])}
                {picker("Outcome", outcome, setOutcome, [["all", "All outcomes"], ...Object.entries(OUTCOME).map(([v, [l]]) => [v, l] as [string, string])])}
                {picker("Model", model, setModel, [["all", "All models"], ...[...new Set(calls.map((c) => c.model))].map((m) => [m, m] as [string, string])])}
                <label className="input sm grow">
                  <Icon name="filter" size="m" />
                  <input className="bare" placeholder="Search requests" aria-label="Search requests" value={q} onChange={(e) => setQ(e.target.value)} />
                </label>
                <button
                  type="button"
                  className="btn"
                  onClick={(e) =>
                    menu.open(e.currentTarget, [
                      { label: "History as CSV", onSelect: () => void backend.aiExport(false).then((ok) => ok && toast("ok", "Exported", "History as CSV")).catch(fail) },
                      { label: "History as JSON", onSelect: () => void backend.aiExport(true).then((ok) => ok && toast("ok", "Exported", "History as JSON")).catch(fail) },
                      "-",
                      {
                        label: "Copy summary",
                        onSelect: () => void navigator.clipboard?.writeText(`${project.name}: ${calls.length} AI calls, ${usd(cost)}`).then(() => toast("ok", "Copied", "")),
                      },
                    ], "Export")
                  }
                >
                  <Icon name="exp" size="m" />
                  Export
                </button>
              </div>
              <div className="col" role="table" aria-label="AI calls">
                <div className="call head" role="row">
                  <span role="columnheader">Time</span><span role="columnheader">Type</span><span role="columnheader">Model · effort</span><span role="columnheader">Tries</span><span role="columnheader" className="r">Tokens</span><span role="columnheader" className="r">Cost</span><span role="columnheader" className="r">Time</span><span role="columnheader">Outcome</span><span role="columnheader" aria-label="Open" />
                </div>
                {shown.map((c) => (
                  <button type="button" role="row" key={c.id} className="call" onClick={() => setView({ usageCall: c.id })}>
                    <span role="cell" className="faint">{hhmm(c.started_ms)}</span>
                    <span role="cell" className="b">{KIND[c.kind]}</span>
                    <span role="cell" className="dim ell">{short(c.model)} · {c.effort}</span>
                    <span role="cell" className="dim">{tries(c)}</span>
                    <span role="cell" className="r">{k(c.tokens_in + c.tokens_out)}</span>
                    <span role="cell" className="r b">{usd(c.cost_usd)}{c.cost_estimated ? " est." : ""}</span>
                    <span role="cell" className="r dim">{dur(c.duration_ms)}</span>
                    <span role="cell"><span className={`tag ${OUTCOME[c.outcome][1]}`}>{OUTCOME[c.outcome][0]}</span></span>
                    <span role="cell"><Icon name="right" size="m" /></span>
                  </button>
                ))}
                {!shown.length && <span className="dim">No call matches the filters.</span>}
              </div>
            </div>
          </section>
        )}
      </div>
      {menu.node}
    </>
  );
}
