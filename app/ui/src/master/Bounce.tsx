// Bounce to Master (design 8a–8c; MDN-1 … MDN-4): an explicit step. Confirm, progress (offline, cancellable),
// result with the mixdown's measures; "Open Master" is the moment the phase changes.
import { useEffect, useState } from "react";
import { formatClock } from "../shared/format";
import { useApp, useBackend } from "../state/context";
import { songFrames } from "../mix/geometry";

const fmt = (v: number | null | undefined, unit: string) => (v === null || v === undefined || !Number.isFinite(v) ? "—" : `${v.toFixed(1).replace("-", "−")} ${unit}`);

export function BounceDialog({ onClose }: { onClose: () => void }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const ms = useApp((s) => s.masterStatus);
  const setError = useApp((s) => s.setError);
  const [started, setStarted] = useState(false);
  const job = ms?.job?.kind === "bounce" ? ms.job : null;
  const rate = project.sample_rate ?? 48000;
  const secs = songFrames(project) / rate;
  const fail = (e: unknown) => setError(String(e));
  const done = started && !job && ms?.bounced;
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && !job && onClose();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [job, onClose]);
  if (job) {
    const elapsed = Math.max(0.001, (Date.now() - job.started_ms) / 1000);
    const speed = (job.progress * secs) / elapsed;
    return (
      <div className="scrim">
        <div className="dialog s" role="dialog" aria-modal="true" aria-label="Bouncing the mix">
          <div className="dlg-h">
            <span className="h l">Bouncing the mix</span>
            <span className="dim text">Rendering offline{speed > 1 ? ` · ${Math.round(speed)}× real time` : ""}</span>
          </div>
          <div className="dlg-b">
            <div className="progress" role="progressbar" aria-valuenow={Math.round(job.progress * 100)}>
              <i style={{ width: `${Math.round(job.progress * 100)}%` }} />
            </div>
            <div className="fx between">
              <span className="dim">{formatClock(job.progress * secs)} of {formatClock(secs)}</span>
              <span className="b">{Math.round(job.progress * 100)} %</span>
            </div>
          </div>
          <div className="dlg-f">
            <button type="button" className="btn" onClick={() => void backend.masterJobCancel()}>Cancel</button>
          </div>
        </div>
      </div>
    );
  }
  if (done && ms?.bounced) {
    const b = ms.bounced;
    return (
      <div className="scrim">
        <div data-phase="master" style={{ display: "contents" }}>
          <div className="dialog" role="dialog" aria-modal="true" aria-label={`Mixdown ${b.number} is ready`}>
            <div className="dlg-h">
              <span className="h l">Mixdown {b.number} is ready</span>
              <span className="dim text">Saved in the project. These are its measures before any mastering.</span>
            </div>
            <div className="dlg-b">
              <div className="fx g2 stretch">
                <div className="stat"><span className="faint s1">Integrated loudness</span><span className="v" data-testid="bounce-lufs">{fmt(b.lufs, "LUFS")}</span></div>
                <div className="stat"><span className="faint s1">True peak</span><span className="v">{fmt(b.true_peak_db, "dBTP")}</span></div>
                <div className="stat"><span className="faint s1">Loudness range</span><span className="v">{fmt(b.lra, "LU")}</span></div>
              </div>
              <div className="mtable" style={{ gridTemplateColumns: "1fr auto" }}>
                <span className="dim">Headroom trim</span>
                <span className="r b">{b.trim_db < 0 ? `${b.trim_db.toFixed(1).replace("-", "−")} dB applied to reach −3 dBTP` : "none needed"}</span>
                <span className="dim">Length</span>
                <span className="r b">{formatClock(b.frames / rate)}</span>
              </div>
            </div>
            <div className="dlg-f">
              <button type="button" className="btn quiet" onClick={onClose}>Stay in Mix</button>
              <button
                type="button"
                className="btn primary"
                autoFocus
                onClick={() => {
                  onClose();
                  void backend.setPhase(true).catch(fail);
                }}
              >
                Open Master
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }
  return (
    <div className="scrim">
      <div className="dialog" role="dialog" aria-modal="true" aria-label="Bounce & go to Master">
        <div className="dlg-h">
          <span className="h l">Bounce &amp; go to Master</span>
          <span className="dim text">The mix is rendered offline to a file inside the project. Mastering works on that frozen mixdown, not on the live mix.</span>
        </div>
        <div className="dlg-b">
          <div className="mtable" style={{ gridTemplateColumns: "1fr auto" }}>
            <span className="dim">Length</span>
            <span className="r b">{formatClock(secs)}</span>
            <span className="dim">Format</span>
            <span className="r b">{rate / 1000} kHz · 32-bit float · stereo</span>
            <span className="dim">Headroom</span>
            <span className="r b">turned down only if above −3 dBTP</span>
          </div>
          <div className="tile row">
            <span className="dim s1">You can come back to Mix at any time. If the mix changes later, Master will tell you and offer to bounce again, keeping your master settings.</span>
          </div>
        </div>
        <div className="dlg-f">
          <button type="button" className="btn quiet" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="btn primary"
            autoFocus
            onClick={() => {
              setStarted(true);
              void backend.bounceStart().catch(fail);
            }}
          >
            Bounce &amp; go to Master
          </button>
        </div>
      </div>
    </div>
  );
}
