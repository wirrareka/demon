//! Health status — the canonical rollup used across the dashboard, the WS stream,
//! and the web UI's `lib/health.ts`.

use serde::{Deserialize, Serialize};

/// The health of a target (host, service, ...) for a given check area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Healthy.
    Up,
    /// Working but impaired (warnings, partial failures).
    Degraded,
    /// Failed / unreachable.
    Down,
    /// Not yet observed, or the contract could not be parsed.
    #[default]
    Unknown,
}

impl HealthStatus {
    /// Canonical lowercase wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            HealthStatus::Up => "up",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Down => "down",
            HealthStatus::Unknown => "unknown",
        }
    }

    /// Worst (most severe) of two statuses, for rolling many areas into one.
    /// Severity order: `Down` > `Degraded` > `Unknown` > `Up`.
    #[must_use]
    pub fn worst(self, other: HealthStatus) -> HealthStatus {
        self.max_by_severity(other)
    }

    const fn severity(self) -> u8 {
        match self {
            HealthStatus::Up => 0,
            HealthStatus::Unknown => 1,
            HealthStatus::Degraded => 2,
            HealthStatus::Down => 3,
        }
    }

    fn max_by_severity(self, other: HealthStatus) -> HealthStatus {
        if other.severity() > self.severity() {
            other
        } else {
            self
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::HealthStatus::{Degraded, Down, Unknown, Up};

    #[test]
    fn worst_picks_most_severe() {
        assert_eq!(Up.worst(Down), Down);
        assert_eq!(Degraded.worst(Up), Degraded);
        assert_eq!(Unknown.worst(Up), Unknown);
        assert_eq!(Down.worst(Degraded), Down);
        assert_eq!(Up.worst(Up), Up);
    }

    #[test]
    fn wire_strings() {
        assert_eq!(Up.as_str(), "up");
        assert_eq!(Degraded.to_string(), "degraded");
    }
}
