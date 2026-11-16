// How a shortcut reads on this computer. The app writes shortcuts the Mac way (⇧⌘S); on Windows and Linux they
// read Ctrl+Shift+S, which is what the handlers already accept there (`metaKey || ctrlKey`).

import { IS_MAC } from "./platform";

const MODS: Record<string, string> = { "⌘": "Ctrl", "⌥": "Alt", "⇧": "Shift" };
const ORDER = ["Ctrl", "Alt", "Shift"];
const ALONE: Record<string, string> = { "⌫": "Del", "↩": "Enter" };

/** Every shortcut in `text` (a label, a tooltip, a key column) written for this platform. */
export function keysInText(text: string, mac = IS_MAC): string {
  if (mac) return text;
  return text
    .replace(/([⌘⌥⇧]+)([A-Z0-9,+−=↑↓]?)/g, (_, mods: string, key: string) => {
      const names = [...mods].map((m) => MODS[m]).sort((a, b) => ORDER.indexOf(a) - ORDER.indexOf(b));
      return key ? [...names, key].join("+") : names.join("+");
    })
    .replace(/[⌫↩]/g, (c) => ALONE[c]);
}
