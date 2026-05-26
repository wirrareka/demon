//! Inventory + health view models — the read-side domain the daemon serves over the
//! API. Pure data; persistence lives in `demon-store`, collection in `demon-collect`.

use serde::{Deserialize, Serialize};

use crate::health::HealthStatus;
use crate::residency::Region;

/// Which fleet a host belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fleet {
    /// FreeBSD core infrastructure.
    Core,
    /// Linux per-tenant enterprise install.
    Tenant,
}

impl Fleet {
    /// Canonical wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Fleet::Core => "core",
            Fleet::Tenant => "tenant",
        }
    }
}

/// A managed host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    /// Stable host id.
    pub id: String,
    /// Fully-qualified domain name.
    pub fqdn: String,
    /// Fleet membership.
    pub fleet: Fleet,
    /// OS family/id string as reported by `check-os.sh`.
    pub os: String,
    /// Residency group (matches the daemon's `Store<R>`).
    pub residency_group: Region,
    /// WireGuard interface address, if enrolled.
    pub wg_ip: Option<String>,
    /// Owning tenant (only for the tenant fleet).
    pub tenant_id: Option<String>,
    /// Enrolment time (epoch ms).
    pub enrolled_at: i64,
    /// Last successful poll (epoch ms), if ever seen.
    pub last_seen: Option<i64>,
}

/// A service running on a host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Service {
    /// Stable service id.
    pub id: String,
    /// Host it runs on.
    pub host_id: String,
    /// Service kind (`opensearch`, `kalista`, `vulture`, ...).
    pub kind: String,
    /// Observed version, if known.
    pub version: Option<String>,
    /// Residency group.
    pub residency_group: Region,
    /// Desired state (reconciler input), if set.
    pub desired_state: Option<String>,
    /// Observed state, if known.
    pub observed_state: Option<String>,
    /// Last update (epoch ms).
    pub updated_at: i64,
}

/// A per-customer tenant (enterprise Linux install).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    /// Tenant id = Vulture `organization_id` (UUID v4).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Residency group.
    pub residency_group: Region,
    /// Lifecycle state (`provisioning`, `active`, `decommissioning`, ...).
    pub lifecycle_state: String,
    /// Plan, if any.
    pub plan: Option<String>,
    /// Creation time (epoch ms).
    pub created_at: i64,
}

/// The kind of thing a health snapshot is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Host,
    Service,
    Tenant,
}

impl TargetKind {
    /// Canonical wire string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TargetKind::Host => "host",
            TargetKind::Service => "service",
            TargetKind::Tenant => "tenant",
        }
    }
}

/// One health observation for a target/area at a point in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Target this is about.
    pub target_id: String,
    /// Target kind.
    pub target_kind: TargetKind,
    /// Check area (`os`, `backup`, `fim`, `access`, ...).
    pub area: String,
    /// Rolled-up status.
    pub status: HealthStatus,
    /// Raw JSON of the parsed contract (for drill-down).
    pub raw_json: String,
    /// Observation time (epoch ms).
    pub observed_at: i64,
}
