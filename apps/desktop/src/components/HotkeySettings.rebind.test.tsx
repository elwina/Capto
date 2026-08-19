// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HotkeySettings, type HotkeyBinding } from "./HotkeySettings";
import "../i18n"; // initializes i18next so useTranslation() resolves copy

const DEFAULT_HOTKEYS: HotkeyBinding[] = [
  { action: "startRecording", shortcut: "Alt+F5", enabled: true },
  { action: "pauseRecording", shortcut: "Alt+F6", enabled: true },
  { action: "stopRecording", shortcut: "Alt+F7", enabled: true },
  { action: "takeScreenshot", shortcut: "Alt+F8", enabled: true },
];

describe("HotkeySettings", () => {
  it("renders a binding row for every action plus the reset button", () => {
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={vi.fn()} />);
    expect(screen.getByText("Global hotkeys")).toBeTruthy();
    expect(screen.getByText("Alt + F5")).toBeTruthy();
    expect(screen.getByText("Alt + F6")).toBeTruthy();
    expect(screen.getByText("Alt + F7")).toBeTruthy();
    expect(screen.getByText("Alt + F8")).toBeTruthy();
    expect(screen.getByText("Reset defaults (Alt+F5–F8)")).toBeTruthy();
  });

  it("enters listening mode after clicking a binding", () => {
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={vi.fn()} />);
    fireEvent.click(screen.getByText("Alt + F5"));
    expect(screen.getByText("Press keys…")).toBeTruthy();
  });

  it("records the new combo and reports it through onChange", () => {
    const onChange = vi.fn();
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={onChange} />);
    fireEvent.click(screen.getByText("Alt + F5"));
    fireEvent.keyDown(window, { key: "X", code: "KeyX", ctrlKey: true, shiftKey: true });
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0][0] as HotkeyBinding[];
    expect(next.find((h) => h.action === "startRecording")).toEqual({
      action: "startRecording",
      shortcut: "Control+Shift+X",
      enabled: true,
    });
    // listening mode exits with the selection
    expect(screen.queryByText("Press keys…")).toBeNull();
  });

  it("rejects Alt+F4 and keeps listening", () => {
    const onChange = vi.fn();
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={onChange} />);
    fireEvent.click(screen.getByText("Alt + F5"));
    fireEvent.keyDown(window, { key: "F4", code: "F4", altKey: true });
    expect(screen.getByText("Alt+F4 closes windows — pick another shortcut")).toBeTruthy();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("rejects a shortcut already used by another binding", () => {
    const onChange = vi.fn();
    const withDuplicate = DEFAULT_HOTKEYS.map((h) =>
      h.action === "stopRecording" ? { ...h, shortcut: "Control+A" } : h,
    );
    render(<HotkeySettings hotkeys={withDuplicate} onChange={onChange} />);
    fireEvent.click(screen.getByText("Alt + F5"));
    fireEvent.keyDown(window, { key: "a", code: "KeyA", ctrlKey: true });
    expect(screen.getByText("That shortcut is already in use")).toBeTruthy();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("leaves listening mode when Escape is pressed", () => {
    const onChange = vi.fn();
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={onChange} />);
    fireEvent.click(screen.getByText("Alt + F5"));
    fireEvent.keyDown(window, { key: "Escape", code: "Escape" });
    expect(screen.queryByText("Press keys…")).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("resets to defaults", () => {
    const onChange = vi.fn();
    render(<HotkeySettings hotkeys={DEFAULT_HOTKEYS} onChange={onChange} />);
    fireEvent.click(screen.getByText("Reset defaults (Alt+F5–F8)"));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0] as HotkeyBinding[]).toEqual(DEFAULT_HOTKEYS);
  });
});
