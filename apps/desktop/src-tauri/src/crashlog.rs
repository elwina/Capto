//! Local, structured crash reporting.
//!
//! Privacy-first error tracking: Capto never uploads telemetry. Instead, when
//! the desktop process panics it writes a `crash-<ms>.json` report (app,
//! version, OS, panic subject, captured backtrace) to `<config>/crashes/` so
//! an agent or maintainer can trace the failure back to a concrete code path
//! (error_tracking_contextualized). Writing is best-effort and never blocks
//! the panic handler.

use capto_core::flags::{is_enabled, CRASH_REPORTING};
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

/// Install the crash-reporting panic hook unless the `crash-reporting`
/// feature flag is disabled in settings.json.
pub fn install_panic_hook() {
    let settings = AppSettings::load();
    if !is_enabled(&settings, CRASH_REPORTING) {
        return;
    }
    std::panic::set_hook(Box::new(|info| {
        let report = CrashReport {
            app: "Capto".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            os: std::env::consts::OS.into(),
            timestamp_ms: now_ms(),
            subject: info.to_string(),
            backtrace: Backtrace::force_capture().to_string(),
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
        let report = CrashReport {
            app: "Capto".into(),
            version: "1.0.0".into(),
            os: "windows".into(),
            timestamp_ms: 1700000000000,
            subject: "boom".into(),
            backtrace: " 0: core::panicking::panic".into(),
        };
        let path = write_crash_report(dir.path(), &report).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["subject"], "boom");
        assert_eq!(parsed["backtrace"], " 0: core::panicking::panic");
        assert_eq!(parsed["os"], "windows");
        assert_eq!(parsed["timestampMs"], 1700000000000_i64);
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
