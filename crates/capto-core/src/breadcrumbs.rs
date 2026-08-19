//! Local breadcrumb trail for contextual crash reports.
//!
//! Privacy-first error tracking (see `docs/crash-tracing.md`): Capto never
//! uploads telemetry. Instead the desktop keeps a small in-memory ring buffer
//! of significant events - control-plane requests, session transitions,
//! lifecycle markers, hotkey actions - that would otherwise be lost the
//! moment the process panics. The panic hook (the desktop crate's
//! `crashlog`) embeds this trail, plus user/session context, in the local
//! `crash-*.json` report so an agent can trace a failure back to the exact
//! sequence of actions that preceded it (`error_tracking_contextualized`).
//!
//! Panic-path safety: the panic hook runs while other threads may still hold
//! locks, so reads go through [`BreadcrumbBus::try_snapshot`], which returns
//! `None` instead of blocking when the lock is contended. The crash report
//! then simply omits the trail rather than deadlocking the panic handler.
//!
//! Scrubbing by construction: callers only ever record scrubbed descriptions
//! (method/path/status, transition names) - never request bodies, query
//! strings, or the bearer token. `capto_ipc::redact` masks any that slip in.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock, TryLockError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// How many most-recent events the ring buffer keeps in memory.
pub const DEFAULT_CAPACITY: usize = 64;

/// One recorded event on the breadcrumb trail.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breadcrumb {
    /// Origin of the event: "lifecycle", "control-plane", "session", "hotkey".
    pub category: String,
    /// Human-readable, already-scrubbed description (never contains tokens or
    /// query values).
    pub message: String,
    /// Correlating `x-request-id` when the event came from the control plane.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Milliseconds since process start (relative ordering of the trail).
    pub rel_ms: u64,
    /// Wall-clock milliseconds since the Unix epoch (log cross-referencing).
    pub at_ms: u64,
}

/// Everything the panic hook copies out of the live bus.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashContext {
    pub uptime_ms: u64,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub last_request_id: Option<String>,
}

struct Inner {
    capacity: usize,
    events: VecDeque<Breadcrumb>,
    last_request_id: Option<String>,
}

/// Thread-safe, fixed-capacity breadcrumb trail scoped to one process.
pub struct BreadcrumbBus {
    started: Instant,
    inner: Mutex<Inner>,
}

impl Default for BreadcrumbBus {
    fn default() -> Self {
        Self::new()
    }
}

impl BreadcrumbBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            started: Instant::now(),
            inner: Mutex::new(Inner {
                capacity: capacity.max(1),
                events: VecDeque::new(),
                last_request_id: None,
            }),
        }
    }

    /// Milliseconds since this bus was created (process uptime proxy).
    pub fn uptime_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Record one event, dropping the oldest when the buffer is full.
    pub fn record(&self, category: &str, message: impl Into<String>, request_id: Option<String>) {
        let rel_ms = self.uptime_ms();
        let at_ms = now_ms();
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(id) = &request_id {
                inner.last_request_id = Some(id.clone());
            }
            if inner.events.len() == inner.capacity {
                inner.events.pop_front();
            }
            inner.events.push_back(Breadcrumb {
                category: category.to_string(),
                message: message.into(),
                request_id,
                rel_ms,
                at_ms,
            });
        }
    }

    /// The most recent control-plane request id observed (log correlation).
    pub fn last_request_id(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|guard| guard.last_request_id.clone())
    }

    /// Snapshot for the panic hook without ever blocking. Returns `None` if
    /// another thread currently holds the lock.
    pub fn try_snapshot(&self) -> Option<CrashContext> {
        let guard = match self.inner.try_lock() {
            Ok(guard) => guard,
            // A panicked recorder poisoned the lock but released it; the
            // data is still safe to read.
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            // Contended at panic time: omit the trail rather than deadlock.
            Err(TryLockError::WouldBlock) => return None,
        };
        Some(CrashContext {
            uptime_ms: self.uptime_ms(),
            breadcrumbs: guard.events.iter().cloned().collect(),
            last_request_id: guard.last_request_id.clone(),
        })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Process-wide bus shared by event recorders and the crash-report panic hook.
/// Capto guarantees a single desktop process per install (single-instance
/// plugin), so one global bus is correct here.
pub fn bus() -> &'static BreadcrumbBus {
    static BUS: OnceLock<BreadcrumbBus> = OnceLock::new();
    BUS.get_or_init(BreadcrumbBus::new)
}

/// Record a breadcrumb with no correlating request id.
pub fn record(category: &str, message: impl Into<String>) {
    bus().record(category, message, None);
}

/// Record a breadcrumb tied to a control-plane request id.
pub fn record_with_request(category: &str, message: impl Into<String>, request_id: Option<String>) {
    bus().record(category, message, request_id);
}

/// Snapshot the process-wide trail for the panic hook (never blocks).
pub fn try_current_context() -> Option<CrashContext> {
    bus().try_snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_events_in_order() {
        let bus = BreadcrumbBus::new();
        bus.record("lifecycle", "app_started", None);
        bus.record("session", "record start ok", None);
        let ctx = bus.try_snapshot().expect("snapshot available");
        assert_eq!(ctx.breadcrumbs.len(), 2);
        assert_eq!(ctx.breadcrumbs[0].category, "lifecycle");
        assert_eq!(ctx.breadcrumbs[1].message, "record start ok");
        assert!(ctx.breadcrumbs[1].rel_ms >= ctx.breadcrumbs[0].rel_ms);
    }

    #[test]
    fn trims_oldest_beyond_capacity() {
        let bus = BreadcrumbBus::with_capacity(3);
        for i in 0..5 {
            bus.record("test", format!("event {i}"), None);
        }
        let ctx = bus.try_snapshot().expect("snapshot available");
        let messages: Vec<&str> = ctx.breadcrumbs.iter().map(|b| b.message.as_str()).collect();
        assert_eq!(messages, ["event 2", "event 3", "event 4"]);
    }

    #[test]
    fn tracks_last_request_id() {
        let bus = BreadcrumbBus::new();
        bus.record(
            "control-plane",
            "POST /v1/record/start -> 200",
            Some("req-1".into()),
        );
        bus.record(
            "control-plane",
            "GET /v1/status -> 200",
            Some("req-2".into()),
        );
        assert_eq!(bus.last_request_id(), Some("req-2".into()));
        let ctx = bus.try_snapshot().unwrap();
        assert_eq!(ctx.last_request_id.as_deref(), Some("req-2"));
        assert_eq!(ctx.breadcrumbs[1].request_id.as_deref(), Some("req-2"));
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let bus = BreadcrumbBus::new();
        bus.record("lifecycle", "app_started", None);
        let json = serde_json::to_value(bus.try_snapshot().unwrap()).unwrap();
        assert!(json.get("uptimeMs").is_some());
        assert!(json.get("breadcrumbs").is_some());
        assert!(json.get("lastRequestId").is_some());
        assert_eq!(json["breadcrumbs"][0]["category"], "lifecycle");
        assert!(json["breadcrumbs"][0]["relMs"].as_u64().is_some());
    }

    #[test]
    fn empty_bus_snapshots_to_empty_trail() {
        let bus = BreadcrumbBus::new();
        let ctx = bus.try_snapshot().expect("snapshot available");
        assert!(ctx.breadcrumbs.is_empty());
        assert_eq!(ctx.last_request_id, None);
    }
}
