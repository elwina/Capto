mod client;
mod launch;

use anyhow::{bail, Context, Result};
use capto_core::{OutputFormat, Region, VideoSourceKind};
use capto_encode::VideoEncoderKind;
use capto_ipc::{
    ExitCode, OpenOutputsRequest, RecordStartRequest, ShotRequest,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::process::ExitCode as StdExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "capto",
    version,
    about = "Capto CLI — control the Capto desktop app (agent-friendly JSON)"
)]
struct Cli {
    /// Pretty-print human text instead of JSON envelope
    #[arg(long, global = true)]
    human: bool,

    /// Do not auto-launch Capto if the control plane is down
    #[arg(long, global = true)]
    no_launch: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Session status
    Status,
    /// Open Capto desktop (does not require control plane)
    Open,
    /// Readiness / environment probe
    Doctor,
    /// Recording controls
    Record {
        #[command(subcommand)]
        action: RecordAction,
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
    },
    /// Settings get / set / path
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// List displays, windows, audio, encoders
    List {
        #[arg(value_enum)]
        what: ListWhat,
    },
    /// Recent outputs / open files
    Outputs {
        #[command(subcommand)]
        action: OutputsAction,
    },
}

#[derive(Subcommand, Debug)]
enum RecordAction {
    Start {
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
        #[arg(long, default_value = "mp4")]
        format: String,
        #[arg(long)]
        fps: Option<u32>,
        #[arg(long)]
        quality: Option<u8>,
        #[arg(long)]
        no_cursor: bool,
        #[arg(long)]
        mic: Option<String>,
        #[arg(long)]
        loopback: Option<String>,
        #[arg(long)]
        encoder: Option<String>,
    },
    Stop,
    Pause,
    Resume,
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Print full settings or a single key
    Get {
        key: Option<String>,
    },
    /// Patch settings: `--json '{...}'` or `key=value` pairs
    Set {
        #[arg(long)]
        json: Option<String>,
        /// key=value pairs (camelCase keys, e.g. fps=60)
        pairs: Vec<String>,
    },
    /// Print settings.json path
    Path,
}

#[derive(Subcommand, Debug)]
enum OutputsAction {
    Recent {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    Open {
        path: Option<String>,
        #[arg(long)]
        last: bool,
        #[arg(long)]
        folder: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum ListWhat {
    Displays,
    Windows,
    Audio,
    Encoders,
}

#[tokio::main]
async fn main() -> StdExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => StdExitCode::from(ExitCode::Ok.as_i32() as u8),
        Err(err) => {
            let code = err.exit_code;
            let envelope = json!({
                "ok": false,
                "error": { "code": err.code, "message": err.message }
            });
            if err.human {
                eprintln!("{}: {}", err.code, err.message);
            } else {
                println!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
            }
            StdExitCode::from(code.as_i32() as u8)
        }
    }
}

struct CliError {
    code: String,
    message: String,
    exit_code: ExitCode,
    human: bool,
}

async fn run(cli: Cli) -> Result<(), CliError> {
    let human = cli.human;
    let auto_launch = !cli.no_launch;

    // `open` only starts the desktop app — no control-plane connection required.
    if matches!(cli.command, Commands::Open) {
        return match launch::open_desktop() {
            Ok(path) => {
                let data = json!({
                    "path": path,
                    "hint": "Wait a few seconds, then run `capto status`. If still unavailable, ask the user to open Capto."
                });
                emit_ok(data, human);
                Ok(())
            }
            Err(e) => Err(CliError {
                code: "desktopUnavailable".into(),
                message: format!(
                    "{e:#}. Ask the user to open Capto from the Start menu, then retry"
                ),
                exit_code: ExitCode::DesktopUnavailable,
                human,
            }),
        };
    }

    let client = client::ControlClient::connect(auto_launch)
        .await
        .map_err(|e| CliError {
            code: "desktopUnavailable".into(),
            message: format!(
                "{e:#}. Try `capto open`, or ask the user to open Capto, then retry"
            ),
            exit_code: ExitCode::DesktopUnavailable,
            human,
        })?;

    let result = match cli.command {
        Commands::Open => unreachable!("handled above"),
        Commands::Status => client.get("/v1/status").await,
        Commands::Doctor => client.get("/v1/doctor").await,
        Commands::Record { action } => match action {
            RecordAction::Start {
                source,
                display,
                window,
                x,
                y,
                width,
                height,
                format,
                fps,
                quality,
                no_cursor,
                mic,
                loopback,
                encoder,
            } => {
                let body = build_start_request(
                    &source, display, window, x, y, width, height, &format, fps, quality,
                    no_cursor, mic, loopback, encoder,
                )
                .map_err(|e| usage(e.to_string(), human))?;
                client.post_json("/v1/record/start", &body).await
            }
            RecordAction::Stop => client.post_empty("/v1/record/stop").await,
            RecordAction::Pause => client.post_empty("/v1/record/pause").await,
            RecordAction::Resume => client.post_empty("/v1/record/resume").await,
        },
        Commands::Shot {
            source,
            display,
            window,
            x,
            y,
            width,
            height,
        } => {
            let body = build_shot_request(&source, display, window, x, y, width, height)
                .map_err(|e| usage(e.to_string(), human))?;
            client.post_json("/v1/shot", &body).await
        }
        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                let data = client.get("/v1/config").await.map_err(|e| map_http(e, human))?;
                if let Some(k) = key {
                    let v = data
                        .get(&k)
                        .cloned()
                        .ok_or_else(|| usage(format!("unknown settings key: {k}"), human))?;
                    Ok(v)
                } else {
                    Ok(data)
                }
            }
            ConfigAction::Set { json, pairs } => {
                let patch = build_config_patch(json, pairs).map_err(|e| usage(e.to_string(), human))?;
                client.patch_json("/v1/config", &patch).await
            }
            ConfigAction::Path => client.get("/v1/config/path").await,
        },
        Commands::List { what } => {
            let path = match what {
                ListWhat::Displays => "/v1/list/displays",
                ListWhat::Windows => "/v1/list/windows",
                ListWhat::Audio => "/v1/list/audio",
                ListWhat::Encoders => "/v1/list/encoders",
            };
            client.get(path).await
        }
        Commands::Outputs { action } => match action {
            OutputsAction::Recent { limit } => {
                client
                    .get(&format!("/v1/outputs/recent?limit={limit}"))
                    .await
            }
            OutputsAction::Open { path, last, folder } => {
                let body = OpenOutputsRequest {
                    path,
                    last,
                    folder,
                };
                client.post_json("/v1/outputs/open", &body).await
            }
        },
    };

    let data = result.map_err(|e| map_http(e, human))?;
    emit_ok(data, human);
    Ok(())
}

fn usage(message: impl Into<String>, human: bool) -> CliError {
    CliError {
        code: "usage".into(),
        message: message.into(),
        exit_code: ExitCode::Usage,
        human,
    }
}

fn map_http(err: client::HttpError, human: bool) -> CliError {
    let exit_code = match err.code.as_str() {
        "unauthorized" | "desktopUnavailable" => ExitCode::DesktopUnavailable,
        "stateConflict" => ExitCode::StateConflict,
        "capture" => ExitCode::Capture,
        "encode" => ExitCode::Encode,
        "configIo" => ExitCode::ConfigIo,
        "badRequest" | "usage" => ExitCode::Usage,
        _ => ExitCode::DesktopUnavailable,
    };
    CliError {
        code: err.code,
        message: err.message,
        exit_code,
        human,
    }
}

fn emit_ok(data: Value, human: bool) {
    if human {
        println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
    } else {
        let envelope = json!({ "ok": true, "data": data });
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).unwrap_or_default()
        );
    }
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

fn region_from(
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Option<Region> {
    match (x, y, width, height) {
        (Some(x), Some(y), Some(w), Some(h)) => Some(Region {
            x,
            y,
            width: w,
            height: h,
        }),
        _ => None,
    }
}

fn build_start_request(
    source: &str,
    display: Option<u32>,
    window: Option<u32>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    format: &str,
    fps: Option<u32>,
    quality: Option<u8>,
    no_cursor: bool,
    mic: Option<String>,
    loopback: Option<String>,
    encoder: Option<String>,
) -> Result<RecordStartRequest> {
    let enc = encoder.as_deref().map(parse_encoder).transpose()?;
    Ok(RecordStartRequest {
        source: parse_source(source)?,
        display_id: display,
        window_id: window,
        region: region_from(x, y, width, height),
        include_cursor: Some(!no_cursor),
        mic_device: mic,
        loopback_device: loopback,
        mic_volume: None,
        loopback_volume: None,
        encoder: enc,
        format: Some(parse_format(format)?),
        fps,
        quality,
    })
}

fn build_shot_request(
    source: &str,
    display: Option<u32>,
    window: Option<u32>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<ShotRequest> {
    Ok(ShotRequest {
        source: parse_source(source)?,
        display_id: display,
        window_id: window,
        region: region_from(x, y, width, height),
    })
}

fn build_config_patch(json: Option<String>, pairs: Vec<String>) -> Result<Value> {
    let mut obj = if let Some(raw) = json {
        let v: Value = serde_json::from_str(&raw).context("invalid --json")?;
        match v {
            Value::Object(map) => map,
            _ => bail!("--json must be an object"),
        }
    } else {
        serde_json::Map::new()
    };
    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("expected key=value, got {pair}"))?;
        let parsed = parse_config_value(v);
        obj.insert(k.to_string(), parsed);
    }
    if obj.is_empty() {
        bail!("provide --json or key=value pairs");
    }
    Ok(Value::Object(obj))
}

fn parse_config_value(raw: &str) -> Value {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return v;
    }
    if let Ok(n) = raw.parse::<i64>() {
        return json!(n);
    }
    if let Ok(n) = raw.parse::<f64>() {
        return json!(n);
    }
    match raw {
        "true" => json!(true),
        "false" => json!(false),
        "null" => Value::Null,
        _ => json!(raw),
    }
}
