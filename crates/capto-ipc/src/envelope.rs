use serde::{Deserialize, Serialize};

/// Stable process exit codes for agent tooling.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Usage = 1,
    DesktopUnavailable = 2,
    StateConflict = 3,
    Capture = 4,
    Encode = 5,
    ConfigIo = 6,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Envelope<()> {
        Envelope {
            ok: false,
            data: None,
            error: Some(ApiError::new(code, message)),
        }
    }

    pub fn from_result(result: Result<T, ApiError>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(error) => Self {
                ok: false,
                data: None,
                error: Some(error),
            },
        }
    }
}
