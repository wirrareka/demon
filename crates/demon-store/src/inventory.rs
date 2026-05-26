//! Inventory + health persistence for [`Store<R>`](crate::Store).
//!
//! Phase 1 read-side: upsert/list hosts, services, tenants, and append/query health
//! snapshots. Domain enums are stored as their canonical wire strings and decoded
//! back on read; an unrecognised string is a [`StoreError::Decode`].

use demon_core::audit::{AuditChain, AuditRecord, GENESIS_HASH};
use demon_core::{
    Fleet, HealthSnapshot, HealthStatus, Host, Region, Residency, Service, TargetKind, Tenant,
};
use sqlx::FromRow;

use crate::{Store, StoreError};

fn parse_region(s: &str) -> Result<Region, StoreError> {
    s.parse()
        .map_err(|_| StoreError::Decode(format!("invalid residency_group {s:?}")))
}

fn parse_fleet(s: &str) -> Result<Fleet, StoreError> {
    match s {
        "core" => Ok(Fleet::Core),
        "tenant" => Ok(Fleet::Tenant),
        other => Err(StoreError::Decode(format!("invalid fleet {other:?}"))),
    }
}

fn parse_status(s: &str) -> HealthStatus {
    match s {
        "up" => HealthStatus::Up,
        "degraded" => HealthStatus::Degraded,
        "down" => HealthStatus::Down,
        _ => HealthStatus::Unknown,
    }
}

fn parse_target_kind(s: &str) -> Result<TargetKind, StoreError> {
    match s {
        "host" => Ok(TargetKind::Host),
        "service" => Ok(TargetKind::Service),
        "tenant" => Ok(TargetKind::Tenant),
        other => Err(StoreError::Decode(format!("invalid target_kind {other:?}"))),
    }
}

#[derive(FromRow)]
struct HostRow {
    id: String,
    fqdn: String,
    fleet: String,
    os: String,
    residency_group: String,
    wg_ip: Option<String>,
    tenant_id: Option<String>,
    enrolled_at: i64,
    last_seen: Option<i64>,
}

impl TryFrom<HostRow> for Host {
    type Error = StoreError;
    fn try_from(r: HostRow) -> Result<Self, StoreError> {
        Ok(Host {
            id: r.id,
            fqdn: r.fqdn,
            fleet: parse_fleet(&r.fleet)?,
            os: r.os,
            residency_group: parse_region(&r.residency_group)?,
            wg_ip: r.wg_ip,
            tenant_id: r.tenant_id,
            enrolled_at: r.enrolled_at,
            last_seen: r.last_seen,
        })
    }
}

#[derive(FromRow)]
struct ServiceRow {
    id: String,
    host_id: String,
    kind: String,
    version: Option<String>,
    residency_group: String,
    desired_state: Option<String>,
    observed_state: Option<String>,
    updated_at: i64,
}

impl TryFrom<ServiceRow> for Service {
    type Error = StoreError;
    fn try_from(r: ServiceRow) -> Result<Self, StoreError> {
        Ok(Service {
            id: r.id,
            host_id: r.host_id,
            kind: r.kind,
            version: r.version,
            residency_group: parse_region(&r.residency_group)?,
            desired_state: r.desired_state,
            observed_state: r.observed_state,
            updated_at: r.updated_at,
        })
    }
}

#[derive(FromRow)]
struct TenantRow {
    id: String,
    name: String,
    residency_group: String,
    lifecycle_state: String,
    plan: Option<String>,
    created_at: i64,
}

impl TryFrom<TenantRow> for Tenant {
    type Error = StoreError;
    fn try_from(r: TenantRow) -> Result<Self, StoreError> {
        Ok(Tenant {
            id: r.id,
            name: r.name,
            residency_group: parse_region(&r.residency_group)?,
            lifecycle_state: r.lifecycle_state,
            plan: r.plan,
            created_at: r.created_at,
        })
    }
}

#[derive(FromRow)]
struct AuditRow {
    seq: i64,
    prev_hash: String,
    hash: String,
    actor: String,
    action: String,
    target: String,
    dry_run: i64,
    redacted_payload: String,
    ts: i64,
}

impl AuditRow {
    fn into_record(self) -> AuditRecord {
        AuditRecord {
            seq: u64::try_from(self.seq).unwrap_or(0),
            prev_hash: self.prev_hash,
            hash: self.hash,
            actor: self.actor,
            action: self.action,
            target: self.target,
            dry_run: self.dry_run != 0,
            redacted_payload: self.redacted_payload,
            ts: self.ts,
        }
    }
}

#[derive(FromRow)]
struct HealthRow {
    target_id: String,
    target_kind: String,
    area: String,
    status: String,
    raw_json: String,
    observed_at: i64,
}

impl TryFrom<HealthRow> for HealthSnapshot {
    type Error = StoreError;
    fn try_from(r: HealthRow) -> Result<Self, StoreError> {
        Ok(HealthSnapshot {
            target_id: r.target_id,
            target_kind: parse_target_kind(&r.target_kind)?,
            area: r.area,
            status: parse_status(&r.status),
            raw_json: r.raw_json,
            observed_at: r.observed_at,
        })
    }
}

impl<R: Residency> Store<R> {
    /// Insert or update a host. Rejects a host from another residency group.
    ///
    /// # Errors
    /// [`StoreError::ResidencyViolation`] if the host's region differs from the store,
    /// or a database error.
    pub async fn upsert_host(&self, h: &Host) -> Result<(), StoreError> {
        self.check_region(h.residency_group)?;
        sqlx::query(
            "INSERT INTO hosts (id, fqdn, fleet, os, residency_group, wg_ip, tenant_id, enrolled_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
               fqdn=excluded.fqdn, fleet=excluded.fleet, os=excluded.os,
               residency_group=excluded.residency_group, wg_ip=excluded.wg_ip,
               tenant_id=excluded.tenant_id, last_seen=excluded.last_seen",
        )
        .bind(&h.id)
        .bind(&h.fqdn)
        .bind(h.fleet.as_str())
        .bind(&h.os)
        .bind(h.residency_group.as_str())
        .bind(&h.wg_ip)
        .bind(&h.tenant_id)
        .bind(h.enrolled_at)
        .bind(h.last_seen)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// List all hosts, ordered by fqdn.
    ///
    /// # Errors
    /// Database or decode error.
    pub async fn list_hosts(&self) -> Result<Vec<Host>, StoreError> {
        let rows: Vec<HostRow> = sqlx::query_as("SELECT * FROM hosts ORDER BY fqdn")
            .fetch_all(self.pool())
            .await?;
        rows.into_iter().map(Host::try_from).collect()
    }

    /// Fetch one host by id.
    ///
    /// # Errors
    /// Database or decode error.
    pub async fn get_host(&self, id: &str) -> Result<Option<Host>, StoreError> {
        let row: Option<HostRow> = sqlx::query_as("SELECT * FROM hosts WHERE id = ?1")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        row.map(Host::try_from).transpose()
    }

    /// Insert or update a service.
    ///
    /// # Errors
    /// [`StoreError::ResidencyViolation`] on region mismatch, or a database error.
    pub async fn upsert_service(&self, s: &Service) -> Result<(), StoreError> {
        self.check_region(s.residency_group)?;
        sqlx::query(
            "INSERT INTO services (id, host_id, kind, version, residency_group, desired_state, observed_state, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               host_id=excluded.host_id, kind=excluded.kind, version=excluded.version,
               residency_group=excluded.residency_group, desired_state=excluded.desired_state,
               observed_state=excluded.observed_state, updated_at=excluded.updated_at",
        )
        .bind(&s.id)
        .bind(&s.host_id)
        .bind(&s.kind)
        .bind(&s.version)
        .bind(s.residency_group.as_str())
        .bind(&s.desired_state)
        .bind(&s.observed_state)
        .bind(s.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// List services, optionally filtered to one host.
    ///
    /// # Errors
    /// Database or decode error.
    pub async fn list_services(&self, host_id: Option<&str>) -> Result<Vec<Service>, StoreError> {
        let rows: Vec<ServiceRow> = match host_id {
            Some(h) => {
                sqlx::query_as("SELECT * FROM services WHERE host_id = ?1 ORDER BY kind")
                    .bind(h)
                    .fetch_all(self.pool())
                    .await?
            }
            None => {
                sqlx::query_as("SELECT * FROM services ORDER BY kind")
                    .fetch_all(self.pool())
                    .await?
            }
        };
        rows.into_iter().map(Service::try_from).collect()
    }

    /// Insert or update a tenant.
    ///
    /// # Errors
    /// [`StoreError::ResidencyViolation`] on region mismatch, or a database error.
    pub async fn upsert_tenant(&self, t: &Tenant) -> Result<(), StoreError> {
        self.check_region(t.residency_group)?;
        sqlx::query(
            "INSERT INTO tenants (id, name, residency_group, lifecycle_state, plan, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, residency_group=excluded.residency_group,
               lifecycle_state=excluded.lifecycle_state, plan=excluded.plan",
        )
        .bind(&t.id)
        .bind(&t.name)
        .bind(t.residency_group.as_str())
        .bind(&t.lifecycle_state)
        .bind(&t.plan)
        .bind(t.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// List all tenants, ordered by name.
    ///
    /// # Errors
    /// Database or decode error.
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, StoreError> {
        let rows: Vec<TenantRow> = sqlx::query_as("SELECT * FROM tenants ORDER BY name")
            .fetch_all(self.pool())
            .await?;
        rows.into_iter().map(Tenant::try_from).collect()
    }

    /// Append a health snapshot. Returns the new row id.
    ///
    /// # Errors
    /// Database error.
    pub async fn insert_health(&self, s: &HealthSnapshot) -> Result<i64, StoreError> {
        let res = sqlx::query(
            "INSERT INTO health_snapshots (target_id, target_kind, area, status, raw_json, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&s.target_id)
        .bind(s.target_kind.as_str())
        .bind(&s.area)
        .bind(s.status.as_str())
        .bind(&s.raw_json)
        .bind(s.observed_at)
        .execute(self.pool())
        .await?;
        Ok(res.last_insert_rowid())
    }

    /// Append a redacted, hash-chained audit record (the durable source of truth).
    /// Reads the chain head, links the new row, and inserts it atomically enough for
    /// the single-writer daemon.
    ///
    /// # Errors
    /// Database error.
    pub async fn append_audit(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        dry_run: bool,
        redacted_payload: &str,
        ts: i64,
    ) -> Result<AuditRecord, StoreError> {
        let head: Option<(i64, String)> =
            sqlx::query_as("SELECT seq, hash FROM audit ORDER BY seq DESC LIMIT 1")
                .fetch_optional(self.pool())
                .await?;
        let (seq, prev_hash) = match head {
            Some((s, h)) => (u64::try_from(s).unwrap_or(0) + 1, h),
            None => (0, GENESIS_HASH.to_owned()),
        };
        let rec = AuditChain::link(
            &prev_hash,
            seq,
            actor,
            action,
            target,
            dry_run,
            redacted_payload,
            ts,
        );
        sqlx::query(
            "INSERT INTO audit (seq, prev_hash, hash, actor, action, target, dry_run, redacted_payload, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(i64::try_from(rec.seq).unwrap_or(i64::MAX))
        .bind(&rec.prev_hash)
        .bind(&rec.hash)
        .bind(&rec.actor)
        .bind(&rec.action)
        .bind(&rec.target)
        .bind(i64::from(rec.dry_run))
        .bind(&rec.redacted_payload)
        .bind(rec.ts)
        .execute(self.pool())
        .await?;
        Ok(rec)
    }

    /// List recent audit records, newest first (durable hash chain).
    ///
    /// # Errors
    /// Database error.
    pub async fn list_audit(&self, limit: i64) -> Result<Vec<AuditRecord>, StoreError> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT seq, prev_hash, hash, actor, action, target, dry_run, redacted_payload, ts
             FROM audit ORDER BY seq DESC LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(AuditRow::into_record).collect())
    }

    /// Verify the whole durable audit chain is intact (no tamper/reorder/deletion).
    ///
    /// # Errors
    /// Database error. Returns `Ok(Err(seq))`-style via the inner result is avoided;
    /// instead returns `Ok(true)` if intact, `Ok(false)` if broken.
    pub async fn verify_audit(&self) -> Result<bool, StoreError> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT seq, prev_hash, hash, actor, action, target, dry_run, redacted_payload, ts
             FROM audit ORDER BY seq ASC",
        )
        .fetch_all(self.pool())
        .await?;
        let chain = AuditChain::from_records(rows.into_iter().map(AuditRow::into_record).collect());
        Ok(chain.verify().is_ok())
    }

    /// The latest health snapshot per area for a target (current health view).
    ///
    /// # Errors
    /// Database or decode error.
    pub async fn latest_health(&self, target_id: &str) -> Result<Vec<HealthSnapshot>, StoreError> {
        let rows: Vec<HealthRow> = sqlx::query_as(
            "SELECT h.target_id, h.target_kind, h.area, h.status, h.raw_json, h.observed_at
             FROM health_snapshots h
             JOIN (SELECT area, MAX(observed_at) AS mo FROM health_snapshots
                   WHERE target_id = ?1 GROUP BY area) m
               ON h.area = m.area AND h.observed_at = m.mo
             WHERE h.target_id = ?1
             ORDER BY h.area",
        )
        .bind(target_id)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(HealthSnapshot::try_from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::Eu;

    fn host() -> Host {
        Host {
            id: "h1".into(),
            fqdn: "core-1.eu".into(),
            fleet: Fleet::Core,
            os: "freebsd".into(),
            residency_group: Region::Eu,
            wg_ip: Some("10.200.0.5".into()),
            tenant_id: None,
            enrolled_at: 1000,
            last_seen: None,
        }
    }

    #[tokio::test]
    async fn host_roundtrip_and_upsert() {
        let s = Store::<Eu>::open_in_memory().await.unwrap();
        s.upsert_host(&host()).await.unwrap();
        let mut h2 = host();
        h2.last_seen = Some(2000);
        s.upsert_host(&h2).await.unwrap();
        let hosts = s.list_hosts().await.unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].last_seen, Some(2000));
        assert_eq!(s.get_host("h1").await.unwrap().unwrap().fqdn, "core-1.eu");
        assert!(s.get_host("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rejects_wrong_region_host() {
        let s = Store::<Eu>::open_in_memory().await.unwrap();
        let mut h = host();
        h.residency_group = Region::Uae;
        assert!(matches!(
            s.upsert_host(&h).await,
            Err(StoreError::ResidencyViolation { .. })
        ));
    }

    #[tokio::test]
    async fn audit_append_chains_and_links() {
        let s = Store::<Eu>::open_in_memory().await.unwrap();
        let r0 = s
            .append_audit("op@x", "session.open", "session:1", false, "{}", 100)
            .await
            .unwrap();
        let r1 = s
            .append_audit("op@x", "service.restart", "host:core-1", false, "ok", 200)
            .await
            .unwrap();
        assert_eq!(r0.seq, 0);
        assert_eq!(r1.seq, 1);
        assert_eq!(r1.prev_hash, r0.hash); // chained
        assert_ne!(r1.hash, r0.hash);
    }

    #[tokio::test]
    async fn latest_health_picks_most_recent_per_area() {
        let s = Store::<Eu>::open_in_memory().await.unwrap();
        for (area, status, ts) in [
            ("os", HealthStatus::Up, 100),
            ("os", HealthStatus::Down, 200),
            ("backup", HealthStatus::Degraded, 150),
        ] {
            s.insert_health(&HealthSnapshot {
                target_id: "h1".into(),
                target_kind: TargetKind::Host,
                area: area.into(),
                status,
                raw_json: "{}".into(),
                observed_at: ts,
            })
            .await
            .unwrap();
        }
        let latest = s.latest_health("h1").await.unwrap();
        assert_eq!(latest.len(), 2);
        let os = latest.iter().find(|h| h.area == "os").unwrap();
        assert_eq!(os.status, HealthStatus::Down);
        assert_eq!(os.observed_at, 200);
    }
}
