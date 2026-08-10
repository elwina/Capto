//! DXGI Desktop Duplication frame source for recording (no GDI BitBlt → no cursor jitter).

use crate::webcam::WebcamFrameSlot;
use crate::{CaptureError, CaptureTarget, Result};
use capto_overlay::WebcamPip;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Optional webcam PiP composited in-process before rawvideo stdin.
#[derive(Clone)]
pub struct RecordPip {
    pub slot: WebcamFrameSlot,
    pub layout: WebcamPip,
}

/// Blocking DXGI → BGRA writer used by the recording session.
pub struct DxgiRecordPump {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DxgiRecordPump {
    /// `on_frame` receives owned BGRA frames. Return `false` to stop the pump
    /// (e.g. encode pipe closed). Taking ownership avoids an extra full-frame copy.
    pub fn start(
        target: CaptureTarget,
        fps: u32,
        include_cursor: bool,
        out_w: u32,
        out_h: u32,
        on_frame: impl FnMut(Vec<u8>) -> bool + Send + 'static,
        pip: Option<RecordPip>,
    ) -> Result<Self> {
        #[cfg(not(windows))]
        {
            let _ = (target, fps, include_cursor, out_w, out_h, on_frame, pip);
            return Err(CaptureError::Unsupported("DXGI recording is Windows-only"));
        }
        #[cfg(windows)]
        {
            crate::preview::release_preview_session();
            let stop = Arc::new(AtomicBool::new(false));
            let paused = Arc::new(AtomicBool::new(false));
            let stop2 = Arc::clone(&stop);
            let paused2 = Arc::clone(&paused);
            let fps = fps.clamp(1, 120);
            let mut on_frame = on_frame;
            let thread = thread::Builder::new()
                .name("capto-dxgi-record".into())
                .spawn(move || {
                    if let Err(e) = windows_impl::run_pump(
                        target,
                        fps,
                        include_cursor,
                        out_w,
                        out_h,
                        stop2,
                        paused2,
                        &mut on_frame,
                        pip,
                    ) {
                        tracing::warn!(%e, "DXGI record pump ended with error");
                    }
                })
                .map_err(|e| CaptureError::Failed(e.to_string()))?;
            Ok(Self {
                stop,
                paused,
                thread: Some(thread),
            })
        }
    }

    /// Soft-pause: stop delivering frames to FFmpeg (timeline excludes paused time).
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for DxgiRecordPump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use crate::{list_monitor_rects, monitor_index_for_rect, window_by_id, Frame, VirtualScreen};
    use std::time::{Duration, Instant};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, DIB_RGB_COLORS, HDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DrawIconEx, GetCursorInfo, GetIconInfo, CURSORINFO, CURSOR_SHOWING, DI_NORMAL, ICONINFO,
    };
    use windows_capture::dxgi_duplication_api::{
        DxgiDuplicationApi, DxgiDuplicationFormat, Error as DxgiError,
    };
    use windows_capture::monitor::Monitor as WcMonitor;

    struct DupSession {
        display_id: u32,
        dup: DxgiDuplicationApi,
        last: Option<Frame>,
        origin: (i32, i32),
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
        let monitor = WcMonitor::from_index(display_id as usize + 1)
            .map_err(|e| CaptureError::Failed(format!("monitor {display_id}: {e}")))?;
        let dup = DxgiDuplicationApi::new(monitor)
            .map_err(|e| CaptureError::Failed(format!("DXGI duplication: {e}")))?;
        Ok((dup, (ox, oy)))
    }

    fn buffer_to_bgra(
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
        let bgra = match format {
            DxgiDuplicationFormat::Bgra8 | DxgiDuplicationFormat::Bgra8Srgb => pixels.to_vec(),
            DxgiDuplicationFormat::Rgba8 | DxgiDuplicationFormat::Rgba8Srgb => {
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
            rgba: bgra,
            timestamp_ms: 0,
        })
    }

    fn grab_monitor(session: &mut DupSession) -> Result<()> {
        match session.dup.acquire_next_frame(8) {
            Ok(mut frame) => {
                let img = buffer_to_bgra(&mut frame)?;
                session.last = Some(img);
                Ok(())
            }
            Err(DxgiError::Timeout) => {
                if session.last.is_some() {
                    Ok(())
                } else {
                    Err(CaptureError::Failed(
                        "DXGI timeout before first frame".into(),
                    ))
                }
            }
            Err(DxgiError::AccessLost) => {
                let (dup, origin) = open_dup(session.display_id)?;
                session.dup = dup;
                session.origin = origin;
                session.last = None;
                let mut frame = session
                    .dup
                    .acquire_next_frame(1000)
                    .map_err(|e| CaptureError::Failed(format!("DXGI after AccessLost: {e}")))?;
                let img = buffer_to_bgra(&mut frame)?;
                session.last = Some(img);
                Ok(())
            }
            Err(e) => Err(CaptureError::Failed(format!("DXGI acquire: {e}"))),
        }
    }

    fn crop_bgra(frame: &Frame, rel_x: u32, rel_y: u32, width: u32, height: u32) -> Result<Frame> {
        let crop_w = width.min(frame.width.saturating_sub(rel_x)).max(1);
        let crop_h = height.min(frame.height.saturating_sub(rel_y)).max(1);
        if rel_x == 0 && rel_y == 0 && crop_w == frame.width && crop_h == frame.height {
            return Ok(Frame {
                width: frame.width,
                height: frame.height,
                rgba: frame.rgba.clone(),
                timestamp_ms: frame.timestamp_ms,
            });
        }
        let mut out = vec![0u8; (crop_w * crop_h * 4) as usize];
        for row in 0..crop_h {
            let src_off = (((rel_y + row) * frame.width + rel_x) * 4) as usize;
            let dst_off = (row * crop_w * 4) as usize;
            let len = (crop_w * 4) as usize;
            out[dst_off..dst_off + len].copy_from_slice(&frame.rgba[src_off..src_off + len]);
        }
        Ok(Frame {
            width: crop_w,
            height: crop_h,
            rgba: out,
            timestamp_ms: frame.timestamp_ms,
        })
    }

    fn scale_to_exact(frame: Frame, out_w: u32, out_h: u32) -> Result<Frame> {
        if frame.width == out_w && frame.height == out_h {
            return Ok(frame);
        }
        // Row-wise nearest neighbor (much cheaper than per-pixel index math in hot loops).
        let mut out = vec![0u8; (out_w * out_h * 4) as usize];
        let mut xs = vec![0u32; out_w as usize];
        for (x, slot) in xs.iter_mut().enumerate() {
            *slot = (u64::from(x as u32) * u64::from(frame.width) / u64::from(out_w)) as u32;
        }
        for y in 0..out_h {
            let sy = (u64::from(y) * u64::from(frame.height) / u64::from(out_h)) as u32;
            let src_row = (sy * frame.width * 4) as usize;
            let dst_row = (y * out_w * 4) as usize;
            for (x, &sx) in xs.iter().enumerate() {
                let si = src_row + (sx * 4) as usize;
                let di = dst_row + x * 4;
                out[di..di + 4].copy_from_slice(&frame.rgba[si..si + 4]);
            }
        }
        Ok(Frame {
            width: out_w,
            height: out_h,
            rgba: out,
            timestamp_ms: frame.timestamp_ms,
        })
    }

    fn resolve_target(target: &CaptureTarget) -> Result<(u32, i32, i32, u32, u32)> {
        match target {
            CaptureTarget::Display { id } => {
                let (ox, oy, rect) = monitor_origin(*id)?;
                Ok((*id, ox, oy, rect.width, rect.height))
            }
            CaptureTarget::Region {
                x,
                y,
                width,
                height,
            } => {
                let id = monitor_index_for_rect(*x, *y, *width, *height);
                Ok((id, *x, *y, *width, *height))
            }
            CaptureTarget::Window { id } => {
                let w = window_by_id(*id)?
                    .ok_or_else(|| CaptureError::WindowNotFound(id.to_string()))?;
                let id = monitor_index_for_rect(w.x, w.y, w.width, w.height);
                Ok((id, w.x, w.y, w.width, w.height))
            }
        }
    }

    /// Blit only the cursor glyph into the frame (not a full-frame GDI round-trip).
    fn composite_cursor_bgra(frame: &mut Frame, origin_x: i32, origin_y: i32) {
        unsafe {
            let mut info = CURSORINFO {
                cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                ..Default::default()
            };
            if GetCursorInfo(&mut info).is_err() {
                return;
            }
            if (info.flags.0 & CURSOR_SHOWING.0) == 0 {
                return;
            }

            let mut icon = ICONINFO::default();
            if GetIconInfo(info.hCursor, &mut icon).is_err() {
                return;
            }
            // Hotspot from ICONINFO; fall back to tip at (0,0).
            let hot_x = icon.xHotspot as i32;
            let hot_y = icon.yHotspot as i32;
            // Typical cursors are ≤64²; draw into a padded scratch so we never
            // touch the full desktop bitmap via GDI.
            let cw = 64i32;
            let ch = 64i32;
            if !icon.hbmMask.is_invalid() {
                let _ = DeleteObject(icon.hbmMask);
            }
            if !icon.hbmColor.is_invalid() {
                let _ = DeleteObject(icon.hbmColor);
            }

            let hx = info.ptScreenPos.x - origin_x - hot_x;
            let hy = info.ptScreenPos.y - origin_y - hot_y;
            if hx >= frame.width as i32 || hy >= frame.height as i32 || hx + cw <= 0 || hy + ch <= 0
            {
                return;
            }

            let hdc = CreateCompatibleDC(HDC::default());
            if hdc.is_invalid() {
                return;
            }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: cw,
                    biHeight: -ch,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let Ok(dib) = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
                let _ = DeleteDC(hdc);
                return;
            };
            if bits.is_null() {
                let _ = DeleteObject(dib);
                let _ = DeleteDC(hdc);
                return;
            }
            // Clear to transparent, then draw the cursor glyph.
            let scratch = std::slice::from_raw_parts_mut(bits as *mut u8, (cw * ch * 4) as usize);
            scratch.fill(0);
            let old = SelectObject(hdc, dib);
            let _ = DrawIconEx(hdc, 0, 0, info.hCursor, 0, 0, 0, None, DI_NORMAL);
            let _ = SelectObject(hdc, old);

            for row in 0..ch {
                let dy = hy + row;
                if dy < 0 || dy as u32 >= frame.height {
                    continue;
                }
                for col in 0..cw {
                    let dx = hx + col;
                    if dx < 0 || dx as u32 >= frame.width {
                        continue;
                    }
                    let si = ((row * cw + col) * 4) as usize;
                    // Skip fully transparent / empty scratch pixels.
                    if scratch[si + 3] == 0
                        && scratch[si] == 0
                        && scratch[si + 1] == 0
                        && scratch[si + 2] == 0
                    {
                        continue;
                    }
                    let di = ((dy as u32 * frame.width + dx as u32) * 4) as usize;
                    frame.rgba[di..di + 4].copy_from_slice(&scratch[si..si + 4]);
                }
            }

            let _ = DeleteObject(dib);
            let _ = DeleteDC(hdc);
        }
    }

    pub fn run_pump(
        target: CaptureTarget,
        fps: u32,
        include_cursor: bool,
        out_w: u32,
        out_h: u32,
        stop: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        on_frame: &mut dyn FnMut(Vec<u8>) -> bool,
        pip: Option<RecordPip>,
    ) -> Result<()> {
        let (display_id, crop_x, crop_y, crop_w, crop_h) = resolve_target(&target)?;
        let (dup, origin) = open_dup(display_id)?;
        let mut session = DupSession {
            display_id,
            dup,
            last: None,
            origin,
        };

        for _ in 0..50 {
            if grab_monitor(&mut session).is_ok() {
                break;
            }
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(20));
        }

        let frame_interval = Duration::from_secs_f64(1.0 / f64::from(fps));
        let mut next_deadline = Instant::now();

        while !stop.load(Ordering::Relaxed) {
            if paused.load(Ordering::Relaxed) {
                // Do not push frames while paused so the encode timeline skips
                // this wall time (CFR / frame-count PTS stays continuous).
                thread::sleep(frame_interval);
                next_deadline = Instant::now() + frame_interval;
                continue;
            }

            let now = Instant::now();
            if now < next_deadline {
                thread::sleep(next_deadline.saturating_duration_since(now));
            }

            if let Err(e) = grab_monitor(&mut session) {
                tracing::debug!(%e, "DXGI frame miss");
                next_deadline += frame_interval;
                continue;
            }
            let Some(full) = session.last.as_ref() else {
                next_deadline += frame_interval;
                continue;
            };

            let rel_x = (crop_x - session.origin.0).max(0) as u32;
            let rel_y = (crop_y - session.origin.1).max(0) as u32;
            let mut frame = if rel_x == 0
                && rel_y == 0
                && crop_w >= full.width.saturating_sub(1)
                && crop_h >= full.height.saturating_sub(1)
            {
                Frame {
                    width: full.width,
                    height: full.height,
                    rgba: full.rgba.clone(),
                    timestamp_ms: full.timestamp_ms,
                }
            } else {
                crop_bgra(full, rel_x, rel_y, crop_w, crop_h)?
            };

            if include_cursor {
                composite_cursor_bgra(&mut frame, crop_x, crop_y);
            }

            let mut frame = scale_to_exact(frame, out_w, out_h)?;
            if let Some(ref pip) = pip {
                match pip.slot.latest() {
                    Some(cam) => {
                        crate::composite_webcam_pip(&mut frame, &cam, &pip.layout);
                    }
                    None => {
                        static EMPTY: std::sync::atomic::AtomicU32 =
                            std::sync::atomic::AtomicU32::new(0);
                        let n = EMPTY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if n == 30 || n == 150 {
                            tracing::warn!(n, "webcam PiP slot empty while recording");
                        }
                    }
                }
            }
            if !on_frame(std::mem::take(&mut frame.rgba)) {
                break;
            }

            next_deadline += frame_interval;
            // Overran the budget: snap forward so we don't flood the encoder
            // with a catch-up burst (that reads as stutter).
            let behind = Instant::now().saturating_duration_since(next_deadline);
            if behind > frame_interval {
                next_deadline = Instant::now() + frame_interval;
            }
        }
        Ok(())
    }
}
