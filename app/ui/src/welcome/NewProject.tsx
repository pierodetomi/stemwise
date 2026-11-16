// New project / Import (design 2a–2d; PRJ-1, PRJ-3, PRJ-5, PRJ-6): drop area, one row per file with format, rate,
// channels, length and a proposed role to confirm; files at another rate need an explicit choice (convert a copy or
// leave out); the brief and up to three references (PRJ-7), chosen or dropped before the project exists and added
// right after it is created. The same screen adds files to an open project ("Audio track from file…").
import { useEffect, useRef, useState } from "react";
import type { ImportCandidate } from "../backend/types";
import type { Role } from "../bindings/Role";
import { Icon, Mark } from "../shared/Icon";
import { formatClock } from "../shared/format";
import { useMenu } from "../shared/Menu";
import { baseName, parentDir } from "../shared/path";
import { useApp, useBackend } from "../state/context";
import { ROLES, roleLabel } from "../mix/menus";
import { MAX_REFERENCES, ReferenceList, inside, isReferenceFile } from "../project/ProjectSettings";

let pending: string[] | null = null;
/** Files dropped on the welcome screen, handed to this screen. */
export function setPendingDrop(paths: string[]) {
  pending = paths;
}

type Row = ImportCandidate & { keep: boolean; convert: boolean; confirmed: boolean };

const fmt = (c: ImportCandidate) => `${c.format.toUpperCase()} · ${c.encoding.replace("PCM ", "")}`;

export function NewProject() {
  const backend = useBackend();
  const project = useApp((s) => s.project);
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const toast = useApp((s) => s.toast);
  const adding = !!project;
  const [name, setName] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [over, setOver] = useState(false);
  const [brief, setBrief] = useState({ genre: "", mood: "", notes: "" });
  const [rateAsk, setRateAsk] = useState<Row | null>(null);
  const [busy, setBusy] = useState(false);
  const [refs, setRefsState] = useState<string[]>([]);
  // The list as it is now, also between two additions in a row (chosen, then dropped, before React renders again).
  const refsNow = useRef<string[]>([]);
  const setRefs = (next: string[]) => {
    refsNow.current = next;
    setRefsState(next);
  };
  const [refOver, setRefOver] = useState(false);
  const refDrop = useRef<HTMLButtonElement>(null);
  const menu = useMenu();

  const addRefs = (paths: string[]) => {
    const files = paths.filter(isReferenceFile);
    if (files.length < paths.length) toast("warn", "Not an audio file", "References can be WAV, FLAC, AIFF, MP3 or AAC.");
    // The notice is given here, not inside a state update: React may run an update while it renders (A-098).
    const was = refsNow.current;
    const next = [...was, ...files.filter((f) => !was.includes(f))];
    if (next.length > MAX_REFERENCES) toast("warn", `At most ${MAX_REFERENCES} references`, `${next.length - MAX_REFERENCES} left out.`);
    setRefs(next.slice(0, MAX_REFERENCES));
  };

  const add = async (paths: string[]) => {
    if (!paths.length) return;
    try {
      // The rate the new files are compared with: the project's when adding, else the first file already listed.
      const first = rows.find((r) => !r.error)?.sample_rate ?? null;
      const c = await backend.importCandidates(paths, adding, adding ? null : first);
      setRows((r) => {
        const known = new Set(r.map((x) => x.path));
        return [...r, ...c.filter((x) => !known.has(x.path)).map((x) => ({ ...x, keep: !x.error, convert: false, confirmed: x.role !== "other" }))];
      });
      if (!name && !adding) {
        const folder = baseName(parentDir(paths[0]));
        if (folder && folder !== "Stems") setName(folder);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    if (pending) {
      const p = pending;
      pending = null;
      void add(p);
    }
    // A drop on the references box adds references; anywhere else, tracks.
    return backend.onFileDrop((e) => {
      const onRefs = !adding && inside(refDrop.current, e.x, e.y);
      setOver(e.type === "over" && !onRefs);
      setRefOver(e.type === "over" && onRefs);
      if (e.type === "drop") {
        if (onRefs) addRefs(e.paths);
        else void add(e.paths);
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend]);

  const firstRate = rows.find((r) => r.keep && !r.rate_mismatch)?.sample_rate ?? project?.sample_rate ?? null;
  const mismatched = rows.filter((r) => r.keep && r.rate_mismatch && !r.convert);
  const kept = rows.filter((r) => r.keep && !r.error);
  const canCreate = kept.length > 0 && (adding || name.trim().length > 0) && !busy;

  const create = async () => {
    const ask = mismatched[0];
    if (ask) {
      setRateAsk(ask);
      return;
    }
    setBusy(true);
    try {
      if (!adding) {
        const parent = await backend.pickFolder("Where to save the project");
        if (!parent) {
          setBusy(false);
          return;
        }
        await backend.createProject(parent, name.trim());
        if (brief.genre || brief.mood || brief.notes) await backend.edit({ op: "set_brief", brief }, "Brief");
      }
      await backend.importFiles(kept.map((r) => ({ path: r.path, role: r.role, name: r.name, convert: r.convert })));
      // After the tracks: a reference is converted to the project rate, which the first file sets.
      const failed: string[] = [];
      if (!adding) for (const r of refs) await backend.addReference(r).catch((e) => failed.push(`${baseName(r)}: ${String(e)}`));
      toast("ok", adding ? "Tracks added" : "Project created", `${kept.length} file${kept.length === 1 ? "" : "s"} imported${refs.length - failed.length ? `, ${refs.length - failed.length} reference${refs.length - failed.length === 1 ? "" : "s"}` : ""}. Your originals were not touched.`);
      if (failed.length) setError(`A reference could not be added (${failed.join("; ")}). Add it again from the project settings.`);
      setView({ screen: "workspace" });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const update = (path: string, f: Partial<Row>) => setRows((r) => r.map((x) => (x.path === path ? { ...x, ...f } : x)));
  const lengthOf = (r: Row) => formatClock(r.frames / Math.max(1, r.sample_rate));

  return (
    <>
      <header className="top">
        <Mark />
        <span className="h">{adding ? "Add audio tracks" : "New project"}</span>
        <div className="grow" />
        <button type="button" className="btn quiet" onClick={() => setView({ screen: adding ? "workspace" : "welcome" })}>
          Cancel
        </button>
        <button type="button" className={`btn primary ${canCreate ? "" : "is-dis"}`} disabled={!canCreate} onClick={() => void create()}>
          {adding ? "Add tracks" : "Create project"}
        </button>
      </header>
      <div className="body">
        <section className="card grow">
          <div className="ph">
            {adding ? (
              <span className="b grow">{project!.name}</span>
            ) : (
              <label className="flabel grow">
                <input className="input sm" placeholder="Project name" aria-label="Project name" value={name} onChange={(e) => setName(e.target.value)} />
              </label>
            )}
            <span className="faint s1 nowrap">{firstRate ? `${firstRate / 1000} kHz · set by the first file` : "Sample rate is set by the first file"}</span>
          </div>
          <div className="sect grow g3 scroll">
            {rows.length ? (
              <>
                <div className="fx between">
                  <div className="fx g2">
                    <span className="b">{kept.length} file{kept.length === 1 ? "" : "s"}</span>
                    <span className="faint s1">One track per file, each clip starts at zero. Roles are guessed from the file names: check them.</span>
                  </div>
                  <button type="button" className="btn sm" onClick={async () => add(await backend.pickAudioFiles())}>
                    <Icon name="plus" size="m" />
                    Add files…
                  </button>
                </div>
                {mismatched.map((r) => (
                  <div key={r.path} className="banner" role="alert">
                    <Icon name="warn" />
                    <span className="grow">
                      “{baseName(r.path)}” is {r.sample_rate / 1000} kHz; the project is {(firstRate ?? 48000) / 1000} kHz (set by the first file). It will not be resampled silently.
                    </span>
                    <button type="button" className="btn sm" onClick={() => update(r.path, { convert: true })}>
                      Convert a copy
                    </button>
                    <button type="button" className="btn quiet sm" onClick={() => update(r.path, { keep: false })}>
                      Remove
                    </button>
                  </div>
                ))}
                <div className="tbl files" role="table" aria-label="Files to import">
                  <span className="th" />
                  <span className="th">File</span>
                  <span className="th">Format</span>
                  <span className="th">Rate</span>
                  <span className="th">Channels</span>
                  <span className="th">Length</span>
                  <span className="th">Role</span>
                  <span className="th" />
                  {rows
                    .filter((r) => r.keep)
                    .map((r, i) => (
                      <FileRow key={r.path} r={r} i={i} length={lengthOf(r)} onRole={(e) =>
                        menu.open(e, ROLES.map((role) => ({ label: roleLabel(role), checked: r.role === role, onSelect: () => update(r.path, { role: role as Role, confirmed: true }) })), "Role")
                      } onRemove={() => update(r.path, { keep: false })} />
                    ))}
                </div>
              </>
            ) : (
              <div className={`drop grow ${over ? "is-over" : ""}`} data-testid="drop-area">
                <Icon name="wave" />
                <span className="h">{over ? "Drop to add these files" : "Drop your stems here"}</span>
                <span className="dim">WAV, FLAC or AIFF · mono or stereo · one file per track</span>
                {!over && (
                  <button type="button" className="btn lg" onClick={async () => add(await backend.pickAudioFiles())}>
                    <Icon name="folder" size="m" />
                    Choose files…
                  </button>
                )}
              </div>
            )}
          </div>
        </section>
        {!adding && (
          <aside className="card brief-side" aria-label="Brief">
            <div className="ph">
              <span className="h">Brief</span>
              <span className="faint s1">What the assistant should aim for</span>
            </div>
            <div className="sect g3">
              <label className="flabel">
                <span className="l">Genre</span>
                <input className="input sm" placeholder="e.g. indie rock" value={brief.genre} onChange={(e) => setBrief({ ...brief, genre: e.target.value })} />
              </label>
              <label className="flabel">
                <span className="l">Mood</span>
                <input className="input sm" placeholder="e.g. warm, intimate" value={brief.mood} onChange={(e) => setBrief({ ...brief, mood: e.target.value })} />
              </label>
              <label className="flabel">
                <span className="l">Notes</span>
                <textarea className="input area" placeholder="Anything that matters to you" rows={3} value={brief.notes} onChange={(e) => setBrief({ ...brief, notes: e.target.value })} />
              </label>
            </div>
            <div className="ph">
              <span className="h">References</span>
              <span className="faint s1">{refs.length} of {MAX_REFERENCES} · optional</span>
            </div>
            <div className="sect g2">
              <ReferenceList
                items={refs.map((r) => ({ key: r, name: baseName(r), detail: (r.split(".").pop() ?? "").toUpperCase() }))}
                onAdd={async () => {
                  const p = await backend.pickReference();
                  if (p) addRefs([p]);
                }}
                onRemove={(k) => setRefs(refsNow.current.filter((x) => x !== k))}
                over={refOver}
                dropRef={refDrop}
              />
            </div>
          </aside>
        )}
      </div>
      {rateAsk && (
        <div className="scrim">
          <div className="dialog s" role="dialog" aria-modal="true" aria-label="Different sample rate">
            <div className="dlg-h">
              <span className="h l">Different sample rate</span>
              <span className="dim text">
                “{baseName(rateAsk.path)}” is {rateAsk.sample_rate / 1000} kHz. This project runs at {(firstRate ?? 48000) / 1000} kHz.
              </span>
            </div>
            <RateChoice
              onCancel={() => setRateAsk(null)}
              onContinue={(convert) => {
                update(rateAsk.path, convert ? { convert: true } : { keep: false });
                setRateAsk(null);
              }}
            />
          </div>
        </div>
      )}
      {menu.node}
    </>
  );
}

function FileRow({ r, i, length, onRole, onRemove }: { r: Row; i: number; length: string; onRole: (e: React.MouseEvent) => void; onRemove: () => void }) {
  return (
    <>
      <span className={`t${(i % 12) + 1}`}>
        <i className="tdot" />
      </span>
      <span className="b ell" title={r.path}>{baseName(r.path)}</span>
      <span className="dim">{fmt(r)}</span>
      <span className={r.rate_mismatch && !r.convert ? "warnc b" : "dim"}>
        {r.sample_rate / 1000} kHz{r.rate_mismatch && !r.convert && <Icon name="warn" size="sm" />}
        {r.convert && " → converted"}
      </span>
      <span className="dim">{r.channels === 1 ? "Mono" : "Stereo"}</span>
      <span className="dim">{length}</span>
      <span>
        <button type="button" className={`select ${r.confirmed ? "" : "is-open"}`} aria-label={`Role of ${r.name}: ${roleLabel(r.role)}`} onClick={onRole}>
          <span className="ell">{roleLabel(r.role)}</span>
          <Icon name="chev" size="m" />
        </button>
      </span>
      <span>
        <button type="button" className="ib" aria-label={`Remove ${r.name}`} onClick={onRemove}>
          <Icon name="x" />
        </button>
      </span>
    </>
  );
}

function RateChoice({ onCancel, onContinue }: { onCancel: () => void; onContinue: (convert: boolean) => void }) {
  const [convert, setConvert] = useState(true);
  return (
    <>
      <div className="dlg-b" role="radiogroup" aria-label="What to do with this file">
        <button type="button" role="radio" aria-checked={convert} className={`tile row ${convert ? "on" : ""}`} onClick={() => setConvert(true)}>
          <span className={`check radio ${convert ? "on" : ""}`}><Icon name="check" /></span>
          <div className="col">
            <span className="b">Convert a copy to the project rate</span>
            <span className="dim s1">High-quality conversion, saved inside the project. Your original file is not changed.</span>
          </div>
        </button>
        <button type="button" role="radio" aria-checked={!convert} className={`tile row ${!convert ? "on" : ""}`} onClick={() => setConvert(false)}>
          <span className={`check radio ${!convert ? "on" : ""}`}><Icon name="check" /></span>
          <div className="col">
            <span className="b">Leave this file out</span>
            <span className="dim s1">Import the other files.</span>
          </div>
        </button>
      </div>
      <div className="dlg-f">
        <button type="button" className="btn quiet" onClick={onCancel}>
          Cancel import
        </button>
        <button type="button" className="btn primary" autoFocus onClick={() => onContinue(convert)}>
          Continue
        </button>
      </div>
    </>
  );
}
