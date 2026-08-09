use anyhow::{bail, Context, Result};
use capto_capture::CaptureTarget;
use capto_core::{
    AppSettings, OutputFormat, RecordRequest, RecordingSession, Region, VideoSourceKind,
};
use capto_encode::VideoEncoderKind;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "capto-cli",
    version,
    about = "Capto — local screen capture CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Record screen / region / window to a file
    Record {
        #[arg(long, default_value = "display")]
        source: String,
        #[arg(long)]
        display: Option<u32>,
        #[arg(long)]
        window: Option<u32>,
        #[arg(long)]
        x: Option<i32>,
        #[arg(long)]
        y: Option<i32>,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long, default_value_t = 5)]
        duration: u64,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "mp4")]
        format: String,
        #[arg(long)]
        fps: Option<u32>,
        #[arg(long)]
        no_cursor: bool,
        #[arg(long)]
        mic: Option<String>,
        #[arg(long)]
        loopback: Option<String>,
        #[arg(long)]
        encoder: Option<String>,
    },
    /// Take a screenshot
    Shot {
        #[arg(long, default_value = "display")]
        source: String,
        #[arg(long)]
        display: Option<u32>,
        #[arg(long)]
        window: Option<u32>,
        #[arg(long)]
        x: Option<i32>,
        #[arg(long)]
        y: Option<i32>,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// List displays, windows, audio devices, encoders
    List {
        #[arg(value_enum)]
        what: ListWhat,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum ListWhat {
    Displays,
    Windows,
    Audio,
    Encoders,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let settings = AppSettings::load();
    let sidecar = default_sidecar_dir();
    let mut session = RecordingSession::new(settings.clone(), sidecar);

    match cli.command {
        Commands::List { what } => match what {
            ListWhat::Displays => {
                let list = session.capture().list_displays()?;
                println!("{}", serde_json::to_string_pretty(&list)?);
            }
            ListWhat::Windows => {
                let list = session.capture().list_windows()?;
                println!("{}", serde_json::to_string_pretty(&list)?);
            }
            ListWhat::Audio => {
                let list = capto_audio::list_devices()?;
                println!("{}", serde_json::to_string_pretty(&list)?);
            }
            ListWhat::Encoders => {
                session.refresh_encoder()?;
                let enc = session
                    .encoder()
                    .context("bundled ffmpeg not found — run scripts/copy-ffmpeg.ps1")?
                    .probe_encoders()
                    .await?;
                println!("{}", serde_json::to_string_pretty(&enc)?);
            }
        },
        Commands::Shot {
            source,
            display,
            window,
            x,
            y,
            width,
            height,
            output,
        } => {
            let target = parse_target(&source, display, window, x, y, width, height)?;
            let path = output.unwrap_or_else(|| session.default_screenshot_path());
            let saved = session.take_screenshot(&target, &path)?;
            println!("{}", saved.display());
        }
        Commands::Record {
            source,
            display,
            window,
            x,
            y,
            width,
            height,
            duration,
            output,
            format,
            fps,
            no_cursor,
            mic,
            loopback,
            encoder,
        } => {
            session
                .refresh_encoder()
                .context("bundled ffmpeg required — run scripts/copy-ffmpeg.ps1")?;
            let format = parse_format(&format)?;
            let out = output.unwrap_or_else(|| session.make_output_path(format));
            let source_kind = parse_source(&source)?;
            let region = match (x, y, width, height) {
                (Some(x), Some(y), Some(w), Some(h)) => Some(Region {
                    x,
                    y,
                    width: w,
                    height: h,
                }),
                _ => None,
            };
            let enc = encoder.as_deref().map(parse_encoder).transpose()?;

            let req = RecordRequest {
                source: source_kind,
                display_id: display,
                window_id: window,
                region,
                include_cursor: !no_cursor,
                mic_device: mic,
                loopback_device: loopback,
                mic_volume: settings.mic_volume,
                loopback_volume: settings.loopback_volume,
                encoder: enc,
                format,
                fps: fps.unwrap_or(settings.fps),
                quality: settings.quality,
                output_path: out.to_string_lossy().into_owned(),
                overlays: settings.overlays.clone(),
                hide_app_while_recording: false,
            };

            let (snap, _) = session.start(req).await?;
            println!(
                "recording {} ({:?})",
                snap.output_path.as_deref().unwrap_or("?"),
                snap.encoder
            );
            tokio::time::sleep(Duration::from_secs(duration)).await;
            let done = session.stop().await?;
            println!("saved {}", done.output_path.as_deref().unwrap_or("?"));
        }
    }

    Ok(())
}

fn parse_source(s: &str) -> Result<VideoSourceKind> {
    Ok(match s {
        "display" | "screen" => VideoSourceKind::Display,
        "window" => VideoSourceKind::Window,
        "region" => VideoSourceKind::Region,
        other => bail!("unknown source: {other}"),
    })
}

fn parse_format(s: &str) -> Result<OutputFormat> {
    Ok(match s {
        "mp4" => OutputFormat::Mp4,
        "gif" => OutputFormat::Gif,
        "audio" | "m4a" => OutputFormat::AudioOnly,
        other => bail!("unknown format: {other}"),
    })
}

fn parse_encoder(s: &str) -> Result<VideoEncoderKind> {
    Ok(match s {
        "h264_nvenc" | "nvenc" => VideoEncoderKind::H264Nvenc,
        "h264_qsv" | "qsv" => VideoEncoderKind::H264Qsv,
        "h264_amf" | "amf" => VideoEncoderKind::H264Amf,
        "libx264" | "x264" => VideoEncoderKind::Libx264,
        "hevc_nvenc" => VideoEncoderKind::HevcNvenc,
        "hevc_qsv" => VideoEncoderKind::HevcQsv,
        "hevc_amf" => VideoEncoderKind::HevcAmf,
        "libx265" | "x265" => VideoEncoderKind::Libx265,
        "gif" => VideoEncoderKind::Gif,
        other => bail!("unknown encoder: {other}"),
    })
}

fn default_sidecar_dir() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/binaries"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.to_path_buf());
            candidates.push(parent.join("binaries"));
        }
    }
    candidates
        .into_iter()
        .find(|dir| capto_encode::FfmpegEncoder::dir_has_ffmpeg(dir))
}

fn parse_target(
    source: &str,
    display: Option<u32>,
    window: Option<u32>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<CaptureTarget> {
    Ok(match source {
        "display" | "screen" => CaptureTarget::Display {
            id: display.unwrap_or(0),
        },
        "window" => CaptureTarget::Window {
            id: window.context("--window required")?,
        },
        "region" => CaptureTarget::Region {
            x: x.context("--x required")?,
            y: y.context("--y required")?,
            width: width.context("--width required")?,
            height: height.context("--height required")?,
        },
        other => bail!("unknown source: {other}"),
    })
}
