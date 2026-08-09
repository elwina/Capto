//! Recording-time click/key overlay: transparent Tauri window + LL input hooks.

use capto_core::Region;
use capto_hooks::{create_input_hook, InputEvent, InputHook, MouseButton};
use capto_overlay::OverlayConfig;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

static EVENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug)]
struct OverlayBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl OverlayBounds {
    fn from_region_or_screen(region: Option<&Region>) -> Self {
        if let Some(r) = region {
            if r.width >= 2 && r.height >= 2 {
                return Self {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                };
            }
        }
        let screen = capto_capture::virtual_screen();
        Self {
            x: screen.x,
            y: screen.y,
            width: screen.width.max(2),
            height: screen.height.max(2),
        }
    }

    fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x + self.width as i32
            && py < self.y + self.height as i32
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ClickPayload {
    button: &'static str,
    x: f64,
    y: f64,
    color: String,
    radius: f64,
    id: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct KeyPayload {
    label: String,
    id: u64,
    font_size: u32,
    color: String,
    background: String,
}

pub struct RecordOverlayController {
    hook: Box<dyn InputHook>,
    pump: Option<JoinHandle<()>>,
    config: OverlayConfig,
    bounds: OverlayBounds,
}

impl RecordOverlayController {
    pub fn start(
        app: &AppHandle,
        config: OverlayConfig,
        region: Option<Region>,
    ) -> Result<Self, String> {
        let need_mouse = config.mouse_clicks.enabled;
        let need_keys = config.keystrokes.enabled;
        let bounds = OverlayBounds::from_region_or_screen(region.as_ref());
        if !need_mouse && !need_keys {
            tracing::info!("record overlay skipped (mouse/keys disabled)");
            return Ok(Self {
                hook: create_input_hook(),
                pump: None,
                config,
                bounds,
            });
        }

        open_record_overlay(app, bounds)?;
        tracing::info!(
            mouse = need_mouse,
            keys = need_keys,
            x = bounds.x,
            y = bounds.y,
            w = bounds.width,
            h = bounds.height,
            "record overlay window ready"
        );

        let (tx, rx) = std::sync::mpsc::channel();
        let mut hook = create_input_hook();
        hook.start(tx).map_err(|e| e.to_string())?;

        let app2 = app.clone();
        let cfg = config.clone();
        let pump = thread::Builder::new()
            .name("capto-overlay-pump".into())
            .spawn(move || pump_events(app2, rx, cfg, bounds))
            .map_err(|e| e.to_string())?;

        Ok(Self {
            hook,
            pump: Some(pump),
            config,
            bounds,
        })
    }

    pub fn pause(&mut self, app: &AppHandle) {
        self.hook.stop();
        emit_overlay(app, "overlay://clear", ());
    }

    pub fn resume(&mut self, app: &AppHandle) -> Result<(), String> {
        if !self.config.mouse_clicks.enabled && !self.config.keystrokes.enabled {
            return Ok(());
        }
        let _ = open_record_overlay(app, self.bounds);
        let (tx, rx) = std::sync::mpsc::channel();
        self.hook.start(tx)?;
        let app2 = app.clone();
        let cfg = self.config.clone();
        let bounds = self.bounds;
        if let Some(old) = self.pump.take() {
            let _ = old.join();
        }
        self.pump = Some(
            thread::Builder::new()
                .name("capto-overlay-pump".into())
                .spawn(move || pump_events(app2, rx, cfg, bounds))
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    pub fn stop(&mut self, app: &AppHandle) {
        self.hook.stop();
        if let Some(t) = self.pump.take() {
            let _ = t.join();
        }
        emit_overlay(app, "overlay://clear", ());
        if let Some(w) = app.get_webview_window("record-overlay") {
            let _ = w.close();
        }
    }
}

impl Drop for RecordOverlayController {
    fn drop(&mut self) {
        self.hook.stop();
    }
}

fn emit_overlay(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    // Emit once. Broadcasting + window.emit doubles every event in the webview.
    if let Some(w) = app.get_webview_window("record-overlay") {
        let _ = w.emit(event, payload);
    } else {
        let _ = app.emit(event, payload);
    }
}

fn open_record_overlay(app: &AppHandle, bounds: OverlayBounds) -> Result<(), String> {
    tracing::info!(
        x = bounds.x,
        y = bounds.y,
        w = bounds.width,
        h = bounds.height,
        "opening record overlay on capture region"
    );

    if let Some(existing) = app.get_webview_window("record-overlay") {
        let _ = existing.set_position(tauri::Position::Physical(PhysicalPosition::new(
            bounds.x, bounds.y,
        )));
        let _ = existing.set_size(tauri::Size::Physical(PhysicalSize::new(
            bounds.width,
            bounds.height,
        )));
        let _ = existing.show();
        let _ = existing.set_always_on_top(true);
        let _ = existing.set_ignore_cursor_events(true);
        return Ok(());
    }

    // Windows: shadow must be off or transparency becomes an opaque white surface.
    let window = WebviewWindowBuilder::new(
        app,
        "record-overlay",
        WebviewUrl::App("index.html".into()),
    )
    .title("Capto Overlay")
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(true)
    .focused(false)
    .background_color(tauri::window::Color(0, 0, 0, 0))
    .build()
    .map_err(|e| format!("create record-overlay window: {e}"))?;

    let _ = window.set_position(tauri::Position::Physical(PhysicalPosition::new(
        bounds.x, bounds.y,
    )));
    let _ = window.set_size(tauri::Size::Physical(PhysicalSize::new(
        bounds.width,
        bounds.height,
    )));
    let _ = window.set_ignore_cursor_events(true);
    Ok(())
}

fn overlay_scale(app: &AppHandle) -> f64 {
    app.get_webview_window("record-overlay")
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0)
        .max(0.5)
}

fn pump_events(
    app: AppHandle,
    rx: Receiver<InputEvent>,
    config: OverlayConfig,
    bounds: OverlayBounds,
) {
    while let Ok(ev) = rx.recv() {
        match ev {
            InputEvent::MouseButton {
                button,
                x,
                y,
                down,
            } if down && config.mouse_clicks.enabled && bounds.contains(x, y) => {
                let (name, color) = match button {
                    MouseButton::Left => ("left", config.mouse_clicks.left_color.clone()),
                    MouseButton::Right => ("right", config.mouse_clicks.right_color.clone()),
                    MouseButton::Middle => ("middle", config.mouse_clicks.middle_color.clone()),
                };
                let scale = overlay_scale(&app);
                let payload = ClickPayload {
                    button: name,
                    x: (f64::from(x) - f64::from(bounds.x)) / scale,
                    y: (f64::from(y) - f64::from(bounds.y)) / scale,
                    color,
                    radius: f64::from(config.mouse_clicks.radius.max(8)) / scale,
                    id: EVENT_ID.fetch_add(1, Ordering::Relaxed),
                };
                emit_overlay(&app, "overlay://click", payload);
            }
            InputEvent::Key { label, down } if down && config.keystrokes.enabled => {
                // Key overlays are expected for every pressed key during recording.
                // Keep them available for deep diagnostics without flooding normal debug logs.
                tracing::trace!(%label, "overlay key");
                let payload = KeyPayload {
                    label,
                    id: EVENT_ID.fetch_add(1, Ordering::Relaxed),
                    font_size: config.keystrokes.font_size.max(12),
                    color: config.keystrokes.color.clone(),
                    background: config.keystrokes.background.clone(),
                };
                emit_overlay(&app, "overlay://key", payload);
            }
            _ => {}
        }
    }
}
