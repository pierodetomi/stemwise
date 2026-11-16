// A proposal in review (design 4a, 4b; AI-2, AI-7, HIS-3): it takes the place of inspector and assistant while the
// timeline and the mixer stay live. A/B without stopping playback and without a history step; apply everything, only
// the ticked tracks, or discard.
import { shortcutOf } from "../shared/shortcuts";
import { useEffect, useState } from "react";
import type { ProposalView } from "../backend/types";
import type { DiffRow } from "../bindings/DiffRow";
import type { IdeaView } from "../bindings/IdeaView";
import type { NewElementView } from "../bindings/NewElementView";
import type { Project } from "../bindings/Project";
import { Icon } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useApp, useBackend } from "../state/context";
import { usd } from "./Assist";
import { ReopenedNote, reapplyDialog } from "./Proposals";

const num = (v: number | null, unit: string) => {
  if (v === null) return "off";
  const s = Math.abs(v) >= 100 ? Math.round(v).toString() : v.toFixed(1);
  const signed = unit === "dB" && v > 0 ? `+${s}` : s;
  return signed.replace("-", "−");
};

export function DiffRows({ rows }: { rows: DiffRow[] }) {
  return (
    <div className="col">
      {rows.map((r, i) => (
        <div key={i} className="dr">
          <span className="dim grow ell">{r.label}</span>
          <span className="was">{r.text != null ? "—" : num(r.before, r.unit)}</span>
          <span className="faint">→</span>
          <span className="to">{r.text != null ? r.text : `${num(r.after, r.unit)} ${r.after !== null ? r.unit : ""}`}</span>
        </div>
      ))}
    </div>
  );
}

/** What a new return or group is, in one line (design 4a). */
function elementKind(e: NewElementView, project: Project): string {
  if (e.kind === "group") {
    return `group · ${e.members.map((m) => project.tracks.find((t) => t.id === m)?.name ?? "?").join(", ")}`;
  }
  if (e.ret?.type === "reverb") return `reverb return · ${e.ret.algorithm}, ${e.ret.decay_s.toFixed(1)} s`;
  if (e.ret?.type === "delay") return `delay return · ${Math.round(e.ret.time_ms)} ms, ${e.ret.feedback < 0.005 ? "no repeats" : `feedback ${Math.round(e.ret.feedback * 100)} %`}`;
  return "return";
}

function kindName(p: ProposalView) {
  return p.kind === "revision" ? "Revision" : "Auto mix";
}

export function ProposalPanel({ p }: { p: ProposalView }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const listening = useApp((s) => s.ai?.listening ?? "current");
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const [off, setOff] = useState<string[]>([]);
  const [open, setOpen] = useState<string | null>(p.diff[0]?.track ?? null);
  const [trims, setTrims] = useState(p.trims);
  const fail = (e: unknown) => setError(String(e));
  const ab = listening === "proposal" || listening.startsWith("idea:");
  const listen = (on: boolean) => void backend.aiListen(on ? "proposal" : "current").catch(fail);
  // Shortcut B: A/B (design 4a).
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input, textarea")) return;
      if (shortcutOf(e) === "ab") listen(!ab);
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  });
  const chosen = p.diff.filter((d) => !off.includes(d.track)).map((d) => d.track);
  // AI-20: a new element is created when it is ticked and enough of the tracks it exists for are applied. The
  // numbers (`depends_on`, `needs`) come from the core, which decides the same way when the proposal is applied. A
  // track with no card of its own (nothing but its routing changes) cannot be unticked, so it always counts.
  const name = (id: string) => project.tracks.find((t) => t.id === id)?.name ?? "?";
  // A new return of a creative idea is on the idea's card and follows its tick (AI-27), not this list.
  const elements = p.new_elements.filter((e) => e.idea === null).map((e) => {
    const left = e.depends_on.filter((d) => !off.includes(d));
    const state: "made" | "off" | "lost" = off.includes(e.id) ? "off" : !e.fits || left.length < e.needs ? "lost" : "made";
    const uses =
      e.kind === "group"
        ? `${left.length} tracks move into it`
        : left.length === 1
          ? `${name(left[0])} sends here`
          : `${left.length} tracks send here`;
    const why = !e.fits
      ? "The project changed since this proposal: it no longer fits."
      : e.kind === "group"
        ? `Needs two of its tracks selected: ${left.length ? `only ${name(left[0])} is` : "none is"}.`
        : "Needs a selected track that sends to it: none is.";
    return { e, state, uses, why };
  });
  const made = elements.filter((x) => x.state === "made");
  // What an application of the ticked cards sends: tracks, new elements, and the tracks without a card.
  const cards = new Set(p.diff.map((d) => d.track));
  const ticked = new Set(p.ideas.filter((i) => i.on).map((i) => i.id));
  const ofIdeas = p.new_elements.filter((e) => e.idea !== null && ticked.has(e.idea)).map((e) => e.id);
  const only = [...chosen, ...made.map((x) => x.e.id), ...ofIdeas, ...p.new_elements.flatMap((e) => e.depends_on.filter((d) => !cards.has(d)))];
  const best = p.iterations.find((i) => i.best);
  const applyNow = async (only: string[] | null) => {
    try {
      const label = await backend.aiApply(only, trims);
      toast("ok", only ? "Proposal applied in part" : "Proposal applied", `${label}. One step: undo brings everything back.`);
    } catch (e) {
      fail(e);
    }
  };
  // AI-23: a proposal received earlier changes a mix that has moved on, so it asks first.
  const apply = (only: string[] | null) =>
    p.reopened ? setView({ dialog: reapplyDialog(p.record.started_ms, "mix", !!only, () => void applyNow(only)) }) : void applyNow(only);
  const discard = () =>
    p.reopened
      ? // Opened again from the list: closing it loses nothing, it stays there.
        void backend.aiDiscard().catch(fail)
      : setView({
      dialog: {
        title: "Discard this proposal?",
        sub: `Nothing was applied. The call stays in AI usage with its cost (${usd(p.record.cost_usd)}).`,
        confirm: "Discard",
        danger: true,
        onConfirm: () => void backend.aiDiscard().catch(fail),
      },
    });
  const tint = (id: string) => {
    const t = project.tracks.find((x) => x.id === id);
    return t?.kind === "audio" ? `t${t.color || 1}` : "tb";
  };
  const tokens = p.record.tokens_in + p.record.tokens_out;
  const b = p.before;
  const a = p.after;
  const dist = b?.distance_db != null && a?.distance_db != null ? Math.max(0, Math.min(1, a.distance_db / Math.max(0.1, b.distance_db))) : null;
  return (
    <aside className="card side wide proposal" aria-label="Proposal">
      <div className="ph">
        <Icon name="spark" className="aic" />
        <span className="h">Proposal · {kindName(p)}</span>
        {p.iterations.length > 1 && best && <span className="tag ai">Best of {p.iterations.length} passes</span>}
        <div className="grow" />
        <span className="faint s1">{usd(p.record.cost_usd)}{p.record.cost_estimated ? " est." : ""} · {(tokens / 1000).toFixed(1)}k tokens</span>
        <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: p.call_id })}>Details</button>
      </div>
      <div className="sect">
        {p.reopened && <ReopenedNote at={p.record.started_ms} phase="mix" />}
        <div className="tile row">
          <Icon name="ear" />
          <div className="col grow">
            <span className="b">{ab ? "Listening to the proposal" : "Listening to your current mix"}</span>
            <span className="faint s1">Switch while it plays. Nothing is applied yet. Shortcut <span className="kbd">B</span></span>
          </div>
          <Seg label="A/B" value={ab ? 1 : 0} items={[{ value: 0, label: "Current mix" }, { value: 1, label: "Proposal" }]} onChange={(v) => listen(v === 1)} />
        </div>
        {p.request && <span className="dim s1">You asked: “{p.request}”</span>}
        <p className="text selectable">{p.summary}</p>
        <div className="fx g2 stretch">
          <div className="stat sm">
            <span className="faint s1">Distance to reference</span>
            <span className="v">{b?.distance_db != null && a?.distance_db != null ? `${b.distance_db.toFixed(1)} → ${a.distance_db.toFixed(1)}` : "no reference"}</span>
            {dist !== null && <div className="hbar ai"><i style={{ width: `${Math.round(dist * 100)}%` }} /></div>}
          </div>
          <div className="stat sm">
            <span className="faint s1">True peak</span>
            <span className="v">{a?.true_peak_db != null ? `${a.true_peak_db.toFixed(1).replace("-", "−")} dBTP` : "—"}</span>
            <span className={`s1 ${a?.true_peak_ok ? "okc" : "warnc"}`}><Icon name={a?.true_peak_ok ? "check" : "warn"} size="sm" /> {a?.true_peak_ok ? "headroom kept" : "over the mix ceiling"}</span>
          </div>
          <div className="stat sm">
            <span className="faint s1">Loudness</span>
            <span className="v">{a?.lufs != null ? `${a.lufs.toFixed(1).replace("-", "−")} LUFS` : "—"}</span>
            <span className={`s1 ${a?.low_corr_ok && a?.dc_ok ? "okc" : "warnc"}`}><Icon name={a?.low_corr_ok ? "check" : "warn"} size="sm" /> {a?.low_corr_ok ? "mono safe" : "low end out of phase"}</span>
          </div>
        </div>
      </div>
      <div className="sect grow scroll">
        {p.ideas.length > 0 && <Ideas ideas={p.ideas} />}
        {p.diff
          .filter((d) => d.track === open)
          .map((d) => (
            <div key={d.track} className={`pcard on ${tint(d.track)}`}>
              <div className="fx g2">
                <Check on={!off.includes(d.track)} label={`Apply ${d.name}`} onChange={(v) => setOff(v ? off.filter((x) => x !== d.track) : [...off, d.track])} />
                <i className="tdot" />
                <span className="b">{d.name}</span>
                {d.note !== null && <span className="badge">{d.note}</span>}
                <span className="grow" />
                <span className="faint s1">{d.rows.length} {d.rows.length === 1 ? "change" : "changes"}</span>
                <button type="button" className="ib" aria-label="Close" onClick={() => setOpen(null)}><Icon name="up" size="m" /></button>
              </div>
              <div className="fx g4 top-al">
                <p className="why grow selectable">{d.rationale}</p>
                <div style={{ width: "var(--field-w-l)" }}>
                  <DiffRows rows={d.rows} />
                </div>
              </div>
            </div>
          ))}
        {elements.length > 0 && (
          <>
            <div className="fx between">
              <span className="b">New in this proposal</span>
              <span className="faint s1">{made.length} of {elements.length} will be created · one undo step removes them with the rest</span>
            </div>
            <div className="pgrid" aria-label="New in this proposal">
              {elements.map(({ e, state, uses, why }) => (
                <div key={e.id} className={`pcard tb ${state === "off" ? "is-off" : ""}`} data-testid={`new-${e.name}`}>
                  <div className="fx g2">
                    <Check
                      on={state === "made"}
                      disabled={state === "lost"}
                      label={`Create ${e.name}`}
                      onChange={(v) => setOff(v ? off.filter((x) => x !== e.id) : [...off, e.id])}
                    />
                    <Icon name={e.kind === "group" ? "group" : "wave"} size="m" />
                    <span className="b ell">{e.name}</span>
                    <span className="tag ai">New</span>
                    <span className="grow" />
                    <span className="faint s1 nowrap">{state === "made" ? uses : state === "off" ? "left out" : "not created"}</span>
                  </div>
                  <span className="faint s1 ell">{elementKind(e, project)}</span>
                  <span className={`${state === "lost" ? "warnc" : "dim"} s1 ell`}>{state === "lost" ? why : e.rationale}</span>
                </div>
              ))}
            </div>
            <div className="fx between">
              <span className="b">Tracks, groups and returns</span>
              <span className="faint s1">{chosen.length} of {p.diff.length} selected</span>
            </div>
          </>
        )}
        <div className="pgrid">
          {p.diff
            .filter((d) => d.track !== open)
            .map((d) => (
              <div key={d.track} className={`pcard ${tint(d.track)} ${off.includes(d.track) ? "is-off" : ""}`} data-testid={`card-${d.name}`}>
                <div className="fx g2">
                  <Check on={!off.includes(d.track)} label={`Apply ${d.name}`} onChange={(v) => setOff(v ? off.filter((x) => x !== d.track) : [...off, d.track])} />
                  <i className="tdot" />
                  <button type="button" className="b ell grow" onClick={() => setOpen(d.track)}>{d.name}</button>
                  {d.note !== null && <span className="badge">{d.note}</span>}
                  <span className="faint s1 nowrap">{d.rows.length} {d.rows.length === 1 ? "change" : "changes"}</span>
                </div>
                <span className="dim s1 ell">{d.rationale}</span>
              </div>
            ))}
          {p.untouched.map((id) => {
            const t = project.tracks.find((x) => x.id === id);
            if (!t) return null;
            return (
              <div key={id} className={`pcard is-lock ${tint(id)}`}>
                <div className="fx g2">
                  <Icon name="lock" size="m" className="aic" />
                  <i className="tdot" />
                  <span className="b ell grow">{t.name}</span>
                  <span className="faint s1">{t.locked ? "locked" : "out of scope"}</span>
                </div>
                <span className="dim s1 ell">Sent as context, left untouched.</span>
              </div>
            );
          })}
        </div>
        {p.warnings.length > 0 && (
          <details className="faint s1">
            <summary>{p.warnings.length} note{p.warnings.length === 1 ? "" : "s"} from validation</summary>
            {p.warnings.map((w, i) => <div key={i}>{w}</div>)}
          </details>
        )}
      </div>
      <div className="foot" style={{ flexDirection: "row", alignItems: "center" }}>
        <button type="button" className="btn quiet" onClick={discard}>
          <Icon name="x" size="m" />
          {p.reopened ? "Close review" : "Discard"}
        </button>
        <span className="fx g2 s1 dim grow">
          {p.trims && (
            <>
              <Check on={trims} label="Level every track to −23 LUFS first" onChange={setTrims} />
              Level every track to −23 LUFS first (trim)
            </>
          )}
        </span>
        <button type="button" className={`btn ${off.length ? "" : "is-dis"}`} disabled={!off.length || !chosen.length} onClick={() => void apply(only)}>
          Apply selected ({chosen.length})
        </button>
        <button type="button" className="btn primary" onClick={() => void apply(null)}>
          <Icon name="check" size="m" />
          Apply all
        </button>
      </div>
    </aside>
  );
}

/** AI-27, design 4c, 4d: the creative ideas, apart from the plan. Each only adds; off until ticked. "Listen" plays the
 * proposal with that idea alone until pressed again or A/B is switched. */
function Ideas({ ideas }: { ideas: IdeaView[] }) {
  const backend = useBackend();
  const listening = useApp((s) => s.ai?.listening ?? "current");
  const setError = useApp((s) => s.setError);
  const fail = (e: unknown) => setError(String(e));
  const ticked = ideas.filter((i) => i.on).map((i) => i.id);
  const tick = (id: string, on: boolean) => void backend.aiSetIdeas(on ? [...ticked, id] : ticked.filter((x) => x !== id)).catch(fail);
  return (
    <>
      <div className="fx between">
        <span className="b">Creative ideas</span>
        <span className="faint s1">optional · off until you tick them · {ticked.length} of {ideas.length} ticked</span>
      </div>
      <div className="pgrid three" aria-label="Creative ideas">
        {ideas.map((i) => {
          const playing = listening === `idea:${i.id}`;
          return (
            <div key={i.id} className={`pcard icard ${i.on ? "on" : ""} ${playing ? "is-play" : ""}`} data-testid={`idea-${i.id}`}>
              <div className="fx g2">
                <Check on={i.on} label={`Apply idea ${i.name}`} onChange={(v) => tick(i.id, v)} />
                <Icon name="spark" size="m" className="aic" />
                <span className="b ell grow" title={i.name}>{i.name}</span>
              </div>
              <span className="dim s1">{i.rationale}</span>
              <div className="col">
                {i.elements.map((e, k) => (
                  <span key={k} className="faint s1 ell" title={e}>· {e}</span>
                ))}
              </div>
              <div className="fx g2 mt-auto">
                {i.sections.map((x) => <span key={x} className="tag">{x}</span>)}
                {i.sections.length === 0 && <span className="tag">Whole song</span>}
                <span className="grow" />
                <button
                  type="button"
                  className={`btn quiet sm ${playing ? "on" : ""}`}
                  aria-pressed={playing}
                  aria-label={`Listen to idea ${i.name}`}
                  onClick={() => void backend.aiListen(playing ? "proposal" : `idea:${i.id}`).catch(fail)}
                >
                  <Icon name="ear" size="m" />
                  {playing ? "Listening" : "Listen"}
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}

function Check({ on, onChange, label, disabled }: { on: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return (
    <button type="button" role="checkbox" aria-checked={on} aria-label={label} aria-disabled={disabled || undefined} disabled={disabled} className={`check ${on ? "on" : ""} ${disabled ? "is-dis" : ""}`} onClick={() => onChange(!on)}>
      <Icon name="check" />
    </button>
  );
}
