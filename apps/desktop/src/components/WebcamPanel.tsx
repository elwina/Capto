import { useTranslation } from "react-i18next";
import { type WebcamPreviewState } from "../hooks/useWebcamPreview";
import { type PreviewFrameState } from "../hooks/usePreviewFrame";
import { useWebcamSoloPreview } from "../hooks/useWebcamSoloPreview";
import { PreviewStage, type WebcamConfig } from "./PreviewStage";

const ANCHORS = ["topLeft", "topRight", "bottomLeft", "bottomRight", "center"] as const;

/** Dedicated webcam PiP tab: camera live preview + placement on the recording stage. */
export function WebcamPanel({
  webcam = {},
  cam,
  preview,
  showStage,
  previewCam = true,
  onChange,
}: {
  webcam?: WebcamConfig;
  cam: WebcamPreviewState;
  preview: PreviewFrameState;
  showStage: boolean;
  /** False while recording so MF camera is free for the encode pump. */
  previewCam?: boolean;
  onChange: (next: WebcamConfig) => void;
}) {
  const { t } = useTranslation();
  const camOn = !!webcam.enabled;
  const { devices } = cam;
  const anchor = webcam.position?.anchor ?? "bottomRight";
  const solo = useWebcamSoloPreview(camOn && previewCam, webcam.deviceId ?? null);

  function webcamErrorText(raw: string | null): string | null {
    if (!raw) return null;
    const s = raw.toLowerCase();
    if (s.includes("busy") || s.includes("in use") || s.includes("占用")) {
      return t("webcamError.busy");
    }
    if (s.includes("denied") || s.includes("permission") || s.includes("权限")) {
      return t("webcamError.denied");
    }
    if (s.includes("not found") || s.includes("no device") || s.includes("未找到")) {
      return t("webcamError.notFound");
    }
    if (s.includes("unsupported") || s.includes("不支持")) {
      return t("webcamError.unsupported");
    }
    return t("webcamError.unknown");
  }

  function patch(partial: Partial<WebcamConfig>) {
    onChange({ ...webcam, ...partial });
  }

  function pickCamera(deviceId: string) {
    const label = deviceId
      ? devices.find((d) => d.deviceId === deviceId)?.label ?? null
      : devices[0]?.label ?? null;
    patch({
      deviceId: deviceId || null,
      deviceLabel: label,
    });
  }

  return (
    <section className="webcam-panel">
      <div className="capto-card">
        <div className="card-label">{t("webcamPip")}</div>
        <label className="check">
          <input
            type="checkbox"
            checked={camOn}
            onChange={(e) => {
              const enabled = e.target.checked;
              if (!enabled) {
                patch({ enabled: false });
                return;
              }
              patch({
                enabled: true,
                deviceId: webcam.deviceId ?? devices[0]?.deviceId ?? null,
                deviceLabel:
                  webcam.deviceLabel ??
                  devices.find((d) => d.deviceId === webcam.deviceId)?.label ??
                  devices[0]?.label ??
                  null,
              });
            }}
          />
          {t("enabled")}
        </label>

        <label className="stack">
          {t("webcamDevice")}
          <select value={webcam.deviceId ?? ""} onChange={(e) => pickCamera(e.target.value)}>
            <option value="">{t("webcamDefault")}</option>
            {devices.map((d) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label}
              </option>
            ))}
          </select>
        </label>

        <label className="check">
          <input
            type="checkbox"
            checked={webcam.mirrored ?? true}
            onChange={(e) => patch({ mirrored: e.target.checked })}
          />
          {t("webcamMirror")}
        </label>

        <div className="webcam-solo-preview">
          <div className="recording-preview-head">
            <span className="block-title">{t("webcamPreview")}</span>
            {camOn && <span className="preview-rate">15 FPS</span>}
          </div>
          <div
            className="webcam-solo-stage"
            style={{
              borderRadius: Math.max(4, (webcam.cornerRadius ?? 12) / 2),
            }}
          >
            {camOn && solo.imageUrl ? (
              <img src={solo.imageUrl} alt={t("webcamPreview")} draggable={false} />
            ) : (
              <span>
                {camOn
                  ? (webcamErrorText(solo.error) ?? t("webcamStarting"))
                  : t("webcamOff")}
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="capto-card">
        <div className="card-label">{t("webcamPlacement")}</div>
        <div className="anchor-grid">
          {ANCHORS.map((a) => (
            <button
              key={a}
              type="button"
              className={`anchor ${a} ${anchor === a ? "active" : ""}`}
              title={t(`anchor.${a}`)}
              onClick={() => patch({ position: { ...(webcam.position ?? {}), anchor: a } })}
            >
              <span />
            </button>
          ))}
        </div>

        <div className="row">
          <label className="stack">
            {t("width")}
            <input
              type="number"
              min={80}
              max={1280}
              value={webcam.width ?? 320}
              onChange={(e) => patch({ width: Number(e.target.value) || 320 })}
            />
          </label>
          <label className="stack">
            {t("height")}
            <input
              type="number"
              min={60}
              max={720}
              value={webcam.height ?? 240}
              onChange={(e) => patch({ height: Number(e.target.value) || 240 })}
            />
          </label>
          <label className="stack">
            {t("cornerRadius")}
            <input
              type="number"
              min={0}
              max={200}
              value={webcam.cornerRadius ?? 12}
              onChange={(e) => patch({ cornerRadius: Number(e.target.value) || 0 })}
            />
          </label>
        </div>
      </div>

      {showStage && (
        <>
          <div className="recording-preview-head">
            <span className="block-title">{t("livePreview")}</span>
            <span className="preview-rate">10 FPS</span>
          </div>
          <PreviewStage preview={preview} webcam={webcam} />
        </>
      )}
    </section>
  );
}
