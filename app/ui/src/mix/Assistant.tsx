// Assistant column (design 3a, 3b, 3c): Notes (brief, the current proposal's summary and numbered notes, the request
// field), History (HIS-1: readable steps, click to go back), Snapshots (HIS-4). The AI calls themselves are wired in
// F7 through `onAutoMix`/`onAsk`; this column only shows and edits the document.
import { useEffect, useState, type ReactNode } from "react";
import type { HistoryEntry } from "../bindings/HistoryEntry";
import { Icon } from "../shared/Icon";
import { Seg } from "../shared/controls";
import { useApp, useBackend } from "../state/context";

function HistoryTab() {
  const backend = useBackend();
  const revision = useApp((s) => s.revision);
  const canUndo = useApp((s) => s.canUndo);
  const canRedo = useApp((s) => s.canRedo);
  const setError = useApp((s) => s.setError);
  const [h, setH] = useState<[HistoryEntry[], number]>([[], 0]);
  useEffect(() => {
    let alive = true;
    backend.history("mix").then((x) => alive && setH(x)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [backend, revision]);
  const [entries, pos] = h;
  const fail = (e: unknown) => setError(String(e));
  const list = [...entries].reverse();
  return (
    <>
      <div className="sect grow scroll">
        {entries.length === 0 ? (
          <div className="empty">
            <Icon name="clock" />
            <span className="b">No steps yet</span>
            <span className="s1">Every change you or the assistant make will be listed here.</span>
          </div>
        ) : (
          <span className="faint s1">Every step can be undone. Click a step to go back to it.</span>
        )}
        <div className="col" role="list" aria-label="History">
          {list.map((e) => {
            const now = e.index === pos;
            return (
              <button
                type="button"
                role="listitem"
                key={e.index}
                className={`mi ${now ? "is-hover" : ""} ${e.undone ? "is-dis" : ""}`}
                aria-current={now ? "step" : undefined}
                onClick={() => void backend.historyGoto("mix", e.index).catch(fail)}
              >
                {e.ai_call ? <Icon name="spark" size="m" className="aic" /> : <span className="chk">{now && <Icon name="right" size="m" />}</span>}
                <span className="grow ell">{e.label}</span>
                <span className="s1 faint">{e.index}</span>
              </button>
            );
          })}
          <button type="button" className={`mi ${pos === 0 ? "is-hover" : ""}`} onClick={() => void backend.historyGoto("mix", 0).catch(fail)}>
            <span className="chk">{pos === 0 && <Icon name="right" size="m" />}</span>
            <span className="grow ell faint">Project opened</span>
          </button>
        </div>
      </div>
      <div className="foot">
        <div className="fx g2">
          <button type="button" className="btn" disabled={!canUndo} onClick={() => void backend.undo("mix").catch(fail)}>
            <Icon name="undo" size="m" />
            Undo
          </button>
          <button type="button" className="btn" disabled={!canRedo} onClick={() => void backend.redo("mix").catch(fail)}>
            <Icon name="redo" size="m" />
            Redo
          </button>
        </div>
        <span className="faint s1">Mix and Master keep separate histories. Mute, solo, zoom and selection are not recorded.</span>
      </div>
    </>
  );
}

function NotesTab({ footer }: { footer?: ReactNode }) {
  const project = useApp((s) => s.project)!;
  const selTrack = useApp((s) => s.selTrack);
  const setView = useApp((s) => s.setView);
  const b = project.brief;
  const tags = [b.genre, ...b.mood.split(",")].map((x) => x.trim()).filter(Boolean);
  const notes = project.ai.notes;
  const hasAudio = project.tracks.some((t) => t.kind === "audio");
  if (!hasAudio) {
    // 13a: the assistant needs tracks to measure.
    return (
      <>
        <div className="empty grow" data-testid="assistant-empty">
          <Icon name="spark" />
          <span className="b">Add tracks first</span>
          <span className="s1">The assistant works from measurements of your tracks, their roles and your brief.</span>
        </div>
        {footer}
      </>
    );
  }
  if (!tags.length && !b.notes && !project.references.length && !notes.length && !project.ai.summary) {
    // 13b: no brief, no references.
    return (
      <>
        <div className="sect grow">
          <div className="empty">
            <Icon name="pen" />
            <span className="b">Tell the assistant what you are after</span>
            <span className="s1">A genre, a mood and a note or two. References are optional.</span>
            <button type="button" className="btn primary" onClick={() => setView({ projectOpen: true })}>Write the brief</button>
          </div>
          <span className="hint faint s1">Auto mix stays available: it runs with the measurements alone.</span>
        </div>
        {footer}
      </>
    );
  }
  return (
    <>
      <div className="sect">
        <div className="fx g1 wrap">
          {tags.map((t) => (
            <span key={t} className="tag">{t}</span>
          ))}
          {project.references.map((r) => (
            <button key={r.id} type="button" className="tag" title="Reference · open the project settings to change it" onClick={() => setView({ projectOpen: true })}>
              <Icon name="music" size="sm" />
              <span className="ell">{r.media.original_name}</span>
            </button>
          ))}
          {!tags.length && <span className="faint s1">No brief yet.</span>}
          <button type="button" className="tag" onClick={() => setView({ projectOpen: true })}>
            <Icon name="pen" size="sm" />
            Edit brief
          </button>
        </div>
        {b.notes && <p className="dim">{b.notes}</p>}
      </div>
      {project.ai.summary && (
        <div className="sect">
          <span className="b">Last applied proposal</span>
          <p className="why dim selectable">{project.ai.summary}</p>
        </div>
      )}
      <div className="sect grow scroll" aria-label="Assistant notes">
        {notes.map((n) => {
          const t = project.tracks.find((x) => x.id === n.track);
          const on = !!t && t.id === selTrack;
          return (
            <button type="button" key={n.number} className={`note ${on ? "on" : ""} t${t?.color || 1}`} onClick={() => t && setView({ selTrack: t.id })}>
              <span className="badge">{n.number}</span>
              <div className="col g1 grow">
                <span className="fx g2">
                  {on && <i className="tdot" />}
                  <span className="b">{t?.name ?? "Whole mix"}</span>
                </span>
                <span className="why dim">{n.text}</span>
              </div>
            </button>
          );
        })}
        {!notes.length && (
          <div className="empty">
            <Icon name="spark" />
            <span>The assistant's notes appear here after an auto mix: one per track it changed, with the reason.</span>
          </div>
        )}
      </div>
      {footer}
    </>
  );
}

function SnapshotsTab() {
  const project = useApp((s) => s.project)!;
  return (
    <div className="sect grow">
      <span className="faint s1">Named states of the mix, saved in the project.</span>
      {project.snapshots.map((s) => (
        <div key={s.id} className="tile row">
          <div className="col grow">
            <span className="b ell">{s.name}</span>
          </div>
        </div>
      ))}
      {!project.snapshots.length && <span className="dim">No snapshots yet.</span>}
    </div>
  );
}

export function Assistant({ footer, snapshots, proposals, working }: { footer?: ReactNode; snapshots?: ReactNode; proposals?: ReactNode; working?: ReactNode }) {
  const tab = useApp((s) => s.asstTab);
  const setView = useApp((s) => s.setView);
  return (
    <aside className="card side asst" aria-label="Assistant">
      <div className={`ph ${proposals ? "rows" : ""}`}>
        <span className="h">Assistant</span>
        <div className="grow" />
        <Seg
          className="quiet sm"
          label="Assistant view"
          value={tab}
          items={[
            { value: 0, label: "Notes" },
            { value: 1, label: "History" },
            { value: 2, label: "Snapshots" },
            ...(proposals ? [{ value: 3, label: "Proposals" }] : []),
          ]}
          onChange={(v) => setView({ asstTab: v })}
        />
      </div>
      {working ?? (
        <>
          {tab === 0 && <NotesTab footer={footer} />}
          {tab === 1 && <HistoryTab />}
          {tab === 2 && (snapshots ?? <SnapshotsTab />)}
          {tab === 3 && proposals}
        </>
      )}
    </aside>
  );
}
