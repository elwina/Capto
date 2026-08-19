use anyhow::{bail, Context, Result};
use capto_ipc::{
    clear_server_lock, is_pid_alive, read_server_lock, redact, Envelope, ServerLock,
    REQUEST_ID_HEADER,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::launch;
use crate::resilience::{backoff_delays, BreakerConfig, CircuitBreaker};

pub struct ControlClient {
    base: String,
    token: String,
    http: reqwest::Client,
    breaker: std::sync::Mutex<CircuitBreaker>,
}

#[derive(Debug)]
pub struct HttpError {
    pub code: String,
    pub message: String,
}

const MAX_RETRIES: u32 = 2;

impl ControlClient {
    pub async fn connect(auto_launch: bool) -> Result<Self> {
        if let Some(client) = try_existing().await? {
            return Ok(client);
        }
        if !auto_launch {
            bail!(
                "Capto desktop control plane is not running (use without --no-launch to start it)"
            );
        }
        let exe = launch::spawn_capto().context("failed to launch Capto desktop")?;
        wait_for_ready(Duration::from_secs(45), &exe).await
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, HttpError> {
        let url = format!("{}{path}", self.base);
        let request_id = Uuid::new_v4().to_string();
        // Only idempotent calls are retried: retrying a mutating POST that
        // already reached the server could double-record. Reads are safe to
        // retry across the ~2s connection window while the desktop restarts.
        let retriable = method == reqwest::Method::GET;

        for (attempt, backoff) in backoff_delays(MAX_RETRIES).iter().enumerate() {
            {
                let breaker = self.breaker.lock().expect("breaker lock");
                if !breaker.allow(Instant::now()) {
                    return Err(HttpError {
                        code: "desktopUnavailable".into(),
                        message:
                            "Capto control plane unavailable (circuit open). Start Capto and retry."
                                .into(),
                    });
                }
            }

            let mut req = self
                .http
                .request(method.clone(), &url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .header(REQUEST_ID_HEADER, request_id.clone());
            if let Some(b) = &body {
                req = req.json(b);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    self.note_failure();
                    if retriable && attempt < MAX_RETRIES as usize {
                        tokio::time::sleep(*backoff).await;
                        continue;
                    }
                    return Err(HttpError {
                        code: "desktopUnavailable".into(),
                        message: redact(&e.to_string()),
                    });
                }
            };
            let status = resp.status();
            let envelope: Envelope<Value> = match resp.json().await {
                Ok(e) => e,
                Err(e) => {
                    self.note_failure();
                    if retriable && attempt < MAX_RETRIES as usize {
                        tokio::time::sleep(*backoff).await;
                        continue;
                    }
                    return Err(HttpError {
                        code: "desktopUnavailable".into(),
                        message: format!("invalid JSON from Capto: {}", redact(&e.to_string())),
                    });
                }
            };
            self.note_success();
            if envelope.ok {
                return Ok(envelope.data.unwrap_or(Value::Null));
            }
            let err = envelope.error.unwrap_or_else(|| {
                capto_ipc::ApiError::new(
                    if status.as_u16() == 409 {
                        "stateConflict"
                    } else {
                        "error"
                    },
                    format!("HTTP {status}"),
                )
            });
            return Err(HttpError {
                code: err.code,
                message: redact(&err.message),
            });
        }
        unreachable!("backoff_delays is non-empty for MAX_RETRIES > 0")
    }

    fn note_success(&self) {
        if let Ok(mut breaker) = self.breaker.lock() {
            breaker.on_success();
        }
    }

    fn note_failure(&self) {
        if let Ok(mut breaker) = self.breaker.lock() {
            breaker.on_failure(Instant::now());
        }
    }

    pub async fn get(&self, path: &str) -> Result<Value, HttpError> {
        self.request(reqwest::Method::GET, path, None).await
    }

    pub async fn post_json<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<Value, HttpError> {
        let v = serde_json::to_value(body).map_err(|e| HttpError {
            code: "usage".into(),
            message: e.to_string(),
        })?;
        self.request(reqwest::Method::POST, path, Some(v)).await
    }

    pub async fn post_empty(&self, path: &str) -> Result<Value, HttpError> {
        self.request(reqwest::Method::POST, path, Some(json_null_obj()))
            .await
    }

    pub async fn patch_json(&self, path: &str, body: &Value) -> Result<Value, HttpError> {
        self.request(reqwest::Method::PATCH, path, Some(body.clone()))
            .await
    }
}

fn json_null_obj() -> Value {
    Value::Object(serde_json::Map::new())
}

async fn try_existing() -> Result<Option<ControlClient>> {
    let lock = match read_server_lock() {
        Ok(l) => l,
        Err(_) => return Ok(None),
    };
    if !is_pid_alive(lock.pid) {
        clear_server_lock();
        return Ok(None);
    }
    let client = from_lock(lock);
    match client.get("/v1/status").await {
        Ok(_) => Ok(Some(client)),
        Err(_) => Ok(None),
    }
}

fn from_lock(lock: ServerLock) -> ControlClient {
    ControlClient {
        base: format!("http://127.0.0.1:{}", lock.port),
        token: lock.token,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest client"),
        breaker: std::sync::Mutex::new(CircuitBreaker::new(BreakerConfig::default())),
    }
}

async fn wait_for_ready(
    timeout: Duration,
    launched_exe: &std::path::Path,
) -> Result<ControlClient> {
    let start = std::time::Instant::now();
    loop {
        if let Some(client) = try_existing().await? {
            return Ok(client);
        }
        if start.elapsed() > timeout {
            bail!(
                "timed out waiting for Capto control plane after opening {}. Run `capto open`, or ask the user to start Capto from the Start menu, then retry",
                launched_exe.display()
            );
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[allow(dead_code)]
async fn _decode<T: DeserializeOwned>(v: Value) -> Result<T> {
    Ok(serde_json::from_value(v)?)
}
