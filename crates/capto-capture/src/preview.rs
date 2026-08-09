//! Live UI preview capture via DXGI Desktop Duplication (no GDI BitBlt).
//!
//! GDI `BitBlt` of the desktop DC makes Windows briefly hide the system cursor
//! on every grab (~5 Hz in Capto) — that reads as mouse jitter. DXGI/WGC do not.

use crate::{
    list_monitor_rects, window_by_id, CaptureError, CaptureTarget, Frame, Result, VirtualScreen,
};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows_capture::dxgi_duplication_api::{
        DxgiDuplicationApi, DxgiDuplicationFormat, Error as DxgiError,
    };
    use windows_capture::monitor::Monitor as WcMonitor;

    struct Session {
        /// Capto 0-based display id.
        display_id: u32,
        dup: DxgiDuplicationApi,
        last: Option<Frame>,
        origin: (i32, i32),
    }

    static SESSION: Mutex<Option<Session>> = Mutex::new(None);

    pub fn release_preview_session() {
        if let Ok(mut g) = SESSION.lock() {
            *g = None;
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn monitor_origin(display_id: u32) -> Result<(i32, i32, VirtualScreen)> {
        let rects = list_monitor_rects();
        let rect = rects
            .get(display_id as usize)
            .copied()
            .ok_or_else(|| CaptureError::DisplayNotFound(display_id.to_string()))?;
        Ok((rect.x, rect.y, rect))
    }

    fn open_dup(display_id: u32) -> Result<(DxgiDuplicationApi, (i32, i32))> {
        let (ox, oy, _) = monitor_origin(display_id)?;
        // windows-capture uses 1-based monitor indices.
        let monitor = WcMonitor::from_index(display_id as usize + 1)
            .map_err(|e| CaptureError::Failed(format!("monitor {display_id}: {e}")))?;
        let dup = DxgiDuplicationApi::new(monitor)
            .map_err(|e| CaptureError::Failed(format!("DXGI duplication: {e}")))?;
        Ok((dup, (ox, oy)))
    }

    fn buffer_to_rgba(
        frame: &mut windows_capture::dxgi_duplication_api::DxgiDuplicationFrame<'_>,
    ) -> Result<Frame> {
        let buf = frame
            .buffer()
            .map_err(|e| CaptureError::Failed(format!("map DXGI frame: {e}")))?;
        let width = buf.width();
        let height = buf.height();
        let format = buf.format();
        let mut packed = Vec::new();
        let pixels = buf.as_nopadding_buffer(&mut packed);
        let rgba = match format {
            DxgiDuplicationFormat::Rgba8 | DxgiDuplicationFormat::Rgba8Srgb => pixels.to_vec(),
            DxgiDuplicationFormat::Bgra8 | DxgiDuplicationFormat::Bgra8Srgb => {
                let mut out = Vec::with_capacity(pixels.len());
                for chunk in pixels.chunks_exact(4) {
                    out.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
                }
                out
            }
            other => {
                return Err(CaptureError::Failed(format!(
                    "unsupported DXGI color format: {other:?}"
                )));
            }
        };
        Ok(Frame {
            width,
            height,
            rgba,
            timestamp_ms: now_ms(),
        })
    }

    fn grab_monitor(display_id: u32) -> Result<(Frame, (i32, i32))> {
        let mut guard = SESSION
            .lock()
            .map_err(|_| CaptureError::Failed("preview capture lock poisoned".into()))?;

        let needs_new = match guard.as_ref() {
            Some(s) => s.display_id != display_id,
            None => true,
        };
        if needs_new {
            let (dup, origin) = open_dup(display_id)?;
            *guard = Some(Session {
                display_id,
                dup,
                last: None,
                origin,
            });
        }

        let session = guard.as_mut().expect("session just created");

        // ~3 frames of budget at 5 FPS; desktop idle often returns Timeout.
        match session.dup.acquire_next_frame(200) {
            Ok(mut frame) => match buffer_to_rgba(&mut frame) {
                Ok(img) => {
                    session.last = Some(img.clone());
                    Ok((img, session.origin))
                }
                Err(e) => Err(e),
            },
            Err(DxgiError::Timeout) => {
                if let Some(last) = session.last.clone() {
                    Ok((last, session.origin))
                } else {
                    // First frame: wait longer for an initial desktop image.
                    match session.dup.acquire_next_frame(1000) {
                        Ok(mut frame) => {
                            let img = buffer_to_rgba(&mut frame)?;
                            session.last = Some(img.clone());
                            Ok((img, session.origin))
                        }
                        Err(e) => Err(CaptureError::Failed(format!(
                            "DXGI waiting for first frame: {e}"
                        ))),
                    }
                }
            }
            Err(DxgiError::AccessLost) => {
                let (dup, origin) = open_dup(display_id)?;
                *session = Session {
                    display_id,
                    dup,
                    last: None,
                    origin,
                };
                let mut frame = session
                    .dup
                    .acquire_next_frame(1000)
                    .map_err(|e| CaptureError::Failed(format!("DXGI after AccessLost: {e}")))?;
                let img = buffer_to_rgba(&mut frame)?;
                session.last = Some(img.clone());
                Ok((img, session.origin))
            }
            Err(e) => Err(CaptureError::Failed(format!("DXGI acquire: {e}"))),
        }
    }

    fn crop_frame(frame: &Frame, rel_x: u32, rel_y: u32, width: u32, height: u32) -> Result<Frame> {
        let img = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
            .ok_or_else(|| CaptureError::Failed("invalid RGBA buffer".into()))?;
        let crop_w = width.min(frame.width.saturating_sub(rel_x)).max(1);
        let crop_h = height.min(frame.height.saturating_sub(rel_y)).max(1);
        let cropped = image::imageops::crop_imm(&img, rel_x, rel_y, crop_w, crop_h).to_image();
        Ok(Frame {
            width: cropped.width(),
            height: cropped.height(),
            rgba: cropped.into_raw(),
            timestamp_ms: frame.timestamp_ms,
        })
    }

    fn display_for_point(x: i32, y: i32) -> Result<u32> {
        let rects = list_monitor_rects();
        for (idx, r) in rects.iter().enumerate() {
            if x >= r.x && y >= r.y && x < r.x + r.width as i32 && y < r.y + r.height as i32 {
                return Ok(idx as u32);
            }
        }
        Ok(0)
    }

    pub fn capture_preview_frame(target: &CaptureTarget) -> Result<(Frame, Option<(i32, i32)>)> {
        match target {
            CaptureTarget::Display { id } => {
                let (frame, origin) = grab_monitor(*id)?;
                Ok((frame, Some(origin)))
            }
            CaptureTarget::Region {
                x,
                y,
                width,
                height,
            } => {
                let id = display_for_point(*x, *y)?;
                let (full, (ox, oy)) = grab_monitor(id)?;
                let rel_x = (*x - ox).max(0) as u32;
                let rel_y = (*y - oy).max(0) as u32;
                let cropped = crop_frame(&full, rel_x, rel_y, *width, *height)?;
                Ok((cropped, Some((*x, *y))))
            }
            CaptureTarget::Window { id } => {
                let info = window_by_id(*id)?
                    .ok_or_else(|| CaptureError::WindowNotFound(id.to_string()))?;
                let display_id = display_for_point(info.x, info.y)?;
                let (full, (ox, oy)) = grab_monitor(display_id)?;
                let rel_x = (info.x - ox).max(0) as u32;
                let rel_y = (info.y - oy).max(0) as u32;
                let cropped = crop_frame(&full, rel_x, rel_y, info.width, info.height)?;
                Ok((cropped, Some((info.x, info.y))))
            }
        }
    }
}

/// Capture a preview frame without GDI cursor flicker.
///
/// Returns `(frame, capture_origin)` where origin is the top-left of the
/// captured content in virtual-screen coordinates (for app window masking).
pub fn capture_preview_frame(target: &CaptureTarget) -> Result<(Frame, Option<(i32, i32)>)> {
    #[cfg(windows)]
    {
        windows_impl::capture_preview_frame(target)
    }
    #[cfg(not(windows))]
    {
        // Non-Windows: keep prior xcap path via backend (may flicker on platforms that BitBlt).
        let backend = crate::create_default_backend();
        let frame = backend.capture_frame(target)?;
        let origin = match target {
            CaptureTarget::Display { id } => {
                list_monitor_rects().get(*id as usize).map(|r| (r.x, r.y))
            }
            CaptureTarget::Region { x, y, .. } => Some((*x, *y)),
            CaptureTarget::Window { id } => window_by_id(*id)?.map(|w| (w.x, w.y)),
        };
        Ok((frame, origin))
    }
}

/// Release the cached DXGI duplication session (required before recording opens its own).
pub fn release_preview_session() {
    #[cfg(windows)]
    {
        windows_impl::release_preview_session();
    }
}
