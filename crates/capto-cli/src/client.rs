use anyhow::{bail, Context, Result};
use capto_ipc::{clear_server_lock, is_pid_alive, read_server_lock, Envelope, ServerLock};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;

use crate::launch;

pub struct ControlClient {
    base: String,
    token: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub struct HttpError {
    pub code: String,
    pub message: String,
}

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
        let mut req = self
            .http
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json");
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await.map_err(|e| HttpError {
            code: "desktopUnavailable".into(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        let envelope: Envelope<Value> = resp.json().await.map_err(|e| HttpError {
            code: "desktopUnavailable".into(),
            message: format!("invalid JSON from Capto: {e}"),
        })?;
        if envelope.ok {
            Ok(envelope.data.unwrap_or(Value::Null))
        } else {
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
            Err(HttpError {
                code: err.code,
                message: err.message,
            })
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
