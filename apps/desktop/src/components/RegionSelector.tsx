import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";

interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface Point {
  x: number;
  y: number;
}

/** Captura-like region chrome: drag to select, then Record / Shot / Cancel.
 *  Final region uses physical cursor positions (matches FFmpeg gdigrab). */
export function RegionSelector({
  onApply,
  onCancel,
  onRecord,
  onShot,
  standalone = false,
}: {
  onApply: (r: Region) => void;
  onCancel: () => void;
  onRecord?: (r: Region) => void;
  onShot?: (r: Region) => void;
  /** When true, runs inside the dedicated region-picker window. */
  standalone?: boolean;
}) {
  const { t } = useTranslation();
  const [startClient, setStartClient] = useState<Point | null>(null);
  const [currentClient, setCurrentClient] = useState<Point | null>(null);
  const [startScreen, setStartScreen] = useState<Point | null>(null);
  const [committedClient, setCommittedClient] = useState<Region | null>(null);
  const [committedScreen, setCommittedScreen] = useState<Region | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (standalone) void invoke("close_region_picker");
        else onCancel();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel, standalone]);

  const drafting =
    startClient && currentClient
      ? {
          x: Math.min(startClient.x, currentClient.x),
          y: Math.min(startClient.y, currentClient.y),
          width: Math.abs(currentClient.x - startClient.x),
          height: Math.abs(currentClient.y - startClient.y),
        }
      : null;

  const rect = committedClient ?? drafting;

  async function readCursor(): Promise<Point> {
    return invoke<Point>("cursor_position");
  }

  async function finishDrag() {
    if (!drafting || drafting.width < 8 || drafting.height < 8 || !startScreen) {
      if (!committedClient) {
        if (standalone) void invoke("close_region_picker");
        else onCancel();
      }
      return;
    }
    const end = await readCursor();
    const screen: Region = {
      x: Math.min(startScreen.x, end.x),
      y: Math.min(startScreen.y, end.y),
      width: Math.max(2, Math.abs(end.x - startScreen.x)),
      height: Math.max(2, Math.abs(end.y - startScreen.y)),
    };
    setCommittedClient(drafting);
    setCommittedScreen(screen);
    setStartClient(null);
    setCurrentClient(null);
    setStartScreen(null);
  }

  async function emitRegion(action: "apply" | "record" | "shot") {
    if (!committedScreen) return;
    if (standalone) {
      await emit("picker://region-selected", { region: committedScreen, action });
      await invoke("close_region_picker");
      return;
    }
    onApply(committedScreen);
    if (action === "record") onRecord?.(committedScreen);
    if (action === "shot") onShot?.(committedScreen);
  }

  return (
    <div
      className="region-overlay"
      onMouseDown={(e) => {
        if (committedClient) return;
        const client = { x: e.clientX, y: e.clientY };
        setStartClient(client);
        setCurrentClient(client);
        void readCursor().then(setStartScreen);
      }}
      onMouseMove={(e) => {
        if (!startClient || committedClient) return;
        setCurrentClient({ x: e.clientX, y: e.clientY });
      }}
      onMouseUp={() => {
        void finishDrag();
      }}
    >
      {rect && (
        <div
          className="region-box"
          style={{
            left: rect.x,
            top: rect.y,
            width: rect.width,
            height: rect.height,
          }}
        >
          {committedClient && committedScreen && (
            <div className="region-toolbar" onMouseDown={(e) => e.stopPropagation()}>
              <button type="button" className="danger" onClick={() => void emitRegion("record")}>
                {t("record")}
              </button>
              <button type="button" className="secondary" onClick={() => void emitRegion("shot")}>
                {t("screenshot")}
              </button>
              <button
                type="button"
                className="ghost"
                onClick={() => {
                  if (standalone) void invoke("close_region_picker");
                  else onCancel();
                }}
              >
                {t("cancel")}
              </button>
              <span className="region-size mono">
                {committedScreen.width}×{committedScreen.height}
              </span>
            </div>
          )}
        </div>
      )}
      {!committedClient && <div className="region-hint">{t("selectRegionHint")}</div>}
    </div>
  );
}
