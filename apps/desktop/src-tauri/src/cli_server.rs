//! Localhost HTTP control plane for Capto CLI / agents.

use crate::session_svc;
use crate::AppState;
use axum::extract::{Query, State as AxumState};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use capto_core::flags::{self, CONTROL_PLANE_METRICS};
use capto_core::metrics::Metrics;
use capto_ipc::{
    clear_server_lock, write_server_lock, Envelope, OpenOutputsRequest, RecordStartRequest,
    ServerLock, ShotRequest, LOCK_VERSION, REQUEST_ID_HEADER,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Clone)]
struct HttpState {
    app: AppHandle,
    token: String,
    port: u16,
    metrics: Metrics,
    register_hotkeys:
        Arc<dyn Fn(&AppHandle, &capto_core::AppSettings) -> Vec<String> + Send + Sync>,
}

fn unauthorized() -> impl IntoResponse {
    (
        StatusCode::UNAUTHORIZED,
        Json(Envelope::<()>::err(
            "unauthorized",
            "invalid or missing bearer token",
        )),
    )
}

fn check_auth(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.strip_prefix("Bearer ")
                .map(|t| t == expected)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn app_state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

fn map_err(code: &str, e: String) -> (StatusCode, Json<Envelope<()>>) {
    let status = match code {
        "stateConflict" => StatusCode::CONFLICT,
        "notFound" => StatusCode::NOT_FOUND,
        "badRequest" => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(Envelope::<()>::err(code, e)))
}

async fn status_handler(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let snap = session_svc::status(&app_state(&st.app)).await;
    Json(Envelope::ok(snap)).into_response()
}

async fn doctor_handler(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let info = session_svc::doctor(&app_state(&st.app), st.port).await;
    Json(Envelope::ok(info)).into_response()
}

async fn config_get(AxumState(st): AxumState<HttpState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let settings = session_svc::get_settings(&app_state(&st.app)).await;
    Json(Envelope::ok(settings)).into_response()
}

async fn config_path(AxumState(st): AxumState<HttpState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    Json(Envelope::ok(session_svc::config_path())).into_response()
}

async fn config_patch(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let register = st.register_hotkeys.clone();
    match session_svc::patch_settings(&st.app, &app_state(&st.app), patch, move |a, s| {
        register(a, s)
    })
    .await
    {
        Ok(settings) => Json(Envelope::ok(settings)).into_response(),
        Err(e) => map_err("configIo", e).into_response(),
    }
}

async fn record_start(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
    Json(body): Json<RecordStartRequest>,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::start_recording(&st.app, &app_state(&st.app), body).await {
        Ok(snap) => Json(Envelope::ok(snap)).into_response(),
        Err(e) => {
            let code = if e.contains("invalid state") || e.contains("already") {
                "stateConflict"
            } else if e.to_lowercase().contains("ffmpeg") || e.to_lowercase().contains("encode") {
                "encode"
            } else {
                "capture"
            };
            map_err(code, e).into_response()
        }
    }
}

async fn record_stop(AxumState(st): AxumState<HttpState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::stop_recording(&st.app, &app_state(&st.app)).await {
        Ok(snap) => Json(Envelope::ok(snap)).into_response(),
        Err(e) => map_err("stateConflict", e).into_response(),
    }
}

async fn record_pause(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::pause_recording(&st.app, &app_state(&st.app)).await {
        Ok(snap) => Json(Envelope::ok(snap)).into_response(),
        Err(e) => map_err("stateConflict", e).into_response(),
    }
}

async fn record_resume(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::resume_recording(&st.app, &app_state(&st.app)).await {
        Ok(snap) => Json(Envelope::ok(snap)).into_response(),
        Err(e) => map_err("stateConflict", e).into_response(),
    }
}

async fn shot_handler(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
    Json(body): Json<ShotRequest>,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::take_screenshot(&app_state(&st.app), body).await {
        Ok(path) => Json(Envelope::ok(serde_json::json!({ "path": path }))).into_response(),
        Err(e) => map_err("capture", e).into_response(),
    }
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

async fn list_displays(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::list_displays(&app_state(&st.app)).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("capture", e).into_response(),
    }
}

async fn list_windows(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::list_windows(&app_state(&st.app)).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("capture", e).into_response(),
    }
}

async fn list_audio(AxumState(st): AxumState<HttpState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::list_audio() {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("capture", e).into_response(),
    }
}

async fn list_encoders(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::list_encoders(&app_state(&st.app)).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("encode", e).into_response(),
    }
}

async fn outputs_recent(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let limit = q.limit.unwrap_or(20);
    match session_svc::outputs_recent(&app_state(&st.app), limit).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("configIo", e).into_response(),
    }
}

async fn outputs_open(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
    Json(body): Json<OpenOutputsRequest>,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    match session_svc::open_outputs(&app_state(&st.app), body).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => map_err("notFound", e).into_response(),
    }
}

/// Per-request telemetry middleware for the control plane:
///
/// - propagates/echoes an `x-request-id` (distributed_tracing)
/// - records request counters + duration histograms into the local `Metrics`
///   registry (metrics_collection)
/// - logs a structured, scrubbed line: only method, path, status, duration
///   and request id. Queries, auth headers and bodies never reach the log
///   (log_scrubbing by construction).
async fn telemetry_layer(
    AxumState(metrics): AxumState<Metrics>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let method = req.method().clone().to_string();
    let path = req.uri().path().to_string();
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let started = Instant::now();
    let resp = next.run(req).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    let status = resp.status();
    let status_code = status.as_u16();

    metrics.incr("http_requests_total");
    metrics.observe_ms("http_request_duration_ms", duration_ms);
    metrics.incr(&format!("http_status_{}", status.as_u16()));
    tracing::info!(
        request_id,
        method,
        path,
        status = status.as_u16(),
        duration_ms,
        "control plane request"
    );
    // Contextual error tracking: keep a scrubbed, capped trail of recent
    // control-plane calls so a later crash report shows what led up to it.
    // Only method/path/status/request-id are recorded - never bodies, query
    // strings, or auth material.
    capto_core::breadcrumbs::record_with_request(
        "control-plane",
        format!("{method} {path} -> {status_code}"),
        Some(request_id.clone()),
    );

    let (mut parts, body) = resp.into_parts();
    parts.headers.insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );
    Response::from_parts(parts, body)
}

async fn metrics_handler(
    AxumState(st): AxumState<HttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &st.token) {
        return unauthorized().into_response();
    }
    let settings = session_svc::get_settings(&app_state(&st.app)).await;
    if !flags::is_enabled(&settings, CONTROL_PLANE_METRICS) {
        return (
            StatusCode::NOT_FOUND,
            Json(Envelope::<()>::err(
                "notFound",
                "metrics disabled by feature flag",
            )),
        )
            .into_response();
    }
    Json(Envelope::ok(st.metrics.snapshot())).into_response()
}

fn build_router(state: HttpState) -> Router {
    Router::new()
        .route("/v1/status", get(status_handler))
        .route("/v1/doctor", get(doctor_handler))
        .route("/v1/config", get(config_get).patch(config_patch))
        .route("/v1/config/path", get(config_path))
        .route("/v1/record/start", post(record_start))
        .route("/v1/record/stop", post(record_stop))
        .route("/v1/record/pause", post(record_pause))
        .route("/v1/record/resume", post(record_resume))
        .route("/v1/shot", post(shot_handler))
        .route("/v1/list/displays", get(list_displays))
        .route("/v1/list/windows", get(list_windows))
        .route("/v1/list/audio", get(list_audio))
        .route("/v1/list/encoders", get(list_encoders))
        .route("/v1/outputs/recent", get(outputs_recent))
        .route("/v1/outputs/open", post(outputs_open))
        .route("/v1/metrics", get(metrics_handler))
        .layer(middleware::from_fn_with_state(
            state.metrics.clone(),
            telemetry_layer,
        ))
        .with_state(state)
}

/// Bind `127.0.0.1:0`, write lock file, spawn axum server. Returns bound port.
pub fn start_control_plane(
    app: AppHandle,
    register_hotkeys: Arc<
        dyn Fn(&AppHandle, &capto_core::AppSettings) -> Vec<String> + Send + Sync,
    >,
    metrics: Metrics,
) -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = Uuid::new_v4().to_string();
    let lock = ServerLock {
        pid: std::process::id(),
        port,
        token: token.clone(),
        version: LOCK_VERSION,
    };
    write_server_lock(&lock).map_err(|e| e.to_string())?;

    let state = HttpState {
        app: app.clone(),
        token,
        port,
        metrics,
        register_hotkeys,
    };
    let router = build_router(state);

    tauri::async_runtime::spawn(async move {
        let std_listener = listener;
        match tokio::net::TcpListener::from_std(std_listener) {
            Ok(tokio_listener) => {
                tracing::info!(port, "CLI control plane listening on 127.0.0.1");
                if let Err(e) = axum::serve(tokio_listener, router).await {
                    tracing::error!(%e, "CLI control plane exited");
                }
            }
            Err(e) => tracing::error!(%e, "failed to convert TCP listener"),
        }
        clear_server_lock();
    });

    Ok(port)
}

pub fn shutdown_control_plane() {
    clear_server_lock();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;

    fn auth_header(value: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        m
    }

    fn bearer(token: &str) -> HeaderMap {
        auth_header(&format!("Bearer {token}"))
    }

    #[test]
    fn auth_accepts_exact_token() {
        assert!(check_auth(&bearer("s3cret-token"), "s3cret-token"));
    }

    #[test]
    fn auth_rejects_missing_header() {
        assert!(!check_auth(&HeaderMap::new(), "s3cret-token"));
    }

    #[test]
    fn auth_rejects_wrong_token() {
        assert!(!check_auth(&bearer("wrong"), "s3cret-token"));
    }

    #[test]
    fn auth_rejects_non_bearer_scheme() {
        assert!(!check_auth(
            &auth_header("Basic dXNlcjpwYXNz"),
            "s3cret-token"
        ));
    }

    #[test]
    fn auth_rejects_bare_bearer_without_value() {
        assert!(!check_auth(&auth_header("Bearer"), "s3cret-token"));
    }
}
