//! Local, structured crash reporting with a breadcrumb trail.
//!
//! Privacy-first error tracking: Capto never uploads telemetry. Instead, when
//! the desktop process panics it writes a `crash-<ms>.json` report to
//! `<config>/crashes/` containing:
//!
//! - identity (app, version, os) + timestamp
//! - the panic subject and exact `panic_location` (file:line)
//! - a captured `backtrace` (full stack, `RUST_BACKTRACE=1` for symbols)
//! - user/session context: pid, process uptime, active feature flags
//! - the `breadcrumbs` trail (recent control-plane requests, session
//!   transitions, lifecycle/hotkey events) and the last `x-request-id`
//!
//! Together these let an agent trace the failure back to a concrete code path
//! and the sequence of actions that preceded it
//! (`error_tracking_contextualized`). Local only; the trail is capped
//! (see `capto_core::breadcrumbs`) and contains no tokens, bodies, or query
//! values. Writing is best-effort and never blocks the panic handler.

use capto_core::breadcrumbs::{self, Breadcrumb};
use capto_core::flags::{all, is_enabled, CRASH_REPORTING};
use capto_core::AppSettings;
use serde::Serialize;
use std::backtrace::Backtrace;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub app: String,
    pub version: String,
    pub os: String,
    pub timestamp_ms: u64,
    pub subject: String,
    pub backtrace: String,
    /// Exact panic site (`file:line:col`), when the panic info carried one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panic_location: Option<String>,
    pub pid: u32,
    /// Milliseconds elapsed since the process started when the panic fired.
    pub uptime_ms: u64,
    /// Feature flags that were active when the crash happened.
    pub feature_flags: Vec<String>,
    /// Most recent control-plane `x-request-id`, to correlate with logs
    /// (see docs/crash-tracing.md).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_request_id: Option<String>,
    /// Most recent significant events recorded before the crash.
    pub breadcrumbs: Vec<Breadcrumb>,
}

/// `<config>/Capto/crashes` — the same folder tree that holds settings.json.
pub fn crash_dir() -> PathBuf {
    AppSettings::config_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("crashes")
}

pub fn write_crash_report(dir: &Path, report: &CrashReport) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("crash-{}.json", report.timestamp_ms));
    fs::write(&path, serde_json::to_string_pretty(report)?)?;
    Ok(path)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Names of every declared flag currently enabled for `settings`.
fn active_flags(settings: &AppSettings) -> Vec<String> {
    all()
        .iter()
        .filter(|flag| is_enabled(settings, flag.name))
        .map(|flag| flag.name.to_string())
        .collect()
}

/// Install the crash-reporting panic hook unless the `crash-reporting`
/// feature flag is disabled in settings.json.
pub fn install_panic_hook() {
    let settings = AppSettings::load();
    if !is_enabled(&settings, CRASH_REPORTING) {
        return;
    }
    let feature_flags = active_flags(&settings);
    // Best-effort snapshot: does not block even if a recorder holds the bus
    // lock at panic time.
    let context = breadcrumbs::try_current_context();
    std::panic::set_hook(Box::new(move |info| {
        let report = CrashReport {
            app: "Capto".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            os: std::env::consts::OS.into(),
            timestamp_ms: now_ms(),
            subject: info.to_string(),
            backtrace: Backtrace::force_capture().to_string(),
            panic_location: info.location().map(|loc| loc.to_string()),
            pid: std::process::id(),
            uptime_ms: context.as_ref().map(|ctx| ctx.uptime_ms).unwrap_or(0),
            feature_flags: feature_flags.clone(),
            last_request_id: context.as_ref().and_then(|ctx| ctx.last_request_id.clone()),
            breadcrumbs: context
                .as_ref()
                .map(|ctx| ctx.breadcrumbs.clone())
                .unwrap_or_default(),
        };
        match write_crash_report(&crash_dir(), &report) {
            Ok(path) => tracing::error!(
                path = %path.display(),
                subject = %report.subject,
                "Capto panicked; crash report written"
            ),
            Err(e) => {
                tracing::error!(%e, subject = %report.subject, "Capto panicked; crash report could not be written")
            }
        }
        // Keep the default behavior visible on stderr for console users.
        eprintln!("Capto panicked: {}", info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_structured_crash_report() {
        let dir = tempdir().unwrap();
        let breadcrumb = Breadcrumb {
            category: "control-plane".into(),
            message: "GET /v1/status -> 200".into(),
            request_id: Some("req-1".into()),
            rel_ms: 12,
            at_ms: 1700000000000,
        };
        let report = CrashReport {
            app: "Capto".into(),
            version: "1.0.0".into(),
            os: "windows".into(),
            timestamp_ms: 1700000000000,
            subject: "boom".into(),
            backtrace: " 0: core::panicking::panic".into(),
            panic_location: Some("src/main.rs:12:5".into()),
            pid: 1234,
            uptime_ms: 42,
            feature_flags: vec!["control-plane-metrics".into(), "crash-reporting".into()],
            last_request_id: Some("req-1".into()),
            breadcrumbs: vec![breadcrumb],
        };
        let path = write_crash_report(dir.path(), &report).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["subject"], "boom");
        assert_eq!(parsed["backtrace"], " 0: core::panicking::panic");
        assert_eq!(parsed["os"], "windows");
        assert_eq!(parsed["timestampMs"], 1700000000000_i64);
        assert_eq!(parsed["panicLocation"], "src/main.rs:12:5");
        assert_eq!(parsed["pid"], 1234);
        assert_eq!(parsed["uptimeMs"], 42);
        assert_eq!(parsed["lastRequestId"], "req-1");
        assert_eq!(parsed["breadcrumbs"][0]["message"], "GET /v1/status -> 200");
        assert_eq!(parsed["breadcrumbs"][0]["requestId"], "req-1");
        assert_eq!(parsed["featureFlags"][0], "control-plane-metrics");
    }

    #[test]
    fn crash_dir_is_under_config_folder() {
        let dir = crash_dir();
        assert!(
            dir.ends_with("crashes"),
            "unexpected crash dir: {}",
            dir.display()
        );
    }
}
