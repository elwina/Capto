import { useTranslation } from "react-i18next";
import { OverlayPreview } from "./OverlayPreview";

type OverlayConfig = Record<string, any>;

export function OverlayPanel({
  overlays,
  onChange,
}: {
  overlays: OverlayConfig;
  onChange: (next: OverlayConfig) => void;
}) {
  const { t } = useTranslation();
  const mouse = overlays.mouseClicks ?? {};
  const keys = overlays.keystrokes ?? {};

  function patch(path: string, value: unknown) {
    const next = structuredClone(overlays);
    const segs = path.split(".");
    let cur: any = next;
    for (let i = 0; i < segs.length - 1; i++) cur = cur[segs[i]];
    cur[segs[segs.length - 1]] = value;
    onChange(next);
  }

  return (
    <section className="panel overlays-panel">
      <OverlayPreview overlays={overlays} />
      <p className="muted overlay-webcam-note">{t("webcamMovedHint")}</p>

      <div className="overlay-grid">
        <article className="card-lite">
          <h3>{t("mouseClicks")}</h3>
          <label className="check">
            <input
              type="checkbox"
              checked={!!mouse.enabled}
              onChange={(e) => patch("mouseClicks.enabled", e.target.checked)}
            />
            {t("enabled")}
          </label>
          <label>
            {t("leftClickColor")}
            <input
              type="color"
              value={mouse.leftColor ?? "#FF5252"}
              onChange={(e) => patch("mouseClicks.leftColor", e.target.value)}
            />
          </label>
          <label>
            {t("rightClickColor")}
            <input
              type="color"
              value={mouse.rightColor ?? "#448AFF"}
              onChange={(e) => patch("mouseClicks.rightColor", e.target.value)}
            />
          </label>
        </article>

        <article className="card-lite">
          <h3>{t("keystrokes")}</h3>
          <label className="check">
            <input
              type="checkbox"
              checked={!!keys.enabled}
              onChange={(e) => patch("keystrokes.enabled", e.target.checked)}
            />
            {t("enabled")}
          </label>
          <label>
            {t("fontSize")}
            <input
              type="number"
              value={keys.fontSize ?? 28}
              onChange={(e) => patch("keystrokes.fontSize", Number(e.target.value))}
            />
          </label>
        </article>
      </div>
    </section>
  );
}
