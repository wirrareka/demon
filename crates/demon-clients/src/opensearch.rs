//! OpenSearch audit shipper — the **fan-out** copy of demon's audit trail.
//!
//! Posts canonical B3 [`AuditEvent`]s to the group-local OpenSearch index
//! `audit-events-{residency_group}-YYYY.MM` (per
//! `coordination/conventions/audit-event-schema.md`). **Best-effort**: a ship failure
//! must never break the user-visible action — the durable hash-chained store is the
//! source of truth. Residency: a shipper instance only ever talks to its own group's
//! cluster (the URL is per-group), so events cannot cross the air-gap.

use demon_core::{AuditEvent, Region};

/// Error shipping an audit event (logged, never propagated to break an action).
#[derive(Debug, thiserror::Error)]
pub enum AuditShipError {
    /// HTTP/transport error.
    #[error("audit ship http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Compute the monthly index name from a residency group and an RFC3339 timestamp.
/// Falls back to `unknown` month if the timestamp is malformed (still indexable).
#[must_use]
pub fn index_for(region: Region, ts_rfc3339: &str) -> String {
    // RFC3339 is `YYYY-MM-DD...`; take year + month.
    let ym = if ts_rfc3339.len() >= 7 && ts_rfc3339.is_char_boundary(7) {
        let year = &ts_rfc3339[0..4];
        let month = &ts_rfc3339[5..7];
        format!("{year}.{month}")
    } else {
        "unknown".to_owned()
    };
    format!("audit-events-{}-{ym}", region.as_str())
}

/// Group-local OpenSearch audit shipper.
#[derive(Debug, Clone)]
pub struct OpenSearchAudit {
    base: String,
    http: reqwest::Client,
}

impl OpenSearchAudit {
    /// Build a shipper for a group-local cluster base URL (e.g. `https://10.200.0.9:9200`).
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Ship one event. Best-effort; the caller logs and ignores errors.
    ///
    /// # Errors
    /// [`AuditShipError`] on transport/HTTP failure.
    pub async fn ship(&self, event: &AuditEvent) -> Result<(), AuditShipError> {
        let index = index_for(event.residency_group, &event.ts);
        self.http
            .post(format!("{}/{index}/_doc", self.base))
            .json(event)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_name_per_group_and_month() {
        assert_eq!(
            index_for(Region::Eu, "2026-05-26T12:00:00.000Z"),
            "audit-events-eu-2026.05"
        );
        assert_eq!(
            index_for(Region::Uae, "2026-12-01T00:00:00Z"),
            "audit-events-uae-2026.12"
        );
    }

    #[test]
    fn malformed_ts_falls_back() {
        assert_eq!(index_for(Region::Eu, "x"), "audit-events-eu-unknown");
    }
}
