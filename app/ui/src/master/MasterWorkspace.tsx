// Master workspace (design 9a–9c, 15a–15j; MAS-1 … MAS-6, MAS-11 … MAS-15, MDN-2, MDN-3): the mixdown's waveform
// with the offsets of its sections, the chain as a rack beside the editor of the selected block, loudness and
// spectrum against the reference, the destination, equal-loudness A/B, the assistant for the master. Its own
// history (HIS-5).
import { DeviceBanner } from "../shared/StateBanners";
import { BusMeters } from "../shared/BusMeters";
import { useEffect, useRef, useState } from "react";
import type { MasterStatus } from "../bindings/MasterStatus";
import type { MasterWave } from "../bindings/MasterWave";
import { Icon, Mark } from "../shared/Icon";
import { ProjectMenuButton } from "../project/ProjectMenu";
import { Field, IconButton, Seg, useBallistics } from "../shared/controls";
import { formatClock, formatTime, meterPos } from "../shared/format";
import { useMenu } from "../shared/Menu";
import { useApp, useBackend, useStore } from "../state/context";
import { shortcutOf } from "../shared/shortcuts";
import { songFrames } from "../mix/geometry";
import { usd } from "../ai/Assist";
import { ProposalsTab, ReopenedNote, reapplyDialog } from "../ai/Proposals";
import { MS, PRESETS, targetName, useMasterCtl, useMasterEdit } from "./bind";
import { BlockEditor, SectionEditor, profileLine, useWidth } from "./Editors";
import { Rack, n1 } from "./Rack";
import { ExportDialog } from "./ExportDialog";
import { PrepGhost, PrepOverlay, RetargetOverlay, useWaitShown } from "./Prep";
import { useProjectCommands } from "../project/commands";
import { useScrub } from "../shared/scrub";
import { ListenControls, ListeningSection, TargetNotice, useTargetGap } from "./Listening";
import { ft, placeOf, proposalHint, refLevels } from "./level";

function MasterTop({ ms, preparing, waiting }: { ms: MasterStatus; preparing: boolean; waiting: boolean }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const meters = useApp((s) => s.meters);
  const revision = useApp((s) => s.revision);
  const setView = useApp((s) => s.setView);
  const openSettings = useApp((s) => s.openSettings);
  const setError = useApp((s) => s.setError);
  const [hist, setHist] = useState<[number, number]>([0, 0]);
  useEffect(() => {
    backend.history("master").then(([e, pos]) => setHist([pos, e.length])).catch(() => {});
  }, [backend, revision]);
  const rate = project.sample_rate ?? 48000;
  const playing = meters?.playing ?? false;
  const fail = (e: unknown) => setError(String(e));
  const demo = useApp((s) => s.demo);
  const md = project.master.mixdown;
  const held = preparing || waiting;
  return (
    <header className="top" aria-label="Top bar">
      <div className="fx g2">
        <Mark />
        <ProjectMenuButton />
        <span className="faint s1 nowrap md-label">{md ? `Mixdown ${md.number}${preparing ? " · preparing" : ms.stale ? " · out of date" : demo ? " · demo" : ""}` : ""}</span>
      </div>
      <Seg
        className="phase"
        label="Phase"
        value="master"
        items={[
          { value: "mix", label: "Mix" },
          { value: "master", label: "Master" },
        ]}
        onChange={(v) => v === "mix" && void backend.setPhase(false).catch(fail)}
      />
      {/* While the master is prepared only the phase selector works (9d). */}
      <div className={`fx g2 ${preparing ? "is-dis" : ""}`} role="group" aria-label="Transport" inert={preparing}>
        <IconButton icon="rew" label="Return to start" onClick={() => void backend.locate(0).catch(fail)} />
        <button type="button" className={`playb ${playing ? "" : "idle"}`} aria-label={playing ? "Pause" : "Play"} onClick={() => void (playing ? backend.pause() : backend.play()).catch(fail)}>
          <Icon name={playing ? "pause" : "play"} />
        </button>
        <IconButton icon="stop" label="Stop" onClick={() => void backend.stop().catch(fail)} />
      </div>
      <div className="fx g2">
        <span className="pos" data-testid="position">{formatTime(meters?.frame ?? 0, rate)}</span>
        <span className="faint s1 nowrap dur">/ {formatClock((md?.frames ?? songFrames(project)) / rate)}</span>
      </div>
      <div className={`fx g1 ${held ? "is-dis" : ""}`} inert={held}>
        <IconButton icon="undo" label="Undo" disabled={hist[0] === 0} className={hist[0] === 0 ? "is-dis" : ""} onClick={() => void backend.undo("master").catch(fail)} />
        <IconButton icon="redo" label="Redo" disabled={hist[0] >= hist[1]} className={hist[0] >= hist[1] ? "is-dis" : ""} onClick={() => void backend.redo("master").catch(fail)} />
      </div>
      {ms.proposal && (
        <span className="tag ai review-tag">
          <Icon name="spark" size="sm" />
          Reviewing a master proposal
        </span>
      )}
      <div className="grow" />
      <div className={`fx g2 ${held ? "is-dis" : ""}`} inert={held}>
        {/* Real level by default, the same loudness on demand (D26, D27), then the source of the A/B. */}
        <ListenControls ms={ms} />
      </div>
      <IconButton icon="gear" label="Settings (⌘,)" onClick={() => openSettings()} />
      <BusMeters label="Master output" engine={false} />
      <button type="button" className="btn primary" disabled={held} onClick={() => setView({ exportOpen: true })}>
        <Icon name="exp" size="m" />
        Export
      </button>
    </header>
  );
}

// ---------- waveform ----------
function WaveCard({ ms }: { ms: MasterStatus }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const frame = useApp((s) => s.meters?.frame ?? 0);
  const playing = useApp((s) => s.meters?.playing ?? false);
  const setView = useApp((s) => s.setView);
  const selSec = useApp((s) => s.masterSection);
  const ref = useRef<HTMLDivElement>(null);
  const w = useWidth(ref, 760);
  const [wave, setWave] = useState<MasterWave | null>(null);
  const md = project.master.mixdown;
  const cols = Math.max(64, Math.round(w / 2));
  // The master is drawn from what was last measured: a new measure brings a new envelope.
  const measured = ms.analysing ? "" : `${ms.loud_gain_db}|${ms.measures?.lufs}|${ms.measures?.true_peak_db}|${ms.measures?.short_crest_db}`;
  useEffect(() => {
    if (!ms.mixdown_measures) return;
    backend.masterWave(cols).then(setWave).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend, cols, md?.number, !!ms.mixdown_measures, measured]);
  const rate = project.sample_rate ?? 48000;
  const total = (md?.frames ?? songFrames(project)) / rate;
  const pps = w / Math.max(1, total);
  // Click to locate, drag to scrub (TRN-1).
  const scrub = useScrub((x) => {
    const r = ref.current?.getBoundingClientRect();
    return r ? ((x - r.left) / pps) * rate : 0;
  });
  const h = 104;
  const mid = h / 2;
  // The height is the square root of the level: the body of the sound stays readable under peaks that all reach the
  // ceiling, and quiet passages do not vanish. `i` picks the value of a column: 2 the RMS, otherwise its extremes.
  const tall = (v: number) => Math.sqrt(Math.min(1, Math.abs(v))) * mid * 0.94;
  const path = (cols: [number, number, number][] | null | undefined, body: boolean) => {
    if (!cols?.length) return "";
    const n = cols.length;
    const x = (i: number) => ((i / Math.max(1, n - 1)) * w).toFixed(1);
    const up = cols.map((c, i) => `${x(i)} ${(mid - tall(body ? c[2] : c[1])).toFixed(1)}`);
    const dn = cols.map((c, i) => `${x(i)} ${(mid + tall(body ? c[2] : c[0])).toFixed(1)}`).reverse();
    return `M${up.join("L")}L${dn.join("L")}Z`;
  };
  // MAS-14: what each section is offset by, in a few words (the lane under the waveform).
  const sign = (v: number) => `${v > 0 ? "+" : ""}${n1(v)}`;
  const offsets = project.master.sections.flatMap((o) => {
    const sec = project.sections.find((x) => x.id === o.section);
    if (!sec) return [];
    const tags = [
      o.level_db ? `${sign(o.level_db)} dB` : "",
      o.side_gain_db ? `${sign(o.side_gain_db)} side` : "",
      o.limiter_release_ms ? `${sign(o.limiter_release_ms)} ms` : "",
      o.eq.some((e) => e.gains_db.some((g) => g !== 0)) ? "EQ" : "",
    ].filter(Boolean);
    return tags.length ? [{ sec, tags }] : [];
  });
  const ticks = [];
  for (let t = 0; t < total; t += 15) ticks.push(t);
  return (
    <section className="card" aria-label="Mixdown">
      <div className="tools">
        <span className="b">Mixdown {md?.number}</span>
        <span className="faint s1">
          {formatClock(total)} · {rate / 1000} kHz · measured {n1(md?.lufs)} LUFS, {n1(md?.true_peak_db)} dBTP
        </span>
        <div className="grow" />
        <span className="tag">{wave?.master ? "Bright: master · dark: mixdown · line: peaks" : "Dark: mixdown"}</span>
        <button type="button" className="btn quiet sm" onClick={() => setView({ bounceOpen: true })}>
          <Icon name="loop" size="m" />
          Bounce again
        </button>
      </div>
      <div
        className={`mwave ${scrub.scrubbing ? "on-head" : ""}`}
        ref={ref}
        data-testid="master-wave"
        title="Click to locate · drag to scrub"
        onPointerDown={(e) => e.button === 0 && scrub.start(e)}
      >
        {ticks.map((t) => (
          <span key={t} className="tick" style={{ left: t * pps }}>{formatClock(t)}</span>
        ))}
        {/* The master: its body filled, its peaks as a thin outline (flat where the limiter holds them at the ceiling);
            the body of the mixdown filled dark inside it. */}
        <svg className="pkw" width={w} height={h} aria-hidden="true"><path d={path(wave?.master, false)} data-testid="master-peaks" /></svg>
        <svg width={w} height={h} aria-hidden="true"><path d={path(wave?.master, true)} data-testid="master-body" /></svg>
        <svg className="dimw" width={w} height={h} aria-hidden="true"><path d={path(wave?.mixdown, true)} /></svg>
        {project.sections.map((s) => (
          // A section opens its offsets in the editor (15e); the rest of the waveform locates and scrubs.
          <button type="button" key={s.id} className={`sec ${/chorus/i.test(s.name) ? "hot" : ""} ${selSec === s.id ? "is-sel" : ""}`} style={{ left: (s.start / rate) * pps, width: ((s.end - s.start) / rate) * pps - 1 }} aria-pressed={selSec === s.id} aria-label={`Offsets of ${s.name}`} title="Offsets of this section" onPointerDown={(e) => e.stopPropagation()} onClick={() => !ms.proposal && setView({ masterSection: selSec === s.id ? null : s.id })}>
            {s.name}
          </button>
        ))}
        <div className={`play ${playing ? "" : "idle"}`} style={{ left: ((scrub.frame ?? frame) / rate) * pps }} data-testid="master-playhead" />
      </div>
      {offsets.length > 0 && (
        <div className="mlane" aria-label="Section offsets" data-testid="section-offsets">
          {offsets.map(({ sec, tags }) => (
            <span key={sec.id} className={`so ${selSec === sec.id ? "is-sel" : ""}`} style={{ left: (sec.start / rate) * pps, width: Math.max(0, ((sec.end - sec.start) / rate) * pps - 1) }}>
              {tags.map((t, i) => (
                <span key={t} className="fx g1">
                  {i > 0 && <i />}
                  <span className="ell">{t}</span>
                </span>
              ))}
            </span>
          ))}
          <span className="faint s1 mlane-l">Offsets</span>
        </div>
      )}
    </section>
  );
}

// ---------- loudness ----------
function LoudMeter({ label, value, db, peak, target, gr }: { label: string; value: string; db: number; peak?: number; target?: number; gr?: boolean }) {
  const b = useBallistics(db, peak ?? db);
  return (
    <div className="lm">
      <span className="b s1">{value}</span>
      <div className={`meter l ${gr ? "gr" : ""}`}>
        {gr ? <i style={{ height: `${Math.min(100, (Math.abs(db) / 12) * 100)}%` }} /> : (
          <>
            <i className="pk" style={{ height: `${meterPos(b.peak) * 100}%` }} />
            <i className="rms" style={{ height: `${meterPos(b.level) * 100}%` }} />
          </>
        )}
        {target !== undefined && <i className="tgt" style={{ top: `${100 - meterPos(target) * 100}%` }} />}
      </div>
      <span className="faint s1">{label}</span>
    </div>
  );
}

function Loudness({ ms }: { ms: MasterStatus }) {
  const project = useApp((s) => s.project)!;
  const meters = useApp((s) => s.meters);
  const edit = useMasterEdit();
  const menu = useMenu();
  const m = project.master;
  const lufsCtl = useMasterCtl(MS.lufs, "Target loudness", (x) => x.target.lufs, (x, v) => { x.target.lufs = v; x.target.preset = "custom"; });
  const tpCtl = useMasterCtl(MS.ceiling, "Ceiling", (x) => x.target.true_peak_db, (x, v) => { x.target.true_peak_db = v; x.target.preset = "custom"; });
  const meas = ms.measures;
  const refs = refLevels(ms);
  const { gap, kept } = useTargetGap(ms);
  // 9q: the select turns amber while the destination is much quieter than the references and not kept.
  const warnSel = !!gap && !kept && gap.kind === "quieter";
  const bus = meters?.bus;
  const playing = meters?.playing ?? false;
  const live = (v: number | undefined) => (playing && v !== undefined && v > -120 ? v : -200);
  const onTarget = meas?.lufs != null && Math.abs(meas.lufs - m.target.lufs) < 0.5 && meas.true_peak_db <= m.target.true_peak_db + 1e-6;
  // 9k: after a change the level match follows in the background; the change already plays. Shown past --wait-show.
  // A new target is not playing yet: that wait has its own overlay (9v).
  const measuring = useWaitShown(ms.analysing && !ms.prep && !ms.retarget).shown && ms.analysing && !ms.retarget;
  const corr = bus?.[7] ?? 0;
  const spectrum = meters?.spectrum ?? null;
  const w = 290;
  // The deepest clipper at work, beside the limiter (15a).
  const clipAt = ms.chain.map((b, i) => ({ b, i })).filter(({ b }) => b.kind === "clipper" && !project.master.chain.blocks.find((x) => x.id === b.id)?.block.bypass);
  const clipGr = clipAt.length ? Math.min(0, ...clipAt.map(({ i }) => meters?.masterBlocks?.[i] ?? 0)) : null;
  const lim = meas?.limiter;
  let spec = "";
  if (spectrum) spec = `M0 64L${spectrum.map((db, k) => `${(((k + 0.5) / spectrum.length) * w).toFixed(1)} ${(64 - Math.max(0, Math.min(1, (db + 90) / 84)) * 64).toFixed(1)}`).join("L")}L${w} 64Z`;
  const refLine = profileLine(ms.target_bands, spectrum, w, 64);
  return (
    <aside className="card side" aria-label="Loudness">
      <div className="ph">
        <span className="h">Loudness</span>
        <div className="grow" />
        {measuring ? (
          <span className="tag acc"><Icon name="clock" size="sm" />Measuring…</span>
        ) : onTarget ? (
          <span className="tag ok"><Icon name="check" size="sm" />On target</span>
        ) : (
          <span className="tag warn">Off target</span>
        )}
      </div>
      <div className="sect">
        <label className="flabel">
          <span className="l">Target</span>
          <button type="button" className={`select lg ${warnSel ? "warn" : ""}`} aria-label="Destination" onClick={(e) => menu.open(e.currentTarget, [
            ...(refs.length ? [{ header: "Destination" }] : []),
            ...PRESETS.map(([k, name, l, tp]) => ({ label: `${name} · ${ft(l)} LUFS · ${ft(tp)} dBTP`, checked: m.target.preset === k, meta: placeOf(l, refs) ?? undefined, onSelect: () => edit(`Destination ${name}`, (x) => (x.target = { preset: k, lufs: l, true_peak_db: tp })) })),
            { label: m.target.preset === "custom" ? `Custom · ${ft(m.target.lufs)} LUFS · ${ft(m.target.true_peak_db)} dBTP` : "Custom", checked: m.target.preset === "custom", meta: m.target.preset === "custom" ? (placeOf(m.target.lufs, refs) ?? undefined) : undefined, onSelect: () => edit("Destination custom", (x) => (x.target.preset = "custom")) },
            // 9s: matching is an entry of its own while the destination is far from the references.
            ...(gap
              ? (["-", { label: `Match the reference${refs.length > 1 ? "s" : ""} · ${ft(gap.edge.lufs)} LUFS`, icon: "level" as const, meta: { text: `Keeps ${ft(m.target.true_peak_db)} dBTP` }, onSelect: () => edit("Target matched to the references", (x) => void (x.target = { preset: "custom", lufs: Math.round(gap.edge.lufs * 10) / 10, true_peak_db: x.target.true_peak_db })) }] as const)
              : []),
          ], "Destination")}>
            {warnSel && <Icon name="warn" size="m" className="warnc" />}
            <span className="ell grow" data-testid="destination">{targetName(m)}</span>
            <Icon name="chev" size="m" />
          </button>
        </label>
        {m.target.preset === "custom" && (
          <div className="fx g2">
            <Field ctl={lufsCtl} className="fill" />
            <Field ctl={tpCtl} className="fill" />
          </div>
        )}
        <TargetNotice ms={ms} where="card" />
        {measuring && (
          <>
            <div className="progress indet" role="progressbar" aria-label="Measuring the loudness"><i /></div>
            <span className="dim s1" data-testid="measuring-note">Your change is already playing. The level match to {n1(m.target.lufs)} LUFS follows when the measure is done.</span>
          </>
        )}
        <div className={`fx g5 ${measuring ? "is-stale" : ""}`}>
          <div className="col"><span className="faint s1">Integrated</span><span className="big" data-testid="master-lufs">{n1(meas?.lufs)}</span><span className="faint s1">{measuring ? "LUFS · before the change" : `LUFS · target ${n1(m.target.lufs)}`}</span></div>
          <div className="col"><span className="faint s1">True peak</span><span className="big">{n1(meas?.true_peak_db)}</span><span className="faint s1">dBTP · ceiling {n1(m.target.true_peak_db)}</span></div>
        </div>
      </div>
      <ListeningSection ms={ms} gap={kept ? null : gap} />
      <div className="sect grow">
        <div className="lmeters">
          <LoudMeter label="Moment." value={n1(live(bus?.[5]) > -120 ? bus?.[5] : null)} db={live(bus?.[5])} target={m.target.lufs} />
          <LoudMeter label="Short" value={n1(live(bus?.[6]) > -120 ? bus?.[6] : null)} db={live(bus?.[6])} target={m.target.lufs} />
          <LoudMeter label="Integr." value={n1(meas?.lufs)} db={meas?.lufs ?? -200} target={m.target.lufs} />
          <LoudMeter label={clipGr === null ? "Peak L" : "Pk L"} value={n1(live(bus?.[0]) > -120 ? bus?.[0] : null)} db={live(bus?.[2])} peak={live(bus?.[0])} />
          <LoudMeter label={clipGr === null ? "Peak R" : "Pk R"} value={n1(live(bus?.[1]) > -120 ? bus?.[1] : null)} db={live(bus?.[3])} peak={live(bus?.[1])} />
          {clipGr !== null && <LoudMeter label="Clipper" value={n1(clipGr)} db={clipGr} gr />}
          <LoudMeter label="Limiter" value={n1(meters?.masterGr?.[1])} db={meters?.masterGr?.[1] ?? 0} gr />
        </div>
      </div>
      <div className="sect">
        {lim && <div className="fx between"><span className="faint s1">Limiter over 3 dB</span><span className={`b ${ms.strained ? "warnc" : ""}`} data-testid="limiter-work">{lim.over_3_pct < 0.5 ? "never" : `${Math.round(lim.over_3_pct)} % of the song`}</span></div>}
        <div className="fx between"><span className="faint s1">Loudness range</span><span className="b">{n1(meas?.lra)} LU</span></div>
        <div className="fx between"><span className="faint s1">Stereo correlation</span><span className="b">{playing ? `${corr >= 0 ? "+" : ""}${corr.toFixed(2)}` : "—"}</span></div>
        <div className="corr"><i style={{ left: `${((corr + 1) / 2) * 100}%` }} /></div>
        {meas?.distance_db != null && <div className="fx between"><span className="faint s1">Distance to reference</span><span className="b">{n1(meas.distance_db)} dB</span></div>}
      </div>
      <div className="sect">
        <span className="faint s1">Spectrum{ms.target_bands ? " against the references (dashed)" : ""}</span>
        <div className="eq" style={{ height: "var(--sp-8)" }}>
          <svg width={w} height={64} aria-hidden="true">
            {spec && <path className="sp" d={spec} />}
            {refLine && <path className="rf" d={refLine} data-testid="meter-ref-line" />}
          </svg>
        </div>
      </div>
      {menu.node}
    </aside>
  );
}

// ---------- assistant for the master ----------
const STATE: Record<string, string> = { new: "new module", on: "switched on", off: "switched off", changed: "", same: "kept" };

function MasterProposalPanel({ ms }: { ms: MasterStatus }) {
  const backend = useBackend();
  const toast = useApp((s) => s.toast);
  const setError = useApp((s) => s.setError);
  const setView = useApp((s) => s.setView);
  const p = ms.proposal!;
  const [off, setOff] = useState<string[]>([]);
  const fail = (e: unknown) => setError(String(e));
  // What the plan changes, entry by entry (AI-2): the ones that change nothing are listed as kept, without a tick.
  const cards = p.changes.map((c) => ({ k: c.key, name: c.name, rows: c.rows, why: c.rationale, state: c.state })).filter((c) => c.state !== "same" || c.why);
  const pick = cards.filter((c) => c.state !== "same");
  const chosen = pick.filter((c) => !off.includes(c.k)).map((c) => c.k);
  // 15f: what the render of the plan measured, against the master as it is.
  const pct = (v: number | undefined) => (v === undefined ? "—" : v < 0.5 ? "never" : `${Math.round(v)} % of the song`);
  const [b0, a0] = [p.before, p.after];
  const ab = ms.listening === "proposal";
  const applyNow = async (only: string[] | null) => {
    try {
      const label = await backend.masterAiApply(only);
      toast("ok", "Master proposal applied", `${label}. One step in the master history.`);
    } catch (e) {
      fail(e);
    }
  };
  // AI-23: a proposal received earlier changes a master that has moved on, so it asks first.
  const apply = (only: string[] | null) =>
    p.reopened ? setView({ dialog: reapplyDialog(p.record.started_ms, "master", !!only, () => void applyNow(only)) }) : void applyNow(only);
  return (
    <aside className="card side" aria-label="Master proposal">
      <div className="ph"><span className="h">Assistant</span></div>
      <div className="sect">
        <div className="fx g2">
          <Icon name="spark" className="aic" />
          <span className="b grow">{p.request ? "Master revision" : "Master proposal"}</span>
          <span className="faint s1">{usd(p.record.cost_usd)}</span>
          <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: p.call_id })}>Details</button>
        </div>
        {p.reopened && <ReopenedNote at={p.record.started_ms} phase="master" />}
        <div className="tile">
          <Seg label="A/B" value={ab ? 1 : 0} items={[{ value: 0, label: "Current master" }, { value: 1, label: "Proposal" }]} onChange={(v) => void backend.masterListen(v ? "proposal" : "master").catch(fail)} />
          {ms.listen_same ? (
            <span className="fx g1 s1 acc">
              <Icon name="level" size="sm" />
              <span className="b">Same loudness</span>
            </span>
          ) : (
            <span className="b s1">Real level</span>
          )}
          <span className="faint s1" data-testid="proposal-level">
            {proposalHint(ms)} <span className="kbd">B</span> switches · <span className="kbd">⇧B</span> {ms.listen_same ? "real level" : "same loudness"}
          </span>
        </div>
        <p className="why dim">{p.summary}</p>
      </div>
      <div className="sect" data-testid="proposal-measured">
        <span className="faint s1">Measured on the whole song · {n1(a0?.true_peak_db)} dBTP guaranteed</span>
        {a0?.limiter && (
          <>
            <div className="dr"><span className="dim grow ell">Limiter over 3 dB</span><span className="was">{pct(b0?.limiter?.over_3_pct)}</span><span className="faint">→</span><span className={`to ${(a0.limiter.over_3_pct ?? 0) < (b0?.limiter?.over_3_pct ?? 0) ? "okc" : ""}`}>{pct(a0.limiter.over_3_pct)}</span></div>
            <div className="dr"><span className="dim grow ell">Limiter at most</span><span className="was">{n1(b0?.limiter?.max_db)} dB</span><span className="faint">→</span><span className="to">{n1(a0.limiter.max_db)} dB</span></div>
          </>
        )}
        {a0?.short_crest_db != null && <div className="dr"><span className="dim grow ell">Peaks above body, short</span><span className="was">{n1(b0?.short_crest_db)} dB</span><span className="faint">→</span><span className="to">{n1(a0.short_crest_db)} dB</span></div>}
        <div className="dr"><span className="dim grow ell">Loudness</span><span className="was">{n1(b0?.lufs)} LUFS</span><span className="faint">→</span><span className="to">{n1(a0?.lufs)} LUFS</span></div>
        {a0?.distance_db != null && <div className="dr"><span className="dim grow ell">Distance to reference</span><span className="was">{n1(b0?.distance_db)}</span><span className="faint">→</span><span className="to">{n1(a0.distance_db)}</span></div>}
      </div>
      <div className="sect grow scroll">
        {cards.map((c) => (
          <div key={c.k} className={`pcard ${off.includes(c.k) ? "is-off" : ""} ${c.state === "same" ? "is-lock" : ""}`} data-testid={`mcard-${c.k.replace("block:", "")}`} data-name={c.name}>
            <div className="fx g2">
              {c.state !== "same" && (
                <button type="button" role="checkbox" aria-checked={!off.includes(c.k)} aria-label={`Apply ${c.name}`} className={`check ${off.includes(c.k) ? "" : "on"}`} onClick={() => setOff(off.includes(c.k) ? off.filter((x) => x !== c.k) : [...off, c.k])}>
                  <Icon name="check" />
                </button>
              )}
              <span className="b grow">{c.name}{STATE[c.state] ? <span className="faint s1"> · {STATE[c.state]}</span> : null}</span>
            </div>
            {c.rows.map((r, i) => (
              <span key={i} className="s1">
                {r.label} {r.before === null ? "—" : n1(r.before)} → {r.after === null ? "—" : n1(r.after)} {r.unit}
              </span>
            ))}
            {c.why && <span className="dim s1">{c.why}</span>}
          </div>
        ))}
      </div>
      <div className="foot">
        <div className="fx g2">
          <button type="button" className="btn quiet" onClick={() => void backend.masterAiDiscard().catch(fail)}>{p.reopened ? "Close review" : "Discard"}</button>
          <span className="grow" />
          <button type="button" className="btn" disabled={!off.length || !chosen.length} onClick={() => void apply(chosen)}>Apply selected ({chosen.length})</button>
          <button type="button" className="btn primary" onClick={() => void apply(null)}>Apply all</button>
        </div>
      </div>
    </aside>
  );
}

function MasterAssistant({ ms }: { ms: MasterStatus }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const ai = useApp((s) => s.ai);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const [text, setText] = useState("");
  const [tab, setTab] = useState(0);
  const revision = useApp((s) => s.revision);
  const [hist, setHist] = useState<{ index: number; label: string; ai_call: string | null; undone: boolean }[]>([]);
  useEffect(() => {
    backend.history("master").then(([e]) => setHist(e)).catch(() => {});
  }, [backend, revision]);
  if (ms.proposal) return <MasterProposalPanel ms={ms} />;
  const running = ms.job?.kind === "master_ai";
  const blocked = !!ai && !ai.has_key && !ai.config.offline_demo;
  const masterCalls = project.ai.calls.filter((c) => c.kind === "master" || c.kind === "master_revision");
  const last = masterCalls[masterCalls.length - 1];
  const start = () => {
    const req = text.trim() || null;
    void backend.masterAiStart(req).catch((e) => setError(String(e) === "no-key" ? "Set up the assistant, or use the offline demo." : String(e)));
    setText("");
  };
  return (
    <aside className="card side" aria-label="Assistant">
      <div className="ph">
        <span className="h">Assistant</span>
        <div className="grow" />
        <Seg className="quiet sm" label="Assistant view" value={tab} items={[{ value: 0, label: "Notes" }, { value: 1, label: "History" }, { value: 2, label: "Proposals" }]} onChange={setTab} />
      </div>
      {tab === 2 ? (
        <ProposalsTab phase="master" busy={running ? "The assistant is working" : ms.job ? "Wait for the current job to finish" : ms.analysing ? "Waiting for the loudness measure" : null} />
      ) : tab === 1 ? (
        <div className="sect grow scroll">
          <span className="faint s1">Master steps only: Mix and Master keep separate histories.</span>
          {[...hist].reverse().map((e) => (
            <button type="button" key={e.index} className={`mi ${e.undone ? "is-dis" : ""}`} onClick={() => void backend.historyGoto("master", e.index)}>
              {e.ai_call ? <Icon name="spark" size="m" className="aic" /> : <span className="chk" />}
              <span className="grow ell">{e.label}</span>
            </button>
          ))}
          {!hist.length && <span className="dim">No master steps yet.</span>}
        </div>
      ) : (
        <>
          <div className="sect">
            <div className="fx g1 wrap">
              <span className="tag">{targetName(project.master)}</span>
              {ms.references.map((r) => <span key={r.id} className="tag">{r.name}</span>)}
              {project.brief.genre && <span className="tag">{project.brief.genre}</span>}
            </div>
          </div>
          <div className="sect grow">
            {running ? (
              <div className="note colm">
                <span className="b">Master with AI in progress</span>
                <div className="progress ai"><i style={{ width: `${Math.round((ms.job?.progress ?? 0) * 100)}%` }} /></div>
                <span className="dim s1">Only measurements of the mixdown are sent. The proposal is measured before you review it.</span>
                <button type="button" className="btn" onClick={() => void backend.masterJobCancel()}>Cancel</button>
              </div>
            ) : last ? (
              <div className="note colm">
                <span className="b">Last master call · {last.outcome.replace("_", " ")}</span>
                <span className="dim s1">{last.history_label ?? (last.request ? `“${last.request}”` : "Master with AI")}</span>
              </div>
            ) : (
              <div className="empty">
                <Icon name="spark" />
                <span className="s1">The assistant proposes a master from the mixdown's measurements, the references, the brief and the destination: every module it sets comes with its reason.</span>
              </div>
            )}
          </div>
          <div className="foot">
            <label className="ask">
              <textarea className="bare grow" rows={2} aria-label="Request for the master" placeholder="Tell the assistant what to change: “a little less bright, keep the punch”" value={text} onChange={(e) => setText(e.target.value)} />
            </label>
            <div className="fx between">
              {blocked ? <button type="button" className="link s1" onClick={() => setView({ setup: "connect" })}>Set up the assistant…</button> : <span className="faint s1">{ai?.config.offline_demo ? "Offline demo" : "This request about $0.01"}</span>}
              <button type="button" className="btn primary" disabled={running || blocked || !!ms.job || ms.analysing} title={ms.analysing ? "Waiting for the loudness measure" : undefined} onClick={start}>
                <Icon name="spark" size="m" />
                Master with AI
              </button>
            </div>
            <div className="fx between">
              <span className="faint s1">Only measurements of the mixdown are sent.</span>
              <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: null })}>AI usage</button>
            </div>
          </div>
        </>
      )}
    </aside>
  );
}

// ---------- the workspace ----------
export function MasterWorkspace({ ms }: { ms: MasterStatus }) {
  const backend = useBackend();
  const section = useApp((s) => s.masterSection);
  const project = useApp((s) => s.project)!;
  const exportOpen = useApp((s) => s.exportOpen);
  // 15g: said once per target; "Keep it" silences it until the target changes.
  const [strainSeen, setStrainSeen] = useState("");
  const strainKey = `${project.master.target.lufs}`;
  const setView = useApp((s) => s.setView);
  const store = useStore();
  const cmd = useProjectCommands();
  // The shortcuts that make sense on the mixdown (UI-3): transport, the master's own history, saving. While the
  // master is prepared none of them works; Esc goes back to Mix and the work goes on (9f).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tgt = e.target as HTMLElement;
      if (tgt.closest("input, textarea, select, [contenteditable], .menu, [role=dialog]")) return;
      const id = shortcutOf(e);
      const s = store.getState();
      const fail = (x: unknown) => s.setError(String(x));
      if (s.masterStatus?.prep) {
        if (e.key === "Escape") {
          e.preventDefault();
          void backend.setPhase(false).catch(fail);
        } else if (id) e.preventDefault();
        return;
      }
      // 9v: while a new target is reached the transport still works; Esc goes back to Mix, the rest waits.
      if (s.masterStatus?.retarget && !s.exportOpen) {
        if (e.key === "Escape") {
          e.preventDefault();
          void backend.setPhase(false).catch(fail);
          return;
        }
        if (id && id !== "play" && id !== "stop" && id !== "start") {
          e.preventDefault();
          return;
        }
      }
      switch (id) {
        case "play":
          e.preventDefault();
          void (s.meters?.playing ? backend.pause() : backend.play()).catch(fail);
          return;
        case "stop":
          void backend.stop().catch(fail);
          return;
        case "start":
          void backend.locate(0).catch(fail);
          return;
        case "undo":
        case "redo":
          e.preventDefault();
          void (id === "redo" ? backend.redo("master") : backend.undo("master")).catch(fail);
          return;
        case "save":
          e.preventDefault();
          void cmd.save();
          return;
        case "saveAs":
          e.preventDefault();
          void cmd.saveAs();
          return;
        case "sameLoudness":
          e.preventDefault();
          void backend.masterListenSame(!s.masterStatus?.listen_same).catch(fail);
          return;
        case "ab":
          // B: current master or proposal while one is in review (9c, 9o).
          if (!s.masterStatus?.proposal) return;
          e.preventDefault();
          void backend.masterListen(s.masterStatus.listening === "proposal" ? "master" : "proposal").catch(fail);
          return;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [backend, store, cmd]);
  // The spectrum of the master output (MAS-4).
  useEffect(() => {
    void backend.watchSpectrum("__bus__").catch(() => {});
    return () => void backend.watchSpectrum(null).catch(() => {});
  }, [backend]);
  // Preparing (9d–9j): the shells, then the overlay past --wait-show; the full workspace when every measure is ready.
  const { shown, leaving } = useWaitShown(!!ms.prep);
  const lastPrep = useRef(ms.prep);
  if (ms.prep) lastPrep.current = ms.prep;
  const prep = ms.prep ?? (shown ? lastPrep.current : null);
  // A new target (9v): the same overlay over the workspace as it is, unless the export dialog is open — there the
  // change was made in the dialog, whose primary waits for the measure (10d).
  const rt = useWaitShown(!!ms.retarget && !ms.prep && !exportOpen);
  const lastRt = useRef(ms.retarget);
  if (ms.retarget) lastRt.current = ms.retarget;
  const retarget = ms.retarget ?? (rt.shown ? lastRt.current : null);
  const focusStore = useStore();
  const rtWas = useRef(false);
  useEffect(() => {
    // Gone: the focus goes back to where the target is chosen, if it was left on the overlay.
    const a = document.activeElement;
    // During the guided tour the focus is the tour's (UI-18): between two steps it rests on nothing for an instant,
    // and taking it here would send the tour's next key to this control.
    const lost = (!a || a === document.body || !a.isConnected) && !focusStore.getState().tour;
    if (rtWas.current && !rt.shown && lost) document.querySelector<HTMLElement>("[aria-label=Destination]")?.focus();
    rtWas.current = rt.shown;
  }, [rt.shown, focusStore]);
  const wasShown = useRef(false);
  useEffect(() => {
    // The overlay gone: the focus goes back to the Master segment (9d "Fine") — only if it was left on the overlay,
    // which is no longer there. While the overlay fades out it lets clicks through, so the user may already be
    // somewhere else (typing in the export dialog): the focus stays there.
    const a = document.activeElement;
    const lost = (!a || a === document.body || !a.isConnected) && !focusStore.getState().tour;
    if (wasShown.current && !shown && lost) document.querySelector<HTMLElement>(".seg.phase [aria-checked=true]")?.focus();
    wasShown.current = shown;
  }, [shown, focusStore]);
  const [skipSeen, setSkipSeen] = useState<string>("");
  const skipped = ms.skipped_references.join(", ");
  const others = ms.references.map((r) => r.name);
  return (
    <>
      <MasterTop ms={ms} preparing={!!ms.prep} waiting={!!ms.retarget && !ms.prep && !exportOpen} />
      <DeviceBanner />
      {ms.prep ? (
        <PrepGhost mixdown={ms.prep.mixdown} />
      ) : (
      <div className="body">
        <div className="left">
          {skipped && skipSeen !== skipped && (
            // 9i: a reference that could not be read is left out, and said so.
            <div className="banner" role="status" data-testid="skipped-references">
              <Icon name="warn" />
              <div className="col grow">
                <span className="b">{skipped} can’t be read</span>
                <span className="dim s1">
                  {others.length
                    ? `The comparison, the match EQ and the A/B use ${others.join(", ")} only until you replace or remove it.`
                    : "The comparison, the match EQ and the A/B have no reference until you replace or remove it."}
                </span>
              </div>
              <button type="button" className="btn sm" onClick={() => setView({ projectOpen: true })}>Project settings…</button>
              <button type="button" className="btn quiet sm" onClick={() => setSkipSeen(skipped)}>Not now</button>
            </div>
          )}
          {ms.stale && (
            <div className="banner" role="status">
              <Icon name="warn" />
              <div className="col grow">
                <span className="b">The mix has changed since this mixdown</span>
                <span className="dim s1">You are still mastering the older version.</span>
              </div>
              <button type="button" className="btn sm" onClick={() => setView({ bounceOpen: true })}>Bounce again and keep these settings</button>
            </div>
          )}
          {ms.strained && !ms.proposal && !ms.analysing && strainSeen !== strainKey && (
            // 15g: the chain has done what it can cleanly and the limiter still works hard.
            <div className="banner stack" role="status" data-testid="strained">
              <div className="fx g3">
                <Icon name="warn" />
                <div className="col grow">
                  <span className="b">This mix is too peaky for {n1(project.master.target.lufs)} LUFS</span>
                  <span className="dim s1">
                    The limiter still takes more than 3 dB off for {Math.round(ms.strained.over_3_pct)} % of the song{ms.measures?.peak_to_loudness_db != null && ms.mixdown_measures?.peak_to_loudness_db != null ? `: the peaks of the mixdown stand ${n1(ms.mixdown_measures.peak_to_loudness_db)} dB above its loudness, and this target leaves room for ${n1(ms.measures.peak_to_loudness_db)}` : ""}. Past what the chain takes off cleanly, the rest dulls the transients whatever does it.
                  </span>
                </div>
              </div>
              <div className="fx g2">
                <span className="grow" />
                <button type="button" className="btn quiet sm" onClick={() => setStrainSeen(strainKey)}>Keep it</button>
                <button type="button" className="btn sm" onClick={() => void backend.edit({ op: "set_master", master: { ...project.master, target: { preset: "custom", lufs: ms.strained!.easier_lufs, true_peak_db: project.master.target.true_peak_db } } }, `Target lowered to ${n1(ms.strained!.easier_lufs)} LUFS`).catch((e) => store.getState().setError(String(e)))}>Lower the target to {n1(ms.strained.easier_lufs)} LUFS</button>
                <button type="button" className="btn sm primary" onClick={() => { setView({ assistDraft: "Lower the tallest peaks of the mix by about 3 dB (the tracks that carry them), keeping their attack, so the master can be louder without dulling them." }); void backend.setPhase(false).catch((e) => store.getState().setError(String(e))); }}>Tame those peaks in Mix…</button>
              </div>
            </div>
          )}
          <WaveCard ms={ms} />
          <div className="rackrow">
            <Rack ms={ms} preview={ms.proposal?.preview} />
            {ms.proposal ? (
              <section className="card grow" aria-label="Proposal in review">
                <div className="empty">
                  <Icon name="spark" />
                  <span className="b">A proposal is in review</span>
                  <span className="s1">The rack shows the chain as the proposal sets it. Listen to it against the current master, then apply all of it, some of it, or discard it: the editors come back with the chain.</span>
                </div>
              </section>
            ) : section && project.sections.some((s) => s.id === section) ? (
              <SectionEditor key={section} ms={ms} id={section} />
            ) : (
              <BlockEditor ms={ms} />
            )}
          </div>
        </div>
        <Loudness ms={ms} />
        <MasterAssistant ms={ms} />
      </div>
      )}
      {prep && shown && <PrepOverlay ms={ms} prep={prep} leaving={leaving} />}
      {!ms.prep && retarget && rt.shown && !exportOpen && <RetargetOverlay retarget={retarget} leaving={rt.leaving} />}
      {exportOpen && <ExportDialog ms={ms} onClose={() => setView({ exportOpen: false })} />}
    </>
  );
}
