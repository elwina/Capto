use crate::ffmpeg_args::Region;
use capto_encode::VideoEncoderKind;
use capto_hooks::{default_hotkeys, normalize_hotkeys, HotkeyBinding};
use capto_overlay::OverlayConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    Mp4,
    Gif,
    AudioOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoSourceKind {
    Display,
    Window,
    Region,
}

fn default_quality() -> u8 {
    60
}

fn default_audio_volume() -> u8 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub output_dir: String,
    pub output_format: OutputFormat,
    pub fps: u32,
    /// Perceptual quality 1..100 (Captura-style slider), mapped to encoder CRF.
    #[serde(default = "default_quality")]
    pub quality: u8,
    pub include_cursor: bool,
    pub preferred_encoder: Option<VideoEncoderKind>,
    pub mic_device: Option<String>,
    pub loopback_device: Option<String>,
    #[serde(default = "default_audio_volume")]
    pub mic_volume: u8,
    #[serde(default = "default_audio_volume")]
    pub loopback_volume: u8,
    pub default_source: VideoSourceKind,
    pub default_display_id: Option<u32>,
    /// Last picked window id (HWND). Restored when still present.
    #[serde(default)]
    pub default_window_id: Option<u32>,
    /// Friendly title used when id is stale after app restart.
    #[serde(default)]
    pub default_window_title: Option<String>,
    pub default_region: Option<Region>,
    pub hide_app_while_recording: bool,
    pub minimize_to_tray_on_close: bool,
    pub show_preview: bool,
    pub locale: String,
    pub hotkeys: Vec<HotkeyBinding>,
    pub overlays: OverlayConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        let output_dir = dirs::video_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Capto")
            .to_string_lossy()
            .to_string();

        Self {
            output_dir,
            output_format: OutputFormat::Mp4,
            fps: 30,
            quality: default_quality(),
            include_cursor: true,
            preferred_encoder: None,
            mic_device: None,
            loopback_device: None,
            mic_volume: default_audio_volume(),
            loopback_volume: default_audio_volume(),
            default_source: VideoSourceKind::Display,
            default_display_id: Some(0),
            default_window_id: None,
            default_window_title: None,
            default_region: None,
            hide_app_while_recording: true,
            minimize_to_tray_on_close: true,
            show_preview: false,
            locale: "zh-CN".into(),
            hotkeys: default_hotkeys(),
            overlays: OverlayConfig::default(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Capto")
            .join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut settings = Self::load_from(&path).unwrap_or_default();
        normalize_hotkeys(&mut settings.hotkeys);
        settings
    }

    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        let data = fs::read_to_string(path)?;
        let mut settings: Self = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        normalize_hotkeys(&mut settings.hotkeys);
        Ok(settings)
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::config_path())
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut normalized = self.clone();
        normalize_hotkeys(&mut normalized.hotkeys);
        let data = serde_json::to_string_pretty(&normalized)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, data)
    }

    pub fn ensure_output_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.output_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = AppSettings::default();
        s.locale = "en".into();
        s.fps = 60;
        s.save_to(&path).unwrap();
        let loaded = AppSettings::load_from(&path).unwrap();
        assert_eq!(loaded.locale, "en");
        assert_eq!(loaded.fps, 60);
    }
}
