import { useTranslation } from "react-i18next";
import { type PreviewFrameState } from "../hooks/usePreviewFrame";
import { PreviewStage, type WebcamConfig } from "./PreviewStage";

/** Low-FPS screen preview on the home / webcam tabs (camera picker lives elsewhere). */
export function RecordingPreview({
  showStage,
  enabled,
  preview,
  webcam = {},
}: {
  showStage: boolean;
  enabled: boolean;
  preview: PreviewFrameState;
  webcam?: WebcamConfig;
}) {
  const { t } = useTranslation();

  if (!showStage) return null;

  return (
    <div className="recording-preview">
      <div className="recording-preview-head">
        <span className="block-title">{t("livePreview")}</span>
        {enabled && <span className="preview-rate">10 FPS</span>}
      </div>
      {enabled && <PreviewStage preview={preview} webcam={webcam} />}
    </div>
  );
}
