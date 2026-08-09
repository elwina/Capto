import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import type { MaskRect, PreviewFrameState } from "../hooks/usePreviewFrame";

export type { PreviewSource, Region } from "../hooks/usePreviewFrame";

export type WebcamConfig = {
  enabled?: boolean;
  deviceId?: string | null;
  deviceLabel?: string | null;
  width?: number;
  height?: number;
  mirrored?: boolean;
  cornerRadius?: number;
  position?: { anchor?: string; x?: number; y?: number };
};

function CaptoMark() {
  return (
    <svg viewBox="0 0 100 100" aria-hidden focusable="false">
      <defs>
        <mask id="capto-mark-cut">
          <rect x="0" y="0" width="100" height="100" fill="black" />
          <circle cx="50.5" cy="49" r="33.4" fill="white" />
          <circle cx="58.3" cy="52.4" r="24.1" fill="black" />
        </mask>
      </defs>
      <circle
        cx="50.5"
        cy="49"
        r="33.4"
        fill="currentColor"
        mask="url(#capto-mark-cut)"
      />
    </svg>
  );
}

function CameraIcon() {
  return (
    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" aria-hidden>
      <path
        d="M4 7h11a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2Zm13 2.5 5-3v11l-5-3v-5Z"
        fill="currentColor"
      />
    </svg>
  );
}

function imageRect(stageW: number, stageH: number, frameW: number, frameH: number) {
  const scale = Math.min(stageW / Math.max(frameW, 1), stageH / Math.max(frameH, 1));
  const width = frameW * scale;
  const height = frameH * scale;
  return {
    width,
    height,
    left: (stageW - width) / 2,
    top: (stageH - height) / 2,
  };
}

function pipStyle(
  webcam: WebcamConfig,
  stageW: number,
  stageH: number,
  frameW: number,
  frameH: number,
  sourceW: number,
  sourceH: number,
): CSSProperties {
  const img = imageRect(stageW, stageH, frameW, frameH);
  const sw = Math.max(1, sourceW);
  const sh = Math.max(1, sourceH);
  const boxW = Math.max(2, ((webcam.width ?? 320) / sw) * img.width);
  const boxH = Math.max(2, ((webcam.height ?? 240) / sh) * img.height);
  const nudgeX = ((webcam.position?.x ?? 0.82) - 0.5) * 40 * (img.width / sw);
  const nudgeY = ((webcam.position?.y ?? 0.78) - 0.5) * 40 * (img.height / sh);
  const anchor = webcam.position?.anchor ?? "bottomRight";
  let left = 0;
  let top = 0;
  switch (anchor) {
    case "topLeft":
      break;
    case "topRight":
      left = img.width - boxW;
      break;
    case "bottomLeft":
      top = img.height - boxH;
      break;
    case "center":
      left = (img.width - boxW) / 2;
      top = (img.height - boxH) / 2;
      break;
    case "bottomRight":
    default:
      left = img.width - boxW;
      top = img.height - boxH;
      break;
  }
  left += nudgeX;
  top += nudgeY;
  return {
    left: img.left + left,
    top: img.top + top,
    width: boxW,
    height: boxH,
    borderRadius: Math.max(2, (webcam.cornerRadius ?? 12) * (img.width / sw)),
  };
}

function maskStyle(
  mask: MaskRect,
  stageW: number,
  stageH: number,
  frameW: number,
  frameH: number,
): CSSProperties {
  const img = imageRect(stageW, stageH, frameW, frameH);
  return {
    left: img.left + mask.x * img.width,
    top: img.top + mask.y * img.height,
    width: Math.max(1, mask.width * img.width),
    height: Math.max(1, mask.height * img.height),
  };
}

/** Screen preview; webcam PiP area is marked with a camera icon (live cam is on Webcam tab). */
export function PreviewStage({
  preview,
  webcam = {},
  showPipGuide = true,
}: {
  preview: PreviewFrameState;
  webcam?: WebcamConfig;
  showPipGuide?: boolean;
}) {
  const { t } = useTranslation();
  const { frame, imageUrl, error, ready } = preview;
  const [stageSize, setStageSize] = useState({ w: 320, h: 180 });
  const stageRef = useRef<HTMLDivElement | null>(null);

  const camOn = !!webcam.enabled && showPipGuide;
  const frameW = frame?.width ?? stageSize.w;
  const frameH = frame?.height ?? stageSize.h;
  const sourceW = frame?.sourceWidth ?? frameW;
  const sourceH = frame?.sourceHeight ?? frameH;

  const pip = useMemo(
    () => pipStyle(webcam, stageSize.w, stageSize.h, frameW, frameH, sourceW, sourceH),
    [webcam, stageSize, frameW, frameH, sourceW, sourceH],
  );

  const mask = useMemo(
    () =>
      frame?.maskRect
        ? maskStyle(frame.maskRect, stageSize.w, stageSize.h, frameW, frameH)
        : null,
    [frame, stageSize, frameW, frameH],
  );

  useEffect(() => {
    const el = stageRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const box = entries[0]?.contentRect;
      if (!box) return;
      setStageSize({ w: Math.max(1, box.width), h: Math.max(1, box.height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  return (
    <>
      <div
        ref={stageRef}
        className="recording-preview-stage"
        style={frame ? { aspectRatio: `${frame.width} / ${frame.height}` } : undefined}
      >
        {imageUrl ? (
          <img src={imageUrl} alt={t("livePreview")} draggable={false} />
        ) : (
          <span>{ready ? t("previewStarting") : t("previewPickSource")}</span>
        )}

        {mask && (
          <div className="preview-mask-brand" style={mask}>
            <CaptoMark />
            <span>{t("appMasked")}</span>
          </div>
        )}

        {camOn && (
          <div className="preview-webcam-mark" style={pip} title={t("webcamPip")} aria-hidden>
            <CameraIcon />
          </div>
        )}
      </div>

      {error && (
        <div className="preview-error" title={error}>
          {t("previewUnavailable")}
        </div>
      )}
    </>
  );
}
