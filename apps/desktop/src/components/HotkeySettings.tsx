import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export type HotkeyAction = "startRecording" | "pauseRecording" | "stopRecording" | "takeScreenshot";

export type HotkeyBinding = {
  action: HotkeyAction;
  shortcut: string;
  enabled: boolean;
};

const ACTIONS: HotkeyAction[] = [
  "startRecording",
  "pauseRecording",
  "stopRecording",
  "takeScreenshot",
];

const ACTION_LABEL: Record<HotkeyAction, string> = {
  startRecording: "hotkeyStart",
  pauseRecording: "hotkeyPause",
  stopRecording: "hotkeyStop",
  takeScreenshot: "hotkeyScreenshot",
};

/** Build a Capto/Tauri shortcut string from a KeyboardEvent. */
export function shortcutFromEvent(e: KeyboardEvent): string | null {
  if (e.key === "Escape" || e.key === "Tab") return null;
  const isMod = e.key === "Control" || e.key === "Shift" || e.key === "Alt" || e.key === "Meta";
  if (isMod) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Control");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  let key: string | null = null;
  if (/^F\d{1,2}$/i.test(e.key)) {
    key = e.key.toUpperCase();
  } else if (/^F\d{1,2}$/i.test(e.code)) {
    key = e.code.toUpperCase();
  } else if (e.code.startsWith("Key") && e.code.length === 4) {
    key = e.code.slice(3);
  } else if (e.code.startsWith("Digit") && e.code.length === 6) {
    key = e.code.slice(5);
  } else if (e.key.length === 1 && /[a-z0-9]/i.test(e.key)) {
    key = e.key.toUpperCase();
  }
  if (!key) return null;

  // Require at least one modifier for global hotkeys (avoids hijacking plain keys).
  if (parts.length === 0) return null;

  // Windows: Alt+F4 closes the focused window — never allow it.
  if (
    parts.includes("Alt") &&
    !parts.includes("Control") &&
    !parts.includes("Shift") &&
    key === "F4"
  ) {
    return null;
  }

  parts.push(key);
  return parts.join("+");
}

export function formatShortcut(shortcut: string): string {
  return shortcut
    .split("+")
    .map((p) => {
      const t = p.trim();
      if (/^CommandOrControl$/i.test(t) || /^Ctrl$/i.test(t) || /^Control$/i.test(t)) {
        return "Ctrl";
      }
      if (/^Alt$/i.test(t) || /^Option$/i.test(t)) return "Alt";
      if (/^Shift$/i.test(t)) return "Shift";
      if (/^Super$/i.test(t) || /^Meta$/i.test(t) || /^Command$/i.test(t)) return "Win";
      return t;
    })
    .join(" + ");
}

function ensureFour(hotkeys: HotkeyBinding[]): HotkeyBinding[] {
  const defaults: HotkeyBinding[] = [
    { action: "startRecording", shortcut: "Alt+F5", enabled: true },
    { action: "pauseRecording", shortcut: "Alt+F6", enabled: true },
    { action: "stopRecording", shortcut: "Alt+F7", enabled: true },
    { action: "takeScreenshot", shortcut: "Alt+F8", enabled: true },
  ];
  return defaults.map((d) => hotkeys.find((h) => h.action === d.action) ?? d);
}

/** Settings block: start / pause / stop / screenshot — click to rebind. */
export function HotkeySettings({
  hotkeys,
  conflicts = [],
  onChange,
}: {
  hotkeys: HotkeyBinding[];
  conflicts?: string[];
  onChange: (next: HotkeyBinding[]) => void;
}) {
  const { t } = useTranslation();
  const rows = ensureFour(hotkeys ?? []);
  const [listening, setListening] = useState<HotkeyAction | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rowsRef = useRef(rows);
  rowsRef.current = rows;

  useEffect(() => {
    if (!listening) return;

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setListening(null);
        setError(null);
        return;
      }
      if (e.altKey && (e.key === "F4" || e.code === "F4")) {
        setError(t("hotkeyAltF4Blocked"));
        return;
      }
      const shortcut = shortcutFromEvent(e);
      if (!shortcut) return;

      const action = listening;
      const next = ensureFour(rowsRef.current).map((h) =>
        h.action === action ? { ...h, shortcut, enabled: true } : h,
      );
      if (next.filter((h) => h.shortcut.toLowerCase() === shortcut.toLowerCase()).length > 1) {
        setError(t("hotkeyDuplicate"));
        return;
      }
      setError(null);
      setListening(null);
      onChange(next);
    }

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [listening, onChange, t]);

  function resetDefaults() {
    setListening(null);
    setError(null);
    onChange(ensureFour([]));
  }

  return (
    <div className="hotkey-settings">
      <div className="card-label">{t("hotkeys")}</div>
      <p className="hint">{t("hotkeysHint")}</p>
      <div className="hotkey-list">
        {ACTIONS.map((action) => {
          const binding = rows.find((h) => h.action === action)!;
          const active = listening === action;
          const unavailable = conflicts.some(
            (shortcut) =>
              shortcut.replace(/\s/g, "").toLowerCase() ===
              binding.shortcut.replace(/\s/g, "").toLowerCase(),
          );
          return (
            <div key={action} className="hotkey-row">
              <span className="hotkey-action">{t(ACTION_LABEL[action])}</span>
              <button
                type="button"
                aria-invalid={unavailable || undefined}
                className={`hotkey-bind ${active ? "listening" : ""} ${unavailable ? "conflict" : ""}`}
                onClick={() => {
                  setError(null);
                  setListening(active ? null : action);
                }}
              >
                {active ? t("hotkeyPress") : formatShortcut(binding.shortcut)}
              </button>
              {unavailable && <span className="hotkey-conflict">{t("hotkeyUnavailable")}</span>}
            </div>
          );
        })}
      </div>
      {error && <p className="hotkey-error">{error}</p>}
      <button type="button" className="ghost-btn settings-reset" onClick={resetDefaults}>
        {t("hotkeyReset")}
      </button>
    </div>
  );
}
