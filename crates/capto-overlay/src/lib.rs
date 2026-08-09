use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPosition {
    pub anchor: OverlayAnchor,
    /// Normalized 0..1 when Custom / fine-tuning.
    pub x: f32,
    pub y: f32,
}

impl Default for OverlayPosition {
    fn default() -> Self {
        Self {
            anchor: OverlayAnchor::BottomRight,
            x: 0.85,
            y: 0.85,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOverlay {
    pub id: String,
    pub text: String,
    pub font_size: u32,
    pub color: String,
    pub position: OverlayPosition,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOverlay {
    pub id: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub position: OverlayPosition,
    pub opacity: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseClickOverlay {
    pub enabled: bool,
    pub left_color: String,
    pub right_color: String,
    pub middle_color: String,
    pub radius: u32,
}

impl Default for MouseClickOverlay {
    fn default() -> Self {
        Self {
            enabled: true,
            left_color: "#FF5252".into(),
            right_color: "#448AFF".into(),
            middle_color: "#69F0AE".into(),
            radius: 18,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeystrokeOverlay {
    pub enabled: bool,
    pub position: OverlayPosition,
    pub font_size: u32,
    pub color: String,
    pub background: String,
}

impl Default for KeystrokeOverlay {
    fn default() -> Self {
        Self {
            enabled: true,
            position: OverlayPosition {
                anchor: OverlayAnchor::BottomLeft,
                x: 0.05,
                y: 0.9,
            },
            font_size: 28,
            color: "#FFFFFF".into(),
            background: "#000000AA".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElapsedOverlay {
    pub enabled: bool,
    pub position: OverlayPosition,
    pub font_size: u32,
    pub color: String,
}

impl Default for ElapsedOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            position: OverlayPosition {
                anchor: OverlayAnchor::TopRight,
                x: 0.92,
                y: 0.05,
            },
            font_size: 24,
            color: "#FFFFFF".into(),
        }
    }
}

/// Layout helper for UI drag preview (PiP is composited in Rust before encode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebcamPip {
    pub enabled: bool,
    /// Device id from `list_webcams` (MF symbolic link) or friendly name.
    pub device_id: Option<String>,
    /// Friendly name shown in UI; also accepted as a match key for MF open.
    #[serde(default)]
    pub device_label: Option<String>,
    pub position: OverlayPosition,
    pub width: u32,
    pub height: u32,
    pub mirrored: bool,
    pub corner_radius: u32,
}

impl Default for WebcamPip {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: None,
            device_label: None,
            position: OverlayPosition {
                anchor: OverlayAnchor::BottomRight,
                x: 0.82,
                y: 0.78,
            },
            width: 320,
            height: 240,
            mirrored: true,
            corner_radius: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayConfig {
    pub mouse_clicks: MouseClickOverlay,
    pub keystrokes: KeystrokeOverlay,
    /// Deprecated: kept for settings JSON compatibility; not burned into recordings.
    #[serde(default)]
    pub elapsed: ElapsedOverlay,
    pub texts: Vec<TextOverlay>,
    pub images: Vec<ImageOverlay>,
    pub webcam: WebcamPip,
}

/// Pixel placement helper for UI drag preview and frame compositor.
pub fn resolve_pixel_position(
    pos: &OverlayPosition,
    frame_w: u32,
    frame_h: u32,
    box_w: u32,
    box_h: u32,
) -> (i32, i32) {
    let fw = frame_w as f32;
    let fh = frame_h as f32;
    let bw = box_w as f32;
    let bh = box_h as f32;

    let (x, y) = match pos.anchor {
        OverlayAnchor::TopLeft => (0.0, 0.0),
        OverlayAnchor::TopRight => (fw - bw, 0.0),
        OverlayAnchor::BottomLeft => (0.0, fh - bh),
        OverlayAnchor::BottomRight => (fw - bw, fh - bh),
        OverlayAnchor::Center => ((fw - bw) / 2.0, (fh - bh) / 2.0),
        OverlayAnchor::Custom => (pos.x * fw, pos.y * fh),
    };

    let x = if matches!(pos.anchor, OverlayAnchor::Custom) {
        x
    } else {
        x + (pos.x - 0.5) * 40.0
    };
    let y = if matches!(pos.anchor, OverlayAnchor::Custom) {
        y
    } else {
        y + (pos.y - 0.5) * 40.0
    };

    (x.round() as i32, y.round() as i32)
}

/// Escape a filesystem path for use inside an FFmpeg filtergraph option.
pub fn escape_filter_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_right_position() {
        let pos = OverlayPosition {
            anchor: OverlayAnchor::BottomRight,
            x: 0.5,
            y: 0.5,
        };
        let (x, y) = resolve_pixel_position(&pos, 1920, 1080, 320, 240);
        assert!(x > 1500);
        assert!(y > 800);
    }

    #[test]
    fn escape_filter_path_escapes_drive_colon() {
        let escaped = escape_filter_path(std::path::Path::new(r"C:\Windows\Fonts\arial.ttf"));
        assert_eq!(escaped, r"C\:/Windows/Fonts/arial.ttf");
    }
}
