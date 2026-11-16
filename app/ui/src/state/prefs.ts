// Appearance preferences (SET-1, design 11c): per user, not per project, so they live in the webview's storage and
// never in the document. Storage can be missing or throw (private mode, cleared data): defaults then.
import type { LaneSize } from "./store";

export interface UiPrefs {
  /** Lane height a project opens with. */
  lane: LaneSize;
  clipNames: boolean;
  waveforms: boolean;
  /** Clip gain badge also at 0 dB. */
  gainBadge: boolean;
  /** Position as bars · beats when the project has a tempo. */
  bars: boolean;
  holdPeaks: boolean;
  peakNumbers: boolean;
  reduceMotion: boolean;
  /** Track header width in px asked for by dragging its edge; 0 = the layout's width (issue #9). */
  headW: number;
}

export const DEFAULT_PREFS: UiPrefs = {
  lane: "m",
  clipNames: true,
  waveforms: true,
  gainBadge: false,
  bars: false,
  holdPeaks: true,
  peakNumbers: false,
  reduceMotion: false,
  headW: 0,
};

const KEY = "stemwise.prefs.v1";

export function loadPrefs(): UiPrefs {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULT_PREFS };
    const v = JSON.parse(raw) as Partial<UiPrefs>;
    return { ...DEFAULT_PREFS, ...v };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

export function savePrefs(p: UiPrefs) {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    // Not persisted: the preference still applies to this session.
  }
}
