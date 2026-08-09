use crate::{AppSettings, OutputFormat, VideoSourceKind};
use capto_audio::{AudioDeviceKind, PcmInputSpec};
use capto_encode::{FfmpegEncoder, VideoEncoderKind};
use capto_overlay::OverlayConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRequest {
    pub source: VideoSourceKind,
    pub display_id: Option<u32>,
    pub window_id: Option<u32>,
    pub region: Option<Region>,
    pub include_cursor: bool,
    pub mic_device: Option<String>,
    pub loopback_device: Option<String>,
    #[serde(default = "default_audio_volume")]
    pub mic_volume: u8,
    #[serde(default = "default_audio_volume")]
    pub loopback_volume: u8,
    pub encoder: Option<VideoEncoderKind>,
    pub format: OutputFormat,
    pub fps: u32,
    /// 1..=100 Captura-style quality; higher = better / larger files.
    #[serde(default = "default_record_quality")]
    pub quality: u8,
    pub output_path: String,
    pub overlays: OverlayConfig,
    pub hide_app_while_recording: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl RecordRequest {
    pub fn from_settings(settings: &AppSettings, output_path: impl Into<String>) -> Self {
        Self {
            source: settings.default_source.clone(),
            display_id: settings.default_display_id,
            window_id: None,
            region: settings.default_region.clone(),
            include_cursor: settings.include_cursor,
            mic_device: settings.mic_device.clone(),
            loopback_device: settings.loopback_device.clone(),
            mic_volume: settings.mic_volume,
            loopback_volume: settings.loopback_volume,
            encoder: settings.preferred_encoder,
            format: settings.output_format,
            fps: settings.fps,
            quality: settings.quality,
            output_path: output_path.into(),
            overlays: settings.overlays.clone(),
            hide_app_while_recording: settings.hide_app_while_recording,
        }
    }
}

/// Even frame size the DXGI pump / FFmpeg rawvideo input will use.
pub fn record_frame_size(req: &RecordRequest) -> (u32, u32) {
    let screen = capto_capture::virtual_screen();
    let (w, h) = req
        .region
        .as_ref()
        .map(|r| (r.width, r.height))
        .unwrap_or((screen.width, screen.height));
    ((w / 2 * 2).max(2), (h / 2 * 2).max(2))
}

/// Build FFmpeg argv for desktop capture + optional audio.
/// Windows: DXGI(+Rust webcam PiP) → `rawvideo` on `pipe:0`.
/// Webcam is composited in-process; FFmpeg never opens dshow.
pub fn build_record_args(
    req: &RecordRequest,
    encoder: VideoEncoderKind,
    pcm_inputs: &[PcmInputSpec],
) -> Vec<String> {
    #[cfg(not(windows))]
    {
        let _ = (req, encoder, pcm_inputs);
        return vec![
            "-y".into(),
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "color=c=black:s=1280x720:r=30".into(),
            "-t".into(),
            "0.1".into(),
            req.output_path.clone(),
        ];
    }

    #[cfg(windows)]
    {
        build_record_args_windows(req, encoder, pcm_inputs)
    }
}

#[cfg(windows)]
fn build_record_args_windows(
    req: &RecordRequest,
    encoder: VideoEncoderKind,
    pcm_inputs: &[PcmInputSpec],
) -> Vec<String> {
    let (out_w, out_h) = record_frame_size(req);

    let mut args = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "warning".into(),
        "-thread_queue_size".into(),
        "512".into(),
    ];

    args.extend([
        // Wall-clock PTS so video duration tracks real time (matches WASAPI PCM).
        // The DXGI pump must stay near target fps; when it falls behind it snaps
        // forward instead of flooding catch-up frames (that stutter).
        "-use_wallclock_as_timestamps".into(),
        "1".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "bgra".into(),
        "-video_size".into(),
        format!("{out_w}x{out_h}"),
        "-framerate".into(),
        req.fps.to_string(),
        "-thread_queue_size".into(),
        "64".into(),
        "-i".into(),
        "pipe:0".into(),
    ]);

    let mut next_index = 1usize;
    let mut audio_input_indices = Vec::new();
    for input in pcm_inputs {
        args.extend([
            "-f".into(),
            "f32le".into(),
            "-ar".into(),
            input.sample_rate.to_string(),
            "-ac".into(),
            input.channels.to_string(),
            // Raw PCM is fully described above. Letting FFmpeg run its default
            // multi-second probe opens live TCP inputs one after another and
            // offsets mic/loopback clocks, which makes amix backpressure video.
            "-analyzeduration".into(),
            "0".into(),
            "-probesize".into(),
            "32".into(),
            "-thread_queue_size".into(),
            "512".into(),
            "-i".into(),
            input.url.clone(),
        ]);
        audio_input_indices.push((next_index, input.kind));
        next_index += 1;
    }

    let need_complex = audio_input_indices.len() > 1;

    if req.format == OutputFormat::Gif {
        let fps = req.fps.min(15);
        let graph = format!(
            "[0:v]fps={fps},scale=iw:-1:flags=lanczos,scale=trunc(iw/2)*2:trunc(ih/2)*2,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
        );
        args.extend(["-filter_complex".into(), graph]);
        args.extend(["-c:v".into(), "gif".into()]);
        args.push(req.output_path.clone());
        return args;
    }

    let vchain = String::from("[0:v]scale=trunc(iw/2)*2:trunc(ih/2)*2");

    if need_complex {
        // Keep video out of the audio filter graph. amix may wait for either
        // live PCM source; coupling [0:v] to that graph backpressures the raw
        // video pipe and collapses screen/webcam capture to single-digit FPS.
        let mut graph = String::new();
        append_audio_filters(&mut graph, &audio_input_indices, req);
        args.extend(["-filter_complex".into(), graph]);
        if req.format == OutputFormat::AudioOnly {
            args.extend(["-vn".into()]);
        } else {
            let vf = vchain.trim_start_matches("[0:v]").to_string();
            args.extend(["-vf".into(), vf]);
            args.extend(FfmpegEncoder::video_encoder_args(
                encoder,
                quality_to_crf(req.quality),
            ));
            args.extend(["-map".into(), "0:v".into()]);
        }
        map_audio_after_complex(&mut args, &audio_input_indices);
    } else {
        let vf = vchain.trim_start_matches("[0:v]").to_string();
        if !vf.is_empty() {
            args.extend(["-vf".into(), vf]);
        }
        finish_simple_av(&mut args, req, encoder, &audio_input_indices);
    }

    args.extend(["-fps_mode".into(), "cfr".into()]);
    if req.format == OutputFormat::Mp4 {
        // Fragmented MP4 stays playable if FFmpeg is killed before writing a final moov.
        args.extend([
            "-movflags".into(),
            "+frag_keyframe+empty_moov+default_base_moof".into(),
        ]);
    }
    args.push(req.output_path.clone());
    args
}

#[cfg(windows)]
fn append_audio_filters(
    graph: &mut String,
    inputs: &[(usize, AudioDeviceKind)],
    req: &RecordRequest,
) {
    if inputs.len() < 2 {
        return;
    }
    if !graph.is_empty() {
        graph.push(';');
    }
    let labeled = inputs
        .iter()
        .map(|(index, kind)| {
            let gain = audio_gain(*kind, req);
            format!("[{index}:a]volume={gain}[a{index}]")
        })
        .collect::<Vec<_>>()
        .join(";");
    let mix_inputs = inputs
        .iter()
        .map(|(index, _)| format!("[a{index}]"))
        .collect::<String>();
    graph.push_str(&labeled);
    graph.push(';');
    graph.push_str(&format!(
        "{mix_inputs}amix=inputs={}:duration=shortest:dropout_transition=0:normalize=0[aout]",
        inputs.len()
    ));
}

#[cfg(windows)]
fn map_audio_after_complex(args: &mut Vec<String>, inputs: &[(usize, AudioDeviceKind)]) {
    match inputs {
        [] => args.extend(["-an".into()]),
        [(index, _)] => {
            args.extend(["-map".into(), format!("{index}:a")]);
            args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
        }
        _ => {
            args.extend(["-map".into(), "[aout]".into()]);
            args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
        }
    }
}

#[cfg(windows)]
fn finish_simple_av(
    args: &mut Vec<String>,
    req: &RecordRequest,
    encoder: VideoEncoderKind,
    audio_input_indices: &[(usize, AudioDeviceKind)],
) {
    if req.format == OutputFormat::AudioOnly {
        args.extend(["-vn".into()]);
        add_audio_mapping_simple(args, audio_input_indices, req);
    } else {
        args.extend(FfmpegEncoder::video_encoder_args(
            encoder,
            quality_to_crf(req.quality),
        ));
        if audio_input_indices.is_empty() {
            args.extend(["-an".into()]);
        } else if audio_input_indices.len() == 1 {
            args.extend(["-map".into(), "0:v".into()]);
            add_audio_mapping_simple(args, audio_input_indices, req);
        } else {
            args.extend(["-map".into(), "0:v".into()]);
            add_audio_mapping_simple(args, audio_input_indices, req);
        }
    }
}

#[cfg(windows)]
fn add_audio_mapping_simple(
    args: &mut Vec<String>,
    inputs: &[(usize, AudioDeviceKind)],
    req: &RecordRequest,
) {
    match inputs {
        [] => args.extend(["-an".into()]),
        [(index, _)] => {
            args.extend(["-map".into(), format!("{index}:a")]);
            let kind = inputs[0].1;
            args.extend(["-af".into(), format!("volume={}", audio_gain(kind, req))]);
            args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
        }
        _ => {
            let labeled = inputs
                .iter()
        .map(|(index, kind)| {
            let gain = audio_gain(*kind, req);
                    format!("[{index}:a]volume={gain}[a{index}]")
                })
                .collect::<Vec<_>>()
                .join(";");
            let mix_inputs = inputs
                .iter()
                .map(|(index, _)| format!("[a{index}]"))
                .collect::<String>();
            args.extend([
                "-filter_complex".into(),
                format!(
                    "{labeled};{mix_inputs}amix=inputs={}:duration=shortest:dropout_transition=0:normalize=0[aout]",
                    inputs.len()
                ),
                "-map".into(),
                "[aout]".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "192k".into(),
            ]);
        }
    }
}

fn default_record_quality() -> u8 {
    60
}

fn default_audio_volume() -> u8 {
    100
}

fn audio_gain(kind: AudioDeviceKind, req: &RecordRequest) -> f32 {
    let percent = match kind {
        AudioDeviceKind::Input => req.mic_volume,
        AudioDeviceKind::Loopback | AudioDeviceKind::Output => req.loopback_volume,
    }
    .min(200);
    f32::from(percent) / 100.0
}

fn quality_to_crf(quality: u8) -> u8 {
    let q = quality.clamp(1, 100) as u32;
    let crf = 51u32.saturating_sub((q * 33) / 100);
    crf.clamp(18, 51) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use capto_overlay::OverlayConfig;

    #[test]
    fn builds_region_args_uses_rawvideo_pipe() {
        let req = RecordRequest {
            source: VideoSourceKind::Region,
            display_id: None,
            window_id: None,
            region: Some(Region {
                x: 10,
                y: 20,
                width: 800,
                height: 600,
            }),
            include_cursor: true,
            mic_device: None,
            loopback_device: None,
            mic_volume: 100,
            loopback_volume: 100,
            encoder: None,
            format: OutputFormat::Mp4,
            fps: 30,
            quality: 60,
            output_path: "out.mp4".into(),
            overlays: OverlayConfig::default(),
            hide_app_while_recording: true,
        };
        let args = build_record_args(&req, VideoEncoderKind::Libx264, &[]);
        assert!(args.iter().any(|a| a.ends_with("out.mp4")));
        #[cfg(windows)]
        {
            assert!(args.windows(2).any(|w| w == ["-f", "rawvideo"]));
            assert!(args.windows(2).any(|w| w == ["-pix_fmt", "bgra"]));
            assert!(args.contains(&"pipe:0".to_string()));
            assert!(args.windows(2).any(|w| w == ["-video_size", "800x600"]));
        }
    }

    #[test]
    #[cfg(windows)]
    fn maps_and_mixes_native_pcm_inputs() {
        let req = RecordRequest {
            source: VideoSourceKind::Display,
            display_id: Some(0),
            window_id: None,
            region: None,
            include_cursor: true,
            mic_device: Some("wasapi:capture:mic-id".into()),
            loopback_device: Some("wasapi:render:speaker-id".into()),
            mic_volume: 125,
            loopback_volume: 75,
            encoder: None,
            format: OutputFormat::Mp4,
            fps: 30,
            quality: 60,
            output_path: "out.mp4".into(),
            overlays: OverlayConfig::default(),
            hide_app_while_recording: false,
        };
        let pcm = vec![
            PcmInputSpec {
                kind: AudioDeviceKind::Input,
                url: "tcp://127.0.0.1:41001".into(),
                sample_rate: 48_000,
                channels: 2,
            },
            PcmInputSpec {
                kind: AudioDeviceKind::Loopback,
                url: "tcp://127.0.0.1:41002".into(),
                sample_rate: 48_000,
                channels: 2,
            },
        ];
        let args = build_record_args(&req, VideoEncoderKind::Libx264, &pcm);
        assert_eq!(args.iter().filter(|arg| *arg == "f32le").count(), 2);
        assert_eq!(
            args.windows(2)
                .filter(|pair| *pair == ["-analyzeduration", "0"])
                .count(),
            2
        );
        assert_eq!(
            args.windows(2)
                .filter(|pair| *pair == ["-probesize", "32"])
                .count(),
            2
        );
        assert!(args.iter().any(|arg| {
            arg.contains("volume=1.25")
                && arg.contains("volume=0.75")
                && arg.contains("amix=inputs=2")
                && arg.contains("normalize=0")
        }));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:v"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[aout]"]));
        let graph = args
            .windows(2)
            .find(|pair| pair[0] == "-filter_complex")
            .map(|pair| pair[1].as_str())
            .expect("audio mix graph");
        assert!(!graph.contains("[0:v]"));
    }

    #[test]
    #[cfg(windows)]
    fn webcam_enabled_does_not_add_dshow_input() {
        let mut overlays = OverlayConfig::default();
        overlays.webcam.enabled = true;
        overlays.webcam.device_label = Some("HD Webcam".into());
        overlays.webcam.width = 320;
        overlays.webcam.height = 240;
        let req = RecordRequest {
            source: VideoSourceKind::Display,
            display_id: Some(0),
            window_id: None,
            region: Some(Region {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }),
            include_cursor: true,
            mic_device: None,
            loopback_device: None,
            mic_volume: 100,
            loopback_volume: 100,
            encoder: None,
            format: OutputFormat::Mp4,
            fps: 30,
            quality: 60,
            output_path: "out.mp4".into(),
            overlays,
            hide_app_while_recording: false,
        };
        let args = build_record_args(&req, VideoEncoderKind::Libx264, &[]);
        assert!(!args.windows(2).any(|w| w == ["-f", "dshow"]));
        assert!(!args.iter().any(|a| a.contains("overlay=")));
        assert!(args.windows(2).any(|w| w == ["-f", "rawvideo"]));
        assert!(args.contains(&"pipe:0".to_string()));
        assert!(!args.iter().any(|a| a == "-shortest"));
        assert!(args.iter().any(|a| a.contains("frag_keyframe")));
        assert!(args
            .windows(2)
            .any(|w| w == ["-use_wallclock_as_timestamps", "1"]));
    }
}
