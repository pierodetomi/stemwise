import { describe, expect, it } from "vitest";
import { keysInText } from "./keys";

// req: UI-3
describe("shortcut labels per platform", () => {
  it("stay as they are on the Mac", () => {
    expect(keysInText("Settings (⌘,)", true)).toBe("Settings (⌘,)");
  });

  it("read Ctrl, Alt and Shift elsewhere, in the usual order", () => {
    expect(keysInText("⇧⌘S", false)).toBe("Ctrl+Shift+S");
    expect(keysInText("⌥⌘S", false)).toBe("Ctrl+Alt+S");
    expect(keysInText("⌘⇧G", false)).toBe("Ctrl+Shift+G");
    expect(keysInText("Settings (⌘,)", false)).toBe("Settings (Ctrl+,)");
    expect(keysInText("⌥⌘−", false)).toBe("Ctrl+Alt+−");
    expect(keysInText("⇧B", false)).toBe("Shift+B");
    expect(keysInText("⌥↑ ⌥↓ to move · ⌫ to remove", false)).toBe("Alt+↑ Alt+↓ to move · Del to remove");
  });

  it("name a held modifier without a key", () => {
    expect(keysInText("Fade in (⌥ for linear)", false)).toBe("Fade in (Alt for linear)");
    expect(keysInText("(⌥-drag anywhere on the ruler)", false)).toBe("(Alt-drag anywhere on the ruler)");
    expect(keysInText("Hold ⇧ for fine steps.", false)).toBe("Hold Shift for fine steps.");
  });
});
