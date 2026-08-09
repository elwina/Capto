import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";

export interface PickedWindow {
  id: number;
  title: string;
  appName: string;
  width: number;
  height: number;
  x: number;
  y: number;
}

/** Fullscreen Captura-style window picker: hover highlight, click to select, Esc cancel.
 *  One overlay window per monitor — map GetWindowRect with this window's origin/scale. */
export function WindowPickerOverlay() {
  const { t } = useTranslation();
  const [hover, setHover] = useState<PickedWindow | null>(null);
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [scale, setScale] = useState(1);

  useEffect(() => {
    let alive = true;
    const win = getCurrentWindow();

    async function refreshGeometry() {
      try {
        const [pos, factor, outer] = await Promise.all([
          win.outerPosition(),
          win.scaleFactor(),
          win.outerSize(),
        ]);
        if (!alive) return;
        setOrigin({ x: pos.x, y: pos.y });
        setScale(factor || 1);
        setSize({ width: outer.width, height: outer.height });
      } catch {
        /* ignore */
      }
    }

    void refreshGeometry();
    const geoTick = window.setInterval(() => {
      void refreshGeometry();
    }, 500);

    const tick = window.setInterval(() => {
      void invoke<PickedWindow | null>("window_under_cursor")
        .then((w) => {
          if (alive) setHover(w);
        })
        .catch(() => undefined);
    }, 40);

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        void invoke("close_window_picker");
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      alive = false;
      window.clearInterval(tick);
      window.clearInterval(geoTick);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  async function select() {
    if (!hover) return;
    await emit("picker://window-selected", hover);
    await invoke("close_window_picker");
  }

  const onThisMonitor =
    !!hover &&
    size.width > 0 &&
    hover.x < origin.x + size.width &&
    hover.x + hover.width > origin.x &&
    hover.y < origin.y + size.height &&
    hover.y + hover.height > origin.y;

  return (
    <div className="picker-root" onClick={() => void select()}>
      <div className="picker-banner">
        {t("pickWindowHint")}
        {hover && onThisMonitor ? ` — ${hover.title}` : ""}
      </div>
      {hover && onThisMonitor && (
        <div
          className="picker-highlight"
          style={{
            // GetWindowRect is physical; CSS layout is logical DIP for this monitor.
            left: (hover.x - origin.x) / scale,
            top: (hover.y - origin.y) / scale,
            width: hover.width / scale,
            height: hover.height / scale,
          }}
        >
          <span className="picker-title">{hover.title}</span>
        </div>
      )}
    </div>
  );
}
