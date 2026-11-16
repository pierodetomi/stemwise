// Marks of the mix at a glance (P-013, design C7): chain labels on headers and strips (UI-8), one send dot per return
// and the automation mark (UI-9), the senders listed in a return's lane. Everything is derived from the document and
// memoized per track: the playhead never renders them (A-064, A-065).
import { memo, useCallback, useRef } from "react";
import type { Project } from "../bindings/Project";
import type { Track } from "../bindings/Track";
import { faderPos } from "../shared/format";
import { useBackend, useStore } from "../state/context";
import type { InspBlock } from "../state/store";
import { blockName } from "./fx";
import { chainGlance, fitMarks, sendDb, type ChainGlance, type Mark } from "./glance";
import { paramLabel } from "./menus";

export const tintOf = (t: Track) => (t.kind === "audio" ? `t${t.color || 1}` : "tb");

/** Class of a chain label (design .sg): faint, outlined or solid; a keyed block in its source's tint. */
function sigClass(m: Mark, project: Project) {
  if (m.state === "off") return "sg";
  if (m.state === "neutral") return "sg nu";
  const keyT = m.key ? project.tracks.find((x) => x.id === m.key) : undefined;
  return keyT ? `sg on key ${tintOf(keyT)}` : "sg on";
}

/** The "+n" mark of a long chain (16t): its tooltip names the blocks it stands for. */
function moreTitle(more: Mark[]) {
  return more.map((m) => m.title).join(" · ");
}

const returnsOf = (p: Project) => p.tracks.filter((t) => t.kind === "return");

function autoTitle(p: Project, g: ChainGlance) {
  if (!g.automation) return "";
  return `Automation · ${g.automation.params.map((x) => paramLabel(p, x)).join(", ")}${g.automation.by === "ai" ? " · written by the assistant" : ""}`;
}

/**
 * The cell of a track or group header (3a, 3f): chain labels above, a dot per return and the automation mark below.
 * Read-only: a click goes through to the header and selects the track.
 */
export const HeaderMarks = memo(function HeaderMarks({ t, project, relRet }: { t: Track; project: Project; relRet: string | null }) {
  if (t.kind === "return" && !t.chain.blocks.length) return <div className="gl" aria-hidden="true" />;
  const g = chainGlance(t, project);
  const { shown, more } = fitMarks(g.marks);
  const rets = t.kind === "return" ? [] : returnsOf(project);
  return (
    <div className={`gl ${rets.length > 5 ? "many" : ""}`} role="group" aria-label={`${t.name} · chain and sends`} data-testid={`marks-${t.name}`}>
      <div>
        {shown.map((m) => (
          <span key={m.id} className={sigClass(m, project)} title={m.title} aria-label={m.title} role="img" data-block={m.id} data-state={m.state}>
            {m.sig}
            {m.parallel !== null && <i className="pl" />}
          </span>
        ))}
        {more.length > 0 && (
          <span className="sg more" title={moreTitle(more)} aria-label={`${more.length} more: ${moreTitle(more)}`} role="img" data-testid="marks-more">
            +{more.length}
          </span>
        )}
      </div>
      <div>
        {rets.map((r) => {
          const s = t.sends.find((x) => x.target === r.id);
          const title = `${r.name} · ${s ? `${sendDb(s.level_db)} dB` : "no send"}`;
          return <i key={r.id} className={`sd ${s ? (relRet === r.id ? "rel" : "on") : ""}`} title={title} aria-label={title} role="img" />;
        })}
        {g.automation && <i className={`pa ${g.automation.by}`} title={autoTitle(project, g)} aria-label={autoTitle(project, g)} role="img" />}
      </div>
    </div>
  );
});

/** Senders listed in a return's lane, selected or not (UI-9, 7f). */
export const ReturnFeeds = memo(function ReturnFeeds({ r, project }: { r: Track; project: Project }) {
  const feeds = project.tracks.flatMap((t) => {
    const s = t.sends.find((x) => x.target === r.id);
    return s ? [{ t, db: s.level_db }] : [];
  });
  if (!feeds.length) return <span className="lane-note">No track sends here yet</span>;
  return (
    <div className="feeds" aria-label={`Sends to ${r.name}`} data-testid={`feeds-${r.name}`}>
      <span>
        {feeds.length} {feeds.length === 1 ? "track sends" : "tracks send"} here
      </span>
      {feeds.map(({ t, db }) => (
        <span key={t.id} className={`feed ${tintOf(t)}`}>
          <i className="tdot" />
          {t.name}
          <b>{sendDb(db)}</b>
        </span>
      ))}
    </div>
  );
});

const DOUBLE_MS = 250;

/**
 * Actions of the marks in strips and Overview cells: bypass with the inspector's own command (one undo step), open a
 * block in the inspector. A click waits for a possible double click so that a double click never toggles twice.
 */
export function useBlockActions(t: Track) {
  const backend = useBackend();
  const store = useStore();
  const timer = useRef<number | undefined>(undefined);
  const toggle = useCallback(
    (param: string, name: string) => {
      const s = store.getState();
      const tr = s.project?.tracks.find((x) => x.id === t.id);
      if (!tr) return;
      const off = param === "bypass.fx" ? !!tr.ret?.bypass : (tr.chain.blocks.find((b) => b.id === param.slice(7))?.fx.bypass ?? false);
      void backend.setParam(t.id, param, off ? 0 : 1, `${tr.name} · ${name} ${off ? "on" : "off"}`).catch((e) => s.setError(String(e)));
    },
    [backend, store, t.id],
  );
  const open = useCallback(
    (block: InspBlock) => {
      const s = store.getState();
      s.setView({ selTrack: t.id, inspBlock: block, hideInspector: false });
      requestAnimationFrame(() => {
        const id = block === "fader" ? "sends" : "editor";
        document.getElementById(`insp-${id}`)?.scrollIntoView({ block: "nearest" });
      });
    },
    [store, t.id],
  );
  /** Click: run `act` unless a double click follows; keyboard activation (detail 0) runs at once. */
  const click = useCallback((e: React.MouseEvent, act: () => void) => {
    window.clearTimeout(timer.current);
    if (e.detail === 0) act();
    else if (e.detail === 1) timer.current = window.setTimeout(act, DOUBLE_MS);
  }, []);
  const dbl = useCallback((block: InspBlock) => {
    window.clearTimeout(timer.current);
    open(block);
  }, [open]);
  return { toggle, open, click, dbl };
}

/** Chain labels in a strip, one under the other at the strip's width (3a, C7, 16t): click bypasses, double click opens. */
export const StripChain = memo(function StripChain({ t, project }: { t: Track; project: Project }) {
  const a = useBlockActions(t);
  if (t.kind === "return" && !t.chain.blocks.length) return <ChainSpacer />;
  const g = chainGlance(t, project);
  const { shown, more } = fitMarks(g.marks);
  return (
    <div className="gl col" role="group" aria-label={`${t.name} · chain`}>
      {shown.map((m) => (
        <button
          type="button"
          key={m.id}
          className={sigClass(m, project)}
          title={`${m.title} · click to ${m.state === "off" ? "turn on" : "bypass"}, double-click to open`}
          aria-label={`${t.name} · ${m.title}`}
          aria-pressed={m.state !== "off"}
          data-block={m.id}
          data-state={m.state}
          onClick={(e) => a.click(e, () => a.toggle(`bypass.${m.id}`, blockName(t.chain, m.id)))}
          onDoubleClick={() => a.dbl(m.id)}
        >
          {m.sig}
          {m.parallel !== null && <i className="pl" />}
        </button>
      ))}
      {more.length > 0 && (
        <button type="button" className="sg more" title={`${moreTitle(more)} · click to open`} aria-label={`${t.name} · ${more.length} more blocks: ${moreTitle(more)}`} onClick={() => a.open(more[0].id)}>
          +{more.length}
        </button>
      )}
      {Array.from({ length: Math.max(0, 5 - shown.length - (more.length ? 1 : 0)) }, (_, i) => (
        <span key={`pad${i}`} className="sg pad" aria-hidden="true" />
      ))}
    </div>
  );
});

/** One send row per return in a strip (UI-9): a thin bar at the send's level, a dotted line on the return's own row. */
export const StripSends = memo(function StripSends({ t, returns, relRet }: { t: Track | null; returns: Track[]; relRet: string | null }) {
  return (
    <div className="snd" role="group" aria-label="Sends">
      {returns.map((r) => {
        if (!t) return <div key={r.id} />;
        if (t.id === r.id) return <div key={r.id}><i className="self" /></div>;
        if (t.kind === "return") return <div key={r.id} />;
        const s = t.sends.find((x) => x.target === r.id);
        const title = `${r.name} · ${s ? `${sendDb(s.level_db)} dB` : "no send"}`;
        return (
          <div key={r.id} title={title}>
            <div className={`hbar ${s && relRet === r.id ? "rel" : ""}`} role="img" aria-label={`${t.name} · send to ${title}`}>
              <i style={{ width: `${s ? faderPos(s.level_db) * 100 : 0}%` }} />
            </div>
          </div>
        );
      })}
    </div>
  );
});

/** Placeholder chain column for the mix bus, so that every strip's rows line up with the rail. */
export function ChainSpacer() {
  return <div className="gl col spacer" aria-hidden="true">{["HP", "LP", "EQ", "CMP", "SAT"].map((b) => <span key={b} className="sg">{b}</span>)}</div>;
}
