//! Local-only metrics registry for the Capto control plane.
//!
//! Privacy-first by design: nothing here is ever sent off the machine. The
//! desktop holds a `Metrics` instance, the control-plane HTTP layer records
//! per-endpoint counters and request durations into it, and the same snapshot
//! is served back over the localhost `/v1/metrics` endpoint (auth required)
//! for agent/operator debugging and build-time performance insight
//! (metrics_collection + product usage instrumentation, local form).

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterPoint {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DurationPoint {
    pub name: String,
    pub count: u64,
    pub total_ms: u64,
    pub avg_ms: u64,
    pub max_ms: u64,
}

/// Point-in-time snapshot, serialized for `/v1/metrics`.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshot {
    /// Process uptime at snapshot time, in milliseconds.
    pub uptime_ms: u64,
    pub counters: Vec<CounterPoint>,
    pub durations: Vec<DurationPoint>,
}

#[derive(Default)]
struct DurationAgg {
    count: u64,
    total_ms: u64,
    max_ms: u64,
}

struct Inner {
    started: Instant,
    counters: BTreeMap<String, u64>,
    durations: BTreeMap<String, DurationAgg>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            counters: BTreeMap::new(),
            durations: BTreeMap::new(),
        }
    }
}

/// Cheap-to-clone (single Arc) thread-safe metrics collector.
#[derive(Clone)]
pub struct Metrics {
    inner: Arc<Mutex<Inner>>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }

    /// Increment a named counter by one (e.g. `"recordings_started"`).
    pub fn incr(&self, name: &str) {
        self.add(name, 1);
    }

    /// Add `n` to a named counter.
    pub fn add(&self, name: &str, n: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner.counters.entry(name.to_string()).or_default() += n;
        }
    }

    /// Observe a single sample (e.g. request duration) for a named series.
    pub fn observe_ms(&self, name: &str, ms: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            let agg = inner.durations.entry(name.to_string()).or_default();
            agg.count += 1;
            agg.total_ms += ms;
            agg.max_ms = agg.max_ms.max(ms);
        }
    }

    /// Snapshot the current counters/durations without disturbing them.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let uptime_ms = inner.started.elapsed().as_millis() as u64;
        let counters = inner
            .counters
            .iter()
            .map(|(name, count)| CounterPoint {
                name: name.clone(),
                count: *count,
            })
            .collect();
        let durations = inner
            .durations
            .iter()
            .map(|(name, agg)| DurationPoint {
                name: name.clone(),
                count: agg.count,
                total_ms: agg.total_ms,
                avg_ms: agg.total_ms.checked_div(agg.count.max(1)).unwrap_or(0),
                max_ms: agg.max_ms,
            })
            .collect();
        MetricsSnapshot {
            uptime_ms,
            counters,
            durations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_accumulate() {
        let m = Metrics::new();
        m.incr("a");
        m.add("a", 2);
        m.incr("b");
        let snap = m.snapshot();
        let a = snap
            .counters
            .iter()
            .find(|c| c.name == "a")
            .expect("a present");
        assert_eq!(a.count, 3);
        let b = snap
            .counters
            .iter()
            .find(|c| c.name == "b")
            .expect("b present");
        assert_eq!(b.count, 1);
    }

    #[test]
    fn durations_track_count_total_avg_max() {
        let m = Metrics::new();
        m.observe_ms("lat", 10);
        m.observe_ms("lat", 30);
        m.observe_ms("lat", 20);
        let snap = m.snapshot();
        let d = snap
            .durations
            .iter()
            .find(|d| d.name == "lat")
            .expect("lat present");
        assert_eq!(d.count, 3);
        assert_eq!(d.total_ms, 60);
        assert_eq!(d.avg_ms, 20);
        assert_eq!(d.max_ms, 30);
    }

    #[test]
    fn snapshot_serializes_as_camel_case() {
        let m = Metrics::new();
        m.incr("recordings_started");
        let json = serde_json::to_value(m.snapshot()).unwrap();
        assert_eq!(
            json["uptimeMs"].as_u64(),
            Some(json["uptimeMs"].as_u64().unwrap())
        );
        assert_eq!(json["counters"][0]["name"], "recordings_started");
    }
}
