// Snapshots (design 3c; HIS-4): named states of the mix saved in the project; restoring one is one undoable step;
// two states compare by ear while playing, without a history step.
import { shortcutOf } from "../shared/shortcuts";
import { useEffect, useState } from "react";
import { Icon } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useApp, useBackend } from "../state/context";

const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

export function Snapshots() {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const listening = useApp((s) => s.ai?.listening ?? "current");
  const proposal = useApp((s) => !!s.ai?.proposal);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const [naming, setNaming] = useState<string | null>(null);
  const snaps = [...project.snapshots].reverse();
  const [picked, setB] = useState<string | null>(null);
  // B is the snapshot picked, or the newest one.
  const b = picked && project.snapshots.some((s) => s.id === picked) ? picked : (snaps[0]?.id ?? null);
  const fail = (e: unknown) => setError(String(e));
  const armed = project.snapshots.find((s) => s.id === b) ?? null;
  const onB = listening.startsWith("snapshot:");
  const listen = (toB: boolean) => void backend.aiListen(toB && armed ? `snapshot:${armed.id}` : "current").catch(fail);
  useEffect(() => {
    if (proposal) return;
    const k = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input, textarea")) return;
      if (shortcutOf(e) === "ab" && armed) listen(!onB);
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  });
  const save = () => {
    const name = (naming ?? "").trim() || `Snapshot ${project.snapshots.length + 1}`;
    setNaming(null);
    void backend.snapshotSave(name).then(() => toast("ok", "Snapshot saved", `“${name}” is in Snapshots.`)).catch(fail);
  };
  return (
    <>
      <div className="sect grow scroll">
        <div className="fx between">
          <span className="faint s1">Named states of the mix, saved in the project.</span>
          <button type="button" className="btn sm" onClick={() => setNaming(`Snapshot ${project.snapshots.length + 1}`)}>
            <Icon name="plus" size="m" />
            Save snapshot
          </button>
        </div>
        {naming !== null && (
          <div className="fx g2">
            <input
              className="input sm grow"
              autoFocus
              aria-label="Snapshot name"
              value={naming}
              onChange={(e) => setNaming(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") save();
                if (e.key === "Escape") setNaming(null);
              }}
            />
            <button type="button" className="btn primary sm" onClick={save}>Save</button>
          </div>
        )}
        <div className="tile row on">
          <div className="col grow">
            <span className="b ell">Current mix</span>
            <span className="faint s1">now</span>
          </div>
          <span className="tag acc">A</span>
        </div>
        {snaps.map((s) => (
          <div key={s.id} className={`tile row ${b === s.id ? "on" : ""}`} data-testid={`snapshot-${s.name}`}>
            <button type="button" className="col grow" onClick={() => setB(s.id)} aria-pressed={b === s.id} aria-label={`Compare with ${s.name}`}>
              <span className="b ell">{s.name}</span>
              <span className="faint s1">{hhmm(s.created_ms)}</span>
            </button>
            {b === s.id ? (
              <span className="tag acc">B</span>
            ) : null}
            <button
              type="button"
              className="btn quiet sm"
              onClick={() =>
                setView({
                  dialog: {
                    title: `Restore “${s.name}”?`,
                    sub: "The mix goes back to that snapshot as one step. You can undo it.",
                    confirm: "Restore",
                    onConfirm: () => void backend.snapshotRestore(s.id).catch(fail),
                  },
                })
              }
            >
              Restore
            </button>
            <button type="button" className="ib" aria-label={`Delete snapshot ${s.name}`} onClick={() => void backend.snapshotDelete(s.id).catch(fail)}>
              <Icon name="x" size="m" />
            </button>
          </div>
        ))}
        {!snaps.length && (
          <div className="empty">
            <Icon name="copy" />
            <span className="b">No snapshots</span>
            <span className="s1">Save the mix as it is now, keep working, and compare the two by ear.</span>
          </div>
        )}
      </div>
      <div className="sect">
        <span className="b">Compare while listening</span>
        <Seg
          label="A/B"
          value={onB ? 1 : 0}
          items={[
            { value: 0, label: "A · Current mix" },
            { value: 1, label: `B · ${armed?.name ?? "pick a snapshot"}` },
          ]}
          onChange={(v) => listen(v === 1)}
        />
        <span className="faint s1">
          Switching does not stop playback and is not an undo step. Shortcut <span className="kbd">B</span>
        </span>
      </div>
      <div className="foot">
        <span className="faint s1">Restoring a snapshot is one undoable step.</span>
      </div>
    </>
  );
}
