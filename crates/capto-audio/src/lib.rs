use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{AudioMeterSession, NativeAudioSession};

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("audio device error: {0}")]
    Device(String),
    #[error("no audio host available")]
    NoHost,
    #[error("audio transport error: {0}")]
    Transport(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AudioDeviceKind {
    Input,
    Output,
    Loopback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: AudioDeviceKind,
    pub is_default: bool,
}

/// One native PCM stream exposed to FFmpeg over localhost TCP.
#[derive(Debug, Clone)]
pub struct PcmInputSpec {
    pub kind: AudioDeviceKind,
    pub url: String,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Recent peak levels for the native recording inputs, normalized to 0.0..=1.0.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevels {
    pub microphone: f32,
    pub system: f32,
}

/// Enumerate capture-relevant devices.
///
/// Windows uses native WASAPI endpoint IDs: capture endpoints are microphones,
/// render endpoints are offered as loopback sources.
#[cfg(windows)]
pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
    windows::list_devices()
}

#[cfg(not(windows))]
pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let mut devices = Vec::new();

    let default_in = host.default_input_device();
    let default_in_name = default_in.as_ref().and_then(|d| d.name().ok());

    if let Ok(inputs) = host.input_devices() {
        for (idx, d) in inputs.enumerate() {
            let name = d.name().unwrap_or_else(|_| format!("Input {idx}"));
            let is_default = default_in_name.as_ref() == Some(&name);
            devices.push(AudioDeviceInfo {
                id: format!("in:{idx}:{name}"),
                name: name.clone(),
                kind: AudioDeviceKind::Input,
                is_default,
            });
        }
    }

    let default_out = host.default_output_device();
    let default_out_name = default_out.as_ref().and_then(|d| d.name().ok());

    if let Ok(outputs) = host.output_devices() {
        for (idx, d) in outputs.enumerate() {
            let name = d.name().unwrap_or_else(|_| format!("Output {idx}"));
            let is_default = default_out_name.as_ref() == Some(&name);
            devices.push(AudioDeviceInfo {
                id: format!("out:{idx}:{name}"),
                name: name.clone(),
                kind: AudioDeviceKind::Output,
                is_default,
            });
        }
    }

    if devices.is_empty() {
        return Err(AudioError::NoHost);
    }
    Ok(devices)
}

/// Non-Windows placeholder; native platform audio backends are implemented
/// incrementally while preserving the same orchestration API.
#[cfg(not(windows))]
pub struct NativeAudioSession;

#[cfg(not(windows))]
impl NativeAudioSession {
    pub fn prepare(_mic: Option<&str>, _loopback: Option<&str>) -> Result<Option<Self>> {
        Ok(None)
    }

    pub fn inputs(&self) -> &[PcmInputSpec] {
        &[]
    }

    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) {}

    pub fn set_paused(&self, _paused: bool) {}

    pub fn levels(&self) -> AudioLevels {
        AudioLevels::default()
    }
}

#[cfg(not(windows))]
pub struct AudioMeterSession;

#[cfg(not(windows))]
impl AudioMeterSession {
    pub fn start(_mic: Option<&str>, _loopback: Option<&str>) -> Result<Option<Self>> {
        Ok(None)
    }

    pub fn levels(&self) -> AudioLevels {
        AudioLevels::default()
    }

    pub fn stop(self) {}
}
