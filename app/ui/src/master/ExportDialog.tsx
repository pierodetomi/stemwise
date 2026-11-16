// Export (design 10a–10c; MAS-7 … MAS-10): formats (several at once), rate, bit depth, dither, metadata with ISRC
// and cover, folder; then progress and the report measured on the written files. Settings and metadata are part
// of the master state (one step each in the master history).
import { Fragment, useState } from "react";
import type { MasterStatus } from "../bindings/MasterStatus";
import { Icon } from "../shared/Icon";
import { useMenu } from "../shared/Menu";
import { baseName, parentDir } from "../shared/path";
import { useApp, useBackend } from "../state/context";
import { targetName, useMasterEdit } from "./bind";
import { ReportLevelLine, TargetNotice } from "./Listening";
import { TOUR_TEXT } from "../tour/text";

const n1 = (v: number | null | undefined) => (v === null || v === undefined || !Number.isFinite(v) ? "—" : v.toFixed(1).replace("-", "−"));
/** Display only, the same rule as the core's (which refuses an invalid ISRC on save, MAS-9): compact or fully
 *  hyphenated. */
const isrcShape = (s: string) => /^([A-Za-z]{2}[A-Za-z0-9]{3}\d{7}|[A-Za-z]{2}-[A-Za-z0-9]{3}-\d{2}-\d{5})$/.test(s.trim());
/** AudioToolbox encodes AAC up to 48 kHz (MAS-10: a format the encoder cannot write is not offered). */
const AAC_MAX_RATE = 48000;

export function ExportDialog({ ms, onClose }: { ms: MasterStatus; onClose: () => void }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const demo = useApp((s) => s.demo);
  const docked = useApp((s) => s.tour?.step === 14);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const edit = useMasterEdit();
  const menu = useMenu();
  const [started, setStarted] = useState(false);
  const m = project.master;
  const x = m.export;
  const md = m.metadata;
  const [meta, setMeta] = useState(md);
  const fail = (e: unknown) => setError(String(e));
  const job = ms.job?.kind === "export" ? ms.job : null;
  const aacOk = ms.aac && x.sample_rate <= AAC_MAX_RATE;
  const count = [x.wav, x.flac, x.aac && aacOk].filter(Boolean).length;
  const rates = [44100, 48000, 88200, 96000];
  const pick = (e: React.MouseEvent<HTMLElement>, items: [string, () => void, boolean][], label: string) =>
    menu.open(e.currentTarget, items.map(([l, f, on]) => ({ label: l, checked: on, onSelect: f })), label);
  const saveMeta = (key: keyof typeof meta, v: string) => {
    if (md[key] === v) return;
    // The core refuses an invalid ISRC; the field keeps it with its message until it is fixed.
    if (key === "isrc" && v.trim() && !isrcShape(v)) return;
    edit(`Metadata · ${key}`, (q) => ((q.metadata as unknown as Record<string, string>)[key] = v));
  };

  if (job) {
    return (
      <div className="scrim">
        <div className="dialog" role="dialog" aria-modal="true" aria-label="Exporting">
          <div className="dlg-h">
            <span className="h l">Exporting</span>
            <span className="dim text">Rendering the master once, then writing each format and measuring it back.</span>
          </div>
          <div className="dlg-b">
            <div className="progress" role="progressbar" aria-valuenow={Math.round(job.progress * 100)}><i style={{ width: `${Math.round(job.progress * 100)}%` }} /></div>
            <span className="dim">{job.progress < 0.5 ? "Rendering the master at the destination's loudness" : "Writing the files"}</span>
          </div>
          <div className="dlg-f"><button type="button" className="btn" onClick={() => void backend.masterJobCancel()}>Cancel</button></div>
        </div>
      </div>
    );
  }
  if (started && ms.last_export) {
    const rep = ms.last_export;
    const text = rep.map((r) => `${baseName(r.path)}\t${n1(r.lufs)} LUFS\t${n1(r.true_peak_db)} dBTP\t${n1(r.lra)} LU`).join("\n");
    return (
      <div className="scrim">
        <div className="dialog l" role="dialog" aria-modal="true" aria-label="Export complete">
          <div className="dlg-h">
            <span className="h l">Export complete</span>
            <span className="dim text">{rep.length} file{rep.length === 1 ? "" : "s"} written to {rep[0] ? parentDir(rep[0].path) : ""}.</span>
          </div>
          <div className="dlg-b">
            <div className="mtable" style={{ gridTemplateColumns: "2fr 1fr 1fr 1fr 1fr 1fr" }} role="table" aria-label="Export report">
              <span className="th">File</span><span className="th r">Loudness</span><span className="th r">True peak</span><span className="th r">Range</span><span className="th r">To reference</span><span className="th">Format</span>
              {rep.map((r) => (
                <Fragment key={r.path}>
                  <span className="b ell">{baseName(r.path)}</span>
                  <span className="r">{n1(r.lufs)} LUFS</span>
                  <span className="r">{n1(r.true_peak_db)} dBTP</span>
                  <span className="r">{n1(r.lra)} LU</span>
                  <span className="r">{n1(r.distance_db)}</span>
                  <span className="dim ell">{r.format}</span>
                </Fragment>
              ))}
            </div>
            <ReportLevelLine ms={ms} rep={rep} />
            <div className="tile row">
              <Icon name="info" />
              <span className="dim s1">True peak measured with 4x oversampling on each written file, against the {n1(m.target.true_peak_db)} dBTP ceiling. Lossy files over it are encoded again with the margin they need.</span>
            </div>
          </div>
          <div className="dlg-f">
            <button type="button" className="btn quiet" onClick={() => void navigator.clipboard?.writeText(text).then(() => toast("ok", "Copied", ""))}>
              <Icon name="copy" size="m" />
              Copy report
            </button>
            <span className="grow" />
            <button type="button" className="btn primary" onClick={onClose}>Done</button>
          </div>
        </div>
      </div>
    );
  }
  const row = (on: boolean, name: string, sub: string, toggle: () => void, opts: React.ReactNode, disabled = false) => (
    <div className={`tile row ${on ? "on" : ""} ${disabled ? "is-dis" : ""}`}>
      <button type="button" role="checkbox" aria-checked={on} aria-label={name} disabled={disabled} className={`check ${on ? "on" : ""}`} onClick={toggle}>
        <Icon name="check" />
      </button>
      <div className="col grow"><span className="b">{name}</span><span className="faint s1">{sub}</span></div>
      {opts}
    </div>
  );
  const sel = (label: string, v: string, items: [string, () => void, boolean][]) => (
    <button type="button" className="select" aria-label={label} onClick={(e) => pick(e, items, label)}>
      <span className="ell">{v}</span>
      <Icon name="chev" size="m" />
    </button>
  );
  const rateSel = sel("Sample rate", `${x.sample_rate / 1000} kHz`, rates.map((r) => [`${r / 1000} kHz`, () => edit("Export rate", (q) => (q.export.sample_rate = r)), x.sample_rate === r]));
  return (
    <div className={`scrim ${docked ? "tour-dock" : ""}`}>
      <div className="dialog l" role="dialog" aria-modal="true" aria-label="Export master" onKeyDown={(e) => e.key === "Escape" && onClose()}>
        <div className="dlg-h">
          <span className="h l">Export master</span>
          <span className="dim text">{targetName(m)}. One export can write several formats.</span>
        </div>
        <div className="dlg-b">
          <TargetNotice ms={ms} where="export" />
          <div className="fx g5 top-al">
            <div className="col g2 grow">
              <span className="b">Formats</span>
              {row(x.wav, "WAV", "Uncompressed", () => edit("Export WAV", (q) => (q.export.wav = !x.wav)), <>{sel("WAV bit depth", x.bit_depth === 32 ? "32-bit float" : `${x.bit_depth}-bit`, [16, 24, 32].map((b) => [b === 32 ? "32-bit float" : `${b}-bit`, () => edit("Export bit depth", (q) => (q.export.bit_depth = b)), x.bit_depth === b]))}{rateSel}</>)}
              {row(x.flac, "FLAC", `Lossless, smaller · ${Math.min(24, x.bit_depth)}-bit`, () => edit("Export FLAC", (q) => (q.export.flac = !x.flac)), rateSel)}
              {row(x.aac && aacOk, "AAC", !ms.aac ? "Not available on this system" : aacOk ? "M4A, for sharing" : "Up to 48 kHz: choose 44.1 or 48 kHz", () => edit("Export AAC", (q) => (q.export.aac = !x.aac)), sel("AAC bit rate", `${x.aac_kbps} kbps`, [128, 192, 256, 320].map((k) => [`${k} kbps`, () => edit("Export AAC bit rate", (q) => (q.export.aac_kbps = k)), x.aac_kbps === k])), !aacOk)}
              <div className="tile row">
                <Icon name="info" />
                <span className="dim s1">
                  {m.chain.dither.bypass ? "Dither is off." : "Dither (TPDF) is added to 16-bit files only."} Files at another rate than the project's are converted at high quality before mastering. The true peak of the AAC is checked on the decoded file.
                </span>
              </div>
              <label className="flabel">
                <span className="l">Folder</span>
                <div className="fx g2">
                  <span className="input sm grow" title="Files with the same name in this folder are replaced"><span className="ell">{x.folder ?? "masters/ inside the project"}</span></span>
                  <button type="button" className="btn" onClick={async () => {
                    const f = await backend.pickFolder("Export folder");
                    if (f) edit("Export folder", (q) => (q.export.folder = f));
                  }}>Choose…</button>
                </div>
              </label>
            </div>
            <div className="col g2" style={{ width: "var(--field-w-l)" }}>
              <span className="b">Metadata</span>
              <div className="fx g3">
                <div className="cover" role="img" aria-label={md.cover ? "Cover art" : "No cover"} />
                <div className="col g1">
                  <button type="button" className="btn sm" onClick={async () => {
                    const p = await backend.pickImage();
                    if (p) await backend.setCover(p).catch(fail);
                  }}>
                    <Icon name="image" size="m" />
                    {md.cover ? "Replace cover…" : "Add cover…"}
                  </button>
                  <span className="faint s1">JPEG or PNG, square</span>
                </div>
              </div>
              {(["title", "artist", "album"] as const).map((k) => (
                <label key={k} className="flabel">
                  <span className="l">{k[0].toUpperCase() + k.slice(1)}</span>
                  <input className="input sm" value={meta[k]} onChange={(e) => setMeta({ ...meta, [k]: e.target.value })} onBlur={(e) => saveMeta(k, e.target.value)} />
                </label>
              ))}
              <div className="fx g2">
                <label className="flabel grow"><span className="l">Year</span><input className="input sm" value={meta.year} onChange={(e) => setMeta({ ...meta, year: e.target.value })} onBlur={(e) => saveMeta("year", e.target.value)} /></label>
                <label className="flabel grow"><span className="l">Track</span><input className="input sm" value={meta.track_number} onChange={(e) => setMeta({ ...meta, track_number: e.target.value })} onBlur={(e) => saveMeta("track_number", e.target.value)} /></label>
              </div>
              <label className="flabel"><span className="l">Genre</span><input className="input sm" value={meta.genre} onChange={(e) => setMeta({ ...meta, genre: e.target.value })} onBlur={(e) => saveMeta("genre", e.target.value)} /></label>
              <label className="flabel">
                <span className="l">ISRC</span>
                <input className={`input sm ${meta.isrc && !isrcShape(meta.isrc) ? "is-err" : ""}`} placeholder="CC-XXX-YY-NNNNN" value={meta.isrc} onChange={(e) => setMeta({ ...meta, isrc: e.target.value })} onBlur={(e) => saveMeta("isrc", e.target.value)} />
              </label>
              {meta.isrc && (
                <span className={`s1 ${isrcShape(meta.isrc) ? "okc" : "dangerc"}`} data-testid="isrc-state">
                  <Icon name={isrcShape(meta.isrc) ? "check" : "warn"} size="sm" /> {isrcShape(meta.isrc) ? "Valid ISRC format" : "Not an ISRC: CC-XXX-YY-NNNNN"}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="dlg-f">
          <span className="faint s1 grow">{demo ? TOUR_TEXT.exportFoot : `${count} file${count === 1 ? "" : "s"}`}</span>
          <button type="button" className="btn quiet" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="btn primary"
            disabled={demo || !count || !ms.mixdown_measures || ms.analysing}
            title={demo ? TOUR_TEXT.exportOff : undefined}
            onClick={() => {
              setStarted(true);
              void backend.exportStart().catch(fail);
            }}
          >
            <Icon name="exp" size="m" />
            {ms.analysing ? "Measuring…" : `Export ${count} file${count === 1 ? "" : "s"}`}
          </button>
        </div>
      </div>
      {menu.node}
    </div>
  );
}
