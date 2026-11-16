// Starting and ending the guided tour (UI-17, UI-19). The demo song is the core's (PRJ-9): the interface asks for it,
// shows it, and asks for it to be deleted at the end, however the tour ends.
import { useCallback } from "react";
import type { AppStore, ViewState } from "../state/store";
import { useApp, useStore } from "../state/context";
import { TOUR_TEXT } from "./text";

/** What the tour changes and puts back at the end: panels, mixer, the assistant's tab, the inspector's block. */
const KEPT: (keyof ViewState)[] = ["hideAssistant", "hideInspector", "hideMixer", "mixerH", "mixerView", "asstTab", "inspBlock", "folded", "lane", "masterModule"];

/** Opens the tour on its first card and asks the core for the demo song; the open project closes first. */
export async function beginTour(store: AppStore) {
  const s = store.getState();
  if (s.tour) return;
  const saved = Object.fromEntries(KEPT.map((k) => [k, s[k]])) as Partial<ViewState>;
  s.closeProject();
  store.setState({
    tour: { step: 1, demo: "building", saved },
    toasts: [],
    dialog: null,
    screen: "welcome",
    explain: false,
    exportOpen: false,
    bounceOpen: false,
    projectOpen: false,
    setup: null,
  });
  // UI-17: shown from the moment the first card is on screen, so a tour cut short never starts by itself again.
  void s.backend.tourMarkShown().catch(() => {});
  try {
    await s.backend.tourStart();
    const t = store.getState().tour;
    if (t) store.setState({ tour: { ...t, demo: "ready" } });
  } catch (e) {
    const m = String(e).replace(/^Error: /, "");
    if (/cancelled/.test(m)) return;
    const t = store.getState().tour;
    // UI-19: the tour says so and ends; the app stays usable (design 18w).
    if (t) store.setState({ tour: { ...t, step: 1, demo: "failed", error: m } });
  }
}

/** UI-19: Done, Skip tour, Esc and closing the app end the tour the same way. After a skip a toast says where to
 *  find it again; "quiet" is for the project commands that end it on their way (New, Open, Close). */
export async function endTour(store: AppStore, kind: "done" | "skip" | "quiet") {
  const s = store.getState();
  const saved = s.tour?.saved ?? {};
  store.setState({ tour: null });
  try {
    await s.backend.tourEnd();
  } catch {
    // The demo is deleted at the next start anyway (PRJ-9).
  }
  const now = store.getState();
  now.closeProject();
  now.setView({ ...saved, screen: "welcome", explain: false, exportOpen: false, bounceOpen: false, projectOpen: false, dialog: null, setup: null });
  if (kind === "skip") now.toast("info", TOUR_TEXT.skippedTitle, TOUR_TEXT.skippedText);
}

/** Help › Take the Tour and the Welcome link: with a project open, the question closing it asks comes first. */
export function useTakeTour() {
  const store = useStore();
  const busy = useApp((s) => !!s.tour || !!s.ai?.running || (!!s.masterStatus?.job && ["bounce", "export"].includes(s.masterStatus.job.kind)));
  const take = useCallback(() => {
    const s = store.getState();
    if (s.tour) return;
    const p = s.project;
    if (!p || !s.dirty || s.demo) {
      void beginTour(store);
      return;
    }
    s.setView({
      dialog: {
        title: `Save changes to “${p.name}”?`,
        sub: TOUR_TEXT.saveFirstSub,
        confirm: "Save",
        onConfirm: () =>
          void s.backend
            .saveProject()
            .then(() => beginTour(store))
            .catch((e) => store.getState().setError(String(e))),
        alt: { label: "Don’t save", danger: true, onSelect: () => void beginTour(store) },
      },
    });
  }, [store]);
  return { take, busy };
}
