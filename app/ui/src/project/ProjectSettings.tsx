// Project settings (PRJ-1, PRJ-7): what was chosen when the project was created and can change later — the name, the
// brief, the references — with the facts that cannot change (sample rate, location). Opened from the project name in
// the top bar of either phase, and from the assistant's brief.
import { useEffect, useRef, useState } from "react";
import type { Project } from "../bindings/Project";
import { Icon } from "../shared/Icon";
import { formatClock } from "../shared/format";
import { useApp, useBackend } from "../state/context";

export const MAX_REFERENCES = 3;
/** Formats a reference may have: it is decoded and converted to the project rate when added. */
export const REFERENCE_EXT = ["wav", "flac", "aif", "aiff", "mp3", "m4a", "aac"];
export const isReferenceFile = (p: string) => REFERENCE_EXT.includes(p.split(".").pop()?.toLowerCase() ?? "");

/** Whether a drop at (x, y) lands on the element. */
export function inside(el: HTMLElement | null, x?: number, y?: number) {
  if (!el || x === undefined || y === undefined) return false;
  const r = el.getBoundingClientRect();
  return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
}

/** The references of the project, or the ones chosen for a project not created yet: list, add (button or drop), remove. */
export function ReferenceList({
  items,
  onAdd,
  onRemove,
  over,
  dropRef,
}: {
  items: { key: string; name: string; detail: string }[];
  onAdd: () => void;
  onRemove: (key: string) => void;
  over: boolean;
  dropRef: React.RefObject<HTMLButtonElement | null>;
}) {
  return (
    <div className="col g2" data-testid="references">
      {items.map((r) => (
        <div key={r.key} className="tile row">
          <Icon name="music" />
          <div className="col grow">
            <span className="b ell">{r.name}</span>
            <span className="faint s1">{r.detail}</span>
          </div>
          <button type="button" className="ib" aria-label={`Remove ${r.name}`} onClick={() => onRemove(r.key)}>
            <Icon name="x" />
          </button>
        </div>
      ))}
      {items.length < MAX_REFERENCES && (
        <button ref={dropRef} type="button" className={`drop ${over ? "is-over" : ""}`} style={{ height: "var(--sp-8)" }} onClick={onAdd} data-testid="reference-drop">
          <span className="dim s1">
            <Icon name="plus" size="sm" /> {over ? "Drop to add as a reference" : "Add a finished song you like · click or drop · WAV, FLAC, AIFF, MP3, AAC"}
          </span>
        </button>
      )}
      <span className="faint s1">References are copied into the project and used only for analysis and comparison by ear. They are never sent anywhere.</span>
    </div>
  );
}

const refDetail = (r: Project["references"][number]) => `${r.media.format.toUpperCase()} · ${formatClock(r.media.frames / Math.max(1, r.media.sample_rate))}`;

export function ProjectSettings({ onClose }: { onClose: () => void }) {
  const backend = useBackend();
  const project = useApp((s) => s.project)!;
  const path = useApp((s) => s.path);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const [name, setName] = useState(project.name);
  const [b, setB] = useState(project.brief);
  const [over, setOver] = useState(false);
  const drop = useRef<HTMLButtonElement>(null);
  const fail = (e: unknown) => setError(String(e));
  const full = project.references.length >= MAX_REFERENCES;
  const addPaths = async (paths: string[]) => {
    const files = paths.filter(isReferenceFile);
    if (files.length < paths.length) toast("warn", "Not an audio file", "References can be WAV, FLAC, AIFF, MP3 or AAC.");
    const room = MAX_REFERENCES - project.references.length;
    if (files.length > room) toast("warn", `At most ${MAX_REFERENCES} references`, `${files.length - room} left out.`);
    for (const f of files.slice(0, Math.max(0, room))) await backend.addReference(f).catch(fail);
  };
  useEffect(
    () =>
      backend.onFileDrop((e) => {
        const on = !full && inside(drop.current, e.x, e.y);
        setOver(e.type === "over" && on);
        if (e.type === "drop" && on) void addPaths(e.paths);
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [backend, full, project.references.length],
  );
  const save = async () => {
    try {
      if (name.trim() && name.trim() !== project.name) await backend.edit({ op: "rename_project", name: name.trim() }, `Rename project to ${name.trim()}`);
      const p = project.brief;
      if (b.genre !== p.genre || b.mood !== p.mood || b.notes !== p.notes) await backend.edit({ op: "set_brief", brief: b }, "Brief");
      onClose();
    } catch (e) {
      fail(e);
    }
  };
  const rate = project.sample_rate;
  const tracks = project.tracks.filter((t) => t.kind === "audio").length;
  return (
    <div className="scrim">
      <div className="dialog" role="dialog" aria-modal="true" aria-label="Project settings" onKeyDown={(e) => e.key === "Escape" && onClose()}>
        <div className="dlg-h">
          <span className="h l">Project settings</span>
          <span className="dim text">The name, what the assistant should aim for, and the songs to compare with.</span>
        </div>
        <div className="dlg-b scroll">
          <label className="flabel">
            <span className="l">Name</span>
            <input className="input sm" aria-label="Project name" value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <div className="mtable" style={{ gridTemplateColumns: "auto 1fr" }}>
            <span className="dim">Sample rate</span>
            <span className="b">{rate ? `${rate / 1000} kHz · set by the first file` : "set by the first file you import"}</span>
            <span className="dim">Tracks</span>
            <span className="b">{tracks} audio track{tracks === 1 ? "" : "s"}</span>
            <span className="dim">Location</span>
            <span className="b ell selectable" title={path}>{path || "—"}</span>
          </div>
          <span className="b">Brief</span>
          <div className="fx g3">
            <label className="flabel grow">
              <span className="l">Genre</span>
              <input className="input sm" placeholder="e.g. indie rock" value={b.genre} onChange={(e) => setB({ ...b, genre: e.target.value })} />
            </label>
            <label className="flabel grow">
              <span className="l">Mood</span>
              <input className="input sm" placeholder="e.g. warm, intimate" value={b.mood} onChange={(e) => setB({ ...b, mood: e.target.value })} />
            </label>
          </div>
          <label className="flabel">
            <span className="l">Notes</span>
            <textarea className="input area" rows={3} placeholder="Anything that matters to you" value={b.notes} onChange={(e) => setB({ ...b, notes: e.target.value })} />
          </label>
          <div className="fx between">
            <span className="b">References</span>
            <span className="faint s1">{project.references.length} of {MAX_REFERENCES} · optional · added and removed at once (undo in the Mix history)</span>
          </div>
          <ReferenceList
            items={project.references.map((r) => ({ key: r.id, name: r.media.original_name, detail: refDetail(r) }))}
            onAdd={async () => {
              const p = await backend.pickReference();
              if (p) await backend.addReference(p).catch(fail);
            }}
            onRemove={(id) => void backend.removeReference(id).catch(fail)}
            over={over}
            dropRef={drop}
          />
        </div>
        <div className="dlg-f">
          <button type="button" className="btn quiet" onClick={onClose}>Cancel</button>
          <button type="button" className="btn primary" disabled={!name.trim()} onClick={() => void save()}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
