import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type ClickEvt = {
  button: "left" | "right" | "middle";
  x: number;
  y: number;
  color: string;
  radius: number;
  id: number;
};

type KeyEvt = {
  label: string;
  id: number;
  fontSize: number;
  color: string;
  background: string;
};

function cssColor(c: string): string {
  // Support #RRGGBBAA from settings on older CSS paths.
  if (/^#[0-9a-fA-F]{8}$/.test(c)) {
    const r = parseInt(c.slice(1, 3), 16);
    const g = parseInt(c.slice(3, 5), 16);
    const b = parseInt(c.slice(5, 7), 16);
    const a = parseInt(c.slice(7, 9), 16) / 255;
    return `rgba(${r},${g},${b},${a})`;
  }
  return c;
}

/** Fullscreen click-through overlay captured by gdigrab during recording. */
export function RecordOverlayRuntime() {
  const [clicks, setClicks] = useState<ClickEvt[]>([]);
  const [keys, setKeys] = useState<KeyEvt[]>([]);

  useEffect(() => {
    document.documentElement.classList.add("record-overlay-mode");
    document.body.classList.add("record-overlay-mode");
    return () => {
      document.documentElement.classList.remove("record-overlay-mode");
      document.body.classList.remove("record-overlay-mode");
    };
  }, []);

  useEffect(() => {
    let alive = true;
    const unsubs: Array<() => void> = [];
    void (async () => {
      unsubs.push(
        await listen<ClickEvt>("overlay://click", (ev) => {
          if (!alive) return;
          const item = ev.payload;
          setClicks((prev) => [...prev.slice(-20), item]);
          window.setTimeout(() => {
            setClicks((prev) => prev.filter((c) => c.id !== item.id));
          }, 550);
        }),
      );
      unsubs.push(
        await listen<KeyEvt>("overlay://key", (ev) => {
          if (!alive) return;
          const item = ev.payload;
          // Replace an existing chip with the same label instead of stacking repeats.
          setKeys((prev) => {
            const without = prev.filter((k) => k.label !== item.label);
            return [...without.slice(-7), item];
          });
          window.setTimeout(() => {
            setKeys((prev) => prev.filter((k) => k.id !== item.id));
          }, 1800);
        }),
      );
      unsubs.push(
        await listen("overlay://clear", () => {
          setClicks([]);
          setKeys([]);
        }),
      );
    })();
    return () => {
      alive = false;
      unsubs.forEach((u) => u());
    };
  }, []);

  return (
    <div className="record-overlay-root">
      {clicks.map((c) => (
        <span
          key={c.id}
          className="record-click-ripple"
          style={{
            left: c.x,
            top: c.y,
            width: c.radius * 2,
            height: c.radius * 2,
            marginLeft: -c.radius,
            marginTop: -c.radius,
            borderColor: c.color,
            background: `${c.color}55`,
          }}
        />
      ))}
      {keys.length > 0 && (
        <div className="record-key-stack">
          {keys.map((k) => (
            <div
              key={k.id}
              className="record-key-chip"
              style={{
                fontSize: Math.max(18, k.fontSize),
                color: cssColor(k.color),
                background: cssColor(k.background),
              }}
            >
              {k.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
