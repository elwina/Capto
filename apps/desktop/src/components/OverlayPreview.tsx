import { useMemo, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";

type OverlayConfig = Record<string, any>;

function posStyle(
  position: { anchor?: string; x?: number; y?: number } | undefined,
  boxW: number,
  boxH: number,
  frameW = 640,
  frameH = 360,
): CSSProperties {
  const anchor = position?.anchor ?? "bottomRight";
  const nx = position?.x ?? 0.5;
  const ny = position?.y ?? 0.5;
  let left = 0;
  let top = 0;
  switch (anchor) {
    case "topLeft":
      left = 0;
      top = 0;
      break;
    case "topRight":
      left = frameW - boxW;
      top = 0;
      break;
    case "bottomLeft":
      left = 0;
      top = frameH - boxH;
      break;
    case "center":
      left = (frameW - boxW) / 2;
      top = (frameH - boxH) / 2;
      break;
    case "custom":
      left = nx * frameW;
      top = ny * frameH;
      break;
    case "bottomRight":
    default:
      left = frameW - boxW;
      top = frameH - boxH;
      break;
  }
  if (anchor !== "custom") {
    left += (nx - 0.5) * 24;
    top += (ny - 0.5) * 24;
  }
  return {
    left: Math.max(0, left),
    top: Math.max(0, top),
    width: boxW,
    height: boxH,
  };
}

/** Live mock preview of overlay layout (Captura-style). */
export function OverlayPreview({ overlays }: { overlays: OverlayConfig }) {
  const { t } = useTranslation();
  const mouse = overlays.mouseClicks ?? {};
  const keys = overlays.keystrokes ?? {};
  const webcam = overlays.webcam ?? {};
  const texts = (overlays.texts ?? []) as any[];
  const images = (overlays.images ?? []) as any[];

  const frameW = 640;
  const frameH = 360;

  const camStyle = useMemo(
    () =>
      posStyle(
        webcam.position,
        Math.min(webcam.width ?? 160, 200) / 2,
        Math.min(webcam.height ?? 120, 150) / 2,
        frameW,
        frameH,
      ),
    [webcam],
  );

  return (
    <div className="overlay-preview-wrap">
      <div className="block-title">{t("overlayPreview")}</div>
      <div className="overlay-preview" style={{ aspectRatio: `${frameW} / ${frameH}` }}>
        <div className="overlay-preview-desktop">
          <div className="preview-taskbar" />
          <div className="preview-window">
            <div className="preview-titlebar">{t("overlayDemoApp")}</div>
            <div className="preview-body">{t("overlayPreviewHint")}</div>
          </div>
        </div>

        {mouse.enabled && (
          <>
            <span
              className="preview-click"
              style={{
                left: "42%",
                top: "48%",
                borderColor: mouse.leftColor ?? "#FF5252",
                background: `${mouse.leftColor ?? "#FF5252"}55`,
              }}
            />
            <span
              className="preview-click"
              style={{
                left: "58%",
                top: "62%",
                borderColor: mouse.rightColor ?? "#448AFF",
                background: `${mouse.rightColor ?? "#448AFF"}55`,
              }}
            />
          </>
        )}

        {keys.enabled && (
          <div
            className="preview-keys"
            style={{
              ...posStyle(keys.position, 120, 36, frameW, frameH),
              fontSize: Math.max(12, (keys.fontSize ?? 28) / 2),
              color: keys.color ?? "#fff",
              background: keys.background ?? "#000000AA",
            }}
          >
            Ctrl + S
          </div>
        )}

        {webcam.enabled && (
          <div
            className="preview-webcam"
            style={{
              ...camStyle,
              borderRadius: webcam.cornerRadius ?? 12,
              transform: webcam.mirrored ? "scaleX(-1)" : undefined,
            }}
          >
            {t("overlayCamera")}
          </div>
        )}

        {texts
          .filter((tx) => tx.enabled !== false)
          .map((tx) => (
            <div
              key={tx.id}
              className="preview-text"
              style={{
                ...posStyle(tx.position, 100, 28, frameW, frameH),
                fontSize: Math.max(11, (tx.fontSize ?? 20) / 2),
                color: tx.color ?? "#fff",
              }}
            >
              {tx.text || t("overlayText")}
            </div>
          ))}

        {images
          .filter((im) => im.enabled !== false)
          .map((im) => (
            <div
              key={im.id}
              className="preview-image"
              style={{
                ...posStyle(im.position, im.width ?? 64, im.height ?? 64, frameW, frameH),
                opacity: im.opacity ?? 1,
              }}
              title={im.path}
            >
              {t("overlayImage")}
            </div>
          ))}
      </div>
    </div>
  );
}
