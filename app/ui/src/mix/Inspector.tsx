// Inspector of the selected track (design 3a, 7a–7d, 16a–16c, 16p): the chain as a list of blocks (FX-10, UI-12) with
// the open block's editor below it, sends (TRK-5), output (TRK-3), the assistant's note for the track. Returns show
// their processor (FX-5, FX-6), the chain after it (FX-11) and who sends to them.
import { useCallback, useEffect } from "react";
import type { Project } from "../bindings/Project";
import type { ReverbAlgorithm } from "../bindings/ReverbAlgorithm";
import type { Track } from "../bindings/Track";
import { Icon } from "../shared/Icon";
import { Field, Knob, Seg } from "../shared/controls";
import { faderPos, formatDb } from "../shared/format";
import { useMenu } from "../shared/Menu";
import { useApp, useBackend } from "../state/context";
import type { InspBlock } from "../state/store";
import { useEdit, useEditCtl, useTrackCtl } from "./bind";
import { BlockEditor, ByToggle } from "./BlockEditors";
import { outName, outputItems, roleLabel } from "./menus";
import { ChainRack } from "./Rack";

function SendKnob({ t, ret }: { t: Track; ret: Track }) {
  const s = t.sends.find((x) => x.target === ret.id);
  const make = useCallback((v: number) => ({ op: "set_send" as const, track: t.id, target: ret.id, level_db: v <= -120 ? null : v }), [t.id, ret.id]);
  const editCtl = useEditCtl(`send.${ret.id}`, `${t.name} · send to ${ret.name}`, s?.level_db ?? -144, make);
  const paramCtl = useTrackCtl(t, `send.${ret.id}`, `send to ${ret.name}`);
  // An existing send is a parameter (no rebuild, automatable); a new one is an edit (TRK-5).
  return <Knob ctl={s ? paramCtl : editCtl} name={ret.name} size="s" />;
}

/** Trim (first, locked) and Fader · Pan (last, locked) open their own small editors. */
function FixedEditor({ t, which }: { t: Track; which: "trim" | "fader" }) {
  const trim = useTrackCtl(t, "trim");
  const fader = useTrackCtl(t, "fader", "level");
  const pan = useTrackCtl(t, "pan");
  return (
    <div className="sect ted" id="insp-editor">
      <div className="fx g2">
        <Icon name="lock" size="sm" className="faint" />
        <span className="b grow">{which === "trim" ? "Trim" : "Fader · Pan"}</span>
        <span className="faint s1">{which === "trim" ? "before every block" : "after every block"}</span>
      </div>
      <div className="knobs">
        {which === "trim" ? (
          <Knob ctl={trim} name="Trim" />
        ) : (
          <>
            <Knob ctl={fader} name="Level" />
            <Knob ctl={pan} name="Pan" bip />
          </>
        )}
      </div>
    </div>
  );
}

function TrackInspector({ t, project }: { t: Track; project: Project }) {
  const open = useApp((s) => s.inspBlock);
  const setView = useApp((s) => s.setView);
  const setOpen = (b: InspBlock) => setView({ inspBlock: b });
  const menu = useMenu();
  const edit = useEdit();
  const compact = useApp((s) => s.compact);
  const returns = project.tracks.filter((x) => x.kind === "return");
  const note = project.ai.notes.find((n) => n.track === t.id);
  const tint = t.kind === "audio" ? `t${t.color || 1}` : "tb";
  // The open block: the one chosen, or the first of the chain when it is not there (another track was selected).
  const sel = open === "trim" || open === "fader" || t.chain.blocks.some((b) => b.id === open) ? open : (t.chain.blocks[0]?.id ?? "fader");
  return (
    <aside className={`card side insp tinsp ${tint}`} aria-label="Inspector">
      <div className="ph">
        {t.kind === "group" ? <Icon name="group" size="m" /> : <i className="tdot" />}
        <span className="h ell" data-testid="inspector-name">{t.name}</span>
        <span className="tag">{t.kind === "group" ? "group" : roleLabel(t.role)}</span>
        <div className="grow" />
        <button type="button" className="route" aria-label={`Output of ${t.name}: ${outName(project, t)}`} onClick={(e) => menu.open(e.currentTarget, outputItems(project, t, edit), "Output")}>
          <Icon name="right" size="sm" />
          {outName(project, t)}
          <Icon name="chev" size="sm" />
        </button>
      </div>
      <div className="insp-body">
        <ChainRack t={t} project={project} sel={sel} onSel={setOpen} />
        {sel === "trim" || sel === "fader" ? <FixedEditor t={t} which={sel} /> : <BlockEditor t={t} project={project} id={sel} />}
        {/* Sends are not a block of the chain: they leave it after the fader. Their own section, at the foot (A-065). */}
        <div className="sect sends tsnd" id="insp-sends">
          <div className="fx between">
            <span className="b">Sends</span>
            <span className="faint s1">post-fader · to the returns</span>
          </div>
          <div className="knobs wrap">
            {returns.map((r) => (
              <SendKnob key={r.id} t={t} ret={r} />
            ))}
            {!returns.length && (
              <div className="empty">
                <Icon name="wave" />
                <span className="b">No returns in this project</span>
                <span className="s1">Add a reverb or a delay to send this track to it.</span>
                <button type="button" className="btn" onClick={() => void edit({ op: "add_return", reverb: true }, "Add reverb return")}>
                  <Icon name="plus" size="m" />
                  Add reverb return
                </button>
              </div>
            )}
          </div>
        </div>
        {note && !compact && (
          <div className="sect">
            <div className="note on colm">
              <div className="fx g2">
                <span className="badge">{note.number}</span>
                <span className="b">Why this track changed</span>
              </div>
              <p className="why">{note.text}</p>
            </div>
          </div>
        )}
      </div>
      {menu.node}
    </aside>
  );
}

const SYNC = ["1/16", "1/8", "1/8.", "1/4", "1/4.", "1/2"];

/** The reverbs (FX-5, FX-22), with the line that says what each one is for. */
const ALGORITHMS: { value: ReverbAlgorithm; label: string; hint: string }[] = [
  { value: "plate", label: "Plate", hint: "Dense and bright from the first instant, no early reflections. At its best from about 1 s up." },
  { value: "room", label: "Room", hint: "A short space with early reflections: the sound of something played in a place." },
  { value: "hall", label: "Hall", hint: "Long and diffuse, with soft early reflections." },
  { value: "shimmer", label: "Shimmer", hint: "A hall whose tail climbs in pitch: an octave or a fifth above, ever brighter." },
];

function ReturnInspector({ t, project }: { t: Track; project: Project }) {
  const edit = useEdit();
  const menu = useMenu();
  const setView = useApp((s) => s.setView);
  const open = useApp((s) => s.inspBlock);
  const sel = t.chain.blocks.some((b) => b.id === open) || open === "fader" ? open : "fx";
  const rev = t.ret?.type === "reverb";
  const decay = useTrackCtl(t, "rev.decay");
  const size = useTrackCtl(t, "rev.size");
  const damp = useTrackCtl(t, "rev.damping");
  const modu = useTrackCtl(t, "rev.modulation");
  const pre = useTrackCtl(t, "rev.predelay");
  const rhpf = useTrackCtl(t, "rev.hpf");
  const rlpf = useTrackCtl(t, "rev.lpf");
  const algorithm = t.ret?.type === "reverb" ? t.ret.algorithm : null;
  const time = useTrackCtl(t, "dly.time");
  const fb = useTrackCtl(t, "dly.feedback");
  const fbh = useTrackCtl(t, "dly.fb_hpf");
  const fbl = useTrackCtl(t, "dly.fb_lpf");
  const wow = useTrackCtl(t, "dly.wow");
  const drive = useTrackCtl(t, "dly.drive");
  const shimmer = useTrackCtl(t, "rev.shimmer");
  const level = useTrackCtl(t, "fader", "return level");
  const sync = t.ret?.type === "delay" ? t.ret.sync : null;
  const senders = project.tracks.filter((x) => x.sends.some((s) => s.target === t.id));
  const returns = project.tracks.filter((x) => x.kind === "return").length;
  return (
    <aside className="card side insp tb" aria-label="Inspector">
      <div className="ph">
        <Icon name="wave" />
        <span className="h ell" data-testid="inspector-name">{t.name}</span>
        <span className="tag">{rev ? "reverb return" : "delay return"}</span>
        <div className="grow" />
        <button type="button" className="route" onClick={(e) => menu.open(e.currentTarget, outputItems(project, t, edit), "Output")}>
          <Icon name="right" size="sm" />
          {outName(project, t)}
          <Icon name="chev" size="sm" />
        </button>
      </div>
      <div className="insp-body">
        <div className="sect">
          <div className="module is-open">
            <div className="mh2">
              <ByToggle t={t} id="fx" name={rev ? "Reverb" : "Delay"} />
              <span className="b grow">{rev ? "Reverb" : "Delay"}</span>
              {algorithm && (
                <Seg
                  className="quiet sm"
                  label="Algorithm"
                  value={algorithm}
                  items={ALGORITHMS.map((a) => ({ value: a.value, label: a.label }))}
                  onChange={(v) => {
                    const a = ALGORITHMS.find((x) => x.value === v);
                    if (a && a.value !== algorithm) void edit({ op: "set_reverb_algorithm", track: t.id, algorithm: a.value }, `${t.name} · ${a.label}`);
                  }}
                />
              )}
              {!rev && (
                <Seg
                  className="quiet sm"
                  label="Time unit"
                  value={sync ?? "ms"}
                  items={["ms", ...SYNC.filter((x) => ["1/8", "1/8.", "1/4"].includes(x))].map((v) => ({ value: v, label: v }))}
                  onChange={(v) => {
                    if (v !== "ms" && !project.tempo) return;
                    void edit({ op: "set_delay_sync", track: t.id, sync: v === "ms" ? null : v }, `${t.name} · time ${v}`);
                  }}
                />
              )}
            </div>
            <div className="knobs">
              {rev ? (
                <>
                  <Knob ctl={decay} name="Decay" size="l" />
                  <Knob ctl={size} name="Size" size="l" />
                  <Knob ctl={pre} name="Pre-delay" size="l" />
                </>
              ) : (
                <>
                  <Knob ctl={time} name="Time" size="l" />
                  <Knob ctl={fb} name="Feedback" size="l" />
                  <Knob ctl={level} name="Level" size="l" />
                </>
              )}
            </div>
            {algorithm && (
              <>
                <div className="knobs">
                  <Knob ctl={damp} name="Damping" />
                  <Knob ctl={modu} name="Modulation" />
                  {algorithm === "shimmer" && <Knob ctl={shimmer} name="Amount" />}
                </div>
                {t.ret?.type === "reverb" && algorithm === "shimmer" && (
                  <div className="fx g2">
                    <span className="faint s1">Interval</span>
                    <Seg
                      className="quiet sm"
                      label="Shimmer interval"
                      value={t.ret.shimmer_semitones}
                      items={[5, 7, 12].map((v) => ({ value: v, label: `+${v}` }))}
                      onChange={(v) => t.ret?.type === "reverb" && void edit({ op: "set_return", track: t.id, ret: { ...t.ret, shimmer_semitones: v } }, `${t.name} · shimmer +${v}`)}
                    />
                    <span className="faint s1">semitones</span>
                  </div>
                )}
                <span className="faint s1">{ALGORITHMS.find((a) => a.value === algorithm)?.hint}{algorithm === "shimmer" ? " Amount 0 is the Hall." : ""}</span>
              </>
            )}
            {t.ret?.type === "delay" && (
              <>
                <div className="fx g2 wrap">
                  <Seg
                    className="quiet sm"
                    label="Delay mode"
                    value={t.ret.mode}
                    items={[{ value: "stereo", label: "Stereo" }, { value: "ping_pong", label: "Ping-pong" }]}
                    onChange={(v) => t.ret?.type === "delay" && void edit({ op: "set_return", track: t.id, ret: { ...t.ret, mode: v } }, `${t.name} · ${v === "stereo" ? "stereo" : "ping-pong"}`)}
                  />
                  <Seg
                    className="quiet sm"
                    label="Delay character"
                    value={t.ret.character}
                    items={[{ value: "clean", label: "Clean" }, { value: "tape", label: "Tape" }]}
                    onChange={(v) => t.ret?.type === "delay" && void edit({ op: "set_return", track: t.id, ret: { ...t.ret, character: v } }, `${t.name} · ${v}`)}
                  />
                </div>
                {t.ret.character === "tape" && (
                  <div className="knobs">
                    <Knob ctl={wow} name="Wow" />
                    <Knob ctl={drive} name="Drive" />
                  </div>
                )}
                <span className="faint s1">Ping-pong bounces the repeats between the sides. Tape adds a slow wobble and a little drive to each repeat.</span>
              </>
            )}
            {!rev && (
              <span className="faint s1">
                {project.tempo ? `${sync ? `${sync} at ${project.tempo.bpm} BPM. ` : ""}Note values follow the project tempo.` : "Note values appear when the project has a tempo."}
              </span>
            )}
          </div>
          <div className="module">
            <div className="mh2">
              <span className="b grow">{rev ? "Return filter" : "Feedback filters"}</span>
            </div>
            <div className="fx g3">
              <label className="flabel">
                <span className="l">High-pass</span>
                <Field ctl={rev ? rhpf : fbh} />
              </label>
              <label className="flabel">
                <span className="l">Low-pass</span>
                <Field ctl={rev ? rlpf : fbl} />
              </label>
              {rev && (
                <label className="flabel">
                  <span className="l">Return level</span>
                  <Field ctl={level} />
                </label>
              )}
            </div>
          </div>
        </div>
        <ChainRack t={t} project={project} sel={sel} onSel={(b) => setView({ inspBlock: b })} />
        {sel === "fader" ? <FixedEditor t={t} which="fader" /> : sel !== "fx" && <BlockEditor t={t} project={project} id={sel} />}
        <div className="sect">
          <div className="fx between">
            <span className="b">Tracks sending here</span>
            <span className="faint s1">post-fader</span>
          </div>
          {senders.map((s) => {
            const lv = s.sends.find((x) => x.target === t.id)!.level_db;
            return (
              <button type="button" key={s.id} className={`fx g2 t${s.color || 1} sender`} onClick={() => setView({ selTrack: s.id })}>
                <i className="tdot" />
                <span className="ell" style={{ width: "var(--sp-8)" }}>{s.name}</span>
                <div className="hbar grow">
                  <i style={{ width: `${faderPos(lv) * 100}%`, background: "var(--tc)" }} />
                </div>
                <span className="s1 b" style={{ width: "var(--sp-7)", textAlign: "right" }}>{formatDb(lv)} dB</span>
              </button>
            );
          })}
          {!senders.length && <span className="faint s1">No track sends here yet. Turn up a send in a track's inspector.</span>}
        </div>
      </div>
      <div className="foot">
        <div className="fx between">
          <span className="faint s1">Returns {returns} of 8</span>
          <button
            type="button"
            className="btn quiet sm"
            onClick={() =>
              setView({
                dialog: {
                  title: `Delete “${t.name}”?`,
                  sub: `${senders.length ? `The sends of ${senders.length} track${senders.length === 1 ? "" : "s"} to it go with it. ` : ""}You can undo this.`,
                  confirm: "Delete return",
                  danger: true,
                  onConfirm: () => void edit({ op: "delete_track", track: t.id }, `Delete ${t.name}`),
                },
              })
            }
          >
            <Icon name="trash" size="m" />
            Delete return…
          </button>
        </div>
      </div>
      {menu.node}
    </aside>
  );
}

export function Inspector() {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const selTrack = useApp((s) => s.selTrack);
  const t = project.tracks.find((x) => x.id === selTrack) ?? null;
  // The EQ editor shows the selected track's live spectrum (FX-7).
  useEffect(() => {
    void backend.watchSpectrum(t && t.kind !== "return" ? t.id : null).catch(() => {});
  }, [backend, t?.id, t?.kind]); // eslint-disable-line react-hooks/exhaustive-deps
  if (!t) {
    return (
      <aside className="card side insp" aria-label="Inspector">
        <div className="empty grow">
          <Icon name="cursor" />
          <span className="b">Nothing selected</span>
          <span className="s1">Select a track to see its chain of blocks, its sends and where it goes.</span>
        </div>
      </aside>
    );
  }
  return t.kind === "return" ? <ReturnInspector t={t} project={project} /> : <TrackInspector key={t.id} t={t} project={project} />;
}
