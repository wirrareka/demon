//! `demon-workers` — supervised background work.
//!
//! Phase 1 ships the **health poller**: on an interval it lists hosts from the store,
//! runs each read-only [`CheckArea`] over a [`Transport`], writes a
//! [`HealthSnapshot`] per host/area, and broadcasts each snapshot to subscribers (the
//! WebSocket live-state stream). A transport failure for one area degrades that area
//! to `Unknown` (recording unreachability) and never aborts the cycle.
//!
//! Schedulers, the bottleneck analyzer, and the drift reconciler arrive in later phases.
#![forbid(unsafe_code)]

use std::time::Duration;

use demon_core::{HealthSnapshot, HealthStatus, Residency, TargetKind};
use demon_collect::{collect, CheckArea, Transport};
use demon_store::{Store, StoreError};
use tokio::sync::broadcast;

/// Errors that abort a poll cycle (per-area transport failures do not).
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// A store operation failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Capacity of the live-state broadcast channel.
pub const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Current wall-clock time in epoch milliseconds.
#[must_use]
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(i64::MAX)
}

fn error_json(msg: &str) -> String {
    // Value::String renders a correctly-escaped JSON string.
    format!(
        "{{\"error\":{}}}",
        serde_json::Value::String(msg.to_owned())
    )
}

/// Run one poll cycle over all hosts in `store`. Returns the number of health
/// snapshots written. Broadcasts each snapshot if `events` is provided.
///
/// # Errors
/// Returns [`WorkerError`] if a store read/write fails. Per-area transport failures
/// are recorded as `Unknown` snapshots, not propagated.
pub async fn poll_once<T, R>(
    store: &Store<R>,
    transport: &T,
    events: Option<&broadcast::Sender<HealthSnapshot>>,
    now_ms: i64,
) -> Result<usize, WorkerError>
where
    T: Transport,
    R: Residency,
{
    let hosts = store.list_hosts().await?;
    let mut written = 0usize;
    for host in &hosts {
        let addr = host.wg_ip.clone().unwrap_or_else(|| host.fqdn.clone());
        for area in CheckArea::ALL {
            let (status, raw_json) = match collect(transport, &addr, area).await {
                Ok(c) => (c.status, c.raw_json),
                Err(e) => {
                    tracing::warn!(host = %host.id, area = area.as_str(), error = %e, "collect failed");
                    (HealthStatus::Unknown, error_json(&e.to_string()))
                }
            };
            let snapshot = HealthSnapshot {
                target_id: host.id.clone(),
                target_kind: TargetKind::Host,
                area: area.as_str().to_owned(),
                status,
                raw_json,
                observed_at: now_ms,
            };
            store.insert_health(&snapshot).await?;
            if let Some(tx) = events {
                // A send error just means nobody is listening; that is fine.
                let _ = tx.send(snapshot);
            }
            written += 1;
        }
    }
    Ok(written)
}

/// Run the poller forever, ticking every `interval`. Logs and continues on cycle
/// errors so a transient store/transport problem never kills the daemon's eyes.
pub async fn run<T, R>(
    store: Store<R>,
    transport: T,
    events: broadcast::Sender<HealthSnapshot>,
    interval: Duration,
) where
    T: Transport,
    R: Residency,
{
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        match poll_once(&store, &transport, Some(&events), now_ms()).await {
            Ok(n) => tracing::debug!(snapshots = n, "poll cycle complete"),
            Err(e) => tracing::error!(error = %e, "poll cycle failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_collect::MockTransport;
    use demon_core::{Eu, Fleet, Host, Region};
    use std::collections::HashMap;

    fn mock() -> MockTransport {
        let mut responses = HashMap::new();
        responses.insert(
            "check-os.sh".to_owned(),
            "OS\thost=core-1\tfamily=freebsd\tid=freebsd\tversion=14.1\tpkg=pkg\tservice=rc\tfirewall=pf\tcontainer=jail".to_owned(),
        );
        responses.insert(
            "check-backup.sh".to_owned(),
            "BACKUP\thost=core-1\tstores=2\tworst_age_hours=3\tverdict=ok".to_owned(),
        );
        responses.insert(
            "check-fim.sh".to_owned(),
            "FIM\thost=core-1\tlast_verify=1700\tdrift=2\tpkg_mismatch=0\tbaseline=present".to_owned(),
        );
        MockTransport { responses }
    }

    async fn seeded_store() -> Store<Eu> {
        let s = Store::<Eu>::open_in_memory().await.unwrap();
        s.upsert_host(&Host {
            id: "h1".into(),
            fqdn: "core-1.eu".into(),
            fleet: Fleet::Core,
            os: "freebsd".into(),
            residency_group: Region::Eu,
            wg_ip: Some("10.200.0.5".into()),
            tenant_id: None,
            enrolled_at: 1,
            last_seen: None,
        })
        .await
        .unwrap();
        s
    }

    #[tokio::test]
    async fn poll_once_writes_all_areas() {
        let store = seeded_store().await;
        let n = poll_once(&store, &mock(), None, 5000).await.unwrap();
        assert_eq!(n, 3);
        let latest = store.latest_health("h1").await.unwrap();
        assert_eq!(latest.len(), 3);
        let fim = latest.iter().find(|h| h.area == "fim").unwrap();
        assert_eq!(fim.status, HealthStatus::Degraded); // drift=2
        let backup = latest.iter().find(|h| h.area == "backup").unwrap();
        assert_eq!(backup.status, HealthStatus::Up);
    }

    #[tokio::test]
    async fn poll_once_broadcasts_snapshots() {
        let store = seeded_store().await;
        let (tx, mut rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let n = poll_once(&store, &mock(), Some(&tx), 5000).await.unwrap();
        assert_eq!(n, 3);
        let mut received = 0;
        while rx.try_recv().is_ok() {
            received += 1;
        }
        assert_eq!(received, 3);
    }

    #[tokio::test]
    async fn transport_failure_records_unknown() {
        let store = seeded_store().await;
        let empty = MockTransport::default(); // no canned responses -> NoMock errors
        let n = poll_once(&store, &empty, None, 5000).await.unwrap();
        assert_eq!(n, 3);
        let latest = store.latest_health("h1").await.unwrap();
        assert!(latest.iter().all(|h| h.status == HealthStatus::Unknown));
    }
}
