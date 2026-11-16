// Listening level and the target warning (design 9l–9u, 10d–10e; MAS-2, MAS-5): the level mode and the A/B source
// in the top bar, the Listening section of the Loudness card, the warning when the destination sits far from the
// references, and its line in the export report.
import type { FileReport } from "../bindings/FileReport";
import { keysInText } from "../shared/keys";
import type { MasterStatus } from "../bindings/MasterStatus";
import { Icon } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useApp, useBackend } from "../state/context";
import { useMasterEdit } from "./bind";
import { f1, ft, gapWords, listenRows, refLevels, targetGap, type TargetGap } from "./level";

/** The warning that applies now: none without references, inside their range or after "Keep" (D29, D30). */
export function useTargetGap(ms: MasterStatus): { gap: TargetGap | null; kept: boolean } {
  const target = useApp((s) => s.project?.master.target.lufs ?? -14);
  const kept = useApp((s) => s.project?.master.target_kept ?? false);
  const gap = targetGap(target, refLevels(ms));
  return { gap, kept: kept && !!gap };
}

/** Top bar of the Master (D27): real level or same loudness, then the source of the A/B. */
export function ListenControls({ ms }: { ms: MasterStatus }) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const fail = (e: unknown) => setError(String(e));
  const refs = ms.references;
  // D31: with two references or more the segments shorten; the full name is in the tooltip.
  const short = refs.length > 1;
  return (
    <>
      <Seg
        className="quiet lmode"
        label="Listening level"
        value={ms.listen_same ? "same" : "real"}
        items={[
          { value: "real", label: <><span className="lbl-l">Real level</span><span className="lbl-s">Real</span></>, aria: "Real level", title: keysInText("The master as the exported file; the other sources at their own level. ⇧B") },
          {
            value: "same",
            className: ms.listen_same ? "same" : "",
            label: (
              <>
                <Icon name="level" size="sm" />
                <span className="lbl-l">Same loudness</span>
                <span className="lbl-s">Same</span>
              </>
            ),
            aria: "Same loudness",
            title: keysInText("Compare at the same loudness: quieter sources are raised, louder ones lowered. ⇧B"),
          },
        ]}
        onChange={(v) => void backend.masterListenSame(v === "same").catch(fail)}
      />
      <Seg
        label="A/B source"
        value={ms.listening === "proposal" ? "master" : ms.listening}
        items={[
          { value: "master", label: "Master" },
          { value: "mixdown", label: "Mixdown" },
          ...refs.map((r, i) => {
            const label = `Reference ${String.fromCharCode(65 + i)}`;
            const ab = `Ref ${String.fromCharCode(65 + i)}`;
            // The short label also in a narrow window (compact), where the full one would push the bar over.
            const text = short ? ab : (
              <>
                <span className="lbl-l">{label}</span>
                <span className="lbl-s">{ab}</span>
              </>
            );
            return { value: `reference:${r.id}`, label: text, aria: r.name, title: `${label} · ${r.name}` };
          }),
        ]}
        onChange={(v) => void backend.masterListen(v).catch(fail)}
      />
    </>
  );
}

/** The Listening section of the Loudness card (9l–9p). */
export function ListeningSection({ ms, gap }: { ms: MasterStatus; gap: TargetGap | null }) {
  const { rows, note } = listenRows(ms, gap);
  const same = ms.listen_same;
  return (
    <div className="sect" aria-label="Listening" role="group">
      <div className="fx between">
        <span className="fx g2">
          <span className="faint s1">Listening</span>
          {same ? (
            <span className="fx g1 b s1 acc" data-testid="listen-mode">
              <Icon name="level" size="sm" />
              Same loudness
            </span>
          ) : (
            <span className="b s1" data-testid="listen-mode">Real level</span>
          )}
        </span>
        <span className="faint s1">
          <span className="kbd">{keysInText("⇧B")}</span> {same ? "real level" : "same loudness"}
        </span>
      </div>
      <div className="lrows" role="list">
        {rows.map((r) => {
          const on = r.value === ms.listening;
          return (
            <div key={r.value} className={`lrow ${on ? "on" : ""}`} role="listitem" data-testid={`lrow-${r.name}`} aria-current={on || undefined}>
              <span className="fx g1 ell">
                {on ? <Icon name="speaker" size="sm" /> : <i className="sp-ic" />}
                <span className="ell">{r.name}</span>
              </span>
              <span className="r">{r.lufs == null ? "—" : `${f1(r.lufs)} LUFS`}</span>
              <span className={`r ${r.warn ? "warnc" : ""}`}>
                {r.warn && <Icon name="warn" size="sm" />}
                {r.offset}
              </span>
            </div>
          );
        })}
      </div>
      {note && <span className="faint s1" data-testid="trim-note">{note}</span>}
    </div>
  );
}

/** The warning or the note about the destination, with its two actions (9q, 9r, 9u, 10d), or the kept line (9t). */
export function TargetNotice({ ms, where }: { ms: MasterStatus; where: "card" | "export" }) {
  const edit = useMasterEdit();
  const target = useApp((s) => s.project?.master.target);
  const { gap, kept } = useTargetGap(ms);
  if (!gap || !target) return null;
  const w = gapWords(gap, target.lufs, target.true_peak_db);
  const match = () =>
    edit("Target matched to the references", (x) => {
      x.target = { preset: "custom", lufs: Math.round(gap.edge.lufs * 10) / 10, true_peak_db: x.target.true_peak_db };
    });
  const keep = () => edit("Kept the target", (x) => void (x.target_kept = true));
  if (kept) {
    if (where === "export") return null;
    return (
      <div className="kept" data-testid="target-kept">
        <Icon name="info" size="sm" />
        <span className="grow">
          {f1(gap.lu)} LU {gap.kind} than {gap.edge.label} · kept on purpose.{" "}
          <button type="button" className="link" onClick={() => edit("Warn me again about the target", (x) => void (x.target_kept = false))}>
            Warn me again
          </button>
        </span>
      </div>
    );
  }
  if (where === "export") {
    // 10d: only the quieter case reaches the export; a loud master is heard at once and shown by the limiter.
    if (gap.kind !== "quieter") return null;
    return (
      <div className="banner" role="alert" data-testid="export-target-warning">
        <Icon name="warn" />
        <div className="col grow">
          <span className="b">This master will be {f1(gap.lu)} LU quieter than your reference{gap.refs.length > 1 ? "s" : ""}</span>
          <span className="dim s1">
            The target is {target.preset === "custom" ? "Custom " : ""}
            {ft(target.lufs)} LUFS; {gap.edge.label} plays at {f1(gap.edge.lufs)} LUFS. Next to similar songs it will sound clearly softer. Matching sets {ft(gap.edge.lufs)} LUFS and keeps the {ft(target.true_peak_db)} dBTP ceiling.
          </span>
        </div>
        <button type="button" className="btn sm" onClick={match}>{w.fix}</button>
        <button type="button" className="btn quiet sm" onClick={keep}>{w.keep}</button>
      </div>
    );
  }
  const quieter = gap.kind === "quieter";
  return (
    <div className={`banner stack ${quieter ? "" : "info"}`} role={quieter ? "alert" : "note"} data-testid="target-warning">
      <div className="fx g2">
        <Icon name={quieter ? "warn" : "info"} />
        <div className="col g1 grow">
          <span className="b">{w.title}</span>
          <span className="dim s1">{w.text}</span>
        </div>
      </div>
      <div className="fx g2 wrap">
        <button type="button" className="btn primary sm" onClick={match}>{w.fix}</button>
        <button type="button" className="btn quiet sm" onClick={keep}>{w.keep}</button>
      </div>
      <span className="faint s1">{w.foot}</span>
    </div>
  );
}

/** The export report's line about the gap (10e): amber, or neutral after "Keep". */
export function ReportLevelLine({ ms, rep }: { ms: MasterStatus; rep: FileReport[] }) {
  const kept = useApp((s) => s.project?.master.target_kept ?? false);
  const lufs = rep.find((r) => r.lufs != null)?.lufs;
  if (lufs == null) return null;
  const gap = targetGap(lufs, refLevels(ms));
  if (!gap || gap.kind !== "quieter") return null;
  if (kept)
    return (
      <div className="tile row" data-testid="report-level">
        <Icon name="info" />
        <span className="dim s1">Exported at {f1(lufs)} LUFS on purpose, {f1(gap.lu)} LU quieter than {gap.edge.label}.</span>
      </div>
    );
  return (
    <div className="tile row" data-testid="report-level">
      <Icon name="warn" className="warnc" />
      <span className="dim s1">
        <span className="b">{f1(gap.lu)} LU quieter than {gap.edge.label}.</span> The files are at {f1(lufs)} LUFS; the reference plays at {f1(gap.edge.lufs)} LUFS. If that wasn’t intended, choose “{gapWords(gap, lufs, 0).fix}” in the Loudness card and export again.
      </span>
    </div>
  );
}
