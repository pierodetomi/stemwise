// Keyboard shortcuts (UI-3): one table, shown in Settings › Shortcuts and matched by the handlers. `keys` is how it
// is shown; `match` decides whether a keydown is this shortcut.

export interface Shortcut {
  id: string;
  group: "Project" | "Transport" | "Editing" | "Tools" | "History" | "View" | "Tracks";
  label: string;
  keys: string;
  match: (e: KeyboardEvent) => boolean;
}

const mod = (e: KeyboardEvent) => e.metaKey || e.ctrlKey;
const plain = (e: KeyboardEvent) => !mod(e) && !e.altKey;
const k = (e: KeyboardEvent) => e.key.toLowerCase();

export const SHORTCUTS: Shortcut[] = [
  // The app menu's commands (UI-20): once in the macOS menu bar, now handled here like every other shortcut.
  { id: "newProject", group: "Project", label: "New project", keys: "⌘N", match: (e) => k(e) === "n" && mod(e) && !e.shiftKey && !e.altKey },
  { id: "openProject", group: "Project", label: "Open project", keys: "⌘O", match: (e) => k(e) === "o" && mod(e) && !e.shiftKey && !e.altKey },
  { id: "play", group: "Transport", label: "Play / pause", keys: "Space", match: (e) => e.key === " " && !mod(e) },
  { id: "stop", group: "Transport", label: "Stop and return", keys: "↩", match: (e) => e.key === "Enter" && plain(e) },
  { id: "start", group: "Transport", label: "Return to start", keys: "Home", match: (e) => e.key === "Home" && plain(e) },
  { id: "loop", group: "Transport", label: "Loop on / off", keys: "L", match: (e) => k(e) === "l" && plain(e) && !e.shiftKey },
  { id: "loopSel", group: "Transport", label: "Set loop to selection", keys: "⌘L", match: (e) => k(e) === "l" && mod(e) && !e.shiftKey },
  { id: "follow", group: "Transport", label: "Follow playhead", keys: "F", match: (e) => k(e) === "f" && plain(e) && !e.shiftKey },
  { id: "split", group: "Editing", label: "Split at playhead", keys: "⌘T", match: (e) => k(e) === "t" && mod(e) },
  { id: "dup", group: "Editing", label: "Duplicate", keys: "⌘D", match: (e) => k(e) === "d" && mod(e) },
  { id: "del", group: "Editing", label: "Delete", keys: "⌫", match: (e) => (e.key === "Backspace" || e.key === "Delete") && !mod(e) },
  { id: "muteClip", group: "Editing", label: "Mute clip", keys: "M", match: (e) => k(e) === "m" && plain(e) && !e.shiftKey },
  { id: "snap", group: "Editing", label: "Snap on / off", keys: "N", match: (e) => k(e) === "n" && plain(e) && !e.shiftKey },
  { id: "copy", group: "Editing", label: "Copy automation range", keys: "⌘C", match: (e) => k(e) === "c" && mod(e) },
  { id: "paste", group: "Editing", label: "Paste automation at playhead", keys: "⌘V", match: (e) => k(e) === "v" && mod(e) },
  { id: "toolSelect", group: "Tools", label: "Select", keys: "V", match: (e) => k(e) === "v" && plain(e) && !e.shiftKey },
  { id: "toolSplit", group: "Tools", label: "Split", keys: "C", match: (e) => k(e) === "c" && plain(e) && !e.shiftKey },
  { id: "toolAuto", group: "Tools", label: "Automation", keys: "A", match: (e) => k(e) === "a" && plain(e) && !e.shiftKey },
  { id: "undo", group: "History", label: "Undo", keys: "⌘Z", match: (e) => k(e) === "z" && mod(e) && !e.shiftKey },
  { id: "redo", group: "History", label: "Redo", keys: "⇧⌘Z", match: (e) => k(e) === "z" && mod(e) && e.shiftKey },
  { id: "save", group: "History", label: "Save project", keys: "⌘S", match: (e) => k(e) === "s" && mod(e) && !e.shiftKey && !e.altKey },
  // ⇧⌘S is Save As on the Mac; the snapshot moved to ⌥⌘S (A-053). With ⌥ the key reads "ß": match the key's code.
  { id: "saveAs", group: "History", label: "Save project as…", keys: "⇧⌘S", match: (e) => k(e) === "s" && mod(e) && e.shiftKey && !e.altKey },
  { id: "snapshot", group: "History", label: "Save snapshot", keys: "⌥⌘S", match: (e) => (e.code === "KeyS" || k(e) === "s") && mod(e) && e.altKey && !e.shiftKey },
  { id: "ab", group: "History", label: "A/B (proposal or snapshot)", keys: "B", match: (e) => k(e) === "b" && plain(e) && !e.shiftKey },
  // Master (design 9l–9o): real level by default, the same loudness on demand (MAS-5).
  { id: "sameLoudness", group: "Transport", label: "Master: same loudness on / off", keys: "⇧B", match: (e) => k(e) === "b" && plain(e) && e.shiftKey },
  { id: "zoomIn", group: "View", label: "Zoom in", keys: "⌘+", match: (e) => (e.key === "+" || e.key === "=") && mod(e) && !e.altKey },
  { id: "zoomOut", group: "View", label: "Zoom out", keys: "⌘−", match: (e) => e.key === "-" && mod(e) && !e.altKey },
  { id: "fit", group: "View", label: "Zoom to fit", keys: "⌘0", match: (e) => e.key === "0" && mod(e) },
  { id: "taller", group: "View", label: "Taller lanes", keys: "⌥⌘+", match: (e) => (e.code === "Equal" || e.key === "+") && mod(e) && e.altKey },
  { id: "shorter", group: "View", label: "Shorter lanes", keys: "⌥⌘−", match: (e) => (e.code === "Minus" || e.key === "-") && mod(e) && e.altKey },
  { id: "mixer", group: "View", label: "Show / hide mixer", keys: "⌘M", match: (e) => k(e) === "m" && mod(e) },
  { id: "inspector", group: "View", label: "Show / hide inspector", keys: "⌘I", match: (e) => k(e) === "i" && mod(e) },
  { id: "assistant", group: "View", label: "Show / hide assistant", keys: "⌘J", match: (e) => k(e) === "j" && mod(e) },
  { id: "explain", group: "View", label: "Explain this mix", keys: "⌘E", match: (e) => k(e) === "e" && mod(e) && !e.shiftKey && !e.altKey },
  { id: "settings", group: "View", label: "Settings", keys: "⌘,", match: (e) => e.key === "," && mod(e) },
  { id: "muteTrack", group: "Tracks", label: "Mute track", keys: "⇧M", match: (e) => k(e) === "m" && e.shiftKey && !mod(e) },
  { id: "soloTrack", group: "Tracks", label: "Solo track", keys: "S", match: (e) => k(e) === "s" && plain(e) && !e.shiftKey },
  { id: "lockTrack", group: "Tracks", label: "Lock for AI", keys: "⇧L", match: (e) => k(e) === "l" && e.shiftKey && !mod(e) },
  { id: "group", group: "Tracks", label: "Group selected track", keys: "⇧⌘G", match: (e) => k(e) === "g" && mod(e) && e.shiftKey },
  { id: "automation", group: "Tracks", label: "Show fader automation", keys: "U", match: (e) => k(e) === "u" && plain(e) && !e.shiftKey },
];

/** The shortcut a keydown matches, if any. */
export function shortcutOf(e: KeyboardEvent): Shortcut["id"] | null {
  return SHORTCUTS.find((s) => s.match(e))?.id ?? null;
}

/** How a shortcut is shown (tooltips, menus). */
export function keysOf(id: string): string {
  return SHORTCUTS.find((s) => s.id === id)?.keys ?? "";
}
