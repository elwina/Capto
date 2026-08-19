pub mod breadcrumbs;
pub mod ffmpeg_args;
pub mod flags;
pub mod metrics;
pub mod session;
pub mod settings;

pub use ffmpeg_args::{record_frame_size, RecordRequest, Region};
pub use session::{RecordingSession, SessionSnapshot, SessionState};
pub use settings::{AppSettings, OutputFormat, VideoSourceKind};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error(transparent)]
    Capture(#[from] capto_capture::CaptureError),
    #[error(transparent)]
    Encode(#[from] capto_encode::EncodeError),
    #[error(transparent)]
    Audio(#[from] capto_audio::AudioError),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
