import { describe, expect, it } from "vitest";
import { formatShortcut, shortcutFromEvent } from "./HotkeySettings";

/** Build a minimal KeyboardEvent-like object with defaulted modifier flags. */
function kev(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "",
    code: "",
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...partial,
  } as KeyboardEvent;
}

describe("shortcutFromEvent", () => {
  it("ignores Escape and Tab", () => {
    expect(shortcutFromEvent(kev({ key: "Escape", code: "Escape", ctrlKey: true }))).toBeNull();
    expect(shortcutFromEvent(kev({ key: "Tab", code: "Tab", ctrlKey: true }))).toBeNull();
  });

  it("ignores modifier-only presses", () => {
    expect(shortcutFromEvent(kev({ key: "Control", code: "ControlLeft" }))).toBeNull();
    expect(shortcutFromEvent(kev({ key: "Shift", code: "ShiftLeft" }))).toBeNull();
    expect(shortcutFromEvent(kev({ key: "Alt", code: "AltLeft" }))).toBeNull();
    expect(shortcutFromEvent(kev({ key: "Meta", code: "MetaLeft" }))).toBeNull();
  });

  it("requires at least one modifier (never hijacks plain keys)", () => {
    expect(shortcutFromEvent(kev({ key: "a", code: "KeyA" }))).toBeNull();
  });

  it("builds 'Control+A' from key + code", () => {
    expect(
      shortcutFromEvent(kev({ key: "a", code: "KeyA", ctrlKey: true })),
    ).toBe("Control+A");
  });

  it("orders modifiers deterministically: Control, Alt, Shift, Super", () => {
    expect(
      shortcutFromEvent(
        kev({ key: "g", code: "KeyG", ctrlKey: true, shiftKey: true }),
      ),
    ).toBe("Control+Shift+G");
    expect(
      shortcutFromEvent(
        kev({ key: "s", code: "KeyS", metaKey: true, ctrlKey: true }),
      ),
    ).toBe("Control+Super+S");
  });

  it("resolves function keys from the KeyboardEvent key/code", () => {
    expect(
      shortcutFromEvent(kev({ key: "F5", code: "F5", ctrlKey: true })),
    ).toBe("Control+F5");
    // Lowercase key (some layouts / sticky situations) still matches.
    expect(
      shortcutFromEvent(kev({ key: "f3", code: "F3", altKey: true })),
    ).toBe("Alt+F3");
  });

  it("resolves digit keys via the Digit code", () => {
    expect(
      shortcutFromEvent(kev({ key: "1", code: "Digit1", ctrlKey: true })),
    ).toBe("Control+1");
  });

  it("blocks bare Alt+F4 (would close the focused window)", () => {
    expect(
      shortcutFromEvent(kev({ key: "F4", code: "F4", altKey: true })),
    ).toBeNull();
  });

  it("allows Alt+F4 when another modifier is held", () => {
    expect(
      shortcutFromEvent(kev({ key: "F4", code: "F4", altKey: true, ctrlKey: true })),
    ).toBe("Control+Alt+F4");
  });

  it("returns null for keys that resolve to nothing (e.g. Enter)", () => {
    expect(
      shortcutFromEvent(kev({ key: "Enter", code: "Enter", ctrlKey: true })),
    ).toBeNull();
  });
});

describe("formatShortcut", () => {
  it("renders human-readable labels with ' + ' separators", () => {
    expect(formatShortcut("Control+S")).toBe("Ctrl + S");
    expect(formatShortcut("Alt+F5")).toBe("Alt + F5");
    expect(formatShortcut("Control+Shift+G")).toBe("Ctrl + Shift + G");
    expect(formatShortcut("Control+Alt+Delete")).toBe("Ctrl + Alt + Delete");
  });

  it("normalizes aliases to their platform labels", () => {
    expect(formatShortcut("CommandOrControl+P")).toBe("Ctrl + P");
    expect(formatShortcut("Ctrl+P")).toBe("Ctrl + P");
    // Meta/Command/Super are all the Windows logo key on this platform.
    expect(formatShortcut("Meta+Shift+S")).toBe("Win + Shift + S");
    expect(formatShortcut("Command+F")).toBe("Win + F");
    expect(formatShortcut("Super+E")).toBe("Win + E");
    expect(formatShortcut("Option+A")).toBe("Alt + A");
  });

  it("trims whitespace around tokens", () => {
    expect(formatShortcut(" Control + Shift + R ")).toBe("Ctrl + Shift + R");
  });
});
