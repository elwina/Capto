import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

export type WebcamSoloFrame = {
  width: number;
  height: number;
  jpeg: number[];
  timestampMs: number;
};

/** Polls Rust MF webcam JPEG for the Webcam settings page. */
export function useWebcamSoloPreview(
  enabled: boolean,
  deviceId: string | null | undefined,
  intervalMs = 66,
) {
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const urlRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;

    const clear = () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      urlRef.current = null;
      setImageUrl(null);
      setReady(false);
    };

    async function tick() {
      if (cancelled || !enabled) return;
      try {
        const next = await invoke<WebcamSoloFrame>("capture_webcam_preview", {
          deviceId: deviceId || null,
        });
        if (cancelled) return;
        const url = URL.createObjectURL(
          new Blob([Uint8Array.from(next.jpeg)], { type: "image/jpeg" }),
        );
        if (urlRef.current) URL.revokeObjectURL(urlRef.current);
        urlRef.current = url;
        setImageUrl(url);
        setError(null);
        setReady(true);
      } catch (e) {
        if (!cancelled) {
          setError(String(e));
          setReady(false);
        }
      } finally {
        if (!cancelled && enabled) {
          timer = window.setTimeout(tick, intervalMs);
        }
      }
    }

    if (!enabled) {
      // Stop polling only — do not release MF here. Recording may be about to
      // take ownership of the same capture for a zero-gap PiP start.
      clear();
      setError(null);
      return;
    }

    void tick();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
      clear();
    };
  }, [enabled, deviceId, intervalMs]);

  return { imageUrl, error, ready };
}
