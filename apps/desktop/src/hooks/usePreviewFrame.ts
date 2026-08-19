import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type PreviewSource = "display" | "window" | "region";
export type Region = { x: number; y: number; width: number; height: number };

export type MaskRect = { x: number; y: number; width: number; height: number };

export type PreviewFrame = {
  width: number;
  height: number;
  /** Native capture size before JPEG downscale; webcam PiP sizes use this space. */
  sourceWidth?: number;
  sourceHeight?: number;
  jpeg?: number[];
  timestampMs: number;
  appMasked: boolean;
  maskRect?: MaskRect | null;
};

export type PreviewFrameState = {
  frame: PreviewFrame | null;
  imageUrl: string | null;
  error: string | null;
  ready: boolean;
};

/**
 * Polls native preview frames. Owned once at app level so switching sidebar
 * tabs reuses the running loop and the last frame instead of restarting it.
 */
export function usePreviewFrame({
  enabled,
  source,
  displayId,
  windowId,
  region,
  intervalMs = 100,
}: {
  enabled: boolean;
  source: PreviewSource;
  displayId: number;
  windowId: number | null;
  region: Region | null;
  intervalMs?: number;
}): PreviewFrameState {
  const [frame, setFrame] = useState<PreviewFrame | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const urlRef = useRef<string | null>(null);
  const ready = source === "display" || (source === "window" ? windowId !== null : region !== null);

  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;

    const clearImage = () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      urlRef.current = null;
      setImageUrl(null);
      setFrame(null);
    };

    async function tick() {
      if (cancelled || !enabled || !ready) return;
      try {
        const next = await invoke<PreviewFrame>("capture_preview", {
          args: { source, displayId, windowId, region },
        });
        if (cancelled) return;
        const bytes = next.jpeg ?? [];
        const url = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: "image/jpeg" }));
        if (urlRef.current) URL.revokeObjectURL(urlRef.current);
        urlRef.current = url;
        setImageUrl(url);
        // Drop jpeg bytes from React state — blob URL is enough for the <img>.
        setFrame({
          width: next.width,
          height: next.height,
          sourceWidth: next.sourceWidth,
          sourceHeight: next.sourceHeight,
          timestampMs: next.timestampMs,
          appMasked: next.appMasked,
          maskRect: next.maskRect,
        });
        setError(null);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) timer = window.setTimeout(() => void tick(), intervalMs);
      }
    }

    if (enabled && ready) {
      void tick();
    } else {
      clearImage();
      void invoke("release_preview_session").catch(() => undefined);
    }

    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [displayId, enabled, ready, region, source, windowId, intervalMs]);

  useEffect(
    () => () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      urlRef.current = null;
      void invoke("release_preview_session").catch(() => undefined);
    },
    [],
  );

  return { frame, imageUrl, error, ready };
}
