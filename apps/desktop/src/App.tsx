import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { OverlayPanel } from "./components/OverlayPanel";
import { AboutPanel } from "./components/AboutPanel";
import { HotkeySettings, type HotkeyBinding } from "./components/HotkeySettings";
import { RecordOverlayRuntime } from "./components/RecordOverlayRuntime";
import { RecordingPreview } from "./components/RecordingPreview";
import { WebcamPanel } from "./components/WebcamPanel";
import { useWebcamPreview } from "./hooks/useWebcamPreview";
import { usePreviewFrame } from "./hooks/usePreviewFrame";
import { RegionSelector } from "./components/RegionSelector";
import { WindowPickerOverlay, type PickedWindow } from "./components/WindowPickerOverlay";
import i18n, { SUPPORTED_LOCALES } from "./i18n";

type VideoSource = "display" | "window" | "region";
type OutputFormat = "mp4" | "gif" | "audioOnly";
type Tab = "main" | "webcam" | "overlays" | "settings" | "about";

interface DisplayInfo {
  id: number;
  name: string;
  width: number;
  height: number;
  isPrimary: boolean;
}

interface WindowInfo {
  id: number;
  title: string;
  appName: string;
  width: number;
  height: number;
  x: number;
  y: number;
}

interface AudioDevice {
  id: string;
  name: string;
  kind: "input" | "output" | "loopback";
  isDefault: boolean;
}

interface EncoderInfo {
  kind: string;
  name: string;
  available: boolean;
  hardware: boolean;
}

interface SessionSnapshot {
  state: "idle" | "starting" | "recording" | "paused" | "stopping";
  elapsedMs: number;
  outputPath?: string | null;
  lastError?: string | null;
  encoder?: string | null;
  hideApp: boolean;
}

interface AudioLevels {
  microphone: number;
  system: number;
}

interface AppSettings {
  outputDir: string;
  outputFormat: OutputFormat;
  fps: number;
  quality: number;
  includeCursor: boolean;
  preferredEncoder?: string | null;
  micDevice?: string | null;
  loopbackDevice?: string | null;
  micVolume?: number;
  loopbackVolume?: number;
  defaultSource: VideoSource;
  defaultDisplayId?: number | null;
  defaultWindowId?: number | null;
  defaultWindowTitle?: string | null;
  defaultRegion?: { x: number; y: number; width: number; height: number } | null;
  hideAppWhileRecording: boolean;
  minimizeToTrayOnClose: boolean;
  showPreview: boolean;
  locale: string;
  hotkeys: HotkeyBinding[];
  overlays: Record<string, any>;
}

function formatMs(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const ss = String(s % 60).padStart(2, "0");
  const mm = String(m % 60).padStart(2, "0");
  const hh = String(h).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

/** Map linear WASAPI peak (0..1) to the familiar -60 dBFS..0 dBFS meter range. */
function audioMeterPercent(peak: number) {
  if (!Number.isFinite(peak) || peak <= 0) return 0;
  const db = 20 * Math.log10(Math.min(1, peak));
  return Math.max(0, Math.min(100, ((db + 60) / 60) * 100));
}

type IconPath = string | { d: string; stroke: true };

function Icon({ path, size = 18 }: { path: IconPath; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden>
      {typeof path === "string" ? (
        <path d={path} fill="currentColor" />
      ) : (
        <path
          d={path.d}
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
    </svg>
  );
}

const I: Record<string, IconPath> = {
  record:
    "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 5.5a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9Z",
  stop: "M7 7h10v10H7V7Z",
  pause: "M8 6h3v12H8V6Zm5 0h3v12h-3V6Z",
  play: "M8 5v14l11-7L8 5Z",
  camera:
    "M9 4 7.5 6H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-3.5L15 4H9Zm3 4.5A4.5 4.5 0 1 1 7.5 13 4.5 4.5 0 0 1 12 8.5Z",
  cursor: {
    stroke: true,
    d: "M5 3.5 19 12l-6.2 1.4L15.8 21l-2.6 1.1-3-7.5L5 17.5V3.5Z",
  },
  click: {
    stroke: true,
    d: "M5 3.5 19 12l-6.2 1.4L15.8 21l-2.6 1.1-3-7.5L5 17.5V3.5ZM17.5 4.5v-2M20.2 6.2l1.4-1.4M20.5 9h2",
  },
  keys: {
    stroke: true,
    d: "M4 6h16v12H4V6ZM7 10h.01M10 10h.01M13 10h.01M16 10h.01M7 14h6M16 14h1",
  },
  clock:
    "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm1 10.6 3.2 1.9-.9 1.5L11 13.5V7h2v5.6Z",
  refresh:
    "M12 6V3L8 7l4 4V8a4 4 0 1 1-4 4H6a6 6 0 1 0 6-6Z",
  folder:
    "M3 6h6l2 2h10v10H3V6Zm2 4v8h14v-6H10l-2-2H5Z",
  preview:
    "M12 5C7 5 2.7 8.1 1 12c1.7 3.9 6 7 11 7s9.3-3.1 11-7c-1.7-3.9-6-7-11-7Zm0 11a4 4 0 1 1 4-4 4 4 0 0 1-4 4Z",
  home:
    "M12 3 3 10h2v9h5v-5h4v5h5v-9h2L12 3Z",
  layers:
    "m12 3 9 5-9 5-9-5 9-5Zm0 8.5 9 5-9 5-9-5 9-5Z",
  gear:
    "M19.1 12.9a7.5 7.5 0 0 0 .1-1.8l2-1.5-2-3.4-2.4 1a7.6 7.6 0 0 0-1.6-.9L14.8 3h-4l-.4 2.3a7.6 7.6 0 0 0-1.6.9l-2.4-1-2 3.4 2 1.5a7.5 7.5 0 0 0-.1 1.8l-2 1.5 2 3.4 2.4-1c.5.4 1 .7 1.6.9L10.8 21h4l.4-2.3c.6-.2 1.1-.5 1.6-.9l2.4 1 2-3.4-2-1.5ZM12 15.5A3.5 3.5 0 1 1 15.5 12 3.5 3.5 0 0 1 12 15.5Z",
  screen:
    "M3 5h18v12H3V5Zm2 2v8h14V7H5Zm4 12h6v2H9v-2Z",
  window:
    "M4 5h16v14H4V5Zm2 2v2h12V7H6Zm0 4v6h12v-6H6Z",
  region:
    "M4 4h6v2H6v4H4V4Zm10 0h6v6h-2V6h-4V4ZM4 14h2v4h4v2H4v-6Zm14 0h2v6h-6v-2h4v-4Z",
  mic:
    "M12 2a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Zm-7 9h2a5 5 0 0 0 10 0h2a7 7 0 0 1-6 6.9V21h-2v-3.1A7 7 0 0 1 5 11Z",
  cam:
    "M4 7h11a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2Zm13 2.5 5-3v11l-5-3v-5Z",
  info:
    "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm1 15h-2v-6h2Zm0-8h-2V7h2Z",
};

function isWindowPickerLabel(l: string) {
  return l === "picker" || l.startsWith("picker-");
}

function isRegionPickerLabel(l: string) {
  return l === "region-picker" || l.startsWith("region-picker-");
}

export default function App() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const l = getCurrentWindow().label;
        if (cancelled) return;
        setLabel(l);
        document.documentElement.classList.toggle(
          "picker-mode",
          isWindowPickerLabel(l) || isRegionPickerLabel(l),
        );
        document.documentElement.classList.toggle(
          "record-overlay-mode",
          l === "record-overlay",
        );
      } catch {
        try {
          const l = await invoke<string>("get_window_label");
          if (!cancelled) setLabel(l);
        } catch {
          if (!cancelled) setLabel("main");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (label === "record-overlay") return <RecordOverlayRuntime />;
  if (label && isWindowPickerLabel(label)) return <WindowPickerOverlay />;
  if (label && isRegionPickerLabel(label)) {
    return (
      <RegionSelector
        standalone
        onCancel={() => void invoke("close_region_picker")}
        onApply={() => undefined}
      />
    );
  }
  if (label === null) return <div className="boot">Capto</div>;
  return <MainApp />;
}

function MainApp() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("main");
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [hotkeyConflicts, setHotkeyConflicts] = useState<string[]>([]);
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [audio, setAudio] = useState<AudioDevice[]>([]);
  const [encoders, setEncoders] = useState<EncoderInfo[]>([]);
  const [session, setSession] = useState<SessionSnapshot | null>(null);
  const [source, setSource] = useState<VideoSource>("display");
  const [displayId, setDisplayId] = useState(0);
  const [pickedWindow, setPickedWindow] = useState<WindowInfo | null>(null);
  const [region, setRegion] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const [mic, setMic] = useState("");
  const [loopback, setLoopback] = useState("");
  const [micVolume, setMicVolume] = useState(100);
  const [loopbackVolume, setLoopbackVolume] = useState(100);
  const [audioLevels, setAudioLevels] = useState<AudioLevels>({ microphone: 0, system: 0 });
  const [audioTesting, setAudioTesting] = useState(false);
  const [encoder, setEncoder] = useState("");
  const [format, setFormat] = useState<OutputFormat>("mp4");
  const [fps, setFps] = useState(30);
  const [fpsLimit, setFpsLimit] = useState(true);
  const [quality, setQuality] = useState(60);
  const [cursor, setCursor] = useState(true);
  const [lastShot, setLastShot] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [startingRecord, setStartingRecord] = useState(false);
  const [stoppingRecord, setStoppingRecord] = useState(false);
  const saveTimer = useRef<number | undefined>(undefined);
  const settingsRef = useRef<AppSettings | null>(null);
  settingsRef.current = settings;

  const recordOptsRef = useRef({
    displayId: 0,
    cursor: true,
    mic: "",
    loopback: "",
    micVolume: 100,
    loopbackVolume: 100,
    encoder: "",
    format: "mp4" as OutputFormat,
    fps: 30,
    quality: 60,
  });
  recordOptsRef.current = {
    displayId,
    cursor,
    mic,
    loopback,
    micVolume,
    loopbackVolume,
    encoder,
    format,
    fps: fpsLimit ? fps : 60,
    quality,
  };

  const refresh = useCallback(async () => {
    try {
      const [s, d, a, st, wins, conflicts] = await Promise.all([
        invoke<AppSettings>("get_settings"),
        invoke<DisplayInfo[]>("list_displays"),
        invoke<AudioDevice[]>("list_audio_devices"),
        invoke<SessionSnapshot>("get_session_state"),
        invoke<WindowInfo[]>("list_windows").catch(() => [] as WindowInfo[]),
        invoke<string[]>("get_hotkey_conflicts").catch(() => [] as string[]),
      ]);
      setSettings(s);
      setDisplays(d);
      setAudio(a);
      setSession(st);
      setHotkeyConflicts(conflicts);

      const displayOk =
        s.defaultDisplayId != null && d.some((x) => x.id === s.defaultDisplayId);
      setDisplayId(displayOk ? s.defaultDisplayId! : (d[0]?.id ?? 0));
      setRegion(s.defaultRegion ?? null);

      const knownAudio = (id: string | null | undefined) =>
        !!id && a.some((dev) => dev.id === id);
      setMic(knownAudio(s.micDevice) ? s.micDevice! : "");
      setLoopback(knownAudio(s.loopbackDevice) ? s.loopbackDevice! : "");
      setMicVolume(s.micVolume ?? 100);
      setLoopbackVolume(s.loopbackVolume ?? 100);

      setFormat(s.outputFormat);
      setFps(s.fps);
      setQuality(s.quality ?? 60);
      setCursor(s.includeCursor);

      let nextSource = s.defaultSource;
      let nextWindow: WindowInfo | null = null;
      if (s.defaultSource === "window") {
        nextWindow =
          wins.find((w) => w.id === s.defaultWindowId) ??
          wins.find((w) => w.title && w.title === s.defaultWindowTitle) ??
          null;
        if (nextWindow) {
          setRegion({
            x: nextWindow.x,
            y: nextWindow.y,
            width: nextWindow.width,
            height: nextWindow.height,
          });
        } else {
          nextSource = "display";
        }
      }
      setSource(nextSource);
      setPickedWindow(nextWindow);

      try {
        const enc = await invoke<EncoderInfo[]>("list_encoders");
        setEncoders(enc);
        const avail = enc.filter((e) => e.available && e.name !== "gif");
        const pref = s.preferredEncoder ?? "";
        const prefOk = !!pref && avail.some((e) => e.kind === pref);
        // An empty value is the intentional "Automatic" choice.  Do not
        // silently turn it into the first probed encoder; that would make a
        // session-specific choice look like a persisted default.
        setEncoder(prefOk ? pref : "");
        setError(null);
      } catch (e) {
        setEncoders([]);
        setEncoder("");
        setError(String(e));
      }
      if (s.locale && s.locale !== i18n.language) void i18n.changeLanguage(s.locale);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unSession = listen<SessionSnapshot>("session://state", (ev) => {
      setSession(ev.payload);
    });
    const unPick = listen<PickedWindow>("picker://window-selected", (ev) => {
      const w = ev.payload;
      setSource("window");
      setPickedWindow(w);
      // Capture path stores the window rect as a region crop of its monitor.
      setRegion({ x: w.x, y: w.y, width: w.width, height: w.height });
      setTab("main");
      const cur = settingsRef.current;
      if (cur) {
        void invoke("save_settings", {
          settings: {
            ...cur,
            defaultSource: "window",
            defaultWindowId: w.id,
            defaultWindowTitle: w.title,
            defaultRegion: { x: w.x, y: w.y, width: w.width, height: w.height },
          },
        }).catch(() => undefined);
      }
    });
    const unRegion = listen<{
      region: { x: number; y: number; width: number; height: number };
    }>("picker://region-selected", (ev) => {
      const r = ev.payload.region;
      setSource("region");
      setPickedWindow(null);
      setRegion(r);
      setTab("main");
      const cur = settingsRef.current;
      if (cur) {
        void invoke("save_settings", {
          settings: {
            ...cur,
            defaultSource: "region",
            defaultRegion: r,
            defaultWindowId: null,
            defaultWindowTitle: null,
          },
        }).catch(() => undefined);
      }
    });
    const timer = window.setInterval(() => {
      void invoke<SessionSnapshot>("get_session_state").then(setSession).catch(() => undefined);
    }, 500);
    return () => {
      void unSession.then((f) => f());
      void unPick.then((f) => f());
      void unRegion.then((f) => f());
      window.clearInterval(timer);
    };
  }, [refresh]);

  const inputs = useMemo(() => audio.filter((a) => a.kind === "input"), [audio]);
  const loops = useMemo(() => audio.filter((a) => a.kind === "loopback"), [audio]);
  const availableEncoders = useMemo(
    () => encoders.filter((e) => e.available && e.name !== "gif"),
    [encoders],
  );
  const automaticEncoder = useMemo(() => {
    const preferredOrder = ["h264Nvenc", "h264Qsv", "h264Amf", "libx264"];
    return preferredOrder
      .map((kind) => availableEncoders.find((encoder) => encoder.kind === kind))
      .find(Boolean);
  }, [availableEncoders]);
  const recording = session?.state === "recording" || session?.state === "paused";
  const sessionBusy =
    recording ||
    session?.state === "starting" ||
    session?.state === "stopping";
  const recordTransitionBusy = startingRecord || stoppingRecord;

  useEffect(() => {
    if (session?.state !== "recording" && !audioTesting) {
      setAudioLevels({ microphone: 0, system: 0 });
      return;
    }
    let cancelled = false;
    const poll = async () => {
      try {
        const levels = await invoke<AudioLevels>("get_audio_levels");
        if (!cancelled) setAudioLevels(levels);
      } catch {
        if (!cancelled) setAudioLevels({ microphone: 0, system: 0 });
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 100);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [audioTesting, session?.state]);
  const overlays = settings?.overlays ?? {};
  const webcam = overlays.webcam ?? {};
  const mouse = overlays.mouseClicks ?? {};
  const keys = overlays.keystrokes ?? {};
  const showPreview = !!settings?.showPreview;

  const camState = useWebcamPreview();

  // Preview switch only controls the stage; recording pauses the feed separately.
  const showPreviewStage = showPreview && !sessionBusy && format !== "audioOnly";
  const previewVisible =
    showPreviewStage && (tab === "main" || tab === "webcam");
  const previewState = usePreviewFrame({
    enabled: previewVisible,
    source,
    displayId,
    windowId: pickedWindow?.id ?? null,
    region,
  });

  // Migrate / fill webcam device ids from Rust MF list (old browser deviceIds won't match).
  useEffect(() => {
    if (!settings || !webcam.enabled) return;
    if (camState.devices.length === 0) return;
    const known = webcam.deviceId
      ? camState.devices.some((d) => d.deviceId === webcam.deviceId)
      : false;
    if (known && webcam.deviceLabel) return;
    const pick =
      (webcam.deviceId && camState.devices.find((d) => d.deviceId === webcam.deviceId)) ||
      camState.devices.find((d) => d.label === webcam.deviceLabel) ||
      camState.devices[0];
    if (!pick) return;
    if (webcam.deviceId === pick.deviceId && webcam.deviceLabel === pick.label) return;
    const next = {
      ...settings,
      overlays: {
        ...settings.overlays,
        webcam: {
          ...webcam,
          deviceId: pick.deviceId,
          deviceLabel: pick.label || null,
        },
      },
    };
    setSettings(next);
    void invoke("save_settings", { settings: next });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    camState.devices,
    webcam.enabled,
    webcam.deviceLabel,
    webcam.deviceId,
  ]);

  // Free the MF camera when PiP is turned off (settings tab may not be mounted).
  useEffect(() => {
    if (webcam.enabled) return;
    void invoke("release_preview_webcam").catch(() => undefined);
  }, [webcam.enabled]);

  async function saveSettings(next: AppSettings) {
    if (saveTimer.current !== undefined) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = undefined;
    }
    // Optimistic UI so toggles (preview etc.) react immediately.
    setSettings(next);
    void i18n.changeLanguage(next.locale);
    try {
      await invoke("save_settings", { settings: next });
      setHotkeyConflicts(await invoke<string[]>("get_hotkey_conflicts"));
    } catch (e) {
      setError(String(e));
    }
  }

  function toggleShowPreview() {
    if (!settings) return;
    const next = { ...settings, showPreview: !settings.showPreview };
    setSettings(next);
    if (saveTimer.current !== undefined) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = undefined;
    }
    void invoke("save_settings", { settings: next }).catch((e) => setError(String(e)));
  }

  async function chooseOutputDir() {
    if (!settings) return;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: settings.outputDir,
        title: t("chooseOutputDir"),
      });
      if (typeof selected === "string") {
        await saveSettings({ ...settings, outputDir: selected });
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function restoreDefaultOutputDir() {
    if (!settings) return;
    try {
      const outputDir = await invoke<string>("default_output_dir");
      await saveSettings({ ...settings, outputDir });
    } catch (e) {
      setError(String(e));
    }
  }

  // Live-only settings change: state updates instantly for the preview, but the
  // disk/backend write is coalesced so dragging the PiP size doesn't hammer it.
  function saveSettingsLive(next: AppSettings) {
    setSettings(next);
    if (saveTimer.current !== undefined) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveTimer.current = undefined;
      void invoke("save_settings", { settings: next });
    }, 400);
  }

  /** Flush pending live settings so recording sees the latest webcam PiP config. */
  async function flushSettings(): Promise<AppSettings | null> {
    if (!settings) return null;
    if (saveTimer.current !== undefined) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = undefined;
    }
    await invoke("save_settings", { settings });
    return settings;
  }

  function patchWebcam(next: Record<string, unknown>) {
    if (!settings) return;
    saveSettingsLive({
      ...settings,
      overlays: { ...settings.overlays, webcam: next },
    });
  }

  /** Persist home-tab capture/encode choices so the next launch restores them. */
  function persistMainPrefs(partial: Partial<AppSettings> = {}) {
    if (!settings) return;
    saveSettingsLive({
      ...settings,
      preferredEncoder: encoder || null,
      micDevice: mic || null,
      loopbackDevice: loopback || null,
      defaultSource: source,
      defaultDisplayId: displayId,
      defaultWindowId: pickedWindow?.id ?? null,
      defaultWindowTitle: pickedWindow?.title ?? null,
      defaultRegion: source === "region" || source === "window" ? region : null,
      fps,
      quality,
      includeCursor: cursor,
      outputFormat: format,
      ...partial,
    });
  }

  function patchOverlay(path: string, value: unknown) {
    if (!settings) return;
    const nextOverlays = structuredClone(settings.overlays ?? {});
    const segs = path.split(".");
    let cur: any = nextOverlays;
    for (let i = 0; i < segs.length - 1; i++) {
      cur[segs[i]] ??= {};
      cur = cur[segs[i]];
    }
    cur[segs[segs.length - 1]] = value;
    void saveSettings({ ...settings, overlays: nextOverlays });
  }

  async function startWith(overrides?: {
    source?: VideoSource;
    region?: { x: number; y: number; width: number; height: number } | null;
  }) {
    setError(null);
    const src = overrides?.source ?? source;
    const reg = overrides?.region !== undefined ? overrides.region : region;
    setStartingRecord(true);
    try {
      // Webcam PiP lives in overlays settings — must hit Rust before start.
      // Do NOT release the preview cam here: record reuses it so PiP is live from t=0.
      await flushSettings();
      const snap = await invoke<SessionSnapshot>("start_recording", {
        args: {
          source: src,
          displayId,
          windowId: pickedWindow?.id ?? null,
          // Stale region from a prior picker must not leak into display capture.
          region: src === "region" || src === "window" ? reg : null,
          includeCursor: cursor,
          micDevice: mic || null,
          loopbackDevice: loopback || null,
          micVolume,
          loopbackVolume,
          encoder: encoder || null,
          format,
          fps: fpsLimit ? fps : Math.max(fps, 60),
          quality,
        },
      });
      setSession(snap);
    } catch (e) {
      setError(String(e));
    } finally {
      setStartingRecord(false);
    }
  }

  async function onRecordToggle() {
    if (recordTransitionBusy) return;
    setError(null);
    try {
      if (recording) {
        setStoppingRecord(true);
        try {
          const snap = await invoke<SessionSnapshot>("stop_recording");
          setSession(snap);
        } finally {
          setStoppingRecord(false);
        }
      } else if (source === "region" && !region) {
        await invoke("open_region_picker");
      } else if (source === "window" && !pickedWindow) {
        await invoke("open_window_picker");
      } else {
        await startWith();
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function onPauseToggle() {
    if (!session) return;
    try {
      if (session.state === "paused") setSession(await invoke("resume_recording"));
      else if (session.state === "recording") setSession(await invoke("pause_recording"));
    } catch (e) {
      setError(String(e));
    }
  }

  async function onShot() {
    try {
      const path = await invoke<string>("take_screenshot", {
        args: {
          source,
          displayId,
          windowId: pickedWindow?.id ?? null,
          region,
        },
      });
      setLastShot(path);
    } catch (e) {
      setError(String(e));
    }
  }

  function selectSource(next: VideoSource) {
    setTab("main");
    setError(null);
    if (next === "display") {
      setSource("display");
      setPickedWindow(null);
      setRegion(null);
      persistMainPrefs({
        defaultSource: "display",
        defaultWindowId: null,
        defaultWindowTitle: null,
        defaultRegion: null,
        defaultDisplayId: displayId,
      });
      return;
    }
    // Always open the picker (including re-pick while already on window/region).
    if (next === "window") {
      void invoke("open_window_picker").catch((e) => setError(String(e)));
      return;
    }
    if (next === "region") {
      void invoke("open_region_picker").catch((e) => setError(String(e)));
    }
  }

  function reselectCurrentSource() {
    if (source === "window" || source === "region") selectSource(source);
  }

  const sourceLabel =
    source === "display"
      ? displays.find((d) => d.id === displayId)?.name ?? t("fullScreen")
      : source === "window"
        ? pickedWindow?.title ?? t("pickWindow")
        : region
          ? `${region.width}×${region.height}`
          : t("selectRegion");

  const formatLabel = format === "gif" ? t("formatGifPill") : t("formatAudioPill");

  return (
    <div className="capto-app">
      <header className="capto-toolbar">
        <button
          type="button"
          className={`tool-btn rec ${recording ? "stop" : ""}`}
          title={recording ? t("stop") : t("record")}
          disabled={recordTransitionBusy}
          onClick={() => void onRecordToggle()}
        >
          <Icon path={recording ? I.stop : I.record} size={22} />
        </button>
        <button
          type="button"
          className="tool-btn"
          disabled={!recording || recordTransitionBusy}
          title={session?.state === "paused" ? t("resume") : t("pause")}
          onClick={() => void onPauseToggle()}
        >
          <Icon path={session?.state === "paused" ? I.play : I.pause} />
        </button>
        <button
          type="button"
          className="tool-btn"
          disabled={recordTransitionBusy}
          title={t("screenshot")}
          onClick={() => void onShot()}
        >
          <Icon path={I.camera} />
        </button>
        {recordTransitionBusy && (
          <div className="toolbar-status" role="status" aria-live="polite">
            <span className="toolbar-spinner" aria-hidden />
            {startingRecord ? t("recordStarting") : t("recordStopping")}
          </div>
        )}
        <div className="tool-timer mono">{formatMs(session?.elapsedMs ?? 0)}</div>
      </header>

      <div className="capto-toggles">
        <button
          type="button"
          className={`toggle ${cursor ? "on" : ""}`}
          disabled={recordTransitionBusy}
          title={t("cursor")}
          onClick={() => setCursor((v) => !v)}
        >
          <Icon path={I.cursor} size={16} />
        </button>
        <button
          type="button"
          className={`toggle ${mouse.enabled ? "on" : ""}`}
          disabled={recordTransitionBusy}
          title={t("mouseClicks")}
          onClick={() => patchOverlay("mouseClicks.enabled", !mouse.enabled)}
        >
          <Icon path={I.click} size={16} />
        </button>
        <button
          type="button"
          className={`toggle ${keys.enabled ? "on" : ""}`}
          disabled={recordTransitionBusy}
          title={t("keystrokes")}
          onClick={() => patchOverlay("keystrokes.enabled", !keys.enabled)}
        >
          <Icon path={I.keys} size={16} />
        </button>
        <span className="toggle-spacer" />
        <button type="button" className="toggle" title={t("refresh")} onClick={() => void refresh()}>
          <Icon path={I.refresh} size={16} />
        </button>
        <button
          type="button"
          className="toggle"
          title={t("openFolder")}
          onClick={() => void invoke("open_output_folder").catch((e) => setError(String(e)))}
        >
          <Icon path={I.folder} size={16} />
        </button>
        <button
          type="button"
          className={`toggle ${showPreview ? "on" : ""}`}
          title={t("showPreview")}
          onClick={toggleShowPreview}
        >
          <Icon path={I.preview} size={16} />
        </button>
      </div>

      <div className="capto-body">
        <nav className="capto-rail">
          {(
            [
              ["main", I.home, t("source")],
              ["webcam", I.cam, t("webcamPip")],
              ["overlays", I.layers, t("overlays")],
              ["settings", I.gear, t("settings")],
              ["about", I.info, t("about")],
            ] as const
          ).map(([id, icon, label]) => (
            <button
              key={id}
              type="button"
              className={tab === id ? "active" : ""}
              title={label}
              onClick={() => setTab(id)}
            >
              <Icon path={icon} />
            </button>
          ))}
        </nav>

        <main className="capto-main">
          {error && (
            <div className="banner error">
              {/bundled ffmpeg not found|ffmpeg not found|未检测到|copy-ffmpeg|reinstall Capto/i.test(
                error,
              )
                ? t("ffmpegMissing")
                : error}
            </div>
          )}

          {tab === "main" && (
            <>
              <section className="capto-card">
                <div className="card-label">{t("videoSource")}</div>
                <div className="source-row">
                  <select
                    className="grow"
                    value={source === "display" ? String(displayId) : source}
                    onChange={(e) => {
                      const v = e.target.value;
                      if (v === "window" || v === "region") selectSource(v);
                      else {
                        const id = Number(v);
                        setSource("display");
                        setDisplayId(id);
                        setPickedWindow(null);
                        persistMainPrefs({
                          defaultSource: "display",
                          defaultDisplayId: id,
                          defaultWindowId: null,
                          defaultWindowTitle: null,
                          defaultRegion: null,
                        });
                      }
                    }}
                  >
                    {displays.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.isPrimary ? t("fullScreen") : d.name} ({d.width}×{d.height})
                      </option>
                    ))}
                    <option value="window">{t("window")}</option>
                    <option value="region">{t("region")}</option>
                  </select>
                  <button
                    type="button"
                    className={`ghost-btn ${showPreview ? "active" : ""}`}
                    onClick={toggleShowPreview}
                  >
                    {t("preview")}
                  </button>
                </div>

                <div className="tile-row">
                  <button
                    type="button"
                    className={`tile ${source === "display" ? "active" : ""}`}
                    title={t("display")}
                    onClick={() => selectSource("display")}
                  >
                    <Icon path={I.screen} />
                  </button>
                  <button
                    type="button"
                    className={`tile ${source === "window" ? "active" : ""}`}
                    title={t("window")}
                    onClick={() => selectSource("window")}
                  >
                    <Icon path={I.window} />
                  </button>
                  <button
                    type="button"
                    className={`tile ${source === "region" ? "active" : ""}`}
                    title={t("region")}
                    onClick={() => selectSource("region")}
                  >
                    <Icon path={I.region} />
                  </button>
                  <button
                    type="button"
                    className={`tile ${webcam.enabled ? "active" : ""}`}
                    title={t("webcamPip")}
                    onClick={() => {
                      const next = !webcam.enabled;
                      if (!next) {
                        patchOverlay("webcam.enabled", false);
                        return;
                      }
                      const label =
                        camState.devices.find((d) => d.deviceId === (webcam.deviceId ?? ""))
                          ?.label ??
                        camState.devices[0]?.label ??
                        null;
                      void saveSettingsLive({
                        ...settings!,
                        overlays: {
                          ...settings!.overlays,
                          webcam: {
                            ...webcam,
                            enabled: true,
                            deviceId: webcam.deviceId ?? camState.devices[0]?.deviceId ?? null,
                            deviceLabel: webcam.deviceLabel ?? label,
                          },
                        },
                      });
                    }}
                  >
                    <Icon path={I.cam} />
                  </button>
                </div>
                <p
                  className="hint mono"
                  style={
                    source === "window" || source === "region"
                      ? { cursor: "pointer", textDecoration: "underline" }
                      : undefined
                  }
                  title={
                    source === "window"
                      ? t("pickWindow")
                      : source === "region"
                        ? t("selectRegion")
                        : undefined
                  }
                  onClick={() => reselectCurrentSource()}
                >
                  {sourceLabel}
                </p>
              </section>

              {settings && format !== "audioOnly" && (
                <RecordingPreview
                  showStage={showPreviewStage}
                  enabled={showPreviewStage}
                  preview={previewState}
                  webcam={webcam}
                />
              )}

              <section className="capto-card">
                <div className="card-label">{t("videoEncoder")}</div>
                <div className="seg-row">
                  {(
                    [
                      ["mp4", "formatMp4"],
                      ["gif", "formatGif"],
                      ["audioOnly", "formatAudio"],
                    ] as const
                  ).map(([id, labelKey]) => (
                    <button
                      key={id}
                      type="button"
                      className={`seg ${format === id ? "active" : ""}`}
                      onClick={() => {
                        setFormat(id);
                        persistMainPrefs({ outputFormat: id });
                      }}
                    >
                      {t(labelKey)}
                    </button>
                  ))}
                </div>

                {format === "mp4" && (
                  <select
                    value={encoder}
                    onChange={(e) => {
                      const v = e.target.value;
                      setEncoder(v);
                      persistMainPrefs({ preferredEncoder: v || null });
                    }}
                  >
                    <option value="">
                      {t("encoderAuto", { encoder: automaticEncoder?.name ?? "libx264" })}
                    </option>
                    {availableEncoders.map((e) => (
                      <option key={e.kind} value={e.kind}>
                        {t("encoderMp4Option", {
                          encoder: e.name,
                          hw: e.hardware ? t("encoderHwSuffix") : "",
                        })}
                      </option>
                    ))}
                  </select>
                )}
                {format !== "mp4" && <div className="format-pill">{formatLabel}</div>}

                <div className="slider-row">
                  <span>
                    {t("fps")}: {fpsLimit ? fps : "∞"}
                  </span>
                  <input
                    type="range"
                    min={1}
                    max={60}
                    value={fps}
                    disabled={!fpsLimit}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      setFps(v);
                      persistMainPrefs({ fps: v });
                    }}
                  />
                  <label className="inline-check">
                    <input
                      type="checkbox"
                      checked={fpsLimit}
                      onChange={(e) => setFpsLimit(e.target.checked)}
                    />
                    {t("limit")}
                  </label>
                </div>

                <div className="slider-row">
                  <span>
                    {t("quality")}: {quality}%
                  </span>
                  <input
                    type="range"
                    min={1}
                    max={100}
                    value={quality}
                    disabled={format === "audioOnly"}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      setQuality(v);
                      persistMainPrefs({ quality: v });
                    }}
                  />
                </div>
              </section>

              <section className="capto-card media-card">
                <div className="media-row">
                  <span className="media-ico">
                    <Icon path={I.mic} />
                  </span>
                  <select
                    value={mic}
                    onChange={(e) => {
                      const v = e.target.value;
                      setMic(v);
                      persistMainPrefs({ micDevice: v || null });
                    }}
                  >
                    <option value="">{t("none")}</option>
                    {inputs.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </div>
                {mic && (
                  <div className="audio-control-row">
                    <span>{t("microphone")}</span>
                    <div className="audio-meter" aria-label={t("audioLevel")}>
                      <i style={{ width: `${audioMeterPercent(audioLevels.microphone)}%` }} />
                    </div>
                    <input
                      type="range"
                      min={0}
                      max={200}
                      value={micVolume}
                      disabled={recording}
                      aria-label={`${t("microphone")} ${t("volume")}`}
                      onChange={(e) => {
                        const v = Number(e.target.value);
                        setMicVolume(v);
                        persistMainPrefs({ micVolume: v });
                      }}
                    />
                    <b>{micVolume}%</b>
                    <button
                      type="button"
                      className="audio-reset-btn"
                      disabled={recording || micVolume === 100}
                      onClick={() => {
                        setMicVolume(100);
                        persistMainPrefs({ micVolume: 100 });
                      }}
                    >
                      {t("resetVolume")}
                    </button>
                  </div>
                )}
                <div className="media-row">
                  <span className="media-ico muted-ico">♪</span>
                  <select
                    value={loopback}
                    onChange={(e) => {
                      const v = e.target.value;
                      setLoopback(v);
                      persistMainPrefs({ loopbackDevice: v || null });
                    }}
                  >
                    <option value="">{t("systemAudio")}: {t("none")}</option>
                    {loops.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </div>
                {loopback && (
                  <div className="audio-control-row">
                    <span>{t("systemAudio")}</span>
                    <div className="audio-meter" aria-label={t("audioLevel")}>
                      <i style={{ width: `${audioMeterPercent(audioLevels.system)}%` }} />
                    </div>
                    <input
                      type="range"
                      min={0}
                      max={200}
                      value={loopbackVolume}
                      disabled={recording}
                      aria-label={`${t("systemAudio")} ${t("volume")}`}
                      onChange={(e) => {
                        const v = Number(e.target.value);
                        setLoopbackVolume(v);
                        persistMainPrefs({ loopbackVolume: v });
                      }}
                    />
                    <b>{loopbackVolume}%</b>
                    <button
                      type="button"
                      className="audio-reset-btn"
                      disabled={recording || loopbackVolume === 100}
                      onClick={() => {
                        setLoopbackVolume(100);
                        persistMainPrefs({ loopbackVolume: 100 });
                      }}
                    >
                      {t("resetVolume")}
                    </button>
                  </div>
                )}
                {loops.length === 0 && <p className="hint">{t("noLoopbackHint")}</p>}
                {(mic || loopback) && (
                  <button
                    type="button"
                    className={`ghost-btn audio-test-btn ${audioTesting ? "active" : ""}`}
                    disabled={recording}
                    onClick={() => {
                      if (audioTesting) {
                        void invoke("stop_audio_meter").finally(() => setAudioTesting(false));
                      } else {
                        void invoke("start_audio_meter", {
                          micDevice: mic || null,
                          loopbackDevice: loopback || null,
                        })
                          .then(() => setAudioTesting(true))
                          .catch((e) => setError(String(e)));
                      }
                    }}
                  >
                    {audioTesting ? t("stopAudioTest") : t("testAudio")}
                  </button>
                )}

                <div className="media-row webcam-home-row">
                  <span className="media-ico">
                    <Icon path={I.cam} />
                  </span>
                  <select
                    value={webcam.enabled ? webcam.deviceId ?? "default" : ""}
                    onChange={(e) => {
                      const v = e.target.value;
                      if (!v) {
                        patchWebcam({ ...webcam, enabled: false });
                        return;
                      }
                      const deviceId = v === "default" ? "" : v;
                      const label = deviceId
                        ? camState.devices.find((d) => d.deviceId === deviceId)?.label ?? null
                        : camState.devices[0]?.label ?? null;
                      patchWebcam({
                        ...webcam,
                        enabled: true,
                        deviceId: deviceId || null,
                        deviceLabel: label,
                      });
                    }}
                  >
                    <option value="">{t("noWebcam")}</option>
                    <option value="default">{t("webcamDefault")}</option>
                    {camState.devices.map((d) => (
                      <option key={d.deviceId} value={d.deviceId}>
                        {d.label}
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    className="ghost-btn"
                    onClick={() => setTab("webcam")}
                  >
                    {t("webcamSettings")}
                  </button>
                </div>
              </section>

              {(session?.outputPath || lastShot) && (
                <div className="last-files mono">
                  {session?.outputPath && <div>{t("lastFile")}: {session.outputPath}</div>}
                  {lastShot && <div>{t("lastShot", { path: lastShot })}</div>}
                </div>
              )}
            </>
          )}

          {tab === "webcam" && settings && (
            <WebcamPanel
              webcam={webcam}
              cam={camState}
              preview={previewState}
              showStage={showPreviewStage}
              previewCam={!sessionBusy && !startingRecord}
              onChange={patchWebcam}
            />
          )}

          {tab === "overlays" && settings && (
            <OverlayPanel
              overlays={settings.overlays}
              onChange={(next) => void saveSettings({ ...settings, overlays: next })}
            />
          )}

          {tab === "settings" && settings && (
            <section className="capto-card">
              <label className="stack">
                {t("outputDir")}
                <div className="settings-path-row">
                  <input
                    value={settings.outputDir}
                    onChange={(e) => setSettings({ ...settings, outputDir: e.target.value })}
                    onBlur={() => void saveSettings(settings)}
                  />
                  <button type="button" className="ghost-btn" onClick={() => void chooseOutputDir()}>
                    {t("browse")}
                  </button>
                </div>
              </label>
              <button
                type="button"
                className="ghost-btn settings-reset"
                onClick={() => void restoreDefaultOutputDir()}
              >
                {t("restoreDefaultOutputDir")}
              </button>
              <label>
                {t("locale")}
                <select
                  value={settings.locale}
                  onChange={(e) => setSettings({ ...settings, locale: e.target.value })}
                >
                  {SUPPORTED_LOCALES.map((loc) => (
                    <option key={loc.id} value={loc.id}>
                      {loc.nativeLabel}
                    </option>
                  ))}
                </select>
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.hideAppWhileRecording}
                  onChange={(e) =>
                    setSettings({ ...settings, hideAppWhileRecording: e.target.checked })
                  }
                />
                {t("hideWhileRecording")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.minimizeToTrayOnClose}
                  onChange={(e) =>
                    setSettings({ ...settings, minimizeToTrayOnClose: e.target.checked })
                  }
                />
                {t("trayOnClose")}
              </label>

              <HotkeySettings
                hotkeys={settings.hotkeys ?? []}
                conflicts={hotkeyConflicts}
                onChange={(hotkeys) => void saveSettings({ ...settings, hotkeys })}
              />

              <button
                type="button"
                className="primary"
                onClick={() =>
                  void saveSettings({
                    ...settings,
                    preferredEncoder: encoder || null,
                    micDevice: mic || null,
                    loopbackDevice: loopback || null,
                    defaultSource: source,
                    defaultDisplayId: displayId,
                    defaultWindowId: pickedWindow?.id ?? null,
                    defaultWindowTitle: pickedWindow?.title ?? null,
                    defaultRegion: source === "region" || source === "window" ? region : null,
                    fps,
                    quality,
                    includeCursor: cursor,
                    outputFormat: format,
                  })
                }
              >
                {t("save")}
              </button>
            </section>
          )}

          {tab === "about" && <AboutPanel />}
        </main>
      </div>

      {settings && (
        <footer className="capto-output">
          <input
            value={settings.outputDir}
            onChange={(e) => setSettings({ ...settings, outputDir: e.target.value })}
            onBlur={() => void saveSettings(settings)}
          />
          <button
            type="button"
            className="ghost-btn"
            title={t("openFolder")}
            onClick={() => void invoke("open_output_folder").catch((e) => setError(String(e)))}
          >
            …
          </button>
        </footer>
      )}
    </div>
  );
}
