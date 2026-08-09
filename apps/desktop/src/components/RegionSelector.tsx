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

/**
 * Region picker only — drag and release to select. Esc cancels.
 * Recording / screenshot stay on the main Capto window.
 */
export function RegionSelector({
  onApply,
  onCancel,
  standalone = false,
}: {
  onApply: (r: Region) => void;
  onCancel: () => void;
  standalone?: boolean;
}) {
  const { t } = useTranslation();
  const [startClient, setStartClient] = useState<Point | null>(null);
  const [currentClient, setCurrentClient] = useState<Point | null>(null);
  const [startScreen, setStartScreen] = useState<Point | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || busy) return;
      if (standalone) void invoke("close_region_picker");
      else onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel, standalone]);

  const drafting =
    startClient && currentClient
      ? {
          x: Math.min(startClient.x, currentClient.x),
          y: Math.min(startClient.y, currentClient.y),
          width: Math.abs(currentClient.x - startClient.x),
          height: Math.abs(currentClient.y - startClient.y),
        }
      : null;

  async function readCursor(): Promise<Point> {
    return invoke<Point>("cursor_position");
  }

  async function finishDrag() {
    if (busy) return;
    if (!drafting || drafting.width < 8 || drafting.height < 8 || !startScreen) {
      setStartClient(null);
      setCurrentClient(null);
      setStartScreen(null);
      return;
    }
    setBusy(true);
    try {
      const end = await readCursor();
      const screen: Region = {
        x: Math.min(startScreen.x, end.x),
        y: Math.min(startScreen.y, end.y),
        width: Math.max(2, Math.abs(end.x - startScreen.x)),
        height: Math.max(2, Math.abs(end.y - startScreen.y)),
      };
      if (standalone) {
        await emit("picker://region-selected", { region: screen });
        await invoke("close_region_picker");
      } else {
        onApply(screen);
      }
    } catch {
      setBusy(false);
      setStartClient(null);
      setCurrentClient(null);
      setStartScreen(null);
    }
  }

  return (
    <div
      className="region-overlay"
      onMouseDown={(e) => {
        if (busy) return;
        e.preventDefault();
        const client = { x: e.clientX, y: e.clientY };
        setStartClient(client);
        setCurrentClient(client);
        void readCursor().then(setStartScreen);
      }}
      onMouseMove={(e) => {
        if (!startClient || busy) return;
        setCurrentClient({ x: e.clientX, y: e.clientY });
      }}
      onMouseUp={() => {
        void finishDrag();
      }}
    >
      {drafting && (
        <div
          className="region-box"
          style={{
            left: drafting.x,
            top: drafting.y,
            width: drafting.width,
            height: drafting.height,
          }}
        >
          <div className="region-toolbar">
            <span className="region-size mono">
              {Math.round(drafting.width)}×{Math.round(drafting.height)}
            </span>
          </div>
        </div>
      )}
      {!drafting && <div className="region-hint">{t("selectRegionHint")}</div>}
    </div>
  );
}
