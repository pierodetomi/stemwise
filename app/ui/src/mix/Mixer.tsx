// Mixer (design 3a, 3e, 7c, 7f; TRK-2, TRK-4, FX-8, UI-8 → UI-11): a label rail on the left, one strip per track,
// group and return in the timeline's order, the mix bus pinned on the right. Every strip carries its chain labels and
// one send row per return; the Overview swaps the faders for a grid on the same columns. Strips and track headers
// read the same document, so they always agree.
import { memo, useCallback, useMemo } from "react";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { Fader, GrMeter, Meter, PanKnob, Seg, Toggle, ValueText, useControl } from "../shared/controls";
import { faderPos } from "../shared/format";
import { Icon } from "../shared/Icon";
import { useApp, useBackend, useStore } from "../state/context";
import { specOf } from "../state/store";
import { useEditCtl, useTrackCtl } from "./bind";
import { stripOrder } from "./geometry";
import type { FxKind } from "../bindings/FxKind";
import { INFO, blockName } from "./fx";
import { cellValue, chainGlance, marksOf, overviewKinds, relations, sendDb, shortHz, type Relations } from "./glance";
import { ChainSpacer, StripChain, StripSends, tintOf, useBlockActions } from "./Marks";
import { outName, paramLabel } from "./menus";

function levels(m: [number, number, number, number, number] | undefined) {
  if (!m) return { rms: -200, peak: -200, gr: 0 };
  return { rms: Math.max(m[2], m[3]), peak: Math.max(m[0], m[1]), gr: m[4] };
}

function DbText({ t }: { t: Track }) {
  const ctl = useTrackCtl(t, "fader", "level");
  const { shown } = useControl(ctl);
  return <ValueText ctl={ctl} shown={shown} className="db" bare />;
}

interface StripProps {
  t: Track;
  parent: string | null;
  kinds: boolean;
  sel: boolean;
  returns: Track[];
  rel: string | undefined;
  relRet: string | null;
  grid: boolean;
  /** Block rows of the Overview (16u). */
  rowKinds: FxKind[];
}

function Strip({ t, parent, kinds, sel, returns, rel, relRet, grid, rowKinds }: StripProps) {
  const backend = useBackend();
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const anySolo = useApp((s) => s.project?.tracks.some((x) => x.solo) ?? false);
  const fail = (e: unknown) => setError(String(e));
  const kind = t.kind === "group" ? (parent ? `→ ${parent}` : "group") : t.kind === "return" ? (t.ret?.type ?? "return") : parent ? `→ ${parent}` : "→ Mix";
  return (
    <div
      className={`strip ${tintOf(t)} ${t.kind !== "audio" ? "bus" : ""} ${parent ? "child" : ""} ${sel ? "is-sel" : ""} ${rel ? "is-rel" : ""}`}
      onPointerDownCapture={() => setView({ selTrack: t.id })}
      aria-label={`${t.name} strip`}
      title={rel ? `${t.name} · ${rel}` : undefined}
      data-testid={`strip-${t.name}`}
    >
      <span className="nm ell" title={t.name}>{t.name}</span>
      {kinds && !grid && <span className="kind ell">{kind}</span>}
      <i className="cbar" />
      {grid ? <Cells t={t} returns={returns} kinds={rowKinds} relRet={relRet} /> : <FaderBody t={t} returns={returns} relRet={relRet} />}
      <div className="fx gh">
        <Toggle kind="mu" small on={t.mute} label={`Mute ${t.name}`} onClick={() => void backend.setView(t.id, "mute", !t.mute).catch(fail)} />
        <Toggle kind="so" small on={t.solo} implied={anySolo && !t.solo} label={`Solo ${t.name}`} onClick={() => void backend.setView(t.id, "solo", !t.solo).catch(fail)} />
      </div>
    </div>
  );
}

/** The faders view of a strip: chain labels, pan, sends, meters and fader. Only this part follows the meters. */
function FaderBody({ t, returns, relRet }: { t: Track; returns: Track[]; relRet: string | null }) {
  const project = useApp((s) => s.project)!;
  const meter = useApp((s) => s.meters?.tracks.find((x) => x[0] === t.id)?.[1]);
  const peakNumbers = useApp((s) => s.prefs.peakNumbers);
  const fader = useTrackCtl(t, "fader", "level");
  const pan = useTrackCtl(t, "pan");
  const l = levels(meter);
  return (
    <>
      <StripChain t={t} project={project} />
      <PanKnob ctl={pan} />
      <StripSends t={t} returns={returns} relRet={relRet} />
      {peakNumbers && <span className="faint s1 num pkn" data-testid="peak-number">{l.peak > -120 ? l.peak.toFixed(1).replace("-", "−") : "−∞"}</span>}
      <div className="fz">
        <Meter rms={l.rms} peak={l.peak} />
        <Fader ctl={fader} />
        {t.kind !== "return" ? <GrMeter gr={l.gr} /> : <div className="meter gr" aria-hidden="true" />}
      </div>
      <DbText t={t} />
    </>
  );
}

/** The Overview of a strip (3e, UI-11, 16u): one cell per kind of block in the project, per return, output and automation. */
const Cells = memo(function Cells({ t, returns, kinds, relRet }: { t: Track; returns: Track[]; kinds: FxKind[]; relRet: string | null }) {
  const project = useApp((s) => s.project)!;
  const specs = useApp((s) => s.specs);
  const backend = useBackend();
  const store = useStore();
  const a = useBlockActions(t);
  const g = chainGlance(t, project);
  const marks = marksOf(t.chain, project);
  const ret = t.ret;
  // A return's processor fills the rows its own chain leaves empty, from the top: its filters on the filter rows.
  const procCells: Record<string, React.ReactNode> = {};
  if (t.kind === "return" && ret) {
    const free = kinds.filter((k) => !t.chain.blocks.some((b) => b.fx.type === k));
    const put = (k: FxKind | undefined, node: React.ReactNode) => {
      if (k !== undefined) procCells[k] = node;
    };
    put(free.includes("hpf") ? "hpf" : undefined, <div key="hp" className="cl on" title={`${t.name} · ${ret.type === "reverb" ? "low cut" : "feedback low cut"}`}>{shortHz(ret.type === "reverb" ? ret.hpf_hz : ret.feedback_hpf_hz)}</div>);
    put(free.includes("lpf") ? "lpf" : undefined, <div key="lp" className="cl on" title={`${t.name} · ${ret.type === "reverb" ? "high cut" : "feedback high cut"}`}>{shortHz(ret.type === "reverb" ? ret.lpf_hz : ret.feedback_lpf_hz)}</div>);
    const rest = free.filter((k) => k !== "hpf" && k !== "lpf");
    put(
      rest[0],
      <button
        type="button"
        key="fx"
        className={`cl ${ret.bypass ? "off" : "on"}`}
        aria-label={`${t.name} · ${ret.type} ${ret.bypass ? "off" : "on"}`}
        aria-pressed={!ret.bypass}
        title={`${t.name} · ${ret.type} · click to ${ret.bypass ? "turn on" : "bypass"}`}
        onClick={(e) => a.click(e, () => a.toggle("bypass.fx", ret.type === "reverb" ? "Reverb" : "Delay"))}
        onDoubleClick={() => a.dbl("fx")}
      >
        {ret.bypass ? "off" : ret.type === "reverb" ? ret.algorithm : "delay"}
      </button>,
    );
    put(rest[1], <div key="v" className="cl on">{ret.type === "reverb" ? `${Number(ret.decay_s.toFixed(1))} s` : (ret.sync ?? `${Math.round(ret.time_ms)} ms`)}</div>);
  }
  const blockCells = kinds.map((k) => {
    const of = t.chain.blocks.filter((b) => b.fx.type === k);
    if (!of.length) {
      if (procCells[k]) return procCells[k];
      return t.kind === "return" ? (
        <div key={k} className="cl none" />
      ) : (
        <div key={k} className="cl off" title={`${t.name} · no ${INFO[k].label} in the chain`}>
          off
        </div>
      );
    }
    const b = of[0];
    const m = marks.find((x) => x.id === b.id)!;
    const st = m.state;
    const keyT = m.key ? project.tracks.find((x) => x.id === m.key) : undefined;
    const value = cellValue(b, st);
    return (
      <button
        type="button"
        key={k}
        className={`cl ${st === "on" ? "on" : st === "neutral" ? "nu" : "off"}`}
        aria-label={`${t.name} · ${m.title} · ${value}${of.length > 1 ? ` · and ${blockName(t.chain, of[1].id)}` : ""}`}
        aria-pressed={st !== "off"}
        title={`${m.title}${of.length > 1 ? ` · ${blockName(t.chain, of[1].id)} too` : ""} · click to ${st === "off" ? "turn on" : "bypass"}, double-click to open`}
        data-block={b.id}
        data-state={st}
        onClick={(e) => a.click(e, () => a.toggle(`bypass.${b.id}`, blockName(t.chain, b.id)))}
        onDoubleClick={() => a.dbl(b.id)}
      >
        {keyT && <i className={`led ${tintOf(keyT)}`} />}
        {value}
        {m.parallel !== null && <span className="dim">{Math.round(m.parallel * 100)}%</span>}
        {of.length > 1 && <span className="dim">×2</span>}
      </button>
    );
  });
  const addSend = (r: Track) => {
    const level = specOf(specs, `send.${r.id}`).default;
    void backend
      .edit({ op: "set_send", track: t.id, target: r.id, level_db: level }, `${t.name} · send to ${r.name}`)
      .catch((e) => store.getState().setError(String(e)));
  };
  const sendCells = returns.map((r, i) => {
    const gap = i ? "" : "gap";
    if (t.id === r.id) {
      const n = project.tracks.filter((x) => x.sends.some((s) => s.target === r.id)).length;
      return <div key={r.id} className={`cl on ${gap}`} title={`${n} tracks send to ${r.name}`}>{n} in</div>;
    }
    if (t.kind === "return") return <div key={r.id} className={`cl none ${gap}`} />;
    const s = t.sends.find((x) => x.target === r.id);
    if (!s)
      return (
        <button type="button" key={r.id} className={`cl off ${gap}`} aria-label={`${t.name} · add a send to ${r.name}`} title={`No send to ${r.name} · click to add one`} onClick={() => addSend(r)} data-send={r.name}>
          ·
        </button>
      );
    return (
      <button
        type="button"
        key={r.id}
        className={`cl on sn ${gap} ${relRet === r.id ? "rel" : ""}`}
        aria-label={`${t.name} · send to ${r.name} · ${sendDb(s.level_db)} dB`}
        title={`${r.name} · ${sendDb(s.level_db)} dB · double-click to open`}
        onDoubleClick={() => a.dbl("sends")}
        data-send={r.name}
      >
        <i style={{ width: `${faderPos(s.level_db) * 100}%` }} />
        <span>{Math.round(s.level_db) === 0 ? "0" : sendDb(Math.round(s.level_db)).replace(/\.0$/, "")}</span>
      </button>
    );
  });
  const auto = g.automation;
  return (
    <div className="cells">
      {blockCells}
      {sendCells}
      <div className="cl gap on" title={`Output · ${outName(project, t)}`}>{outName(project, t)}</div>
      {auto ? (
        <div className={`cl on ${auto.by === "ai" ? "ai" : "acc"}`} title={`Automation · ${auto.params.map((x) => paramLabel(project, x)).join(", ")}`}>
          {paramLabel(project, auto.params[0])}
          {auto.params.length > 1 ? ` +${auto.params.length - 1}` : ""}
        </div>
      ) : (
        <div className="cl off" title="No automation">·</div>
      )}
    </div>
  );
});

/** Row labels on the left of the mixer, and the switch between the two views (3a, 3e). */
function Rail({ returns, kinds, grid, relRet, rowKinds }: { returns: Track[]; kinds: boolean; grid: boolean; relRet: string | null; rowKinds: FxKind[] }) {
  const setView = useApp((s) => s.setView);
  return (
    <div className={`strip mxrail ${grid ? "wide" : ""}`} aria-label="Mixer rows">
      <span className="nm">&nbsp;</span>
      {kinds && !grid && <span className="kind">&nbsp;</span>}
      <i className="cbar" style={{ visibility: "hidden" }} />
      {grid ? (
        <div className="cells">
          {rowKinds.map((k) => (
            <div key={k} className="cl">
              {INFO[k].label}
              <span className="sg nu">{INFO[k].mark}</span>
            </div>
          ))}
          {returns.map((r, i) => (
            <div key={r.id} className={`cl ${i ? "" : "gap"} ${relRet === r.id ? "rel" : ""}`}>
              <span className="ell">{r.name}</span>
            </div>
          ))}
          <div className="cl gap">Output</div>
          <div className="cl">Automation</div>
        </div>
      ) : (
        <>
          <div className="lb pp-h" style={{ ["--chain-h" as string]: "var(--chain-col-h)" }}>Chain</div>
          <div className="lb kn-h">Pan</div>
          <div className="snd">
            {returns.map((r) => (
              <div key={r.id} className={`lb ${relRet === r.id ? "rel" : ""}`} title={`Send to ${r.name}`}>
                {r.name}
              </div>
            ))}
          </div>
        </>
      )}
      <Seg
        className="quiet sm"
        label="Mixer view"
        value={grid ? "grid" : "faders"}
        onChange={(v) => setView({ mixerView: v })}
        items={[
          { value: "faders", label: <Icon name="faders" />, aria: "Faders", title: "Faders" },
          { value: "grid", label: <Icon name="grid" />, aria: "Overview", title: "Overview" },
        ]}
      />
    </div>
  );
}

function BusStrip({ returns, grid, rowKinds }: { returns: Track[]; grid: boolean; rowKinds: FxKind[] }) {
  const project = useApp((s) => s.project)!;
  const bus = useApp((s) => s.meters?.bus);
  const make = useCallback((v: number) => ({ op: "set_mix_bus" as const, fader_db: v }), []);
  const ctl = useEditCtl("bus", "Mix bus · level", project.mix_bus.fader_db, make);
  const { shown } = useControl(ctl);
  const kinds = project.tracks.some((t) => t.kind === "group");
  return (
    <div className="strip tm bus mixbus" aria-label="Mix bus strip" data-testid="strip-Mix">
      <span className="nm ell">Mix</span>
      {kinds && !grid && <span className="kind ell">mix bus</span>}
      <i className="cbar" />
      {grid ? (
        <div className="cells">
          {rowKinds.map((k) => <div key={k} className="cl none" />)}
          {returns.map((r, i) => <div key={r.id} className={`cl none ${i ? "" : "gap"}`} />)}
          <div className="cl none gap" />
          <div className="cl none" />
        </div>
      ) : (
        <>
          <ChainSpacer />
          <div className="knob s" aria-hidden="true" />
          <StripSends t={null} returns={returns} relRet={null} />
          <div className="fz">
            <Meter rms={Math.max(bus?.[2] ?? -200, bus?.[3] ?? -200)} peak={Math.max(bus?.[0] ?? -200, bus?.[1] ?? -200)} />
            <Fader ctl={ctl} />
            <Meter rms={Math.max(bus?.[2] ?? -200, bus?.[3] ?? -200)} peak={Math.max(bus?.[0] ?? -200, bus?.[1] ?? -200)} />
          </div>
          <ValueText ctl={ctl} shown={shown} className="db" bare />
        </>
      )}
      <div className="fx gh" style={{ height: "var(--tgl-s)" }} />
    </div>
  );
}

/** What the selection is connected to, memoized on the document and the selection (UI-10). */
export function useRelations(project: Project): Relations {
  const selTrack = useApp((s) => s.selTrack);
  return useMemo(() => relations(project, selTrack), [project, selTrack]);
}

export function Mixer() {
  const project = useApp((s) => s.project)!;
  const selTrack = useApp((s) => s.selTrack);
  const grid = useApp((s) => s.mixerView === "grid");
  const strips = useMemo(() => stripOrder(project), [project]);
  const kinds = project.tracks.some((t) => t.kind === "group");
  const returns = useMemo(() => project.tracks.filter((t) => t.kind === "return"), [project]);
  const rel = useRelations(project);
  const rowKinds = useMemo(() => overviewKinds(project), [project]);
  return (
    <section className={`card mixer ${grid ? "is-grid" : ""}`} aria-label="Mixer">
      <Rail returns={returns} kinds={kinds} grid={grid} relRet={rel.ret} rowKinds={rowKinds} />
      <div className="mixer-scroll mx-scroll">
        {!project.tracks.some((t) => t.kind === "audio") && (
          <div className="empty grow">
            <span className="faint">Strips appear here as you add tracks.</span>
          </div>
        )}
        {strips.map((s) => (
          <Strip
            key={s.t.id}
            t={s.t}
            parent={s.parent}
            kinds={kinds}
            sel={selTrack === s.t.id}
            returns={returns}
            rel={rel.reasons.get(s.t.id)}
            relRet={rel.ret}
            grid={grid}
            rowKinds={rowKinds}
          />
        ))}
      </div>
      <BusStrip returns={returns} grid={grid} rowKinds={rowKinds} />
    </section>
  );
}
