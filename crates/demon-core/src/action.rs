//! Actions and capabilities.
//!
//! Every mutation in the daemon is described by an [`ActionSpec`] and classified by
//! an [`ActionClass`]. The *only* way to obtain a [`Capability`] — the unforgeable
//! proof that an action is permitted — is through
//! [`authorize()`](crate::authorize::authorize). `Capability` has no public
//! constructor, so a caller cannot fabricate one: possessing it is proof the central
//! gate said yes.

use serde::{Deserialize, Serialize};

/// How dangerous an action is. Drives the authz policy and (later) the
/// `factor_policy` step-up requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionClass {
    /// Pure observation; no state change. The `check-*.sh` read path.
    ReadOnly,
    /// A reversible state change (restart a service, edit non-destructive config).
    Mutating,
    /// Hard to reverse / data-affecting (decommission, purge, mass token revoke).
    Destructive,
    /// Reads or brokers a secret.
    SecretAccess,
    /// Uses a signing CA (SSH cert issuance, PKI).
    CaUse,
    /// Touches more than one residency group. Should be structurally impossible;
    /// kept as a class so the gate can hard-deny it.
    CrossResidency,
}

impl ActionClass {
    /// `true` for classes that require two-person dual-control by default
    /// (security doc §2.3).
    #[must_use]
    pub const fn requires_dual_control(self) -> bool {
        matches!(
            self,
            ActionClass::Destructive | ActionClass::CaUse | ActionClass::CrossResidency
        )
    }
}

/// A fully-described intended action. Built by callers, validated by the gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSpec {
    /// Stable action identifier, e.g. `"service.restart"`.
    pub action: String,
    /// The target the action applies to, e.g. `"host:abc/service:opensearch"`.
    pub target: String,
    /// Danger classification.
    pub class: ActionClass,
}

impl ActionSpec {
    /// Construct an action spec.
    #[must_use]
    pub fn new(action: impl Into<String>, target: impl Into<String>, class: ActionClass) -> Self {
        Self {
            action: action.into(),
            target: target.into(),
            class,
        }
    }
}

/// Proof that an [`ActionSpec`] was authorized for a principal.
///
/// No public constructor — only [`authorize()`](crate::authorize::authorize) (in
/// this crate) can mint one. Downstream code requires a `&Capability` to execute a
/// mutation, so the type system enforces "no action without authorization".
#[derive(Debug, Clone)]
pub struct Capability {
    spec: ActionSpec,
    granted_to: String,
}

impl Capability {
    /// Crate-internal mint point. Intentionally not `pub`.
    pub(crate) fn mint(spec: ActionSpec, granted_to: String) -> Self {
        Self { spec, granted_to }
    }

    /// The action this capability authorizes.
    #[must_use]
    pub fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    /// The principal (`sub`) the capability was granted to.
    #[must_use]
    pub fn granted_to(&self) -> &str {
        &self.granted_to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_control_classes() {
        assert!(ActionClass::Destructive.requires_dual_control());
        assert!(ActionClass::CaUse.requires_dual_control());
        assert!(ActionClass::CrossResidency.requires_dual_control());
        assert!(!ActionClass::ReadOnly.requires_dual_control());
        assert!(!ActionClass::Mutating.requires_dual_control());
        assert!(!ActionClass::SecretAccess.requires_dual_control());
    }
}
