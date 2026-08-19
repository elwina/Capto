//! Short record with Rust-composited webcam PiP (no FFmpeg dshow).
//!
//! ```bash
//! cargo run -p capto-core --example pip_record_smoke
//! ```

use capto_core::{OutputFormat, RecordRequest, RecordingSession, VideoSourceKind};
use capto_overlay::OverlayConfig;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let out = std::env::temp_dir().join(format!(
        "capto-pip-smoke-{}.mp4",
        chrono::Local::now().format("%H%M%S")
    ));
    let sidecar =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/binaries");

    let mut overlays = OverlayConfig::default();
    overlays.webcam.enabled = true;
    overlays.webcam.width = 320;
    overlays.webcam.height = 240;

    let mut req = RecordRequest {
        source: VideoSourceKind::Display,
        display_id: Some(0),
        window_id: None,
        region: None,
        include_cursor: true,
        mic_device: None,
        loopback_device: None,
        mic_volume: 100,
        loopback_volume: 100,
        encoder: None,
        format: OutputFormat::Mp4,
        fps: 30,
        quality: 60,
        output_path: out.to_string_lossy().into_owned(),
        overlays,
        hide_app_while_recording: false,
    };

    // Prefer a small region so the smoke finishes quickly.
    let screen = capto_capture::virtual_screen();
    req.region = Some(capto_core::Region {
        x: screen.x,
        y: screen.y,
        width: 1280.min(screen.width / 2 * 2).max(640),
        height: 720.min(screen.height / 2 * 2).max(360),
    });
    req.source = VideoSourceKind::Region;

    println!("recording → {}", req.output_path);
    let session = RecordingSession::new(Default::default(), Some(sidecar));
    let (snap, _) = session.start(req).await.expect("start");
    println!("encoder={:?} state={:?}", snap.encoder, snap.state);
    tokio::time::sleep(Duration::from_secs(4)).await;
    let stop = session.stop().await.expect("stop");
    let meta = std::fs::metadata(stop.output_path.as_ref().unwrap()).expect("meta");
    println!(
        "done: {} bytes, path={}",
        meta.len(),
        stop.output_path.as_deref().unwrap_or("?")
    );
    assert!(meta.len() > 50_000, "output too small");
}
