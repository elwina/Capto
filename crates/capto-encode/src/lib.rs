use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error(
        "bundled ffmpeg not found — run scripts/download-ffmpeg.ps1 (dev) or reinstall Capto (release)"
    )]
    FfmpegNotFound,
    #[error("ffmpeg failed: {0}")]
    FfmpegFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EncodeError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoEncoderKind {
    H264Nvenc,
    H264Qsv,
    H264Amf,
    Libx264,
    HevcNvenc,
    HevcQsv,
    HevcAmf,
    Libx265,
    Gif,
}

impl VideoEncoderKind {
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::H264Nvenc => "h264_nvenc",
            Self::H264Qsv => "h264_qsv",
            Self::H264Amf => "h264_amf",
            Self::Libx264 => "libx264",
            Self::HevcNvenc => "hevc_nvenc",
            Self::HevcQsv => "hevc_qsv",
            Self::HevcAmf => "hevc_amf",
            Self::Libx265 => "libx265",
            Self::Gif => "gif",
        }
    }

    pub fn is_hardware(self) -> bool {
        matches!(
            self,
            Self::H264Nvenc
                | Self::H264Qsv
                | Self::H264Amf
                | Self::HevcNvenc
                | Self::HevcQsv
                | Self::HevcAmf
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderInfo {
    pub kind: VideoEncoderKind,
    pub name: String,
    pub available: bool,
    pub hardware: bool,
}

#[derive(Debug, Clone)]
pub struct FfmpegEncoder {
    pub binary: PathBuf,
}

impl FfmpegEncoder {
    pub fn discover(sidecar_dir: Option<&Path>) -> Result<Self> {
        if let Some(p) = Self::resolve_binary(sidecar_dir) {
            return Ok(Self {
                binary: normalize_spawn_path(p),
            });
        }
        Err(EncodeError::FfmpegNotFound)
    }

    /// Resolve only the bundled sidecar — never PATH / WinGet / system installs.
    pub fn resolve_binary(sidecar_dir: Option<&Path>) -> Option<PathBuf> {
        sidecar_dir
            .and_then(find_ffmpeg_in_dir)
            .map(normalize_spawn_path)
    }

    /// True if `dir` contains a usable bundled ffmpeg binary.
    pub fn dir_has_ffmpeg(dir: &Path) -> bool {
        find_ffmpeg_in_dir(dir).is_some()
    }

    pub fn binary_path(&self) -> &Path {
        &self.binary
    }

    /// First line of `ffmpeg -version` (e.g. `ffmpeg version 7.1 Copyright …`).
    pub async fn version_line(&self) -> Result<String> {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_no_window(&mut cmd);
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        parse_ffmpeg_version_line(&stdout)
            .or_else(|| parse_ffmpeg_version_line(&stderr))
            .ok_or_else(|| {
                EncodeError::FfmpegFailed(if output.status.success() {
                    "ffmpeg -version produced no version line".into()
                } else {
                    stderr.trim().to_string()
                })
            })
    }

    pub async fn probe_encoders(&self) -> Result<Vec<EncoderInfo>> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(["-hide_banner", "-encoders"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_no_window(&mut cmd);
        let output = cmd.output().await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            let detail = [err.trim(), out.trim()]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or("ffmpeg -encoders failed with no output");
            return Err(EncodeError::FfmpegFailed(detail.to_string()));
        }

        // FFmpeg historically printed the encoder list on stderr; keep both.
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let candidates = [
            VideoEncoderKind::H264Nvenc,
            VideoEncoderKind::H264Qsv,
            VideoEncoderKind::H264Amf,
            VideoEncoderKind::Libx264,
            VideoEncoderKind::HevcNvenc,
            VideoEncoderKind::HevcQsv,
            VideoEncoderKind::HevcAmf,
            VideoEncoderKind::Libx265,
            VideoEncoderKind::Gif,
        ];

        Ok(candidates
            .into_iter()
            .map(|kind| {
                let name = kind.ffmpeg_name().to_string();
                let available = text.contains(&name);
                EncoderInfo {
                    kind,
                    name,
                    available,
                    hardware: kind.is_hardware(),
                }
            })
            .collect())
    }

    /// Prefer hardware when present — soft x264 cannot keep realtime at 1440p+ with PiP.
    pub async fn pick_best_h264(&self) -> Result<VideoEncoderKind> {
        let encoders = self.probe_encoders().await?;
        for kind in [
            VideoEncoderKind::H264Nvenc,
            VideoEncoderKind::H264Qsv,
            VideoEncoderKind::H264Amf,
            VideoEncoderKind::Libx264,
        ] {
            if encoders.iter().any(|e| e.kind == kind && e.available) {
                return Ok(kind);
            }
        }
        Ok(VideoEncoderKind::Libx264)
    }

    pub async fn spawn(&self, args: &[String]) -> Result<(Child, Arc<Mutex<String>>)> {
        tracing::info!(?args, ffmpeg = %self.binary.display(), "starting ffmpeg");
        let stderr_log = Arc::new(Mutex::new(String::new()));
        let stderr_log_task = Arc::clone(&stderr_log);

        let mut cmd = Command::new(&self.binary);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_no_window(&mut cmd);

        let mut child = cmd.spawn()?;

        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buf = vec![0u8; 4096];
                loop {
                    match stderr.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buf[..n]);
                            tracing::debug!(ffmpeg_stderr = %chunk.trim_end(), "ffmpeg");
                            let mut guard = stderr_log_task.lock().await;
                            guard.push_str(&chunk);
                            if guard.len() > 12_000 {
                                let start = guard.len() - 6_000;
                                let keep = guard[start..].to_string();
                                *guard = keep;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        Ok((child, stderr_log))
    }

    /// Run FFmpeg to completion (no long-lived stdin pump). Used for remux/finalize.
    pub async fn run_once(&self, args: &[String]) -> Result<()> {
        tracing::info!(?args, ffmpeg = %self.binary.display(), "running ffmpeg once");
        let mut cmd = Command::new(&self.binary);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_no_window(&mut cmd);
        let output = cmd.output().await?;
        if output.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&output.stderr);
        Err(EncodeError::FfmpegFailed(err.trim().to_string()))
    }

    /// Spawn and ensure the process stays alive briefly (catches immediate encoder/device errors).
    /// The returned stderr log keeps filling for the whole run, so a later crash
    /// (dshow device, encoder session, disk) can still be reported.
    pub async fn spawn_checked(&self, args: &[String]) -> Result<(Child, Arc<Mutex<String>>)> {
        let (mut child, stderr_log) = self.spawn(args).await?;
        self.check_started(&mut child, &stderr_log).await?;
        Ok((child, stderr_log))
    }

    /// Ensure a previously spawned FFmpeg process survives initialization.
    /// This split form lets native producers (WASAPI) connect after spawning
    /// but before the health check.
    pub async fn check_started(
        &self,
        child: &mut Child,
        stderr_log: &Arc<Mutex<String>>,
    ) -> Result<()> {
        // gdigrab/NVENC/dshow can take a moment to initialize on some machines.
        for _ in 0..25 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Give the stderr drain task time to finish after exit.
                    for _ in 0..10 {
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                        if !stderr_log.lock().await.trim().is_empty() {
                            break;
                        }
                    }
                    let err = summarize_ffmpeg_stderr(stderr_log.lock().await.trim());
                    let detail = if err.is_empty() {
                        format!(
                            "ffmpeg exited immediately ({status}) with no stderr — fully quit Capto (including tray) and retry; if it persists try encoder libx264 / disable mic & system audio"
                        )
                    } else {
                        format!(
                            "ffmpeg exited immediately ({status}): {err}. Try encoder libx264 / disable mic & system audio."
                        )
                    };
                    return Err(EncodeError::FfmpegFailed(detail));
                }
                Ok(None) => {}
                Err(e) => return Err(EncodeError::Io(e)),
            }
        }
        Ok(())
    }

    /// Prefer NVENC → QSV → AMF → libx264, returning only available ones in priority order.
    pub async fn ranked_h264(&self) -> Result<Vec<VideoEncoderKind>> {
        let encoders = self.probe_encoders().await?;
        let order = [
            VideoEncoderKind::H264Nvenc,
            VideoEncoderKind::H264Qsv,
            VideoEncoderKind::H264Amf,
            VideoEncoderKind::Libx264,
        ];
        Ok(order
            .into_iter()
            .filter(|kind| encoders.iter().any(|e| e.kind == *kind && e.available))
            .collect())
    }

    pub fn video_encoder_args(kind: VideoEncoderKind, crf_or_cq: u8) -> Vec<String> {
        match kind {
            VideoEncoderKind::Libx264 => vec![
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                crf_or_cq.to_string(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ],
            VideoEncoderKind::Libx265 => vec![
                "-c:v".into(),
                "libx265".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                crf_or_cq.to_string(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ],
            VideoEncoderKind::Gif => vec!["-c:v".into(), "gif".into()],
            hw => {
                let mut args = vec!["-c:v".into(), hw.ffmpeg_name().into()];
                match hw {
                    VideoEncoderKind::H264Nvenc | VideoEncoderKind::HevcNvenc => {
                        args.extend([
                            "-preset".into(),
                            "p4".into(),
                            "-rc".into(),
                            "vbr".into(),
                            "-cq".into(),
                            crf_or_cq.to_string(),
                            "-b:v".into(),
                            "0".into(),
                        ]);
                    }
                    VideoEncoderKind::H264Qsv | VideoEncoderKind::HevcQsv => {
                        args.extend([
                            "-global_quality".into(),
                            crf_or_cq.to_string(),
                            "-look_ahead".into(),
                            "1".into(),
                        ]);
                    }
                    VideoEncoderKind::H264Amf | VideoEncoderKind::HevcAmf => {
                        args.extend([
                            "-quality".into(),
                            "balanced".into(),
                            "-rc".into(),
                            "cqp".into(),
                            "-qp_i".into(),
                            crf_or_cq.to_string(),
                            "-qp_p".into(),
                            crf_or_cq.to_string(),
                        ]);
                    }
                    _ => {}
                }
                args.extend(["-pix_fmt".into(), "yuv420p".into()]);
                args
            }
        }
    }

    /// List DirectShow video capture device friendly names (Windows).
    pub async fn list_dshow_video_devices(&self) -> Result<Vec<String>> {
        let mut cmd = Command::new(&self.binary);
        cmd.args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
        apply_no_window(&mut cmd);
        let output = cmd.output().await?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(parse_dshow_video_devices(&stderr))
    }
}

/// Parse `ffmpeg -f dshow -list_devices` stderr into video device names.
pub fn parse_dshow_video_devices(stderr: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stderr.lines() {
        // Example: [dshow @ ...] "Integrated Camera" (video)
        // Newer builds: [in#0 @ ...] "Integrated Camera" (video)
        let Some(start) = line.find('"') else {
            continue;
        };
        if !line.contains("(video)") {
            continue;
        }
        let rest = &line[start + 1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = rest[..end].trim();
        if !name.is_empty() && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Pick a dshow name for recording: exact label, fuzzy match, or first device.
pub fn resolve_dshow_video_name(preferred: Option<&str>, devices: &[String]) -> Option<String> {
    if devices.is_empty() {
        return None;
    }
    let Some(pref) = preferred.map(str::trim).filter(|s| !s.is_empty()) else {
        return devices.first().cloned();
    };
    if let Some(exact) = devices.iter().find(|d| d.eq_ignore_ascii_case(pref)) {
        return Some(exact.clone());
    }
    // Browser labels often look like "Integrated Camera (00:11:22:…)" — match prefix.
    let pref_base = pref.split('(').next().unwrap_or(pref).trim();
    if let Some(fuzzy) = devices.iter().find(|d| {
        let base = d.split('(').next().unwrap_or(d).trim();
        base.eq_ignore_ascii_case(pref_base)
            || d.to_ascii_lowercase()
                .contains(&pref_base.to_ascii_lowercase())
            || pref_base
                .to_ascii_lowercase()
                .contains(&d.to_ascii_lowercase())
    }) {
        return Some(fuzzy.clone());
    }
    devices.first().cloned()
}

#[cfg(test)]
mod dshow_tests {
    use super::*;

    #[test]
    fn parses_dshow_video_device_lines() {
        let sample = r#"
[dshow @ 000001] "Integrated Camera" (video)
[dshow @ 000001]   Alternative name "@device_pnp_..."
[dshow @ 000001] "Microphone" (audio)
[in#0 @ 000002] "OBS Virtual Camera" (video)
"#;
        let devices = parse_dshow_video_devices(sample);
        assert_eq!(
            devices,
            vec![
                "Integrated Camera".to_string(),
                "OBS Virtual Camera".to_string()
            ]
        );
    }

    #[test]
    fn resolves_preferred_or_first() {
        let devices = vec!["Integrated Camera".into(), "USB Cam".into()];
        assert_eq!(
            resolve_dshow_video_name(None, &devices).as_deref(),
            Some("Integrated Camera")
        );
        assert_eq!(
            resolve_dshow_video_name(Some("USB Cam"), &devices).as_deref(),
            Some("USB Cam")
        );
        assert_eq!(
            resolve_dshow_video_name(Some("Integrated Camera (abc)"), &devices).as_deref(),
            Some("Integrated Camera")
        );
    }
}

fn parse_ffmpeg_version_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| line.to_ascii_lowercase().starts_with("ffmpeg version"))
        .map(str::to_string)
}

/// Short token after `ffmpeg version` (e.g. `7.1`, `N-115123-g…`).
pub fn parse_ffmpeg_version_token(version_line: &str) -> Option<String> {
    let lower = version_line.to_ascii_lowercase();
    let rest = lower.strip_prefix("ffmpeg version")?;
    let original_rest = version_line.get(version_line.len() - rest.len()..)?;
    let token = original_rest
        .split_whitespace()
        .next()
        .filter(|t| !t.is_empty())?;
    Some(token.to_string())
}

/// Prefer plain `ffmpeg(.exe)` (installed sidecar), then Tauri triple-suffixed names.
fn find_ffmpeg_in_dir(dir: &Path) -> Option<PathBuf> {
    let dir = normalize_spawn_path(dir.to_path_buf());
    for name in ["ffmpeg.exe", "ffmpeg"] {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(normalize_spawn_path(candidate));
        }
    }

    let entries = std::fs::read_dir(&dir).ok()?;
    let mut triple_hits: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            let lower = name.to_ascii_lowercase();
            lower.starts_with("ffmpeg-") && (lower.ends_with(".exe") || !lower.contains('.'))
        })
        .collect();
    triple_hits.sort();
    triple_hits.into_iter().next().map(normalize_spawn_path)
}

/// Strip Windows verbatim (`\\?\`) prefixes so CreateProcess is reliable from GUI apps.
fn normalize_spawn_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

fn apply_no_window(cmd: &mut Command) {
    // Prevent console-subsystem ffmpeg from misbehaving when spawned by a GUI app.
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

fn summarize_ffmpeg_stderr(err: &str) -> String {
    let trimmed = err.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Keep the last meaningful chunk; FFmpeg often prints a banner then the real error.
    let lines: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let take = lines.len().saturating_sub(8);
    lines[take..].join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_binary_ignores_path_without_sidecar() {
        assert!(FfmpegEncoder::resolve_binary(None).is_none());
    }

    #[test]
    fn find_ffmpeg_prefers_plain_name() {
        let dir = tempfile_dir();
        fs::write(dir.join("ffmpeg-x86_64-pc-windows-msvc.exe"), b"triple").unwrap();
        fs::write(dir.join("ffmpeg.exe"), b"plain").unwrap();
        let found = find_ffmpeg_in_dir(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "ffmpeg.exe");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_ffmpeg_accepts_triple_only() {
        let dir = tempfile_dir();
        fs::write(dir.join("ffmpeg-x86_64-pc-windows-msvc.exe"), b"triple").unwrap();
        let found = find_ffmpeg_in_dir(&dir).unwrap();
        assert!(found
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("ffmpeg-"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn normalize_spawn_path_strips_verbatim_prefix() {
        let p = normalize_spawn_path(PathBuf::from(r"\\?\C:\Capto\ffmpeg.exe"));
        assert_eq!(p, PathBuf::from(r"C:\Capto\ffmpeg.exe"));
    }

    #[test]
    fn parse_ffmpeg_version_line_reads_first_match() {
        let out = "ffmpeg version 7.1 Copyright (c) 2000-2024\nconfiguration: --enable-gpl\n";
        assert_eq!(
            parse_ffmpeg_version_line(out).as_deref(),
            Some("ffmpeg version 7.1 Copyright (c) 2000-2024")
        );
        assert_eq!(
            parse_ffmpeg_version_token("ffmpeg version 7.1 Copyright (c) 2000-2024").as_deref(),
            Some("7.1")
        );
    }

    #[test]
    fn parse_ffmpeg_version_token_accepts_capto_branded_builds() {
        // capto-ffmpeg builds with `--extra-version=capto`; the token must parse
        // without strict-equality assumptions so `n9.0-capto` survives.
        let branded = "ffmpeg version n9.0-capto Copyright (c) 2000-2026 the FFmpeg developers";
        assert_eq!(parse_ffmpeg_version_line(branded).as_deref(), Some(branded));
        assert_eq!(
            parse_ffmpeg_version_token(branded).as_deref(),
            Some("n9.0-capto")
        );
    }

    fn tempfile_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("capto-encode-test-{}-{nonce}", std::process::id(),));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
