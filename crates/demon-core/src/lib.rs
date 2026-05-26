//! `demon-core` — the **pure** domain core of proximiio.demon.
//!
//! Non-negotiable: this crate performs **no I/O**. No SSH, no DB, no network, no
//! filesystem, no clock reads. Everything here is a deterministic, unit-testable
//! function of its inputs. All side effects live in driver crates (`demon-store`,
//! `demon-collect`, `demon-clients`, ...).
//!
//! It currently provides the four foundations the rest of the daemon is built on:
//! - [`residency`] — the compile-time EU/UAE air-gap invariant.
//! - [`action`] — [`ActionSpec`](action::ActionSpec) / [`ActionClass`](action::ActionClass).
//! - [`authorize`] — the single `authorize()` gate that mints a [`Capability`](action::Capability).
//! - [`audit`] — the append-only, hash-chained, redacted audit record.
#![forbid(unsafe_code)]

pub mod action;
pub mod audit;
pub mod audit_event;
pub mod auth;
pub mod authorize;
pub mod contracts;
pub mod health;
pub mod inventory;
pub mod mutation;
pub mod residency;
pub mod runbook;

pub use action::{ActionClass, ActionSpec, Capability};
pub use audit_event::{Actor, ActorKind, AuditEvent, Outcome, Target};
pub use auth::{principal_from_claims, AuthnError, Claims, FactorLevel, FactorPolicy};
pub use audit::{AuditChain, AuditRecord, GENESIS_HASH};
pub use authorize::{authorize, AuthzError, Principal, Role};
pub use contracts::{
    parse_access, parse_audit, parse_backup, parse_compliance, parse_drift, parse_fim, parse_line,
    parse_os, parse_residency, AccessStatus, AuditStatus, BackupStatus, ComplianceStatus,
    DriftStatus, FimStatus, OsFamily, OsStatus, ResidencyStatus,
};
pub use health::HealthStatus;
pub use inventory::{Fleet, HealthSnapshot, Host, Service, TargetKind, Tenant};
pub use mutation::{
    available_actions, ApprovalError, DualControl, GuardedAction, JobState, Plan,
};
pub use runbook::{RunStatus, RunbookId, RunbookRun, RunbookStep};
pub use residency::{Eu, Region, Residency, Uae};
