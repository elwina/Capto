import { useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTranslation } from "react-i18next";

type Phase =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate" }
  | { kind: "available"; update: Update }
  | { kind: "downloading"; percent: number | null }
  | { kind: "installing" }
  | { kind: "error"; message: string };

export function UpdateSettings() {
  const { t } = useTranslation();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });

  async function onCheck() {
    setPhase({ kind: "checking" });
    try {
      const update = await check();
      if (!update) {
        setPhase({ kind: "upToDate" });
        return;
      }
      setPhase({ kind: "available", update });
    } catch (e) {
      setPhase({ kind: "error", message: String(e) });
    }
  }

  async function onInstall(update: Update) {
    setPhase({ kind: "downloading", percent: null });
    try {
      let downloaded = 0;
      let total: number | null = null;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? null;
            setPhase({ kind: "downloading", percent: total ? 0 : null });
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (total && total > 0) {
              setPhase({
                kind: "downloading",
                percent: Math.min(99, Math.round((downloaded / total) * 100)),
              });
            }
            break;
          case "Finished":
            setPhase({ kind: "installing" });
            break;
        }
      });
      await relaunch();
    } catch (e) {
      setPhase({ kind: "error", message: String(e) });
    }
  }

  const statusText = (() => {
    switch (phase.kind) {
      case "idle":
        return null;
      case "checking":
        return t("updateChecking");
      case "upToDate":
        return t("updateUpToDate");
      case "available":
        return t("updateAvailable", { version: phase.update.version });
      case "downloading":
        return phase.percent == null
          ? t("updateDownloading")
          : t("updateDownloadingPercent", { percent: phase.percent });
      case "installing":
        return t("updateInstalling");
      case "error":
        return t("updateError", { message: phase.message });
    }
  })();

  const busy =
    phase.kind === "checking" ||
    phase.kind === "downloading" ||
    phase.kind === "installing";

  return (
    <div className="update-settings">
      <div className="update-settings-row">
        <button
          type="button"
          className="ghost-btn"
          disabled={busy}
          onClick={() => void onCheck()}
        >
          {t("checkForUpdates")}
        </button>
        {phase.kind === "available" && (
          <button
            type="button"
            className="primary"
            disabled={busy}
            onClick={() => void onInstall(phase.update)}
          >
            {t("installUpdate")}
          </button>
        )}
      </div>
      {statusText && (
        <p className={`update-settings-status${phase.kind === "error" ? " is-error" : ""}`}>
          {statusText}
        </p>
      )}
    </div>
  );
}
