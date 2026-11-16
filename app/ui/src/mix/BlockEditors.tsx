// The editor of the open block, below the chain (design 16a, 16d–16s; UI-12): what it does to the sound, then its
// controls. Every control is a parameter of the core (automatable, AUT-1) or an edit of the block's settings.
import { useState } from "react";
import type { Fx } from "../bindings/Fx";
import type { FxBlock } from "../bindings/FxBlock";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { Field, Knob, Seg, Toggle } from "../shared/controls";
import { formatDb } from "../shared/format";
import { Icon } from "../shared/Icon";
import { useMenu } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import { useEdit, useTrackCtl } from "./bind";
import { CompCurve, EqGraph, eqBands } from "./EqGraph";
import { INFO, blockName, blockSummary } from "./fx";
import { bandName } from "./geometry";
import { marksOf } from "./glance";
import { tintOf } from "./Marks";
import { blockParams, sidechainItems } from "./menus";
import { FX_EDITORS } from "./fxEditors";

/** Switches a block's bypass with the inspector's command (one undo step). */
export function ByToggle({ t, id, name }: { t: Track; id: string; name: string }) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const b = t.chain.blocks.find((x) => x.id === id);
  const on = id === "fx" ? !t.ret?.bypass : !(b?.fx.bypass ?? true);
  return (
    <Toggle
      kind="by"
      on={on}
      label={`${name} on`}
      onClick={() => void backend.setParam(t.id, `bypass.${id}`, on ? 1 : 0, `${t.name} · ${name} ${on ? "off" : "on"}`).catch((e) => setError(String(e)))}
    />
  );
}

/** Replaces a block's settings that are not continuous (modes, character, voices): one undo step. */
export function useSetBlock(t: Track) {
  const edit = useEdit();
  return (b: FxBlock, fx: Fx, what: string) => edit({ op: "set_block", track: t.id, block: { id: b.id, fx } }, `${t.name} · ${blockName(t.chain, b.id)} ${what}`);
}

/** A knob on a block parameter (`key` within the block: `freq`, `2.gain`). */
export function BKnob({ t, b, k, name, size, bip }: { t: Track; b: FxBlock; k: string; name?: string; size?: "" | "s" | "l"; bip?: boolean }) {
  const ctl = useTrackCtl(t, `${b.id}.${k}`);
  return <Knob ctl={ctl} name={name ?? ctl.spec.label} size={size} bip={bip} />;
}

export function BField({ t, b, k }: { t: Track; b: FxBlock; k: string }) {
  const ctl = useTrackCtl(t, `${b.id}.${k}`);
  return <Field ctl={ctl} className="fill" />;
}

/** The sidechain button of a block (FX-3, FX-13, FX-15): its source's colour, a menu of the sources the core allows. */
export function SidechainButton({ t, project, b, band, label }: { t: Track; project: Project; b: FxBlock; band?: number; label?: string }) {
  const menu = useMenu();
  const edit = useEdit();
  const fx = b.fx;
  const src = fx.type === "compressor" ? (fx.sidechain?.source ?? null) : fx.type === "gate" ? fx.sidechain : fx.type === "dynamic_eq" ? (fx.bands[band ?? 0]?.sidechain ?? null) : null;
  const st = src ? project.tracks.find((x) => x.id === src) : null;
  const scHpf = useTrackCtl(t, `${b.id}.sc_hpf`, "sidechain low cut");
  return (
    <>
      <button
        type="button"
        className={`sc ${st ? `on ${tintOf(st)}` : ""}`}
        aria-label={`Sidechain of ${blockName(t.chain, b.id)}: ${st?.name ?? "none"}`}
        onClick={(e) =>
          menu.open(
            e.currentTarget,
            [
              ...sidechainItems(project, t, edit, b.id, band),
              ...(st && fx.type === "compressor" ? [{ label: "", custom: <label className="flabel"><span className="l">Detector high-pass</span><Field ctl={scHpf} /></label> }] : []),
            ],
            "Sidechain",
          )
        }
      >
        <Icon name="link" size="sm" />
        {st ? `${label ?? "Listens to"} ${st.name}` : "Sidechain · none"}
        {st && <Icon name="chev" size="sm" />}
      </button>
      {menu.node}
    </>
  );
}

function FilterEditor({ t, b }: { t: Track; b: FxBlock }) {
  return (
    <>
      <EqGraph t={t} block="" height={76} selBand={-1} onSelBand={() => {}} />
      <div className="knobs">
        <BKnob t={t} b={b} k="freq" name="Frequency" />
        <BKnob t={t} b={b} k="res" name="Resonance" />
      </div>
      <div className="tile row tinfo">
        <Icon name="info" size="sm" />
        <span className="dim s1">12 dB per octave. Resonance lifts the frequency itself: with automation or per-section offsets it makes a filter that opens and closes.</span>
      </div>
    </>
  );
}

function BandEditor({ t, b, band }: { t: Track; b: FxBlock; band: number }) {
  const edit = useEdit();
  const bands = eqBands(t, b.id);
  const x = bands[band];
  if (!x) return null;
  const set = (enabled: boolean, kind = x.kind) =>
    void edit({ op: "set_eq_band", track: t.id, band, enabled, kind, block: b.id }, `${t.name} · EQ band ${band + 1} ${enabled ? (kind === x.kind ? "on" : kind.replace("_", " ")) : "off"}`);
  return (
    <div className="col g2">
      <div className="fx g2">
        <Toggle kind="by" on={x.enabled} label={`EQ band ${band + 1} on`} onClick={() => set(!x.enabled)} />
        <Seg
          className="quiet sm"
          label={`Type of EQ band ${band + 1}`}
          value={x.kind}
          items={[
            { value: "low_shelf", label: "Low shelf" },
            { value: "peak", label: "Peak" },
            { value: "high_shelf", label: "High shelf" },
          ]}
          onChange={(k) => set(true, k)}
        />
      </div>
      <div className="fx g2">
        <BField t={t} b={b} k={`${band + 1}.freq`} />
        <BField t={t} b={b} k={`${band + 1}.gain`} />
        <BField t={t} b={b} k={`${band + 1}.q`} />
      </div>
    </div>
  );
}

function EqEditor({ t, b }: { t: Track; b: FxBlock }) {
  const [band, setBand] = useState(1);
  const bands = eqBands(t, b.id);
  return (
    <>
      <span className="faint s1">drag a handle · scroll for Q</span>
      <EqGraph t={t} block={b.id} height={112} selBand={band} onSelBand={setBand} />
      <div className="fx g1">
        {bands.map((x, i) => (
          <button type="button" key={i} className={`pill ${band === i ? "on" : ""} ${x.enabled ? "" : "is-off"}`} onClick={() => setBand(i)} aria-pressed={band === i}>
            <span className="faint s1">{bandName(x.freq_hz, x.enabled)}</span>
            <span className="b s1">{x.enabled ? `${formatDb(x.gain_db)} dB` : "—"}</span>
          </button>
        ))}
      </div>
      <BandEditor t={t} b={b} band={band} />
    </>
  );
}

function CompEditor({ t, project, b }: { t: Track; project: Project; b: FxBlock }) {
  const compact = useApp((s) => s.compact);
  const knee = useTrackCtl(t, `${b.id}.knee`);
  const cmix = useTrackCtl(t, `${b.id}.mix`, "compressor mix");
  if (b.fx.type !== "compressor") return null;
  const c = b.fx;
  return (
    <>
      <div className="fx g2">
        <SidechainButton t={t} project={project} b={b} />
      </div>
      <div className="fx g3">
        {!compact && <CompCurve threshold={c.threshold_db} ratio={c.ratio} knee={c.knee_db} />}
        <div className="knobs">
          <BKnob t={t} b={b} k="threshold" name="Threshold" />
          <BKnob t={t} b={b} k="ratio" name="Ratio" />
          <BKnob t={t} b={b} k="attack" name="Attack" />
          <BKnob t={t} b={b} k="release" name="Release" />
          <BKnob t={t} b={b} k="makeup" name="Makeup" />
        </div>
      </div>
      <div className="comp-extra">
        <span className="faint s1">Knee</span>
        <Field ctl={knee} />
        <span className="faint s1">Mix</span>
        <Field ctl={cmix} />
        <span className="faint s1 grow">{c.mix < 0.995 ? "Parallel" : "Stereo-linked"}</span>
      </div>
    </>
  );
}

/** Controls for a kind without an editor of its own: a knob per parameter, in catalog order. */
export function GenericEditor({ t, b }: { t: Track; b: FxBlock }) {
  const ids = blockParams(b);
  return (
    <div className="knobs wrap">
      {ids.map((id) => (
        <BKnob key={id} t={t} b={b} k={id.slice(b.id.length + 1)} />
      ))}
    </div>
  );
}

/** What a block's header says about it, beside its name (16a: "split · above 6.5 kHz"). */
function subtitle(project: Project, t: Track, b: FxBlock): string {
  const m = marksOf(t.chain, project).find((x) => x.id === b.id);
  const key = m?.key ? project.tracks.find((x) => x.id === m.key)?.name : null;
  return [blockSummary(b.fx), key ? `listens to ${key}` : ""].filter(Boolean).join(" · ");
}

/** The editor of block `id` of a track's chain. */
export function BlockEditor({ t, project, id }: { t: Track; project: Project; id: string }) {
  const b = t.chain.blocks.find((x) => x.id === id);
  if (!b) return null;
  const name = blockName(t.chain, id);
  const m = marksOf(t.chain, project).find((x) => x.id === id)!;
  const keyT = m.key ? project.tracks.find((x) => x.id === m.key) : undefined;
  const Special = FX_EDITORS[b.fx.type];
  return (
    <div className="sect ted" id="insp-editor" aria-label={`${name} editor`}>
      <div className="fx g2">
        <ByToggle t={t} id={id} name={name} />
        <span className={`sg ${m.state === "off" ? "" : m.state === "neutral" ? "nu" : "on"} ${keyT ? `key ${tintOf(keyT)}` : ""}`}>{INFO[b.fx.type].mark}</span>
        <span className="b">{name}</span>
        <span className="faint s1 grow ell">{subtitle(project, t, b)}</span>
      </div>
      {b.fx.type === "hpf" || b.fx.type === "lpf" ? (
        <FilterEditor t={t} b={b} />
      ) : b.fx.type === "eq" ? (
        <EqEditor t={t} b={b} />
      ) : b.fx.type === "compressor" ? (
        <CompEditor t={t} project={project} b={b} />
      ) : Special ? (
        <Special t={t} project={project} b={b} />
      ) : (
        <GenericEditor t={t} b={b} />
      )}
    </div>
  );
}
