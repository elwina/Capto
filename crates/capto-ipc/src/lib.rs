//! Shared local control-plane types for Capto desktop ↔ CLI.
//!
//! Transport is localhost HTTP. Discovery uses `cli-server.json` under the
//! Capto config directory (same folder as `settings.json`).

mod envelope;
mod lockfile;
mod redact;
mod types;

pub use envelope::{ApiError, Envelope, ExitCode};
pub use lockfile::{
    clear_server_lock, is_pid_alive, lock_path, read_server_lock, write_server_lock, ServerLock,
    LOCK_VERSION,
};
pub use redact::{redact, REQUEST_ID_HEADER};
pub use types::{
    ConfigPathInfo, DoctorInfo, OpenOutputsRequest, OutputEntry, OutputsList, RecordStartRequest,
    ShotRequest,
};

pub const API_PREFIX: &str = "/v1";
