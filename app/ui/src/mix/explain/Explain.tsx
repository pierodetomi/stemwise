// Explain this mix (P-019, design 17): the map of the signal on the left (UI-15) and the same mix in words on the
// right (UI-16). A read-only view of the Mix workspace (UI-14): it never edits the project, calls no model and uses
// no network. The levels it shows are measured by the core in the background (ANA-20).
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Project } from "../../bindings/Project";
import { Icon } from "../../shared/Icon";
import { IconButton } from "../../shared/controls";
import { useApp, useBackend, useStore } from "../../state/context";
import { songFrames } from "../geometry";
import { layoutMap, modeFor, type Layout } from "./layout";
import { MIX, buildMap, membersOf, parentOf, related, sectionFacts, sectionGroups, type MixMap } from "./model";
import { GLOSSARY, explainElement, explainMix, explainSection, toPlainText, type Block, type Doc, type Part } from "./text";

const clockOf = (frames: number, rate: number) => {
  const s = Math.floor(frames / rate);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/** Asks the core for the levels of the mix as it is, again after each change (ANA-20). */
function useLevels(project: Project) {
  const backend = useBackend();
  const setView = useApp((s) => s.setView);
  const revision = useApp((s) => s.revision);
  useEffect(() => {
    let alive = true;
    const off = backend.onMixLevels((l) => alive && setView({ mixLevels: l }));
    // A burst of edits (a fader being dragged) asks once, when it settles.
    const timer = setTimeout(() => {
      backend
        .mixLevels()
        .then((l) => alive && setView({ mixLevels: l }))
        .catch(() => alive && setView({ mixLevels: { mix_lufs: null, sections: [], families: [], returns: [] } }));
    }, 250);
    return () => {
      alive = false;
      clearTimeout(timer);
      off();
    };
  }, [backend, setView, revision, project.name]);
}

// ---------- the song strip ----------
const Playhead = memo(function Playhead({ total }: { total: number }) {
  const frame = useApp((s) => s.meters?.frame ?? 0);
  return <i className="xplay" style={{ left: `${Math.min(100, (frame / Math.max(1, total)) * 100)}%` }} />;
});

function SongStrip({ project, section, onPick }: { project: Project; section: string | null; onPick: (name: string | null) => void }) {
  const total = Math.max(1, songFrames(project));
  if (!project.sections.length) return null;
  return (
    <div className="xsong" role="group" aria-label="Whole song or one section">
      <button type="button" className={`xs all ${section === null ? "on" : ""}`} aria-pressed={section === null} onClick={() => onPick(null)}>
        Whole song
      </button>
      <div className="xsl grow">
        {[...project.sections]
          .sort((a, b) => a.start - b.start)
          .map((s) => (
            <button
              key={s.id}
              type="button"
              className={`xs ${section === s.name ? "on" : ""}`}
              aria-pressed={section === s.name}
              title={s.name}
              style={{ left: `${(s.start / total) * 100}%`, width: `calc(${((s.end - s.start) / total) * 100}% - var(--sp-h))` }}
              onClick={() => onPick(section === s.name ? null : s.name)}
            >
              <span className="ell">{s.name}</span>
            </button>
          ))}
        <Playhead total={total} />
      </div>
    </div>
  );
}

// ---------- the map ----------
interface MapProps {
  map: MixMap;
  project: Project;
  sel: string | null;
  section: string | null;
  compact: boolean;
  onSelect: (id: string | null) => void;
  onOpen: (id: string) => void;
  onFold: (group: string) => void;
}

function MapStage({ map, project, sel, section, compact, onSelect, onOpen, onFold }: MapProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(920);
  const [height, setHeight] = useState(728);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      setWidth(Math.max(560, Math.floor(el.clientWidth)));
      setHeight(Math.floor(el.clientHeight));
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const mode = useMemo(() => modeFor(map, compact, width, height), [map, compact, width, height]);
  const lay: Layout = useMemo(() => layoutMap(map, width, mode), [map, width, mode]);
  const rel = useMemo(() => related(map, sel), [map, sel]);
  const facts = useMemo(() => (section ? sectionFacts(project, section) : null), [project, section]);
  const off = new Set(facts?.silent ?? []);
  const tagOf = (id: string): { text: string; tone: "acc" | "ai" } | null => {
    if (!facts) return null;
    const c = facts.changes.find((x) => x.track === id);
    if (c) return { text: `${c.delta > 0 ? "+" : "−"}${Math.abs(c.delta).toFixed(1)} dB`, tone: c.by === "ai" ? "ai" : "acc" };
    if (facts.onlyHere.includes(id)) return { text: "only here", tone: "acc" };
    if (facts.blocks.some((x) => x.track === id)) return { text: "block on", tone: "acc" };
    return null;
  };
  const dim = sel !== null;
  const cls = (id: string) => `${id === sel ? "is-sel" : rel.nodes.has(id) ? "is-rel" : ""} ${off.has(id) ? "is-off" : ""}`;
  const style = (id: string) => {
    const b = lay.boxes.get(id)!;
    return { left: b.x, top: b.y, width: b.w, height: b.h };
  };
  const pick = (id: string) => onSelect(id === sel ? null : id);
  const mix = lay.boxes.get(MIX)!;
  const rows = map.families.flatMap((f) => f.rows);
  return (
    <div className="xscroll grow" ref={ref}>
      <div className={`xstage ${dim ? "has-sel" : ""} ${mode === "compact" ? "compact" : ""}`} style={{ width: lay.width, height: lay.height }} data-testid="mix-map" data-mode={mode}>
        <svg width={lay.width} height={lay.height} aria-hidden="true">
          {lay.links.map((l) => (
            <g key={l.id}>
              <path className={`lk ${l.kind === "out" ? "" : l.kind} ${l.tint} ${rel.links.has(l.id) ? "rel" : ""} ${l.kind === "out" && off.has(l.id.slice(2)) ? "is-off" : ""}`} strokeWidth={l.kind === "out" ? l.width : undefined} d={l.d} data-link={l.id} data-lit={rel.links.has(l.id)} />
              {l.head && <path className={`lkh ${l.tint} ${rel.links.has(l.id) ? "rel" : ""}`} d={l.head} />}
            </g>
          ))}
          <path className="lk tm rel" strokeWidth={8} d={lay.exit.d} />
          <path className="lkh tm rel" d={lay.exit.head} />
        </svg>
        {lay.labels.map((l) => (
          <span key={`${l.text}-${l.x}-${l.y}`} className={l.strong ? "xcolh" : "xfam"} style={{ left: l.x, top: l.y }}>
            {l.text}
          </span>
        ))}
        {rows.map((r) =>
          r.fold ? (
            <button key={r.id} type="button" className={`mn fold ${r.tint} ${cls(r.id)}`} style={style(r.id)} title={`Unfold ${map.groups.find((g) => g.id === r.fold!.group)?.name ?? "group"}`} data-testid={`node-${r.id}`} onClick={() => onFold(r.fold!.group)}>
              <Icon name="right" size="sm" />
              <span className="nm ell grow">{r.name}</span>
              {r.note !== null && <span className="badge">{r.note}</span>}
            </button>
          ) : (
            <button key={r.id} type="button" className={`mn ${r.tint} ${cls(r.id)}`} style={style(r.id)} aria-pressed={r.id === sel} data-testid={`node-${r.name}`} onClick={() => pick(r.id)} onDoubleClick={() => onOpen(r.id)}>
              <i className="tdot" />
              <span className="nm ell grow">{r.name}</span>
              {tagOf(r.id) && <span className={`tag ${tagOf(r.id)!.tone}`}>{tagOf(r.id)!.text}</span>}
              {r.lock && <Icon name="lock" size="sm" className="aic" />}
              {r.auto && <i className={`pa ${r.auto}`} title={r.auto === "ai" ? "Automation by the assistant" : "Automation by hand"} />}
              {r.note !== null && !facts && <span className="badge">{r.note}</span>}
            </button>
          ),
        )}
        {map.groups.map((g) => (
          <div key={g.id} className={`mn grp tb ${cls(g.id)}`} style={style(g.id)}>
            <button type="button" className="mn-fold" aria-label={`${g.folded ? "Unfold" : "Fold"} ${g.name}`} aria-expanded={!g.folded} onClick={() => onFold(g.id)}>
              <Icon name={g.folded ? "right" : "chev"} size="m" />
            </button>
            <button type="button" className="mn-main" aria-pressed={g.id === sel} data-testid={`node-${g.name}`} onClick={() => pick(g.id)} onDoubleClick={() => onOpen(g.id)}>
              <span className="nm ell">{g.name}</span>
              {lay.boxes.get(g.id)!.h >= 40 && <span className="faint s1 ell">{`${g.count} track${g.count === 1 ? "" : "s"}${g.folded ? " · folded" : ""}`}</span>}
            </button>
            {tagOf(g.id) && <span className={`tag ${tagOf(g.id)!.tone}`}>{tagOf(g.id)!.text}</span>}
            {g.note !== null && !facts && <span className="badge">{g.note}</span>}
          </div>
        ))}
        {map.returns.map((r) => (
          <button key={r.id} type="button" className={`mn ret tb ${cls(r.id)}`} style={style(r.id)} title={r.sub} aria-pressed={r.id === sel} data-testid={`node-${r.name}`} onClick={() => pick(r.id)} onDoubleClick={() => onOpen(r.id)}>
            <Icon name="wave" size="m" />
            <span className="col grow">
              <span className="nm ell">{r.name}</span>
              {lay.boxes.get(r.id)!.h >= 32 && <span className="faint s1 ell">{r.sub}</span>}
            </span>
            <span className="faint s1 nowrap">{r.senders} in</span>
          </button>
        ))}
        <div className={`mn mix tm ${dim && rel.nodes.has(MIX) ? "is-rel" : ""}`} style={{ left: mix.x, top: mix.y, width: mix.w, height: mix.h }} data-testid="node-mix">
          <span className="nm">Mix bus</span>
          <MixLoudness />
          <span className="faint s1">to Master</span>
        </div>
        {lay.links
          .filter((l) => l.kind !== "out" && l.text && rel.links.has(l.id))
          .map((l) => (
            <span key={l.id} className={`xcall ${l.tint}`} style={{ left: l.at[0], top: l.at[1] }}>
              {l.text}
            </span>
          ))}
      </div>
    </div>
  );
}

function MixLoudness() {
  const lufs = useApp((s) => s.mixLevels?.mix_lufs ?? null);
  return <span className="faint s1">{lufs === null ? "—" : `${lufs.toFixed(1).replace("-", "−")} LUFS`}</span>;
}

const Legend = ({ plain, compact }: { plain: boolean; compact: boolean }) => (
  <div className="xlegend">
    <span>
      <i className="lg out" />
      Output · thicker is louder
    </span>
    {!plain && (
      <>
        <span>
          <i className="lg send" />
          Send · a copy to a shared effect
        </span>
        <span>
          <i className="lg key" />
          Listens to · sidechain
        </span>
        <span>
          <i className="pa ai" />
          Automation by the assistant
        </span>
        <span>
          <i className="pa user" />
          by hand
        </span>
      </>
    )}
    <span className="grow" />
    {!compact && <span className="faint">Click a box to explain it</span>}
  </div>
);

// ---------- the text ----------
function Parts({ parts, onSelect }: { parts: Part[]; onSelect: (id: string) => void }) {
  return (
    <>
      {parts.map((x, i) => {
        if (typeof x === "string") return <span key={i}>{x}</span>;
        if ("b" in x) return <b key={i}>{x.b}</b>;
        if ("id" in x)
          return (
            <button key={i} type="button" className={`xn ${x.tint}`} onClick={() => onSelect(x.id)} title={`Show ${x.name} in the map`}>
              <i className="tdot" />
              {x.name}
            </button>
          );
        if ("term" in x)
          return (
            <span key={i} className="gt" tabIndex={0} role="note" aria-label={`${x.text}: ${GLOSSARY[x.term] ?? ""}`}>
              {x.text}
              <span className="tip" role="tooltip">
                {GLOSSARY[x.term]}
              </span>
            </span>
          );
        return (
          <span key={i} className={x.tone === "ai" ? "aic" : "acc"}>
            {x.text}
          </span>
        );
      })}
    </>
  );
}

function BlockView({ b, onSelect }: { b: Block; onSelect: (id: string) => void }) {
  switch (b.kind) {
    case "p":
      return (
        <p className={b.faint ? "faint s1" : ""}>
          <Parts parts={b.parts} onSelect={onSelect} />
        </p>
      );
    case "tags":
      return (
        <div className="fx g1 wrap">
          {b.tags.map((t) => (
            <span key={t.text} className={`tag ${t.tone ?? ""}`}>
              {t.text}
            </span>
          ))}
        </div>
      );
    case "item":
      return (
        <div className="xli">
          <div className="xlh">
            {b.icon && <Icon name={b.icon} size="m" />}
            <span>
              <Parts parts={b.head} onSelect={onSelect} />
            </span>
            {b.hint && <span className="faint s1">{b.hint}</span>}
          </div>
          <p>
            <Parts parts={b.body} onSelect={onSelect} />
          </p>
        </div>
      );
    case "bars":
      return (
        <div className="xbal" role="list">
          {b.bars.map((r) => (
            <div key={r.label} className="xbal-r" role="listitem" aria-label={`${r.label}: ${r.text}`}>
              <span className="ell">{r.label}</span>
              <div className={`hbar ${r.tint ?? ""} ${r.accent ? "acc" : ""}`}>
                <i style={{ width: `${Math.round(r.value * 100)}%` }} />
              </div>
              <span className="faint s1">{r.text}</span>
            </div>
          ))}
        </div>
      );
    case "pan":
      return (
        <>
          <div className="xpan" role="img" aria-label="Where each track sits between left and right">
            <i className="c" />
            {b.dots.map((d, i) => (
              <i key={d.id} className={`tdot ${d.tint}`} title={d.name} style={{ left: `${50 + d.pan * 46}%`, top: `${10 + ((i * 80) / Math.max(1, b.dots.length - 1) || 0)}%` }} />
            ))}
          </div>
          <div className="fx between faint s1">
            <span>Left</span>
            <span>Centre</span>
            <span>Right</span>
          </div>
        </>
      );
    case "steps":
      return (
        <ol className="xsteps">
          {b.steps.map((s, i) => (
            <li key={`${s.name}-${i}`} className="xstep">
              <span className={`sg ${s.state === "on" ? "on" : "nu"} ${s.keyTint ? `key ${s.keyTint}` : ""}`}>{s.sig}</span>
              <div className="col grow">
                <div className="fx g2">
                  <span className="b">{s.name}</span>
                  <span className="faint s1">{s.value}</span>
                </div>
                <p>
                  <Parts parts={s.body} onSelect={onSelect} />
                </p>
              </div>
            </li>
          ))}
        </ol>
      );
    case "note":
      return (
        <div className={`note colm ${b.number !== null ? "on" : ""}`}>
          <div className="fx g2">
            {b.number !== null ? <span className="badge">{b.number}</span> : <Icon name="spark" size="m" className="aic" />}
            <span className="b">{b.title}</span>
          </div>
          <p className="why">{b.text}</p>
          {b.foot && <span className="faint s1">{b.foot}</span>}
        </div>
      );
    case "pending":
      return (
        <div className="fx g2 xpend" role="status">
          <Icon name="clock" size="m" />
          <span className="dim">{b.text}</span>
        </div>
      );
  }
}

function DocView({ doc, nav, onSelect }: { doc: Doc; nav: boolean; onSelect: (id: string) => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState(doc.sections[0]?.id ?? "");
  const go = (id: string) => {
    setAt(id);
    ref.current?.querySelector(`[data-sec="${id}"]`)?.scrollIntoView({ block: "start", behavior: "smooth" });
  };
  useEffect(() => {
    ref.current?.scrollTo({ top: 0 });
    setAt(doc.sections[0]?.id ?? "");
    // A new document (another element, another section) starts from its top.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [doc.title]);
  return (
    <>
      {nav && doc.sections.length > 2 && (
        <div className="xnav">
          <div className="seg quiet sm" role="tablist" aria-label="Parts of the explanation">
            {doc.sections.map((s) => (
              <button key={s.id} type="button" role="tab" aria-selected={at === s.id} className={at === s.id ? "on" : ""} onClick={() => go(s.id)}>
                {s.nav}
              </button>
            ))}
          </div>
        </div>
      )}
      <div className="xdoc scroll" ref={ref} data-testid="explain-text">
        {doc.sections.map((s) => (
          <section key={s.id} className="xsec" data-sec={s.id} aria-label={s.title}>
            <div className="fx g2">
              <h3>{s.title}</h3>
              {s.hint && <span className="faint s1">{s.hint}</span>}
            </div>
            {s.blocks.map((b, i) => (
              <BlockView key={i} b={b} onSelect={onSelect} />
            ))}
          </section>
        ))}
      </div>
    </>
  );
}

// ---------- the view ----------
export function Explain() {
  const backend = useBackend();
  const store = useStore();
  const project = useApp((s) => s.project)!;
  const sel = useApp((s) => s.explainSel);
  const section = useApp((s) => s.explainSection);
  const folded = useApp((s) => s.explainFolded);
  const levels = useApp((s) => s.mixLevels);
  const compact = useApp((s) => s.compact);
  const setView = useApp((s) => s.setView);
  const toast = useApp((s) => s.toast);
  useLevels(project);

  // A selection that no longer exists (undo, a deleted track, a renamed section) goes back to the whole mix.
  const selTrack = sel ? (project.tracks.find((t) => t.id === sel) ?? null) : null;
  const secName = section && project.sections.some((s) => s.name === section) ? section : null;
  const map = useMemo(() => buildMap(project, folded), [project, folded]);
  const doc = useMemo(() => (selTrack ? explainElement(project, selTrack.id, levels) : secName ? explainSection(project, secName, levels) : null) ?? explainMix(project, levels), [project, selTrack, secName, levels]);

  const close = () => setView({ explain: false });
  const whole = () => setView({ explainSel: null, explainSection: null });
  /** Folded groups a track or group sits in: selecting it opens them. */
  const hiding = (id: string) => {
    const out: string[] = [];
    let g = project.tracks.find((t) => t.id === id);
    for (let i = 0; g && i < 16; i++) {
      g = parentOf(project, g) ?? undefined;
      if (g && folded.includes(g.id)) out.push(g.id);
    }
    return out;
  };
  const select = (id: string | null) => {
    if (id === null) return setView({ explainSel: null });
    const open = hiding(id);
    setView({ explainSel: id, explainSection: null, ...(open.length ? { explainFolded: folded.filter((g) => !open.includes(g)) } : {}) });
  };
  const open = (id: string) => setView({ explain: false, selTrack: id, hideInspector: false });
  const fold = (g: string) => {
    if (folded.includes(g)) return setView({ explainFolded: folded.filter((x) => x !== g) });
    const next = [...folded, g];
    // Folding what holds the selection takes it out of sight: back to the whole mix.
    const gone = !!sel && membersOf(project, g).some((t) => t.id === sel);
    setView({ explainFolded: next, ...(gone ? { explainSel: null } : {}) });
  };

  // Esc: back to the whole mix first, then out of the view (UI-14).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || (e.target as HTMLElement).closest("[role=dialog], .menu")) return;
      const s = store.getState();
      if (s.explainSel || s.explainSection) s.setView({ explainSel: null, explainSection: null });
      else s.setView({ explain: false });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [store]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(toPlainText(doc));
      toast("ok", "Copied", "The explanation is on the clipboard as plain text.");
    } catch {
      toast("warn", "Could not copy", "The clipboard is not available.");
    }
  };
  const audio = project.tracks.filter((t) => t.kind === "audio").length;
  const groups = project.tracks.filter((t) => t.kind === "group").length;
  const returns = project.tracks.filter((t) => t.kind === "return").length;
  const count = `${audio} track${audio === 1 ? "" : "s"} · ${groups ? `${groups} group${groups === 1 ? "" : "s"}` : "no groups"} · ${returns ? `${returns} shared effect${returns === 1 ? "" : "s"}` : "no shared effects"}`;
  const rate = project.sample_rate ?? 48000;
  const secGroup = secName ? sectionGroups(project).find((g) => g.name === secName) : undefined;
  const kind = selTrack ? (selTrack.kind === "audio" ? (selTrack.role ?? "track").replace(/_/g, " ") : selTrack.kind === "group" ? "group" : `${selTrack.ret?.type ?? ""} return`.trim()) : "";
  return (
    <div className="body xbody" data-testid="explain">
      <section className="card xmap" aria-label="Mix map">
        <div className="ph">
          <span className="h">Mix map</span>
          <span className="faint s1 ell">{count}</span>
          <div className="grow" />
          {folded.length > 0 && (
            <button type="button" className="btn quiet sm" onClick={() => setView({ explainFolded: [] })}>
              Unfold all
            </button>
          )}
        </div>
        <SongStrip project={project} section={secName} onPick={(name) => setView({ explainSection: name, explainSel: null })} />
        <MapStage map={map} project={project} sel={selTrack?.id ?? null} section={secName} compact={compact} onSelect={select} onOpen={open} onFold={fold} />
        <Legend plain={!groups && !returns} compact={compact} />
      </section>
      <aside className="card side xside" aria-label="Explanation">
        <div className="ph">
          {selTrack || secName ? (
            <>
              <IconButton icon="left" label="Back to the whole mix" onClick={whole} />
              <i className={`tdot ${selTrack ? (selTrack.kind === "audio" ? `t${selTrack.color || 1}` : "tb") : "tm"}`} />
              <h2 className="h ell">{selTrack?.name ?? secName}</h2>
              <span className="tag">{selTrack ? kind : `${secGroup?.spans.length === 1 ? "once" : `${secGroup?.spans.length} times`} · ${secGroup?.spans.map(([a]) => clockOf(a, rate)).join(", ")}`}</span>
              <div className="grow" />
              {selTrack && (
                <>
                  <button type="button" className={`btn quiet sm ${selTrack.solo ? "on" : ""}`} aria-pressed={selTrack.solo} onClick={() => void backend.setView(selTrack.id, "solo", !selTrack.solo).catch((e) => store.getState().setError(String(e)))}>
                    <Icon name="ear" size="m" />
                    Solo
                  </button>
                  <button type="button" className="btn sm" onClick={() => open(selTrack.id)}>
                    Open in inspector
                  </button>
                </>
              )}
            </>
          ) : (
            <>
              <h2 className="h">Explain this mix</h2>
              <div className="grow" />
              <span className="kbd">Esc</span>
            </>
          )}
          <IconButton icon="x" label="Close the explanation" onClick={close} />
        </div>
        <DocView doc={doc} nav={!selTrack && !secName} onSelect={(id) => select(id)} />
        <div className="xfoot">
          <span className="faint s1 grow">Read from the project as it is now · no AI call, no cost</span>
          <button type="button" className="btn quiet sm" onClick={() => void copy()}>
            <Icon name="copy" size="m" />
            Copy as text
          </button>
        </div>
      </aside>
    </div>
  );
}
