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
pub mod authorize;
pub mod residency;

pub use action::{ActionClass, ActionSpec, Capability};
pub use audit::{AuditChain, AuditRecord, GENESIS_HASH};
pub use authorize::{authorize, AuthzError, Principal, Role};
pub use residency::{Eu, Region, Residency, Uae};
