// The guided tour (P-020, design 18): a spotlight on the area a step talks about, a bubble beside it, fifteen steps.
// Every step declares the state it shows and sets it up itself, going forward as going back (UI-18); the lit area
// stays usable, clicks outside it do nothing; →/↩ next, ← back, Esc ends the tour, Space stays with the transport.
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import type { Backend, HistoryEntry } from "../backend/types";
import type { AppState, AppStore, ViewState } from "../state/store";
import { Icon } from "../shared/Icon";
import { useApp, useStore } from "../state/context";
import { isCard, keyEffect, move, TOUR_STEPS, type KeyFocus, type TourMove } from "./machine";
import { place, type Rect, type Side } from "./place";
import { STEP_TEXT, TOUR_TEXT } from "./text";
import { endTour } from "./useTour";

type Wait = { text: string; meta: string; fraction: number | null; ai: boolean } | null;

interface Ctx {
  store: AppStore;
  backend: Backend;
  /** False once the tour has moved to another step: a setup still running stops there. */
  alive: () => boolean;
}

interface StepDef {
  /** The lit area: one element or several (their union), found by role and accessible name. */
  target?: (s: AppState) => Element[];
  /** What the bubble's arrow points at, inside the lit area. */
  anchor?: (s: AppState) => Element | null;
  side?: Side;
  setup?: (c: Ctx) => Promise<void>;
  /** Something the step needs that is not ready yet: it says so and Next waits (design 18p, 18q). */
  wait?: (s: AppState) => Wait;
}

// ---------- finding the areas ----------
const q = (sel: string) => document.querySelector(`.sw.app ${sel}`);
const some = (...els: (Element | null)[]) => els.filter((e): e is Element => !!e);
const leadVocal = (s: AppState) => s.project?.tracks.find((t) => t.role === "lead_vocal") ?? s.project?.tracks.find((t) => t.kind === "audio") ?? null;

// ---------- setting the state a step declares ----------
function until(store: AppStore, ok: (s: AppState) => boolean, alive: () => boolean, ms = 120_000): Promise<boolean> {
  return new Promise((resolve) => {
    if (ok(store.getState())) return resolve(true);
    const stop = () => {
      off();
      clearInterval(poll);
      clearTimeout(timer);
    };
    const off = store.subscribe((s) => {
      if (ok(s) || !alive()) {
        stop();
        resolve(ok(s));
      }
    });
    const poll = setInterval(() => {
      if (!alive()) {
        stop();
        resolve(false);
      }
    }, 200);
    const timer = setTimeout(() => {
      stop();
      resolve(false);
    }, ms);
  });
}

const demoOpen = (s: AppState) => s.demo && !!s.project;
const applied = (h: HistoryEntry[]) => h.some((e) => e.ai_call && !e.undone);

async function history(c: Ctx) {
  return (await c.backend.history("mix"))[0];
}

/** The proposal is not applied: undone, step by step, back to before it (going back from step 9). */
async function unapply(c: Ctx) {
  for (let i = 0; i < 16 && c.alive() && applied(await history(c)); i++) await c.backend.undo("mix");
}

/** Mix phase, Explain and dialogs closed, every panel open (the layout the steps were drawn on). */
async function mix(c: Ctx, v: Partial<ViewState> = {}) {
  if (!(await until(c.store, demoOpen, c.alive)) || !c.alive()) return;
  if (c.store.getState().masterStatus?.phase === "master") await c.backend.setPhase(false);
  c.store.getState().setView({
    screen: "workspace", explain: false, exportOpen: false, bounceOpen: false, projectOpen: false, setup: null, dialog: null,
    hideAssistant: false, hideInspector: false, hideMixer: false, ...v,
  });
}

/** Waits for the tour's proposal to be in review; gives up as soon as the assistant stops working without one (the
 *  user cancelled the wait, or it failed), so the step is never held by something that will not come. */
function proposalShown(c: Ctx): Promise<boolean> {
  let ran = false;
  return until(
    c.store,
    (s) => {
      if (s.ai?.running) ran = true;
      return !!s.ai?.proposal || (ran && !s.ai?.running);
    },
    c.alive,
  ).then(() => !!c.store.getState().ai?.proposal);
}

async function proposalInReview(c: Ctx) {
  // The proposal was made on the demo as the tour opened it: whatever was changed in the lit areas since (a fader
  // moved, the proposal applied) is taken back first, so the step always shows the same thing (UI-18).
  if ((await history(c)).some((e) => !e.undone)) await c.backend.historyGoto("mix", 0);
  if (!c.alive()) return;
  await c.backend.tourProposal();
  if (await proposalShown(c)) await c.backend.aiListen("proposal");
}

async function proposalApplied(c: Ctx) {
  if (applied(await history(c)) || !c.alive()) return;
  if (!c.store.getState().ai?.proposal) await c.backend.tourProposal();
  if (!(await proposalShown(c)) || !c.alive()) return;
  await c.backend.aiApply(null, c.store.getState().project?.ai.normalize_trim ?? true);
}

/** The mixdown is made in the background as soon as the proposal is applied (design 18k). */
async function mixdown(c: Ctx) {
  const s = c.store.getState();
  if (!s.project?.master.mixdown && !s.masterStatus?.job) await c.backend.bounceStart().catch(() => {});
}

async function master(c: Ctx, v: Partial<ViewState> = {}) {
  if (!(await until(c.store, demoOpen, c.alive))) return;
  await mixdown(c);
  if (!(await until(c.store, (s) => !!s.project?.master.mixdown, c.alive)) || !c.alive()) return;
  c.store.getState().setView({ screen: "workspace", explain: false, exportOpen: false, bounceOpen: false, projectOpen: false, dialog: null, ...v });
  if (c.store.getState().masterStatus?.phase !== "master") await c.backend.setPhase(true);
}

const waitDemo = (s: AppState): Wait => (demoOpen(s) ? null : { text: TOUR_TEXT.waitDemo[0], meta: TOUR_TEXT.waitDemo[1], fraction: null, ai: false });
/** The step waits only while the proposal is really being made. Once it has been shown, what the user does with it
 *  in the lit area (apply, discard, cancel the wait) never holds Next back: no step waits for an action (UI-18). */
const waitProposal = (s: AppState): Wait =>
  s.ai?.proposal || !s.ai?.running ? null : { text: TOUR_TEXT.waitProposal[0], meta: TOUR_TEXT.waitProposal[1], fraction: null, ai: true };
const waitMixdown = (s: AppState): Wait => {
  if (s.project?.master.mixdown) return null;
  const j = s.masterStatus?.job;
  return { text: TOUR_TEXT.waitMixdown[0], meta: TOUR_TEXT.waitMixdown[1], fraction: j?.kind === "bounce" ? j.progress : null, ai: false };
};

const STEPS: Record<number, StepDef> = {
  2: {
    target: () => some(q('header[aria-label="Top bar"]')),
    anchor: () => q('[aria-label="Transport"] .playb'),
    side: "b",
    wait: waitDemo,
    setup: async (c) => {
      await mix(c);
      const s = c.store.getState();
      const chorus = s.project?.sections.find((x) => /chorus/i.test(x.name));
      if (chorus && !s.transport?.playing && c.alive()) await c.backend.locate(chorus.start);
    },
  },
  3: { target: () => some(q('[aria-label="Timeline"]')), wait: waitDemo, setup: (c) => mix(c) },
  4: {
    target: (s) => some(q(`[aria-label="${CSS.escape(leadVocal(s)?.name ?? "")} header"]`)),
    wait: waitDemo,
    setup: (c) => mix(c, { selTrack: leadVocal(c.store.getState())?.id ?? null }),
  },
  5: { target: () => some(q('[aria-label="Mixer"]')), wait: waitDemo, setup: (c) => mix(c, { mixerView: "faders" }) },
  6: {
    target: () => some(q('[aria-label="Inspector"]')),
    wait: waitDemo,
    setup: async (c) => {
      const t = leadVocal(c.store.getState());
      const eq = t?.chain.blocks.find((b) => b.id === "eq" || b.id.startsWith("eq"))?.id ?? "eq";
      await mix(c, { selTrack: t?.id ?? null, inspBlock: eq });
    },
  },
  7: {
    target: () => some(q('aside[aria-label="Assistant"]')),
    anchor: () => q('aside[aria-label="Assistant"] textarea, aside[aria-label="Assistant"] input[type="text"]'),
    wait: waitDemo,
    setup: async (c) => {
      await mix(c, { asstTab: 0 });
      if (c.store.getState().ai?.proposal && c.alive()) await c.backend.aiDiscard();
      await unapply(c);
    },
  },
  8: {
    target: (s) => some(q(s.ai?.proposal ? 'aside[aria-label="Proposal"]' : 'aside[aria-label="Assistant"]')),
    wait: (s) => waitDemo(s) ?? waitProposal(s),
    setup: async (c) => {
      await mix(c, { asstTab: 0 });
      await proposalInReview(c);
    },
  },
  9: {
    target: () => some(q('aside[aria-label="Assistant"]')),
    wait: (s) => waitDemo(s) ?? waitProposal(s),
    setup: async (c) => {
      await mix(c, { asstTab: 1 });
      await proposalApplied(c);
      if (c.alive()) {
        c.store.getState().setView({ asstTab: 1 });
        await mixdown(c);
      }
    },
  },
  10: {
    target: () => some(q('[aria-label="Mix map"]'), q('[aria-label="Explanation"]')),
    wait: waitDemo,
    setup: (c) => mix(c, { explain: true, explainSel: null, explainSection: null }),
  },
  11: {
    target: () => some(q('[role="radiogroup"][aria-label="Phase"]')),
    side: "b",
    wait: (s) => waitDemo(s) ?? waitMixdown(s),
    setup: async (c) => {
      await mix(c);
      await mixdown(c);
    },
  },
  12: {
    target: () => {
      const rack = q('[aria-label="Master chain"]');
      return some(rack, rack?.nextElementSibling ?? null);
    },
    wait: (s) =>
      waitMixdown(s) ??
      (s.masterStatus?.phase !== "master" || s.masterStatus.prep
        ? { text: TOUR_TEXT.waitMaster[0], meta: TOUR_TEXT.waitMaster[1], fraction: s.masterStatus?.prep?.fraction ?? null, ai: false }
        : null),
    setup: async (c) => {
      const chain = c.store.getState().project?.master.chain.blocks ?? [];
      const open = chain.find((b) => /clip/i.test(b.id))?.id ?? "limiter";
      await master(c, { masterModule: open, masterSection: null });
    },
  },
  13: { target: () => some(q('aside[aria-label="Loudness"]')), wait: waitMixdown, setup: (c) => master(c) },
  14: { target: () => some(q('[role="dialog"][aria-label="Export master"]')), wait: waitMixdown, setup: (c) => master(c, { exportOpen: true }) },
  15: {
    setup: async (c) => {
      c.store.getState().setView({ exportOpen: false });
    },
  },
};

// ---------- the layer ----------
const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** The part of an element its scrolling ancestors let through. */
function visible(e: Element): DOMRect {
  let r = e.getBoundingClientRect();
  for (let p = e.parentElement; p && !p.classList.contains("app"); p = p.parentElement) {
    const o = getComputedStyle(p);
    if (o.overflowX === "visible" && o.overflowY === "visible") continue;
    const c = p.getBoundingClientRect();
    const x0 = Math.max(r.left, c.left), y0 = Math.max(r.top, c.top);
    r = new DOMRect(x0, y0, Math.max(0, Math.min(r.right, c.right) - x0), Math.max(0, Math.min(r.bottom, c.bottom) - y0));
  }
  return r;
}

function union(els: Element[], origin: DOMRect): Rect | null {
  const rs = els.map(visible).filter((r) => r.width > 0 && r.height > 0);
  if (!rs.length) return null;
  const x0 = Math.min(...rs.map((r) => r.left)), y0 = Math.min(...rs.map((r) => r.top));
  const x1 = Math.max(...rs.map((r) => r.right)), y1 = Math.max(...rs.map((r) => r.bottom));
  return { x: x0 - origin.left, y: y0 - origin.top, w: x1 - x0, h: y1 - y0 };
}

const editable = (t: EventTarget | null) => t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));
/** What has the focus, for the keys: a real button or link, a control the arrows adjust. */
const focusOf = (t: EventTarget | null): KeyFocus =>
  t instanceof HTMLElement
    ? { button: !!t.closest("button, a[href], summary"), adjust: !!t.closest('[role="slider"], [role="radio"], [role="radiogroup"], [role="tab"], [role="spinbutton"], [role="option"], [role="menuitem"]') }
    : {};

export function Tour() {
  const tour = useApp((s) => s.tour);
  if (!tour) return null;
  return <TourLayer key="tour" />;
}

function TourLayer() {
  const store = useStore();
  const tour = useApp((s) => s.tour)!;
  const step = tour.step;
  const def = useMemo(() => STEPS[step] ?? {}, [step]);
  const card = isCard(step) || tour.demo === "failed";
  const text = STEP_TEXT[step - 1];
  const reduce = useApp((s) => s.prefs.reduceMotion);
  // What a step waits for, as a string so the store's selector stays stable.
  const waitKey = useApp((s) => JSON.stringify(def.wait?.(s) ?? null));
  const wait = JSON.parse(waitKey) as Wait;
  // A step with a setup is busy from the very render that shows it until the setup ends: Next never looks ready
  // for a frame in between (the reset happens while rendering, not in an effect).
  const [setupOf, setSetupOf] = useState(step);
  const [setupDone, setSetupDone] = useState(false);
  if (setupOf !== step) {
    setSetupOf(step);
    setSetupDone(false);
  }
  const busy = !!STEPS[step]?.setup && (setupOf !== step || !setupDone);
  const [rect, setRect] = useState<Rect | null>(null);
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const [win, setWin] = useState({ w: window.innerWidth, h: window.innerHeight });
  const [pos, setPos] = useState<ReturnType<typeof place> | null>(null);
  const bubble = useRef<HTMLDivElement>(null);
  const primary = useRef<HTMLButtonElement>(null);
  const layer = useRef<HTMLDivElement>(null);

  // Each step sets up what it declares (UI-18); a setup still running when the step changes stops where it is.
  useEffect(() => {
    let alive = true;
    const run = STEPS[step]?.setup;
    if (!run) return;
    setSetupDone(false);
    run({ store, backend: store.getState().backend, alive: () => alive && store.getState().tour?.step === step })
      .catch((e) => console.warn("tour step", step, e))
      .finally(() => alive && setSetupDone(true));
    return () => {
      alive = false;
    };
  }, [step, store]);

  // The lit area follows the interface: panels open, the window changes size, a dialog appears.
  useEffect(() => {
    if (card) {
      setRect(null);
      return;
    }
    const measure = () => {
      const s = store.getState();
      const origin = layer.current?.parentElement?.getBoundingClientRect() ?? new DOMRect(0, 0, window.innerWidth, window.innerHeight);
      const els = def.target?.(s) ?? [];
      // An area scrolled out of its panel (a track header at the minimum window) is brought into view first (UI-18).
      for (const e of els) {
        const v = visible(e), full = e.getBoundingClientRect();
        if (full.height > 0 && v.height < Math.min(full.height, 24) - 1) e.scrollIntoView({ block: "nearest" });
      }
      const r = union(els, origin);
      setRect((o) => (JSON.stringify(o) === JSON.stringify(r) ? o : r));
      const a = def.anchor?.(s)?.getBoundingClientRect();
      const an = a && a.width ? { x: a.left + a.width / 2 - origin.left, y: a.top + a.height / 2 - origin.top } : null;
      setAnchor((o) => (JSON.stringify(o) === JSON.stringify(an) ? o : an));
      setWin((o) => (o.w === origin.width && o.h === origin.height ? o : { w: origin.width, h: origin.height }));
    };
    measure();
    const t = setInterval(measure, 150);
    window.addEventListener("resize", measure);
    return () => {
      clearInterval(t);
      window.removeEventListener("resize", measure);
    };
  }, [def, card, store]);

  // The bubble is measured as drawn, then placed (design/README.md, "Fumetto: posizione").
  useLayoutEffect(() => {
    const b = bubble.current;
    if (!b || !rect || card) {
      setPos(null);
      return;
    }
    const p = place(rect, { w: b.offsetWidth, h: b.offsetHeight }, win, { side: def.side, anchor: anchor ?? undefined });
    setPos((o) => (JSON.stringify(o) === JSON.stringify(p) ? o : p));
    // The bubble's size changes with the step and with what it waits for.
  }, [rect, card, win, def.side, anchor, waitKey, step, tour.demo]);

  const go = (m: TourMove) => {
    const s = store.getState();
    if (!s.tour) return;
    if (m === "next" && (busy || wait)) return;
    const n = move(s.tour.step, m);
    if (n === "end") void endTour(store, m === "done" || (m === "next" && s.tour.step === TOUR_STEPS) ? "done" : "skip");
    else store.setState({ tour: { ...s.tour, step: n } });
  };
  const goRef = useRef(go);
  goRef.current = go;

  // The tour's keys come first; Space stays with the transport; the app's other shortcuts wait (UI-18).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = store.getState().tour;
      if (!t) return;
      // A confirmation the lit area asked for (discard the proposal?) sits above the tour and has the keyboard.
      if (store.getState().dialog) return;
      const fx = keyEffect(e, t.step, editable(e.target), focusOf(e.target));
      if (fx === "native") {
        e.stopImmediatePropagation();
        return;
      }
      if (e.key === " " && fx === "pass" && e.isTrusted && e.target instanceof HTMLElement && e.target.closest(".tour-tip, .tour-card")) {
        // Space stays with the transport also while the focus is on the bubble's buttons: it is handed to the app
        // as if nothing had the focus, and never presses Next.
        e.preventDefault();
        e.stopImmediatePropagation();
        document.body.dispatchEvent(new KeyboardEvent("keydown", { key: " ", code: "Space", bubbles: true, cancelable: true }));
        return;
      }
      if (e.key === "Tab" && fx === "pass") {
        // Focus moves between the bubble and the lit area, and only there.
        const els = [bubble.current, ...(def.target?.(store.getState()) ?? [])].filter((x): x is HTMLElement => !!x);
        const list = els.flatMap((x) => [...x.querySelectorAll<HTMLElement>(FOCUSABLE)]).filter((x) => x.offsetParent !== null);
        if (!list.length) return;
        e.preventDefault();
        const i = list.indexOf(document.activeElement as HTMLElement);
        list[(i + (e.shiftKey ? -1 : 1) + list.length) % list.length]?.focus();
        return;
      }
      if (fx === "pass") return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (fx === "hold") return;
      if (t.demo === "failed") {
        if (fx === "skip" || fx === "next") void endTour(store, "quiet");
        return;
      }
      goRef.current(fx);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [store, def]);

  // The focus lands on the bubble's main button at every step, once the bubble is placed (a hidden one can't take it).
  const shown = card || !rect || !!pos;
  useEffect(() => {
    if (!shown) return;
    const t = setTimeout(() => primary.current?.focus({ preventScroll: true }), 30);
    return () => clearTimeout(t);
  }, [step, card, tour.demo, shown]);

  const announce = TOUR_TEXT.announce(step, TOUR_STEPS, text.title);
  const failed = tour.demo === "failed";
  const counter = (
    <div className="tt-c">
      <span className="nowrap">{TOUR_TEXT.counter(step, TOUR_STEPS)}</span>
      <span className="tt-bar" aria-hidden="true">
        {Array.from({ length: TOUR_STEPS }, (_, i) => (
          <i key={i} className={i + 1 < step ? "on" : i + 1 === step ? "now" : ""} />
        ))}
      </span>
    </div>
  );
  const body = failed ? (
    <>
      <span className="h l" id="tt-title">{TOUR_TEXT.failedTitle}</span>
      <p className="tt-p">{TOUR_TEXT.failedText}</p>
      <span className="faint s1">{TOUR_TEXT.failedNote}</span>
      <div className="tt-f">
        <button type="button" className="btn quiet" onClick={() => void navigator.clipboard?.writeText(tour.error ?? "").catch(() => {})}>
          {TOUR_TEXT.copyDetails}
        </button>
        <span className="grow" />
        <button type="button" ref={primary} className="btn primary lg" onClick={() => void endTour(store, "quiet")}>
          {TOUR_TEXT.close}
        </button>
      </div>
    </>
  ) : (
    <>
      {counter}
      <span className={`h ${card ? "l" : ""}`} id="tt-title">{text.title}</span>
      <p className="tt-p">{text.text}</p>
      {text.list && (
        <div className="tt-list">
          {text.list.map(([icon, title, t]) => (
            <div key={title}>
              <Icon name={icon} size="m" />
              <div className="col">
                <span className="b">{title}</span>
                <span className="dim s1">{t}</span>
              </div>
            </div>
          ))}
        </div>
      )}
      {text.try && !wait && (
        <div className="tt-try">
          <Icon name="cursor" size="sm" />
          {TOUR_TEXT.tryIt} {text.try}
        </div>
      )}
      {wait && (
        <div className="tt-wait" role="status">
          <span className="fx g2">
            <Icon name="clock" size="sm" />
            <span className="grow">{wait.text}</span>
            <span className="faint">{wait.fraction !== null ? `${Math.round(wait.fraction * 100)} %` : wait.meta}</span>
          </span>
          <div className={`progress ${wait.ai ? "ai" : ""} ${wait.fraction === null ? "indet" : ""}`} role="progressbar" aria-valuenow={wait.fraction === null ? undefined : Math.round(wait.fraction * 100)}>
            <i style={wait.fraction === null ? undefined : { width: `${Math.round(wait.fraction * 100)}%` }} />
          </div>
        </div>
      )}
      {step === 1 && (
        <>
          <span className="faint s1">{TOUR_TEXT.note}</span>
          <div className="tt-keys">
            <span className="kbd">→</span> {TOUR_TEXT.keys.next} <span className="kbd">←</span> {TOUR_TEXT.keys.back} <span className="kbd">Esc</span> {TOUR_TEXT.keys.esc} · <span className="kbd">Space</span> {TOUR_TEXT.keys.space}
          </div>
        </>
      )}
      <div className="tt-f">
        {step === 1 ? (
          <>
            <button type="button" className="btn quiet" onClick={() => go("skip")}>{TOUR_TEXT.skip}</button>
            <span className="grow" />
            <button type="button" ref={primary} className="btn primary lg" onClick={() => go("next")}>
              <Icon name="play" size="m" />
              {TOUR_TEXT.start}
            </button>
          </>
        ) : (
          <>
            {step < TOUR_STEPS && <button type="button" className="btn quiet sm" onClick={() => go("skip")}>{TOUR_TEXT.skip}</button>}
            <span className="grow" />
            <button type="button" className="btn sm" onClick={() => go("back")}>
              <Icon name="left" size="m" />
              {TOUR_TEXT.back}
            </button>
            {step === TOUR_STEPS ? (
              <button type="button" ref={primary} className="btn primary" onClick={() => go("done")}>
                <Icon name="check" size="m" />
                {TOUR_TEXT.done}
              </button>
            ) : (
              <button type="button" ref={primary} className="btn primary" aria-disabled={busy || !!wait} disabled={!!wait} onClick={() => go("next")}>
                {TOUR_TEXT.next}
                <Icon name="right" size="m" />
              </button>
            )}
          </>
        )}
      </div>
    </>
  );

  // A step whose area is not on screen yet (a panel opening, the demo still being built) shows its text as a card.
  const lit = !card && !!rect;
  const hole = lit ? pos?.hole : undefined;
  const blockers: CSSProperties[] = hole
    ? [
        { left: 0, top: 0, width: "100%", height: hole.y },
        { left: 0, top: hole.y + hole.h, width: "100%", bottom: 0 },
        { left: 0, top: hole.y, width: hole.x, height: hole.h },
        { left: hole.x + hole.w, top: hole.y, right: 0, height: hole.h },
      ]
    : [];
  return (
    <div ref={layer} className={`tour ${lit ? "" : "full"} ${reduce ? "reduce" : ""}`} data-testid="tour" data-step={step} style={lit ? undefined : { pointerEvents: "auto" }}>
      {hole && (
        <>
          <div className="tour-spot" style={{ transform: `translate(${hole.x}px, ${hole.y}px)`, width: hole.w, height: hole.h }} />
          {blockers.map((b, i) => (
            <div key={i} className="tour-block" style={b} aria-hidden="true" />
          ))}
        </>
      )}
      {!lit ? (
        <div className="tour-card" key={`c${step}${tour.demo}`} role="dialog" aria-modal="false" aria-labelledby="tt-title">
          {body}
        </div>
      ) : (
        <div
          ref={bubble}
          key={`b${step}`}
          className={`tour-tip ${pos?.arrowAt ? `at-${pos.arrowAt}` : ""}`}
          role="dialog"
          aria-modal="false"
          aria-labelledby="tt-title"
          style={{ left: pos?.x ?? 0, top: pos?.y ?? 0, visibility: pos ? "visible" : "hidden", ["--ax" as string]: `${pos?.arrow ?? 0}px` } as CSSProperties}
        >
          {body}
        </div>
      )}
      <div className="sr" aria-live="polite" role="status">
        {announce}
      </div>
    </div>
  );
}
