// The tour's steps as a small state machine (UI-18, UI-19): fifteen steps, the first and the last are cards.
import { STEP_TEXT } from "./text";

export const TOUR_STEPS = STEP_TEXT.length;

export type TourMove = "next" | "back" | "skip" | "done";

/** Where a move leads: another step, or the end of the tour. Back on the first step stays there. */
export function move(step: number, m: TourMove): number | "end" {
  if (m === "skip" || m === "done") return "end";
  if (m === "back") return Math.max(1, step - 1);
  return step >= TOUR_STEPS ? "end" : step + 1;
}

/** The opening and the closing step are centred cards without a lit area. */
export const isCard = (step: number) => step === 1 || step === TOUR_STEPS;

/** What a key does while the tour runs: its own keys, Space to the transport, every other shortcut held back. */
/** "native": the key does what the focused control does with it by itself (a button pressed, text edited) and
 *  nothing else: the app's shortcuts never see it. */
export type KeyEffect = TourMove | "pass" | "hold" | "native";

/** What has the focus: a button (↩ presses it) or a control the arrows adjust (a fader, a choice among several). */
export interface KeyFocus {
  button?: boolean;
  adjust?: boolean;
}

/** ⌘ with these edits the text of a field; every other ⌘ shortcut is the app's, and waits. */
const TEXT_KEYS = new Set(["a", "c", "v", "x", "z", "y", "arrowleft", "arrowright", "arrowup", "arrowdown", "backspace", "delete"]);

export function keyEffect(e: { key: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }, step: number, editable: boolean, focus: KeyFocus = {}): KeyEffect {
  const mod = e.metaKey || e.ctrlKey || e.altKey;
  if (e.key === "Escape") return "skip";
  // Typing in a field of the lit area (the request to the assistant) is the user's; the app's shortcuts still wait.
  if (editable) {
    if (!e.metaKey && !e.ctrlKey) return "pass";
    return TEXT_KEYS.has(e.key.toLowerCase()) ? "native" : "hold";
  }
  // ↩ on a focused button presses that button (Back, Skip tour, one of the lit area), it is not Next.
  if (!mod && e.key === "Enter" && focus.button) return "native";
  // The arrows on a fader or on a choice of the lit area adjust it.
  if (!mod && focus.adjust && /^Arrow/.test(e.key)) return "pass";
  if (!mod && (e.key === "ArrowRight" || e.key === "Enter")) return step >= TOUR_STEPS ? "done" : "next";
  if (!mod && e.key === "ArrowLeft") return "back";
  if (!mod && !e.shiftKey && e.key === " ") return "pass";
  if (e.key === "Tab") return "pass";
  // ⌘Q and the other keys macOS handles never reach the page; the app's own shortcuts wait for the tour to end.
  return "hold";
}
