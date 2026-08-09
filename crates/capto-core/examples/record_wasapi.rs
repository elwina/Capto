use capto_audio::AudioDeviceKind;
use capto_core::{AppSettings, OutputFormat, RecordRequest, RecordingSession};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let devices = capto_audio::list_devices().expect("list WASAPI endpoints");
    let loopback = devices
        .iter()
        .find(|device| device.kind == AudioDeviceKind::Loopback && device.is_default)
        .expect("default render endpoint");
    let microphone = devices
        .iter()
        .find(|device| device.kind == AudioDeviceKind::Input && device.is_default)
        .expect("default capture endpoint");
    println!("loopback endpoint: {}", loopback.name);
    println!("microphone endpoint: {}", microphone.name);

    let temp = tempfile::tempdir().expect("temporary directory");
    let output = temp.path().join("wasapi-smoke.mp4");
    let settings = AppSettings::default();
    let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src-tauri/binaries");
    let mut session = RecordingSession::new(settings.clone(), Some(sidecar));
    session.refresh_encoder().expect("discover bundled FFmpeg");
    let mut request = RecordRequest::from_settings(&settings, output.to_string_lossy());
    request.format = OutputFormat::Mp4;
    request.mic_device = Some(microphone.id.clone());
    request.loopback_device = Some(loopback.id.clone());
    request.hide_app_while_recording = false;

    let (_snap, _) = session.start(request).await.expect("start recording");
    tokio::time::sleep(Duration::from_secs(3)).await;
    session.stop().await.expect("stop recording");
    let bytes = std::fs::metadata(&output).expect("recording file").len();
    println!("recorded {bytes} bytes to {}", output.display());
    assert!(bytes > 10_000);

    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name",
            "-of",
            "csv=p=0",
        ])
        .arg(&output)
        .output()
        .expect("run ffprobe");
    let streams = String::from_utf8_lossy(&probe.stdout);
    println!("streams:\n{streams}");
    assert!(streams.lines().any(|line| line.starts_with("h264,video")));
    assert!(streams.lines().any(|line| line.starts_with("aac,audio")));
}
