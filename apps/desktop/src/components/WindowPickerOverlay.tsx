import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface PickedWindow {
  id: number;
  title: string;
  appName: string;
  width: number;
  height: number;
  x: number;
  y: number;
}

/** Fullscreen Captura-style window picker: hover highlight, click to select, Esc cancel. */
export function WindowPickerOverlay() {
  const [hover, setHover] = useState<PickedWindow | null>(null);
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  const [scale, setScale] = useState(1);

  useEffect(() => {
    let alive = true;
    const win = getCurrentWindow();
    void Promise.all([win.outerPosition(), win.scaleFactor()])
      .then(([pos, factor]) => {
        if (!alive) return;
        setOrigin({ x: pos.x, y: pos.y });
        setScale(factor || 1);
      })
      .catch(() => undefined);

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
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  async function select() {
    if (!hover) return;
    await emit("picker://window-selected", hover);
    await invoke("close_window_picker");
  }

  return (
    <div className="picker-root" onClick={() => void select()}>
      <div className="picker-banner">
        将鼠标移到目标窗口上，单击选择 · Esc 取消
        {hover ? ` — ${hover.title}` : ""}
      </div>
      {hover && (
        <div
          className="picker-highlight"
          style={{
            // GetWindowRect is physical; CSS layout is logical DIP.
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
