//! Resilience for the CLI -> desktop control-plane channel.
//!
//! The control plane is a localhost HTTP server owned by the desktop app;
//! when the app is quitting/starting the plane can flap. As defensive
//! programming (and to keep agents' `capto` calls from hammering a dead
//! socket), `CircuitBreaker` opens after repeated failures, fails fast while
//! open, and half-opens to probe the transport again after a cooldown.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct BreakerConfig {
    /// Consecutive failures before the breaker opens.
    pub max_failures: u32,
    /// How long the breaker stays open before allowing a probe.
    pub cooldown: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            max_failures: 3,
            cooldown: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Open,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    cfg: BreakerConfig,
    state: State,
    consecutive_failures: u32,
    last_failure: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(cfg: BreakerConfig) -> Self {
        Self {
            cfg,
            state: State::Closed,
            consecutive_failures: 0,
            last_failure: None,
        }
    }

    /// Whether a call may be attempted at `now`. While closed this is always
    /// true; while open it becomes true again once the cooldown has elapsed
    /// ("half-open": the next call is a probe).
    pub fn allow(&self, now: Instant) -> bool {
        match self.state {
            State::Closed => true,
            State::Open => self
                .last_failure
                .map(|since| now.duration_since(since) >= self.cfg.cooldown)
                .unwrap_or(true),
        }
    }

    /// Record a successful call; resets the breaker to closed.
    pub fn on_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure = None;
        self.state = State::Closed;
    }

    /// Record a failed call at `now`. The breaker opens once the failure
    /// budget is exhausted; once open it stays open (a half-open probe that
    /// fails simply resets the cooldown timer).
    pub fn on_failure(&mut self, now: Instant) {
        self.consecutive_failures += 1;
        self.last_failure = Some(now);
        if self.state == State::Closed && self.consecutive_failures >= self.cfg.max_failures {
            self.state = State::Open;
        }
    }

    #[cfg(test)]
    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }
}

/// Exponential backoff schedule for retries: 250ms, 500ms, 1s, ...
pub fn backoff_delays(max_retries: u32) -> Vec<Duration> {
    (0..max_retries)
        .map(|i| Duration::from_millis(250 * 2_u64.pow(i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ten_ms_ago() -> Instant {
        Instant::now() - Duration::from_millis(10)
    }

    #[test]
    fn opens_after_max_failures() {
        let mut b = CircuitBreaker::new(BreakerConfig {
            max_failures: 2,
            cooldown: Duration::from_secs(5),
        });
        assert!(!b.is_open());
        b.on_failure(Instant::now());
        assert!(!b.is_open(), "one failure below budget");
        b.on_failure(Instant::now());
        assert!(b.is_open(), "budget exhausted opens breaker");
        assert!(!b.allow(Instant::now()), "fails fast while open");
    }

    #[test]
    fn reopens_after_cooldown_via_probe() {
        let mut b = CircuitBreaker::new(BreakerConfig {
            max_failures: 1,
            cooldown: Duration::from_millis(50),
        });
        b.on_failure(Instant::now());
        assert!(b.is_open());
        // Not yet cooled down -> still refused.
        assert!(!b.allow(Instant::now()));
        // After cooldown we probe (half-open).
        let after = Instant::now() + Duration::from_millis(100);
        assert!(b.allow(after));
        // A failed probe re-opens.
        b.on_failure(after);
        assert!(b.is_open());
        // A successful probe resets to closed.
        let after2 = Instant::now() + Duration::from_millis(100);
        b.on_success();
        assert!(!b.is_open());
        assert!(b.allow(after2), "closed breaker allows calls");
    }

    #[test]
    fn success_resets_failure_count() {
        let mut b = CircuitBreaker::new(BreakerConfig {
            max_failures: 3,
            cooldown: Duration::from_secs(5),
        });
        b.on_failure(Instant::now());
        b.on_failure(ten_ms_ago());
        b.on_success();
        b.on_failure(Instant::now());
        b.on_failure(Instant::now());
        assert!(!b.is_open(), "two failures after reset stay below budget");
    }

    #[test]
    fn backoff_delays_are_exponential() {
        let d = backoff_delays(3);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0], Duration::from_millis(250));
        assert_eq!(d[1], Duration::from_millis(500));
        assert_eq!(d[2], Duration::from_millis(1000));
        assert_eq!(backoff_delays(0).len(), 0);
    }
}
