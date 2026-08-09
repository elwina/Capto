import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type WebcamDevice = {
  deviceId: string;
  label: string;
};

export type WebcamPreviewState = {
  devices: WebcamDevice[];
  stream: null;
  error: null;
  starting: false;
  retry: () => void;
  refreshDevices: () => Promise<void>;
  releaseNow: () => Promise<void>;
};

type RustWebcam = { id: string; name: string };

/** Device list only — live preview is composited into the main DXGI JPEG. */
export function useWebcamPreview(_enabled = false, _deviceId?: string | null): WebcamPreviewState {
  const [devices, setDevices] = useState<WebcamDevice[]>([]);

  const refreshDevices = useCallback(async () => {
    try {
      const list = await invoke<RustWebcam[]>("list_webcams");
      setDevices(
        list.map((d) => ({
          deviceId: d.id,
          label: d.name?.trim() || d.id,
        })),
      );
    } catch {
      setDevices([]);
    }
  }, []);

  useEffect(() => {
    void refreshDevices();
  }, [refreshDevices]);

  return {
    devices,
    stream: null,
    error: null,
    starting: false,
    retry: refreshDevices,
    refreshDevices,
    releaseNow: async () => undefined,
  };
}
