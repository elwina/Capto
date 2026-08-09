use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod backend;
pub mod composite;
pub mod desktop;
pub mod pick;
pub mod preview;
pub mod record_dxgi;
pub mod webcam;

pub use backend::{
    create_default_backend, CaptureBackend, UnsupportedCaptureBackend, XcapCaptureBackend,
};
pub use composite::{composite_webcam_pip, swap_rb_inplace};
pub use desktop::{
    cursor_position, list_monitor_rects, list_windows, virtual_screen, window_by_id, ScreenPoint,
    VirtualScreen,
};
pub use pick::{capture_window_by_id, window_under_cursor};
pub use preview::{capture_preview_frame, release_preview_session};
pub use record_dxgi::{DxgiRecordPump, RecordPip};
pub use webcam::{
    ensure_preview_webcam, list_webcams, preview_webcam_slot, release_preview_webcam,
    take_webcam_for_record, WebcamCapture, WebcamFrameSlot, WebcamInfo,
};

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture backend unavailable on this platform: {0}")]
    Unsupported(&'static str),
    #[error("display not found: {0}")]
    DisplayNotFound(String),
    #[error("window not found: {0}")]
    WindowNotFound(String),
    #[error("capture failed: {0}")]
    Failed(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, CaptureError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app_name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CaptureTarget {
    Display {
        id: u32,
    },
    Window {
        id: u32,
    },
    Region {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub timestamp_ms: u64,
}

/// Frame-local rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    /// Fill a frame-local rectangle with opaque black pixels. Returns the
    /// clipped rectangle that was actually painted so callers can label it.
    pub fn blackout_rect(&mut self, x: i32, y: i32, width: u32, height: u32) -> Option<PixelRect> {
        let left = x.max(0).min(self.width as i32) as u32;
        let top = y.max(0).min(self.height as i32) as u32;
        let right = (x.saturating_add(width as i32))
            .max(0)
            .min(self.width as i32) as u32;
        let bottom = (y.saturating_add(height as i32))
            .max(0)
            .min(self.height as i32) as u32;
        if left >= right || top >= bottom {
            return None;
        }

        for py in top..bottom {
            for px in left..right {
                let offset = ((py * self.width + px) * 4) as usize;
                if let Some(pixel) = self.rgba.get_mut(offset..offset + 4) {
                    pixel.copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        Some(PixelRect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    /// Downscale and encode a lightweight JPEG suitable for UI preview.
    pub fn preview_jpeg(&self, max_width: u32, quality: u8) -> Result<(u32, u32, Vec<u8>)> {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
            .ok_or_else(|| CaptureError::Failed("invalid RGBA buffer".into()))?;
        let max_width = max_width.max(2);
        let resized = if img.width() > max_width {
            let height =
                ((img.height() as u64 * max_width as u64) / img.width() as u64).max(2) as u32;
            image::imageops::resize(
                &img,
                max_width,
                height,
                image::imageops::FilterType::Triangle,
            )
        } else {
            img
        };

        let mut bytes = Vec::new();
        let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();
        let mut encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality.clamp(1, 100));
        encoder
            .encode_image(&rgb)
            .map_err(|e| CaptureError::Failed(e.to_string()))?;
        Ok((rgb.width(), rgb.height(), bytes))
    }

    pub fn save_png(&self, path: &std::path::Path) -> Result<()> {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
            .ok_or_else(|| CaptureError::Failed("invalid RGBA buffer".into()))?;
        img.save(path)
            .map_err(|e| CaptureError::Failed(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Frame;

    #[test]
    fn blackout_clips_to_frame_and_jpeg_encodes() {
        let mut frame = Frame {
            width: 4,
            height: 4,
            rgba: vec![255; 4 * 4 * 4],
            timestamp_ms: 0,
        };

        let painted = frame.blackout_rect(-1, -1, 3, 3).expect("clipped rect");
        assert_eq!(
            (painted.x, painted.y, painted.width, painted.height),
            (0, 0, 2, 2)
        );
        assert_eq!(&frame.rgba[0..4], &[0, 0, 0, 255]);
        assert!(frame.blackout_rect(10, 10, 2, 2).is_none());

        let (width, height, jpeg) = frame.preview_jpeg(2, 65).expect("preview JPEG");
        assert_eq!((width, height), (2, 2));
        assert!(jpeg.starts_with(&[0xff, 0xd8]));
    }
}
