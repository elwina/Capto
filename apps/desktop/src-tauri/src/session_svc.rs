//! Shared session operations used by Tauri commands and the CLI HTTP control plane.

use crate::record_overlay;
use crate::AppState;
use capto_capture::CaptureTarget;
use capto_core::{AppSettings, RecordRequest, SessionSnapshot, VideoSourceKind};
use capto_ipc::{
    ConfigPathInfo, DoctorInfo, OpenOutputsRequest, OutputEntry, OutputsList, RecordStartRequest,
    ShotRequest,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};

pub fn capture_target(args: &ShotRequest) -> Result<CaptureTarget, String> {
    match args.source {
        VideoSourceKind::Display => Ok(CaptureTarget::Display {
            id: args.display_id.unwrap_or(0),
        }),
        VideoSourceKind::Window => {
            let id = args
                .window_id
                .ok_or_else(|| "windowId required".to_string())?;
            match capto_capture::window_by_id(id).map_err(|e| e.to_string())? {
                Some(w) => Ok(CaptureTarget::Region {
                    x: w.x,
                    y: w.y,
                    width: w.width.max(2),
                    height: w.height.max(2),
                }),
                None => match &args.region {
                    Some(r) => Ok(CaptureTarget::Region {
                        x: r.x,
                        y: r.y,
                        width: r.width,
                        height: r.height,
                    }),
                    None => Err("selected window is gone — pick it again".into()),
                },
            }
        }
        VideoSourceKind::Region => {
            let r = args
                .region
                .as_ref()
                .ok_or_else(|| "region required".to_string())?;
            Ok(CaptureTarget::Region {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            })
        }
    }
}

pub async fn get_settings(state: &AppState) -> AppSettings {
    state.session.lock().await.settings().clone()
}

pub async fn save_settings(
    app: &AppHandle,
    state: &AppState,
    settings: AppSettings,
    register_hotkeys: impl FnOnce(&AppHandle, &AppSettings) -> Vec<String>,
) -> Result<(), String> {
    let mut session = state.session.lock().await;
    let hotkeys_changed = session.settings().hotkeys != settings.hotkeys;
    *session.settings_mut() = settings.clone();
    session.settings().save().map_err(|e| e.to_string())?;
    drop(session);
    if hotkeys_changed {
        let conflicts = register_hotkeys(app, &settings);
        *state
            .hotkey_conflicts
            .lock()
            .expect("hotkey conflict state poisoned") = conflicts;
    }
    let _ = app.emit("settings://changed", &settings);
    Ok(())
}

/// Patch settings from a JSON object (partial update).
pub async fn patch_settings(
    app: &AppHandle,
    state: &AppState,
    patch: Value,
    register_hotkeys: impl FnOnce(&AppHandle, &AppSettings) -> Vec<String>,
) -> Result<AppSettings, String> {
    if !patch.is_object() {
        return Err("config patch must be a JSON object".into());
    }
    let current = get_settings(state).await;
    let mut merged = serde_json::to_value(&current).map_err(|e| e.to_string())?;
    if let (Some(base), Some(patch_obj)) = (merged.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj {
            base.insert(k.clone(), v.clone());
        }
    }
    let settings: AppSettings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    save_settings(app, state, settings.clone(), register_hotkeys).await?;
    Ok(settings)
}

pub fn config_path() -> ConfigPathInfo {
    ConfigPathInfo {
        path: AppSettings::config_path().to_string_lossy().into_owned(),
    }
}

pub async fn status(state: &AppState) -> SessionSnapshot {
    state.session.lock().await.snapshot().await
}

pub async fn start_recording(
    app: &AppHandle,
    state: &AppState,
    args: RecordStartRequest,
) -> Result<SessionSnapshot, String> {
    if let Some(meter) = state.audio_meter.lock().await.take() {
        meter.stop();
    }
    let session = state.session.lock().await;
    let settings = session.settings().clone();
    let format = args.format.unwrap_or(settings.output_format);
    let output_path = session.make_output_path(format);
    let mic_device = args.mic_device;
    let loopback_device = args.loopback_device;

    let req = RecordRequest {
        source: args.source,
        display_id: args.display_id.or(settings.default_display_id),
        window_id: args.window_id,
        region: args.region.or(settings.default_region.clone()),
        include_cursor: args.include_cursor.unwrap_or(settings.include_cursor),
        mic_device,
        loopback_device,
        mic_volume: args.mic_volume.unwrap_or(settings.mic_volume).min(200),
        loopback_volume: args
            .loopback_volume
            .unwrap_or(settings.loopback_volume)
            .min(200),
        encoder: args.encoder.or(settings.preferred_encoder),
        format,
        fps: args.fps.unwrap_or(settings.fps),
        quality: args.quality.unwrap_or(settings.quality).clamp(1, 100),
        output_path: output_path.to_string_lossy().into_owned(),
        overlays: settings.overlays.clone(),
        hide_app_while_recording: settings.hide_app_while_recording,
    };
    capto_capture::release_preview_session();
    let (snap, region) = match session.start(req).await {
        Ok(v) => v,
        Err(e) => return Err(e.to_string()),
    };
    if snap.hide_app {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
    }
    {
        let overlays = session.settings().overlays.clone();
        drop(session);
        let mut overlay = state.overlay.lock().await;
        if let Some(mut old) = overlay.take() {
            old.stop(app);
        }
        match record_overlay::RecordOverlayController::start(app, overlays, region) {
            Ok(ctrl) => *overlay = Some(ctrl),
            Err(e) => tracing::warn!(%e, "record overlay failed to start"),
        }
    }
    let _ = app.emit("session://state", &snap);
    Ok(snap)
}

pub async fn pause_recording(app: &AppHandle, state: &AppState) -> Result<SessionSnapshot, String> {
    let session = state.session.lock().await;
    let snap = session.pause().await.map_err(|e| e.to_string())?;
    drop(session);
    if let Some(overlay) = state.overlay.lock().await.as_mut() {
        overlay.pause(app);
    }
    let _ = app.emit("session://state", &snap);
    Ok(snap)
}

pub async fn resume_recording(
    app: &AppHandle,
    state: &AppState,
) -> Result<SessionSnapshot, String> {
    let session = state.session.lock().await;
    let snap = session.resume().await.map_err(|e| e.to_string())?;
    drop(session);
    if let Some(overlay) = state.overlay.lock().await.as_mut() {
        if let Err(e) = overlay.resume(app) {
            tracing::warn!(%e, "record overlay resume failed");
        }
    }
    let _ = app.emit("session://state", &snap);
    Ok(snap)
}

pub async fn stop_recording(app: &AppHandle, state: &AppState) -> Result<SessionSnapshot, String> {
    {
        let mut overlay = state.overlay.lock().await;
        if let Some(mut ctrl) = overlay.take() {
            ctrl.stop(app);
        }
    }
    let session = state.session.lock().await;
    let snap = session.stop().await.map_err(|e| e.to_string())?;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    let _ = app.emit("session://state", &snap);
    Ok(snap)
}

pub async fn take_screenshot(state: &AppState, args: ShotRequest) -> Result<String, String> {
    let session = state.session.lock().await;
    let target = capture_target(&args)?;
    let path = session.default_screenshot_path();
    let saved = session
        .take_screenshot(&target, &path)
        .map_err(|e| e.to_string())?;
    Ok(saved.to_string_lossy().into_owned())
}

pub async fn list_displays(state: &AppState) -> Result<Value, String> {
    let session = state.session.lock().await;
    let list = session
        .capture()
        .list_displays()
        .map_err(|e| e.to_string())?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

pub async fn list_windows(state: &AppState) -> Result<Value, String> {
    let session = state.session.lock().await;
    let list = session
        .capture()
        .list_windows()
        .map_err(|e| e.to_string())?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

pub fn list_audio() -> Result<Value, String> {
    let list = capto_audio::list_devices().map_err(|e| e.to_string())?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

pub async fn list_encoders(state: &AppState) -> Result<Value, String> {
    let mut session = state.session.lock().await;
    session.refresh_encoder().map_err(|e| e.to_string())?;
    let enc = session.encoder().ok_or_else(|| {
        "Bundled FFmpeg not found. Dev: run scripts/copy-ffmpeg.ps1. Release: reinstall Capto."
            .to_string()
    })?;
    let list = enc.probe_encoders().await.map_err(|e| e.to_string())?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

fn file_modified_ms(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub async fn outputs_recent(state: &AppState, limit: usize) -> Result<OutputsList, String> {
    let session = state.session.lock().await;
    let output_dir = session.output_dir().to_string();
    drop(session);
    let dir = PathBuf::from(&output_dir);
    let mut items = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if !(name.starts_with("capto-") || name.starts_with("capto_")) {
                // still include common media extensions from Capto folder
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !matches!(
                    ext.as_str(),
                    "mp4" | "gif" | "m4a" | "png" | "jpg" | "jpeg" | "webp"
                ) {
                    continue;
                }
            }
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            items.push(OutputEntry {
                path: path.to_string_lossy().into_owned(),
                name,
                bytes: meta.len(),
                modified_ms: file_modified_ms(&meta),
            });
        }
    }
    items.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    items.truncate(limit.max(1));
    Ok(OutputsList { output_dir, items })
}

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn open_outputs(
    state: &AppState,
    req: OpenOutputsRequest,
) -> Result<serde_json::Value, String> {
    if req.folder {
        let session = state.session.lock().await;
        let dir = session.output_dir().to_string();
        drop(session);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        open_path(Path::new(&dir))?;
        return Ok(serde_json::json!({ "opened": dir, "kind": "folder" }));
    }
    let path = if let Some(p) = req.path {
        p
    } else if req.last {
        let list = outputs_recent(state, 1).await?;
        list.items
            .first()
            .map(|i| i.path.clone())
            .ok_or_else(|| "no outputs found".to_string())?
    } else {
        return Err("path, last, or folder required".into());
    };
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("path not found: {path}"));
    }
    open_path(&p)?;
    Ok(serde_json::json!({ "opened": path, "kind": "file" }))
}

pub async fn doctor(state: &AppState, port: u16) -> DoctorInfo {
    let mut session = state.session.lock().await;
    let _ = session.refresh_encoder();
    let ffmpeg_path = session
        .encoder()
        .map(|e| e.binary_path().to_string_lossy().into_owned());
    // Path presence alone is not enough — a wedged Capto process can still
    // fail to spawn ffmpeg (empty stderr / exit 1). Probe a real `-version`.
    let ffmpeg_ok = match session.encoder() {
        Some(enc) => enc.version_line().await.is_ok(),
        None => false,
    };
    let preferred = session
        .settings()
        .preferred_encoder
        .map(|e| e.ffmpeg_name().to_string());
    DoctorInfo {
        os: std::env::consts::OS.into(),
        capture_backend: session.capture().platform_name().into(),
        ffmpeg_ok,
        ffmpeg_path,
        control_plane: true,
        pid: std::process::id(),
        port,
        preferred_encoder: preferred,
    }
}

pub fn default_start_from_settings(settings: &AppSettings) -> RecordStartRequest {
    RecordStartRequest {
        source: settings.default_source.clone(),
        display_id: settings.default_display_id.or(Some(0)),
        window_id: settings.default_window_id,
        region: settings.default_region.clone(),
        include_cursor: Some(settings.include_cursor),
        mic_device: settings.mic_device.clone(),
        loopback_device: settings.loopback_device.clone(),
        mic_volume: Some(settings.mic_volume),
        loopback_volume: Some(settings.loopback_volume),
        encoder: settings.preferred_encoder,
        format: Some(settings.output_format),
        fps: Some(settings.fps),
        quality: Some(settings.quality),
    }
}
