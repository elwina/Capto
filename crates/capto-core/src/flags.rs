//! Declarative feature-flag registry for Capto.
//!
//! Capto is local-first, so flags are read from the local `settings.json`
//! (`AppSettings::enabled_flags` / `disabled_flags`) rather than a remote
//! service. The single source of truth for flag identifiers lives in this
//! module and is mirrored by `scripts/scan-dead-flags.ps1`, which fails CI if
//! a declared flag is never referenced by runtime code (dead feature-flag
//! detection). See `docs/feature-flags.md` for the lifecycle.

use crate::settings::AppSettings;

/// A declared, documented feature flag.
pub struct FeatureFlag {
    pub name: &'static str,
    pub description: &'static str,
    pub default: bool,
}

/// Local `/v1/metrics` snapshots on the control plane.
pub const CONTROL_PLANE_METRICS: &str = "control-plane-metrics";
/// Local crash reports (`crash-*.json`) written on panic.
pub const CRASH_REPORTING: &str = "crash-reporting";

/// Registry of every declared flag. Names must be unique.
pub fn all() -> &'static [FeatureFlag] {
    &[
        FeatureFlag {
            name: CONTROL_PLANE_METRICS,
            description: "Expose local metrics snapshots at /v1/metrics (localhost, auth required)",
            default: true,
        },
        FeatureFlag {
            name: CRASH_REPORTING,
            description: "Write a structured crash-*.json report to the config dir on panic",
            default: true,
        },
    ]
}

/// Resolve whether `name` is currently enabled for `settings`, with
/// explicit lists beating defaults: `disabled_flags` wins over
/// `enabled_flags` wins over the declared default.
pub fn is_enabled(settings: &AppSettings, name: &str) -> bool {
    if settings.disabled_flags.iter().any(|f| f == name) {
        return false;
    }
    if settings.enabled_flags.iter().any(|f| f == name) {
        return true;
    }
    all()
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.default)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_unset() {
        let s = AppSettings::default();
        assert!(is_enabled(&s, CONTROL_PLANE_METRICS));
        assert!(is_enabled(&s, CRASH_REPORTING));
        // Unknown flags use a safe default of false.
        assert!(!is_enabled(&s, "no-such-flag"));
    }

    #[test]
    fn explicit_lists_override_defaults() {
        let mut s = AppSettings::default();
        s.disabled_flags.push(CONTROL_PLANE_METRICS.to_string());
        assert!(!is_enabled(&s, CONTROL_PLANE_METRICS));

        let mut s2 = AppSettings::default();
        s2.disabled_flags.push(CRASH_REPORTING.to_string());
        assert!(is_enabled(&s2, CONTROL_PLANE_METRICS));
    }

    #[test]
    fn registry_has_unique_names() {
        let names: Vec<&str> = all().iter().map(|f| f.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "flag names must be unique");
    }

    #[test]
    fn disabled_wins_over_enabled() {
        let mut s = AppSettings::default();
        s.enabled_flags.push(CRASH_REPORTING.to_string());
        s.disabled_flags.push(CRASH_REPORTING.to_string());
        assert!(!is_enabled(&s, CRASH_REPORTING));
    }
}
