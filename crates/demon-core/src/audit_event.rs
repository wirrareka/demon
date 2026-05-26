//! The canonical **B3 audit-event** (`coordination/conventions/audit-event-schema.md`)
//! — the one document every service ships to its group-local OpenSearch. demon emits
//! with `source: "control-plane"` (it is the next-gen of the admin TUI's `audit_emit`).
//!
//! This is the *fan-out* copy; the durable, hash-chained source of truth is
//! [`crate::audit::AuditChain`]. Pure: the caller supplies the RFC3339 timestamp and
//! pre-redacted detail (redaction happens at the emitter — never raw secrets).

use serde::{Deserialize, Serialize};

use crate::residency::Region;

/// Schema version of the B3 document.
pub const SCHEMA_VERSION: u32 = 1;
/// demon's `source` value.
pub const SOURCE_CONTROL_PLANE: &str = "control-plane";

/// What kind of principal acted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    /// A human operator.
    User,
    /// A service/API token principal.
    ApiToken,
    /// The daemon itself (scheduler, reconciler).
    System,
    /// An explicit dev auth bypass (must be loud + rare).
    DevBypass,
}

/// The acting principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// Human-readable label.
    pub label: String,
    /// Principal kind.
    pub kind: ActorKind,
    /// Stable id (OIDC `sub`, token id, ...).
    pub id: Option<String>,
    /// Tenant/org, if scoped.
    pub tenant: Option<String>,
}

impl Actor {
    /// A human operator actor from an OIDC subject.
    #[must_use]
    pub fn user(sub: impl Into<String>, tenant: Option<String>) -> Self {
        let sub = sub.into();
        Self {
            label: sub.clone(),
            kind: ActorKind::User,
            id: Some(sub),
            tenant,
        }
    }

    /// The daemon acting on its own (no human).
    #[must_use]
    pub fn system() -> Self {
        Self {
            label: "demon".into(),
            kind: ActorKind::System,
            id: None,
            tenant: None,
        }
    }
}

/// The thing acted upon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// Target kind (`host`, `service`, `session`, `api`, ...).
    pub kind: String,
    /// Target id, if any.
    pub id: Option<String>,
}

impl Target {
    /// Build a target.
    #[must_use]
    pub fn new(kind: impl Into<String>, id: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            id,
        }
    }
}

/// Action result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Failure,
}

/// A canonical B3 audit-event document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// RFC3339 millis UTC. Supplied by the caller (keeps this module pure).
    pub ts: String,
    /// Schema version (always [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// Emitter (`control-plane` for demon).
    pub source: String,
    /// Emitting node hostname.
    pub node: String,
    /// Residency group (`eu`/`uae`).
    pub residency_group: Region,
    /// Acting principal.
    pub actor: Actor,
    /// `<noun>.<verb>` action.
    pub action: String,
    /// Target.
    pub target: Target,
    /// Outcome.
    pub outcome: Outcome,
    /// Optional, emitter-redacted detail (`{before, after}`). NEVER raw secrets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_detail: Option<serde_json::Value>,
    /// Correlation id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AuditEvent {
    /// Build a `control-plane` event.
    #[must_use]
    pub fn control_plane(
        ts: impl Into<String>,
        node: impl Into<String>,
        residency_group: Region,
        actor: Actor,
        action: impl Into<String>,
        target: Target,
        outcome: Outcome,
    ) -> Self {
        Self {
            ts: ts.into(),
            schema_version: SCHEMA_VERSION,
            source: SOURCE_CONTROL_PLANE.to_owned(),
            node: node.into(),
            residency_group,
            actor,
            action: action.into(),
            target,
            outcome,
            redacted_detail: None,
            request_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_b3_shape() {
        let ev = AuditEvent::control_plane(
            "2026-05-26T12:00:00.000Z",
            "demon-eu-1",
            Region::Eu,
            Actor::user("op@x", Some("00000000-0000-4000-8000-000000000000".into())),
            "session.open",
            Target::new("session", None),
            Outcome::Success,
        );
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["source"], "control-plane");
        assert_eq!(v["residency_group"], "eu");
        assert_eq!(v["actor"]["kind"], "user");
        assert_eq!(v["outcome"], "success");
        // optional fields omitted when None
        assert!(v.get("redacted_detail").is_none());
        assert!(v.get("request_id").is_none());
    }

    #[test]
    fn system_actor_has_no_id() {
        let a = Actor::system();
        assert_eq!(a.kind, ActorKind::System);
        assert!(a.id.is_none());
    }
}
