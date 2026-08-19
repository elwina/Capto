use crate::ffmpeg_args::{build_record_args, record_frame_size, RecordRequest, Region};
use crate::settings::{AppSettings, OutputFormat};
use crate::{CoreError, Result};
use capto_capture::{
    create_default_backend, CaptureBackend, CaptureTarget, DxgiRecordPump, RecordPip, WebcamCapture,
};
use capto_encode::{FfmpegEncoder, VideoEncoderKind};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Child;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Idle,
    Starting,
    Recording,
    Paused,
    Stopping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub elapsed_ms: u64,
    pub output_path: Option<String>,
    pub last_error: Option<String>,
    pub encoder: Option<String>,
    pub hide_app: bool,
}

struct LiveRecording {
    child: Child,
    audio_session: Option<capto_audio::NativeAudioSession>,
    video_pump: Option<DxgiRecordPump>,
    /// Keeps MF webcam alive while the DXGI pump composites PiP.
    _webcam: Option<WebcamCapture>,
    /// Rolling tail of ffmpeg stderr for this run, used to explain failures.
    stderr_log: Arc<Mutex<String>>,
    output_path: PathBuf,
    started_at: std::time::Instant,
    paused_accum_ms: u64,
    pause_started: Option<std::time::Instant>,
    encoder: VideoEncoderKind,
    hide_app: bool,
}

fn capture_target_for(req: &RecordRequest) -> CaptureTarget {
    match req.source {
        crate::VideoSourceKind::Display => CaptureTarget::Display {
            id: req.display_id.unwrap_or(0),
        },
        crate::VideoSourceKind::Window => {
            if let Some(id) = req.window_id {
                CaptureTarget::Window { id }
            } else if let Some(r) = &req.region {
                CaptureTarget::Region {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                }
            } else {
                CaptureTarget::Display {
                    id: req.display_id.unwrap_or(0),
                }
            }
        }
        crate::VideoSourceKind::Region => {
            if let Some(r) = &req.region {
                CaptureTarget::Region {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                }
            } else {
                CaptureTarget::Display {
                    id: req.display_id.unwrap_or(0),
                }
            }
        }
    }
}

async fn attach_dxgi_pump(
    child: &mut Child,
    req: &RecordRequest,
    pip: Option<RecordPip>,
) -> Result<DxgiRecordPump> {
    capto_capture::release_preview_session();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| CoreError::Message("ffmpeg stdin unavailable".into()))?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);
    let (out_w, out_h) = record_frame_size(req);
    let target = capture_target_for(req);
    let pump = DxgiRecordPump::start(
        target,
        req.fps,
        req.include_cursor,
        out_w,
        out_h,
        move |frame| tx.blocking_send(frame).is_ok(),
        pip,
    )
    .map_err(|e| CoreError::Message(e.to_string()))?;

    tokio::spawn(async move {
        let mut stdin = stdin;
        while let Some(buf) = rx.recv().await {
            if stdin.write_all(&buf).await.is_err() {
                break;
            }
        }
        // Closing stdin signals EOF on the rawvideo input.
        drop(stdin);
    });

    Ok(pump)
}

struct BootedPipeline {
    child: Child,
    stderr_log: Arc<Mutex<String>>,
    audio_session: Option<capto_audio::NativeAudioSession>,
    video_pump: Option<DxgiRecordPump>,
    webcam: Option<WebcamCapture>,
    encoder: VideoEncoderKind,
    /// When DXGI/WASAPI producers began feeding FFmpeg (before health check).
    encode_started_at: std::time::Instant,
}

/// Open webcam (if needed) before FFmpeg so PiP frames exist from frame 0,
/// then spawn FFmpeg and the DXGI pump.
async fn boot_pipeline(
    ffmpeg: &FfmpegEncoder,
    req: &RecordRequest,
    encoder: VideoEncoderKind,
) -> Result<BootedPipeline> {
    let mut audio_session = if req.format == OutputFormat::Gif {
        None
    } else {
        capto_audio::NativeAudioSession::prepare(
            req.mic_device.as_deref(),
            req.loopback_device.as_deref(),
        )
        .map_err(|e| CoreError::Message(e.to_string()))?
    };
    let pcm_inputs = audio_session
        .as_ref()
        .map(|session| session.inputs())
        .unwrap_or(&[]);

    // Warm the camera before the encode clock starts — reopening MF after
    // preview can take seconds; doing it post-spawn leaves a blank PiP intro.
    let mut webcam = None;
    let pip = if req.overlays.webcam.enabled {
        let cam = &req.overlays.webcam;
        let device = cam
            .device_id
            .as_deref()
            .or(cam.device_label.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match tokio::task::spawn_blocking({
            let device = device.map(str::to_owned);
            let w = cam.width.max(2);
            let h = cam.height.max(2);
            move || capto_capture::take_webcam_for_record(device.as_deref(), w, h)
        })
        .await
        .map_err(|e| CoreError::Message(e.to_string()))?
        {
            Ok(capture) => {
                tracing::info!(
                    device = %capture.device_id(),
                    w = cam.width,
                    h = cam.height,
                    has_frame = capture.slot().latest().is_some(),
                    "webcam PiP capture ready for record"
                );
                let layout = cam.clone();
                let slot = capture.slot();
                webcam = Some(capture);
                Some(RecordPip { slot, layout })
            }
            Err(e) => {
                tracing::warn!(%e, "webcam open failed; recording screen only");
                None
            }
        }
    } else {
        capto_capture::release_preview_webcam();
        None
    };

    let args = build_record_args(req, encoder, pcm_inputs);
    let (mut child, stderr_log) = ffmpeg.spawn(&args).await?;

    let video_pump = if req.format == OutputFormat::AudioOnly {
        None
    } else {
        match attach_dxgi_pump(&mut child, req, pip).await {
            Ok(pump) => Some(pump),
            Err(e) => {
                drop(webcam);
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(e);
            }
        }
    };

    if let Some(audio) = audio_session.as_mut() {
        if let Err(error) = audio.start() {
            drop(video_pump);
            drop(webcam);
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(CoreError::Message(error.to_string()));
        }
    }

    // Producers are live here — UI elapsed must match encode timeline, not the
    // later return from check_started (which sleeps ~2.5s while frames already flow).
    let encode_started_at = std::time::Instant::now();

    if let Err(error) = ffmpeg.check_started(&mut child, &stderr_log).await {
        if let Some(audio) = audio_session.as_mut() {
            audio.stop();
        }
        drop(video_pump);
        drop(webcam);
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err(error.into());
    }

    Ok(BootedPipeline {
        child,
        stderr_log,
        audio_session,
        video_pump,
        webcam,
        encoder,
        encode_started_at,
    })
}

pub struct RecordingSession {
    settings: AppSettings,
    capture: Box<dyn CaptureBackend>,
    encoder: Option<FfmpegEncoder>,
    live: Arc<Mutex<Option<LiveRecording>>>,
    last_error: Arc<Mutex<Option<String>>>,
    sidecar_dir: Option<PathBuf>,
}

impl RecordingSession {
    pub fn new(settings: AppSettings, sidecar_dir: Option<PathBuf>) -> Self {
        Self {
            settings,
            capture: create_default_backend(),
            encoder: FfmpegEncoder::discover(sidecar_dir.as_deref()).ok(),
            live: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
            sidecar_dir,
        }
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut AppSettings {
        &mut self.settings
    }

    pub fn capture(&self) -> &dyn CaptureBackend {
        self.capture.as_ref()
    }

    pub fn refresh_encoder(&mut self) -> Result<()> {
        self.encoder = Some(FfmpegEncoder::discover(self.sidecar_dir.as_deref())?);
        Ok(())
    }

    pub fn encoder(&self) -> Option<&FfmpegEncoder> {
        self.encoder.as_ref()
    }

    /// Consumes recent native audio peaks for lightweight UI metering.
    pub fn audio_levels(&self) -> capto_audio::AudioLevels {
        let Ok(live) = self.live.try_lock() else {
            return Default::default();
        };
        live.as_ref()
            .and_then(|rec| rec.audio_session.as_ref())
            .map(|audio| audio.levels())
            .unwrap_or_default()
    }

    pub async fn snapshot(&self) -> SessionSnapshot {
        let live = self.live.lock().await;
        let err = self.last_error.lock().await.clone();
        if let Some(rec) = live.as_ref() {
            let mut elapsed = rec.started_at.elapsed().as_millis() as u64;
            elapsed = elapsed.saturating_sub(rec.paused_accum_ms);
            if let Some(p) = rec.pause_started {
                elapsed = elapsed.saturating_sub(p.elapsed().as_millis() as u64);
            }
            let state = if rec.pause_started.is_some() {
                SessionState::Paused
            } else {
                SessionState::Recording
            };
            SessionSnapshot {
                state,
                elapsed_ms: elapsed,
                output_path: Some(rec.output_path.to_string_lossy().into_owned()),
                last_error: err,
                encoder: Some(rec.encoder.ffmpeg_name().into()),
                hide_app: rec.hide_app,
            }
        } else {
            SessionSnapshot {
                state: SessionState::Idle,
                elapsed_ms: 0,
                output_path: None,
                last_error: err,
                encoder: None,
                hide_app: false,
            }
        }
    }

    pub fn make_output_path(&self, format: OutputFormat) -> PathBuf {
        let _ = self.settings.ensure_output_dir();
        let ext = match format {
            OutputFormat::Mp4 => "mp4",
            OutputFormat::Gif => "gif",
            OutputFormat::AudioOnly => "m4a",
        };
        let stamp = Local::now().format("%Y%m%d-%H%M%S");
        PathBuf::from(&self.settings.output_dir).join(format!(
            "capto-{stamp}-{}.{ext}",
            &Uuid::new_v4().to_string()[..8]
        ))
    }

    pub async fn start(&self, mut req: RecordRequest) -> Result<(SessionSnapshot, Option<Region>)> {
        {
            let live = self.live.lock().await;
            if live.is_some() {
                return Err(CoreError::InvalidState("already recording".into()));
            }
        }

        let ffmpeg = self
            .encoder
            .as_ref()
            .ok_or(capto_encode::EncodeError::FfmpegNotFound)?;

        // Resolve capture geometry into a physical screen region for gdigrab crop.
        // Window ids come from HWND (picker), not xcap list indices.
        if req.source == crate::VideoSourceKind::Window {
            if let Some(id) = req.window_id {
                if let Ok(Some(w)) = capto_capture::window_by_id(id) {
                    req.region = Some(crate::ffmpeg_args::Region {
                        x: w.x,
                        y: w.y,
                        width: w.width,
                        height: w.height,
                    });
                }
            }
        }

        if req.source == crate::VideoSourceKind::Display {
            let id = req.display_id.unwrap_or(0);
            // Prefer Win32 physical monitor rects (same space as DXGI). xcap sizes can
            // disagree under mixed DPI and produced wrong crops (e.g. 2570x1550).
            let monitors = capto_capture::list_monitor_rects();
            if let Some(m) = monitors.get(id as usize) {
                req.region = Some(crate::ffmpeg_args::Region {
                    x: m.x,
                    y: m.y,
                    width: m.width,
                    height: m.height,
                });
            } else if let Ok(displays) = self.capture.list_displays() {
                if let Some(d) = displays.into_iter().find(|d| d.id == id) {
                    let screen = capto_capture::virtual_screen();
                    let (x, y, width, height) =
                        normalize_display_rect(d.x, d.y, d.width, d.height, &screen);
                    req.region = Some(crate::ffmpeg_args::Region {
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
        }

        if let Some(region) = req.region.as_mut() {
            let screen = capto_capture::virtual_screen();
            if let Some((x, y, w, h)) =
                screen.clamp_rect(region.x, region.y, region.width, region.height)
            {
                region.x = x;
                region.y = y;
                region.width = w;
                region.height = h;
            } else {
                return Err(CoreError::InvalidState(
                    "capture region is outside the virtual desktop".into(),
                ));
            }
        }

        let encoder = if req.format == OutputFormat::Gif {
            VideoEncoderKind::Gif
        } else if let Some(e) = req.encoder {
            e
        } else if let Some(pref) = self.settings.preferred_encoder {
            pref
        } else {
            ffmpeg
                .pick_best_h264()
                .await
                .unwrap_or(VideoEncoderKind::Libx264)
        };

        // Webcam PiP is composited in-process; no FFmpeg dshow resolve needed.
        // Soft-fail happens inside boot_pipeline if MF cannot open the camera.

        let out = PathBuf::from(&req.output_path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut pipeline = match boot_pipeline(ffmpeg, &req, encoder).await {
            Ok(p) => p,
            Err(error)
                if encoder != VideoEncoderKind::Libx264 && req.format == OutputFormat::Mp4 =>
            {
                tracing::warn!(%error, "encoder failed, falling back to libx264");
                match boot_pipeline(ffmpeg, &req, VideoEncoderKind::Libx264).await {
                    Ok(p) => p,
                    Err(fallback_error) => {
                        *self.last_error.lock().await = Some(fallback_error.to_string());
                        return Err(fallback_error);
                    }
                }
            }
            Err(error) => {
                *self.last_error.lock().await = Some(error.to_string());
                return Err(error);
            }
        };

        let hide_app = req.hide_app_while_recording;
        let output_path = req.output_path.clone();
        let encoder_name = pipeline.encoder.ffmpeg_name().to_string();
        let encode_started_at = pipeline.encode_started_at;
        let elapsed_ms = encode_started_at.elapsed().as_millis() as u64;
        *self.live.lock().await = Some(LiveRecording {
            child: pipeline.child,
            audio_session: pipeline.audio_session.take(),
            video_pump: pipeline.video_pump.take(),
            _webcam: pipeline.webcam.take(),
            stderr_log: pipeline.stderr_log,
            output_path: out,
            started_at: encode_started_at,
            paused_accum_ms: 0,
            pause_started: None,
            encoder: pipeline.encoder,
            hide_app,
        });
        *self.last_error.lock().await = None;
        let region = req.region.clone();
        Ok((
            SessionSnapshot {
                state: SessionState::Recording,
                elapsed_ms,
                output_path: Some(output_path),
                last_error: None,
                encoder: Some(encoder_name),
                hide_app,
            },
            region,
        ))
    }

    pub async fn pause(&self) -> Result<SessionSnapshot> {
        let mut live = self.live.lock().await;
        let rec = live
            .as_mut()
            .ok_or_else(|| CoreError::InvalidState("not recording".into()))?;
        if rec.pause_started.is_some() {
            return Err(CoreError::InvalidState("already paused".into()));
        }
        // Stop feeding FFmpeg so encode timeline skips paused wall time.
        if let Some(pump) = rec.video_pump.as_ref() {
            pump.set_paused(true);
        }
        if let Some(audio) = rec.audio_session.as_ref() {
            audio.set_paused(true);
        }
        rec.pause_started = Some(std::time::Instant::now());
        drop(live);
        Ok(self.snapshot().await)
    }

    pub async fn resume(&self) -> Result<SessionSnapshot> {
        let mut live = self.live.lock().await;
        let rec = live
            .as_mut()
            .ok_or_else(|| CoreError::InvalidState("not recording".into()))?;
        if let Some(p) = rec.pause_started.take() {
            rec.paused_accum_ms += p.elapsed().as_millis() as u64;
        } else {
            return Err(CoreError::InvalidState("not paused".into()));
        }
        if let Some(pump) = rec.video_pump.as_ref() {
            pump.set_paused(false);
        }
        if let Some(audio) = rec.audio_session.as_ref() {
            audio.set_paused(false);
        }
        drop(live);
        Ok(self.snapshot().await)
    }

    pub async fn stop(&self) -> Result<SessionSnapshot> {
        let mut live = self.live.lock().await;
        let Some(mut rec) = live.take() else {
            return Err(CoreError::InvalidState("not recording".into()));
        };
        // Close all live inputs together. In particular, do not keep PCM flowing
        // while waiting for the rawvideo writer to deliver EOF: FFmpeg no longer
        // uses `-shortest`, because it stalls live multi-input filter graphs.
        // Drop the MF webcam promptly so preview can reopen the device.
        if let Some(pump) = rec.video_pump.take() {
            pump.stop();
        }
        drop(rec._webcam.take());
        if let Some(audio) = rec.audio_session.as_mut() {
            audio.stop();
        }
        // Let the stdin writer observe channel close and deliver EOF to FFmpeg.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        // stdin was moved into the frame writer; this is a no-op if already taken.
        drop(rec.child.stdin.take());
        match tokio::time::timeout(std::time::Duration::from_secs(12), rec.child.wait()).await {
            Ok(Ok(status)) if !status.success() => {
                tracing::warn!(?status, "ffmpeg quit with non-zero status");
            }
            Ok(_) => {}
            Err(_) => {
                tracing::warn!("ffmpeg did not exit after stdin EOF; killing");
                let _ = rec.child.kill().await;
                let _ = rec.child.wait().await;
            }
        }

        // Prefer a progressive MP4 with faststart when FFmpeg exited cleanly enough
        // to leave a readable fragmented file.
        if req_is_mp4(&rec.output_path) {
            if let Some(enc) = self.encoder.as_ref() {
                if let Err(e) = remux_frag_to_faststart(enc, &rec.output_path).await {
                    tracing::warn!(%e, "faststart remux skipped; fragmented mp4 kept");
                }
            }
        }

        let path = rec.output_path.clone();
        let meta_ok = std::fs::metadata(&path)
            .map(|m| m.is_file() && m.len() > 1024)
            .unwrap_or(false);
        if !meta_ok {
            // Without ffmpeg's own words this failure is undebuggable, so the
            // tail of its stderr goes straight into the reported error.
            let log = rec.stderr_log.lock().await.clone();
            let detail = log.trim();
            let detail = detail
                .lines()
                .rev()
                .filter(|l| !l.trim().is_empty())
                .take(6)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" | ");
            let msg = if detail.is_empty() {
                format!(
                    "recording produced no usable file: {} (ffmpeg wrote nothing to stderr)",
                    path.display()
                )
            } else {
                format!("recording failed: {detail}")
            };
            tracing::error!(path = %path.display(), ffmpeg = %log.trim(), "recording produced no usable file");
            *self.last_error.lock().await = Some(msg.clone());
            return Err(CoreError::Message(msg));
        }

        drop(live);
        Ok(SessionSnapshot {
            state: SessionState::Idle,
            elapsed_ms: 0,
            output_path: Some(path.to_string_lossy().into_owned()),
            last_error: None,
            encoder: Some(rec.encoder.ffmpeg_name().into()),
            hide_app: false,
        })
    }

    pub fn output_dir(&self) -> &str {
        &self.settings.output_dir
    }

    pub fn take_screenshot(&self, target: &CaptureTarget, path: &Path) -> Result<PathBuf> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let frame = self.capture.capture_frame(target)?;
        frame.save_png(path)?;
        Ok(path.to_path_buf())
    }

    pub fn default_screenshot_path(&self) -> PathBuf {
        let _ = self.settings.ensure_output_dir();
        let stamp = Local::now().format("%Y%m%d-%H%M%S");
        PathBuf::from(&self.settings.output_dir).join(format!(
            "capto-shot-{stamp}-{}.png",
            &Uuid::new_v4().to_string()[..8]
        ))
    }
}

/// Map an xcap display rect (may be DIP-scaled) onto a physical monitor rect.
fn normalize_display_rect(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    screen: &capto_capture::VirtualScreen,
) -> (i32, i32, u32, u32) {
    let monitors = capto_capture::list_monitor_rects();
    let cx = x + width as i32 / 2;
    let cy = y + height as i32 / 2;
    if let Some(m) = monitors.iter().find(|m| m.contains_point(cx, cy)) {
        return (m.x, m.y, m.width, m.height);
    }
    if let Some(m) = monitors.iter().min_by_key(|m| {
        let mx = m.x + m.width as i32 / 2;
        let my = m.y + m.height as i32 / 2;
        (mx - cx).unsigned_abs() + (my - cy).unsigned_abs()
    }) {
        return (m.x, m.y, m.width, m.height);
    }
    screen
        .clamp_rect(x, y, width, height)
        .unwrap_or((x, y, width, height))
}

fn req_is_mp4(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
}

/// Rewrite a fragmented recording into a progressive MP4 with `faststart` so
/// common players (Movies & TV, QuickTime) can open it reliably.
async fn remux_frag_to_faststart(ffmpeg: &FfmpegEncoder, path: &Path) -> Result<()> {
    let tmp = path.with_extension("capto-remux.tmp.mp4");
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }
    std::fs::rename(path, &tmp).map_err(|e| CoreError::Message(e.to_string()))?;
    let args = [
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-i".into(),
        tmp.to_string_lossy().into_owned(),
        "-c".into(),
        "copy".into(),
        "-movflags".into(),
        "+faststart".into(),
        path.to_string_lossy().into_owned(),
    ];
    match ffmpeg.run_once(&args).await {
        Ok(()) => {
            let _ = std::fs::remove_file(&tmp);
            Ok(())
        }
        Err(e) => {
            // Put the fragmented file back so the user still has something.
            let _ = std::fs::rename(&tmp, path);
            Err(CoreError::Message(e.to_string()))
        }
    }
}
