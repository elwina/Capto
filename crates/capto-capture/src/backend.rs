use crate::{CaptureError, CaptureTarget, DisplayInfo, Frame, Result, WindowInfo};
use std::time::{SystemTime, UNIX_EPOCH};
use xcap::{Monitor, Window};

/// Platform-agnostic capture surface. New OS backends must implement this trait.
pub trait CaptureBackend: Send + Sync {
    fn platform_name(&self) -> &'static str;
    fn list_displays(&self) -> Result<Vec<DisplayInfo>>;
    fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    fn capture_frame(&self, target: &CaptureTarget) -> Result<Frame>;
    fn supports_streaming(&self) -> bool {
        false
    }
}

/// Default backend: `xcap` (list + still capture). Windows live preview uses DXGI
/// Desktop Duplication in `preview` (avoids GDI BitBlt cursor flicker).
/// Continuous recording still goes through FFmpeg in `capto-core` (Windows gdigrab first).
pub fn create_default_backend() -> Box<dyn CaptureBackend> {
    Box::new(XcapCaptureBackend::new())
}

pub struct UnsupportedCaptureBackend {
    pub platform: &'static str,
    pub hint: &'static str,
}

impl CaptureBackend for UnsupportedCaptureBackend {
    fn platform_name(&self) -> &'static str {
        self.platform
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        Err(CaptureError::Unsupported(self.hint))
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(CaptureError::Unsupported(self.hint))
    }

    fn capture_frame(&self, _target: &CaptureTarget) -> Result<Frame> {
        Err(CaptureError::Unsupported(self.hint))
    }
}

pub struct XcapCaptureBackend;

impl XcapCaptureBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for XcapCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBackend for XcapCaptureBackend {
    fn platform_name(&self) -> &'static str {
        #[cfg(windows)]
        {
            "windows-wgc-xcap"
        }
        #[cfg(target_os = "macos")]
        {
            "macos-sck-xcap"
        }
        #[cfg(target_os = "linux")]
        {
            "linux-xcap"
        }
        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            "xcap"
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>> {
        let monitors = Monitor::all().map_err(|e| CaptureError::Failed(e.to_string()))?;
        let mut out = Vec::with_capacity(monitors.len());
        for (idx, m) in monitors.into_iter().enumerate() {
            out.push(DisplayInfo {
                id: idx as u32,
                name: m.name().to_string(),
                width: m.width(),
                height: m.height(),
                x: m.x(),
                y: m.y(),
                is_primary: m.is_primary(),
                scale_factor: m.scale_factor() as f64,
            });
        }
        Ok(out)
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        #[cfg(windows)]
        {
            return crate::list_windows();
        }

        #[cfg(not(windows))]
        {
            let windows = Window::all().map_err(|e| CaptureError::Failed(e.to_string()))?;
            let mut out = Vec::new();
            for w in windows.into_iter().filter(|w| !w.title().trim().is_empty()) {
                let id = out.len() as u32;
                out.push(WindowInfo {
                    id,
                    title: w.title().to_string(),
                    app_name: w.app_name().to_string(),
                    width: w.width(),
                    height: w.height(),
                    x: w.x(),
                    y: w.y(),
                });
            }
            Ok(out)
        }
    }

    fn capture_frame(&self, target: &CaptureTarget) -> Result<Frame> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        match target {
            CaptureTarget::Display { id } => {
                let monitors = Monitor::all().map_err(|e| CaptureError::Failed(e.to_string()))?;
                let m = monitors
                    .into_iter()
                    .nth(*id as usize)
                    .ok_or_else(|| CaptureError::DisplayNotFound(id.to_string()))?;
                let img = m
                    .capture_image()
                    .map_err(|e| CaptureError::Failed(e.to_string()))?;
                Ok(Frame {
                    width: img.width(),
                    height: img.height(),
                    rgba: img.into_raw(),
                    timestamp_ms,
                })
            }
            CaptureTarget::Window { id } => {
                let windows = Window::all().map_err(|e| CaptureError::Failed(e.to_string()))?;
                let visible: Vec<_> = windows
                    .into_iter()
                    .filter(|w| !w.title().trim().is_empty())
                    .collect();
                let w = visible
                    .get(*id as usize)
                    .ok_or_else(|| CaptureError::WindowNotFound(id.to_string()))?;
                let img = w
                    .capture_image()
                    .map_err(|e| CaptureError::Failed(e.to_string()))?;
                Ok(Frame {
                    width: img.width(),
                    height: img.height(),
                    rgba: img.into_raw(),
                    timestamp_ms,
                })
            }
            CaptureTarget::Region {
                x,
                y,
                width,
                height,
            } => {
                let monitors = Monitor::all().map_err(|e| CaptureError::Failed(e.to_string()))?;
                let m = monitors
                    .into_iter()
                    .find(|mon| {
                        let mx = mon.x();
                        let my = mon.y();
                        let mw = mon.width() as i32;
                        let mh = mon.height() as i32;
                        *x >= mx && *y >= my && *x < mx + mw && *y < my + mh
                    })
                    .or_else(|| Monitor::all().ok().and_then(|m| m.into_iter().next()))
                    .ok_or_else(|| CaptureError::Failed("no monitor for region".into()))?;

                let full = m
                    .capture_image()
                    .map_err(|e| CaptureError::Failed(e.to_string()))?;
                let mx = m.x();
                let my = m.y();
                let rel_x = (*x - mx).max(0) as u32;
                let rel_y = (*y - my).max(0) as u32;
                let crop_w = (*width).min(full.width().saturating_sub(rel_x));
                let crop_h = (*height).min(full.height().saturating_sub(rel_y));
                let cropped =
                    image::imageops::crop_imm(&full, rel_x, rel_y, crop_w, crop_h).to_image();
                Ok(Frame {
                    width: cropped.width(),
                    height: cropped.height(),
                    rgba: cropped.into_raw(),
                    timestamp_ms,
                })
            }
        }
    }
}
