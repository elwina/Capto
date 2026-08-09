use capto_core::{OutputFormat, Region, VideoSourceKind};
use capto_encode::VideoEncoderKind;
use serde::{Deserialize, Serialize};

/// Body for `POST /v1/record/start` (mirrors desktop StartArgs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordStartRequest {
    pub source: VideoSourceKind,
    #[serde(default)]
    pub display_id: Option<u32>,
    #[serde(default)]
    pub window_id: Option<u32>,
    #[serde(default)]
    pub region: Option<Region>,
    #[serde(default)]
    pub include_cursor: Option<bool>,
    #[serde(default)]
    pub mic_device: Option<String>,
    #[serde(default)]
    pub loopback_device: Option<String>,
    #[serde(default)]
    pub mic_volume: Option<u8>,
    #[serde(default)]
    pub loopback_volume: Option<u8>,
    #[serde(default)]
    pub encoder: Option<VideoEncoderKind>,
    #[serde(default)]
    pub format: Option<OutputFormat>,
    #[serde(default)]
    pub fps: Option<u32>,
    #[serde(default)]
    pub quality: Option<u8>,
}

impl Default for RecordStartRequest {
    fn default() -> Self {
        Self {
            source: VideoSourceKind::Display,
            display_id: None,
            window_id: None,
            region: None,
            include_cursor: None,
            mic_device: None,
            loopback_device: None,
            mic_volume: None,
            loopback_volume: None,
            encoder: None,
            format: None,
            fps: None,
            quality: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShotRequest {
    pub source: VideoSourceKind,
    #[serde(default)]
    pub display_id: Option<u32>,
    #[serde(default)]
    pub window_id: Option<u32>,
    #[serde(default)]
    pub region: Option<Region>,
}

impl Default for ShotRequest {
    fn default() -> Self {
        Self {
            source: VideoSourceKind::Display,
            display_id: None,
            window_id: None,
            region: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPathInfo {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEntry {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputsList {
    pub output_dir: String,
    pub items: Vec<OutputEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOutputsRequest {
    /// Absolute path to open. If omitted and `last` is true, open newest output.
    #[serde(default)]
    pub path: Option<String>,
    /// Open the output folder instead of a file.
    #[serde(default)]
    pub folder: bool,
    /// Open the most recent file under output_dir.
    #[serde(default)]
    pub last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorInfo {
    pub os: String,
    pub capture_backend: String,
    pub ffmpeg_path: Option<String>,
    pub ffmpeg_ok: bool,
    pub control_plane: bool,
    pub pid: u32,
    pub port: u16,
    pub preferred_encoder: Option<String>,
}
