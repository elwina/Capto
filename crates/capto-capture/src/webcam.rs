//! Native webcam capture via Media Foundation (Captura-style: continuous frames,
//! shared by preview + recording — not FFmpeg dshow).

use crate::{CaptureError, Frame, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebcamInfo {
    /// Stable id for selection (MF symbolic link).
    pub id: String,
    pub name: String,
}

/// Latest webcam frame shared across preview and the DXGI record pump.
/// Frames are `Arc` so the record pump can composite without cloning pixels.
#[derive(Clone)]
pub struct WebcamFrameSlot {
    inner: Arc<Mutex<Option<Arc<Frame>>>>,
}

impl WebcamFrameSlot {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn store(&self, frame: Frame) {
        if let Ok(mut g) = self.inner.lock() {
            *g = Some(Arc::new(frame));
        }
    }

    pub fn latest(&self) -> Option<Arc<Frame>> {
        self.inner.lock().ok().and_then(|g| g.clone())
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            *g = None;
        }
    }
}

impl Default for WebcamFrameSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// Background MF reader that keeps `WebcamFrameSlot` updated.
pub struct WebcamCapture {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    slot: WebcamFrameSlot,
    device_id: String,
}

impl WebcamCapture {
    pub fn slot(&self) -> WebcamFrameSlot {
        self.slot.clone()
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn start(device_id: Option<&str>, target_w: u32, target_h: u32) -> Result<Self> {
        #[cfg(not(windows))]
        {
            let _ = (device_id, target_w, target_h);
            return Err(CaptureError::Unsupported("webcam capture is Windows-only"));
        }
        #[cfg(windows)]
        {
            windows_impl::start(device_id, target_w.max(2), target_h.max(2))
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.slot.clear();
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.slot.clear();
    }
}

pub fn list_webcams() -> Result<Vec<WebcamInfo>> {
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
    #[cfg(windows)]
    {
        windows_impl::list_webcams()
    }
}

/// Process-wide webcam for preview (released when preview off or recording takes over).
static PREVIEW_CAM: Mutex<Option<WebcamCapture>> = Mutex::new(None);

pub fn ensure_preview_webcam(
    device_id: Option<&str>,
    target_w: u32,
    target_h: u32,
) -> Result<WebcamFrameSlot> {
    let mut guard = PREVIEW_CAM
        .lock()
        .map_err(|_| CaptureError::Failed("webcam lock poisoned".into()))?;
    let want = device_id.unwrap_or("").to_string();
    let reuse = guard.as_ref().is_some_and(|c| {
        if want.is_empty() {
            !c.device_id().is_empty()
        } else {
            c.device_id() == want
        }
    });
    if !reuse {
        *guard = None;
        let cam = WebcamCapture::start(device_id, target_w, target_h)?;
        let slot = cam.slot();
        *guard = Some(cam);
        Ok(slot)
    } else {
        Ok(guard.as_ref().unwrap().slot())
    }
}

pub fn preview_webcam_slot() -> Option<WebcamFrameSlot> {
    PREVIEW_CAM
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.slot()))
}

pub fn release_preview_webcam() {
    if let Ok(mut g) = PREVIEW_CAM.lock() {
        *g = None;
    }
}

/// Take ownership of the process webcam for recording.
/// Prefers reusing the live preview capture (already producing frames) to avoid
/// a long MF reopen gap at the start of the file.
pub fn take_webcam_for_record(
    device_id: Option<&str>,
    target_w: u32,
    target_h: u32,
) -> Result<WebcamCapture> {
    let want = device_id.map(str::trim).filter(|s| !s.is_empty());
    {
        let mut guard = PREVIEW_CAM
            .lock()
            .map_err(|_| CaptureError::Failed("webcam lock poisoned".into()))?;
        if let Some(existing) = guard.as_ref() {
            let same = match want {
                None => true,
                Some(w) => {
                    let id = existing.device_id();
                    id == w || id.contains(w) || w.contains(id)
                }
            };
            if same && existing.slot().latest().is_some() {
                tracing::info!(device = %existing.device_id(), "reusing preview webcam for record");
                return Ok(guard.take().expect("webcam present"));
            }
        }
        *guard = None;
    }
    // Previous MF graph needs a moment to release the device exclusively.
    thread::sleep(Duration::from_millis(400));
    let cam = WebcamCapture::start(device_id, target_w, target_h)?;
    if cam.slot().latest().is_none() {
        return Err(CaptureError::Failed(
            "webcam opened but produced no frames".into(),
        ));
    }
    Ok(cam)
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::core::{Interface, GUID, PWSTR};
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_MULTITHREADED};

    static MF_ONCE: Once = Once::new();

    fn hr_err(ctx: &str, e: windows::core::Error) -> CaptureError {
        CaptureError::Failed(format!("{ctx}: {e}"))
    }

    fn ensure_mf() -> Result<()> {
        let mut err: Option<windows::core::Error> = None;
        MF_ONCE.call_once(|| unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            if let Err(e) = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) {
                err = Some(e);
            }
        });
        if let Some(e) = err {
            return Err(hr_err("MFStartup", e));
        }
        Ok(())
    }

    fn create_attributes(capacity: u32) -> Result<IMFAttributes> {
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, capacity)
                .map_err(|e| hr_err("MFCreateAttributes", e))?;
            attrs.ok_or_else(|| CaptureError::Failed("MFCreateAttributes returned null".into()))
        }
    }

    pub fn list_webcams() -> Result<Vec<WebcamInfo>> {
        ensure_mf()?;
        unsafe {
            let devices = enum_devices()?;
            Ok(devices
                .into_iter()
                .map(|(_, id, name)| WebcamInfo { id, name })
                .collect())
        }
    }

    unsafe fn read_string_attr(activate: &IMFActivate, key: &GUID) -> Option<String> {
        let mut len = 0u32;
        let mut ptr = PWSTR::null();
        activate.GetAllocatedString(key, &mut ptr, &mut len).ok()?;
        if ptr.is_null() {
            return None;
        }
        let s = ptr.to_string().ok();
        CoTaskMemFree(Some(ptr.0 as *const _));
        s
    }

    unsafe fn enum_devices() -> Result<Vec<(IMFActivate, String, String)>> {
        let attrs = create_attributes(1)?;
        attrs
            .SetGUID(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
            )
            .map_err(|e| hr_err("SetGUID", e))?;

        let mut devices: *mut Option<IMFActivate> = std::ptr::null_mut();
        let mut count = 0u32;
        MFEnumDeviceSources(&attrs, &mut devices, &mut count)
            .map_err(|e| hr_err("MFEnumDeviceSources", e))?;
        if devices.is_null() || count == 0 {
            return Ok(Vec::new());
        }

        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            let slot = &mut *devices.add(i);
            let Some(activate) = slot.take() else {
                continue;
            };
            let name = read_string_attr(&activate, &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME)
                .unwrap_or_else(|| format!("Camera {i}"));
            let id = read_string_attr(
                &activate,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
            )
            .unwrap_or_else(|| format!("cam-{i}"));
            out.push((activate, id, name));
        }
        CoTaskMemFree(Some(devices as *const _));
        Ok(out)
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum CamFormat {
        Yuy2,
        Nv12,
        Rgb32,
    }

    pub fn start(device_id: Option<&str>, target_w: u32, target_h: u32) -> Result<WebcamCapture> {
        ensure_mf()?;

        let slot = WebcamFrameSlot::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let slot2 = slot.clone();
        let want = device_id.unwrap_or("").to_string();

        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<String>>();

        let join_handle = thread::Builder::new()
            .name("capto-webcam".into())
            .spawn(move || {
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                let open = open_reader(
                    if want.is_empty() { None } else { Some(&want) },
                    target_w,
                    target_h,
                );
                let (reader, id, format) = match open {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = ready_tx.send(Err(CaptureError::Failed(e.to_string())));
                        return;
                    }
                };

                // Don't signal ready until a real frame is in the slot — otherwise
                // recording starts with an empty PiP for several seconds.
                let mut announced = false;
                let mut empty_spins = 0u32;
                let mut err_spins = 0u32;
                while !stop2.load(Ordering::Relaxed) {
                    match read_frame(&reader, format, target_w, target_h) {
                        ReadOutcome::Frame(frame) => {
                            empty_spins = 0;
                            err_spins = 0;
                            slot2.store(frame);
                            if !announced {
                                announced = true;
                                let _ = ready_tx.send(Ok(id.clone()));
                            }
                        }
                        ReadOutcome::Again => {
                            // Stream tick / no sample: ReadSample already waited;
                            // do not sleep or we throttle the cam to ~<15 FPS.
                            empty_spins = empty_spins.saturating_add(1);
                            if !announced && empty_spins > 300 {
                                let _ = ready_tx.send(Err(CaptureError::Failed(
                                    "webcam produced no frames after open".into(),
                                )));
                                return;
                            }
                        }
                        ReadOutcome::Error => {
                            err_spins = err_spins.saturating_add(1);
                            if !announced && err_spins > 200 {
                                let _ = ready_tx.send(Err(CaptureError::Failed(
                                    "webcam produced no frames after open".into(),
                                )));
                                return;
                            }
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
                drop(reader);
            })
            .map_err(|e| CaptureError::Failed(e.to_string()))?;

        let device_id_owned = match ready_rx.recv_timeout(Duration::from_secs(8)) {
            Ok(Ok(id)) => id,
            Ok(Err(e)) => {
                stop.store(true, Ordering::SeqCst);
                let _ = join_handle.join();
                return Err(e);
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                let _ = join_handle.join();
                return Err(CaptureError::Failed("webcam open timed out".into()));
            }
        };

        Ok(WebcamCapture {
            stop,
            thread: Some(join_handle),
            slot,
            device_id: device_id_owned,
        })
    }

    fn open_reader(
        device_id: Option<&str>,
        target_w: u32,
        target_h: u32,
    ) -> Result<(IMFSourceReader, String, CamFormat)> {
        unsafe {
            let devices = enum_devices()?;
            if devices.is_empty() {
                return Err(CaptureError::Failed("no webcam devices found".into()));
            }

            let want = device_id.map(str::trim).filter(|s| !s.is_empty());
            let mut chosen: Option<(IMFActivate, String)> = None;
            let mut first: Option<(IMFActivate, String)> = None;
            for (activate, id, name) in devices {
                if first.is_none() {
                    first = Some((activate.clone(), id.clone()));
                }
                let match_id = want.is_some_and(|w| {
                    w == id
                        || w == name
                        || name.eq_ignore_ascii_case(w)
                        || name.contains(w)
                        || id.contains(w)
                });
                if want.is_none() && chosen.is_none() {
                    chosen = Some((activate, id));
                } else if match_id {
                    chosen = Some((activate, id));
                    break;
                }
            }

            // Stale browser deviceIds / renamed cameras: fall back to default.
            let (activate, id) = chosen
                .or(first)
                .ok_or_else(|| CaptureError::Failed("requested webcam not found".into()))?;
            if want.is_some() && want != Some(id.as_str()) {
                tracing::info!(
                    requested = want.unwrap_or(""),
                    using = %id,
                    "webcam id not exact; using fallback device"
                );
            }

            let source: IMFMediaSource = activate
                .ActivateObject()
                .map_err(|e| hr_err("ActivateObject", e))?;

            // Enable converters so MJPG→RGB32 still works as a last resort, but we
            // prefer a native YUY2/NV12 type so MF does not software-RGB every frame.
            let reader_attrs = create_attributes(2)?;
            let _ = reader_attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1);
            let _ = reader_attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1);

            let reader = MFCreateSourceReaderFromMediaSource(&source, &reader_attrs)
                .map_err(|e| hr_err("MFCreateSourceReaderFromMediaSource", e))?;

            let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            let (format, w, h, fps_n, fps_d) =
                configure_media_type(&reader, stream, target_w, target_h)?;
            tracing::info!(
                %id,
                ?format,
                width = w,
                height = h,
                fps = format!("{fps_n}/{fps_d}"),
                target_w,
                target_h,
                "webcam media type selected"
            );

            Ok((reader, id, format))
        }
    }

    /// Pick a native type that can sustain ~30 FPS for PiP. Forced RGB32 via MF
    /// converters often collapses consumer cameras to single-digit FPS.
    unsafe fn configure_media_type(
        reader: &IMFSourceReader,
        stream: u32,
        target_w: u32,
        target_h: u32,
    ) -> Result<(CamFormat, u32, u32, u32, u32)> {
        let ideal_w = target_w.saturating_mul(2).clamp(640, 1280);
        let ideal_h = target_h.saturating_mul(2).clamp(480, 720);

        let mut best: Option<(i64, IMFMediaType, CamFormat)> = None;
        let mut index = 0u32;
        loop {
            let mt = match reader.GetNativeMediaType(stream, index) {
                Ok(mt) => mt,
                Err(_) => break,
            };
            index += 1;
            let Ok(major) = mt.GetGUID(&MF_MT_MAJOR_TYPE) else {
                continue;
            };
            if major != MFMediaType_Video {
                continue;
            }
            let Ok(subtype) = mt.GetGUID(&MF_MT_SUBTYPE) else {
                continue;
            };
            let format = if subtype == MFVideoFormat_YUY2 {
                CamFormat::Yuy2
            } else if subtype == MFVideoFormat_NV12 {
                CamFormat::Nv12
            } else if subtype == MFVideoFormat_RGB32 {
                CamFormat::Rgb32
            } else {
                // Skip MJPG/etc. here — decode path is the RGB32 fallback below.
                continue;
            };
            let Ok(packed) = mt.GetUINT64(&MF_MT_FRAME_SIZE) else {
                continue;
            };
            let w = (packed >> 32) as u32;
            let h = (packed & 0xffff_ffff) as u32;
            if w < 160 || h < 120 {
                continue;
            }
            let (fps_n, fps_d) = frame_rate_of(&mt);
            let fps = if fps_d == 0 {
                0i64
            } else {
                i64::from(fps_n) / i64::from(fps_d)
            };
            let format_bias = match format {
                CamFormat::Yuy2 => 80_000,
                CamFormat::Nv12 => 70_000,
                CamFormat::Rgb32 => 20_000,
            };
            let size_penalty =
                ((w as i64) - ideal_w as i64).abs() + ((h as i64) - ideal_h as i64).abs();
            // Prefer ≥24 FPS heavily; 30 FPS is the PiP sweet spot.
            let fps_score = if fps >= 28 {
                fps * 2_000
            } else if fps >= 24 {
                fps * 1_500
            } else if fps >= 15 {
                fps * 400
            } else {
                fps * 50 - 30_000
            };
            let score = format_bias + fps_score - size_penalty * 15;
            if best.as_ref().is_none_or(|(s, _, _)| score > *s) {
                best = Some((score, mt, format));
            }
        }

        if let Some((_, mt, format)) = best {
            reader
                .SetCurrentMediaType(stream, None, &mt)
                .map_err(|e| hr_err("SetCurrentMediaType native", e))?;
            let (w, h) = size_of(&mt).unwrap_or((ideal_w, ideal_h));
            let (fps_n, fps_d) = frame_rate_of(&mt);
            return Ok((format, w, h, fps_n, fps_d));
        }

        // Fallback: ask MF to deliver RGB32 (may decode MJPG). Still aim for 640x480@30.
        for (w, h) in [(ideal_w, ideal_h), (640, 480), (1280, 720)] {
            let media_type = MFCreateMediaType().map_err(|e| hr_err("MFCreateMediaType", e))?;
            media_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| hr_err("major type", e))?;
            media_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
                .map_err(|e| hr_err("subtype", e))?;
            let _ = media_type.SetUINT64(&MF_MT_FRAME_SIZE, (u64::from(w) << 32) | u64::from(h));
            let _ = media_type.SetUINT64(&MF_MT_FRAME_RATE, (30u64 << 32) | 1);
            if reader
                .SetCurrentMediaType(stream, None, &media_type)
                .is_ok()
            {
                let mt = reader
                    .GetCurrentMediaType(stream)
                    .map_err(|e| hr_err("GetCurrentMediaType", e))?;
                let subtype = mt
                    .GetGUID(&MF_MT_SUBTYPE)
                    .map_err(|e| hr_err("subtype confirm", e))?;
                if subtype != MFVideoFormat_RGB32 {
                    continue;
                }
                let (cw, ch) = size_of(&mt).unwrap_or((w, h));
                let (fps_n, fps_d) = frame_rate_of(&mt);
                return Ok((CamFormat::Rgb32, cw, ch, fps_n, fps_d));
            }
        }

        Err(CaptureError::Failed(
            "no usable webcam media type (YUY2/NV12/RGB32)".into(),
        ))
    }

    unsafe fn size_of(mt: &IMFMediaType) -> Option<(u32, u32)> {
        let packed = mt.GetUINT64(&MF_MT_FRAME_SIZE).ok()?;
        let width = (packed >> 32) as u32;
        let height = (packed & 0xffff_ffff) as u32;
        if width < 2 || height < 2 {
            None
        } else {
            Some((width, height))
        }
    }

    unsafe fn frame_rate_of(mt: &IMFMediaType) -> (u32, u32) {
        mt.GetUINT64(&MF_MT_FRAME_RATE)
            .ok()
            .map(|packed| ((packed >> 32) as u32, (packed & 0xffff_ffff) as u32))
            .filter(|(_, d)| *d > 0)
            .unwrap_or((30, 1))
    }

    enum ReadOutcome {
        Frame(Frame),
        Again,
        Error,
    }

    fn read_frame(
        reader: &IMFSourceReader,
        format: CamFormat,
        target_w: u32,
        target_h: u32,
    ) -> ReadOutcome {
        unsafe {
            let mut flags = 0u32;
            let mut timestamp = 0i64;
            let mut sample: Option<IMFSample> = None;
            if reader
                .ReadSample(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    0,
                    None,
                    Some(&mut flags),
                    Some(&mut timestamp),
                    Some(&mut sample as *mut _),
                )
                .is_err()
            {
                return ReadOutcome::Error;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return ReadOutcome::Error;
            }
            let Some(sample) = sample else {
                return ReadOutcome::Again;
            };
            let Ok(buffer) = sample.ConvertToContiguousBuffer() else {
                return ReadOutcome::Again;
            };
            let Some((width, height)) = current_size(reader) else {
                return ReadOutcome::Again;
            };
            let ts = (timestamp / 10_000).max(0) as u64;

            let bgra = match format {
                CamFormat::Yuy2 => match with_locked_buffer(&buffer, |ptr, len| {
                    yuy2_to_bgra(ptr, len, width, height)
                }) {
                    Some(v) => v,
                    None => return ReadOutcome::Again,
                },
                CamFormat::Nv12 => match with_locked_buffer(&buffer, |ptr, len| {
                    nv12_to_bgra(ptr, len, width, height)
                }) {
                    Some(v) => v,
                    None => return ReadOutcome::Again,
                },
                CamFormat::Rgb32 => match read_rgb32(&buffer, reader, width, height) {
                    Some(v) => v,
                    None => return ReadOutcome::Again,
                },
            };

            let frame = Frame {
                width,
                height,
                rgba: bgra,
                timestamp_ms: ts,
            };
            ReadOutcome::Frame(scale_bgra(frame, target_w, target_h))
        }
    }

    unsafe fn current_size(reader: &IMFSourceReader) -> Option<(u32, u32)> {
        let mt = reader
            .GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32)
            .ok()?;
        size_of(&mt)
    }

    unsafe fn current_bottom_up(reader: &IMFSourceReader) -> Option<bool> {
        let mt = reader
            .GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32)
            .ok()?;
        mt.GetUINT32(&MF_MT_DEFAULT_STRIDE)
            .ok()
            .map(|s| (s as i32) < 0)
    }

    unsafe fn with_locked_buffer<R>(
        buffer: &IMFMediaBuffer,
        f: impl FnOnce(*const u8, usize) -> Option<R>,
    ) -> Option<R> {
        let mut scanline0: *mut u8 = std::ptr::null_mut();
        let mut max_len = 0u32;
        let mut cur_len = 0u32;
        buffer
            .Lock(&mut scanline0, Some(&mut max_len), Some(&mut cur_len))
            .ok()?;
        if scanline0.is_null() || cur_len == 0 {
            let _ = buffer.Unlock();
            return None;
        }
        let out = f(scanline0 as *const u8, cur_len as usize);
        let _ = buffer.Unlock();
        out
    }

    fn yuy2_to_bgra(ptr: *const u8, len: usize, width: u32, height: u32) -> Option<Vec<u8>> {
        let needed = (width as usize) * (height as usize) * 2;
        if len < needed || width < 2 {
            return None;
        }
        let src = unsafe { std::slice::from_raw_parts(ptr, needed) };
        let mut out = vec![0u8; (width * height * 4) as usize];
        let row_pairs = (width as usize) / 2;
        for y in 0..height as usize {
            let src_row = y * width as usize * 2;
            let dst_row = y * width as usize * 4;
            for p in 0..row_pairs {
                let i = src_row + p * 4;
                let y0 = src[i] as i32;
                let u = src[i + 1] as i32 - 128;
                let y1 = src[i + 2] as i32;
                let v = src[i + 3] as i32 - 128;
                let (b0, g0, r0) = yuv_to_bgr(y0, u, v);
                let (b1, g1, r1) = yuv_to_bgr(y1, u, v);
                let o = dst_row + p * 8;
                out[o] = b0;
                out[o + 1] = g0;
                out[o + 2] = r0;
                out[o + 3] = 255;
                out[o + 4] = b1;
                out[o + 5] = g1;
                out[o + 6] = r1;
                out[o + 7] = 255;
            }
        }
        Some(out)
    }

    fn nv12_to_bgra(ptr: *const u8, len: usize, width: u32, height: u32) -> Option<Vec<u8>> {
        let y_size = (width as usize) * (height as usize);
        let needed = y_size + y_size / 2;
        if len < needed || width < 2 || height < 2 {
            return None;
        }
        let src = unsafe { std::slice::from_raw_parts(ptr, needed) };
        let mut out = vec![0u8; (width * height * 4) as usize];
        let uv_base = y_size;
        for y in 0..height as usize {
            let y_row = y * width as usize;
            let uv_row = uv_base + (y / 2) * width as usize;
            let dst_row = y_row * 4;
            for x in 0..width as usize {
                let yi = src[y_row + x] as i32;
                let uv = uv_row + (x & !1);
                let u = src[uv] as i32 - 128;
                let v = src[uv + 1] as i32 - 128;
                let (b, g, r) = yuv_to_bgr(yi, u, v);
                let o = dst_row + x * 4;
                out[o] = b;
                out[o + 1] = g;
                out[o + 2] = r;
                out[o + 3] = 255;
            }
        }
        Some(out)
    }

    #[inline]
    fn yuv_to_bgr(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
        let r = y + ((359 * v) >> 8);
        let g = y - ((88 * u + 183 * v) >> 8);
        let b = y + ((454 * u) >> 8);
        (
            b.clamp(0, 255) as u8,
            g.clamp(0, 255) as u8,
            r.clamp(0, 255) as u8,
        )
    }

    unsafe fn read_rgb32(
        buffer: &IMFMediaBuffer,
        reader: &IMFSourceReader,
        width: u32,
        height: u32,
    ) -> Option<Vec<u8>> {
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        if let Ok(buf2d) = buffer.cast::<IMF2DBuffer>() {
            let mut scanline0: *mut u8 = std::ptr::null_mut();
            let mut pitch = 0i32;
            if buf2d.Lock2D(&mut scanline0, &mut pitch).is_err() || scanline0.is_null() {
                return None;
            }
            let abs_pitch = pitch.unsigned_abs() as usize;
            let row_bytes = (width * 4) as usize;
            if pitch == 0 || abs_pitch < row_bytes {
                let _ = buf2d.Unlock2D();
                return None;
            }
            for y in 0..height as isize {
                let src = scanline0.offset(y * pitch as isize);
                let dst = (y as usize) * row_bytes;
                std::ptr::copy_nonoverlapping(src, bgra[dst..].as_mut_ptr(), row_bytes);
                for px in bgra[dst..dst + row_bytes].chunks_exact_mut(4) {
                    px[3] = 255;
                }
            }
            let _ = buf2d.Unlock2D();
            return Some(bgra);
        }

        let mut scanline0: *mut u8 = std::ptr::null_mut();
        let mut max_len = 0u32;
        let mut cur_len = 0u32;
        buffer
            .Lock(&mut scanline0, Some(&mut max_len), Some(&mut cur_len))
            .ok()?;
        let expected = (width * height * 4) as u32;
        if scanline0.is_null() || cur_len < expected {
            let _ = buffer.Unlock();
            return None;
        }
        let src = std::slice::from_raw_parts(scanline0, expected as usize);
        let stride = (width * 4) as usize;
        let bottom_up = current_bottom_up(reader).unwrap_or(false);
        for y in 0..height as usize {
            let src_y = if bottom_up {
                height as usize - 1 - y
            } else {
                y
            };
            let src_row = src_y * stride;
            let dst_row = y * stride;
            bgra[dst_row..dst_row + stride].copy_from_slice(&src[src_row..src_row + stride]);
            for px in bgra[dst_row..dst_row + stride].chunks_exact_mut(4) {
                px[3] = 255;
            }
        }
        let _ = buffer.Unlock();
        Some(bgra)
    }

    fn scale_bgra(frame: Frame, out_w: u32, out_h: u32) -> Frame {
        if frame.width == out_w && frame.height == out_h {
            return frame;
        }
        let out_w = out_w.max(1);
        let out_h = out_h.max(1);
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
        Frame {
            width: out_w,
            height: out_h,
            rgba: out,
            timestamp_ms: frame.timestamp_ms,
        }
    }
}
