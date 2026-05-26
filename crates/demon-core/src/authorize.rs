//! The single authorization gate.
//!
//! `authorize(principal, spec) -> Result<Capability, AuthzError>` is the *only* path
//! to a [`Capability`]. It is pure: a deterministic function of the principal's roles
//! and the action class. Residency is enforced separately by the type system
//! (`Store<R>`); this gate additionally hard-denies any [`ActionClass::CrossResidency`]
//! as a runtime backstop.
//!
//! Phase 0 policy is intentionally simple (role → permitted classes). Per-action
//! policy, dual-control, and step-up factors are layered on in later phases.

use crate::action::{ActionClass, ActionSpec, Capability};
use crate::residency::Region;

/// Operator roles (security doc §2.1), least- to most-privileged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Read-only observability.
    Viewer,
    /// Day-to-day reversible mutations.
    Operator,
    /// Destructive ops, secret access, CA use.
    Senior,
    /// Emergency override; everything, extra-audited.
    BreakGlass,
}

impl Role {
    /// Whether this role may perform the given action class.
    #[must_use]
    pub const fn permits(self, class: ActionClass) -> bool {
        match self {
            Role::Viewer => matches!(class, ActionClass::ReadOnly),
            Role::Operator => matches!(class, ActionClass::ReadOnly | ActionClass::Mutating),
            Role::Senior => matches!(
                class,
                ActionClass::ReadOnly
                    | ActionClass::Mutating
                    | ActionClass::Destructive
                    | ActionClass::SecretAccess
                    | ActionClass::CaUse
            ),
            // Even break-glass cannot cross residency — that is structural.
            Role::BreakGlass => !matches!(class, ActionClass::CrossResidency),
        }
    }
}

/// An authenticated operator, mapped from the OIDC token's claims.
#[derive(Debug, Clone)]
pub struct Principal {
    /// OIDC `sub`.
    pub sub: String,
    /// Roles mapped from the `roles` claim.
    pub roles: Vec<Role>,
    /// The residency group the principal is acting in (`residency_group` claim).
    pub residency: Region,
}

impl Principal {
    /// Construct a principal.
    #[must_use]
    pub fn new(sub: impl Into<String>, roles: Vec<Role>, residency: Region) -> Self {
        Self {
            sub: sub.into(),
            roles,
            residency,
        }
    }
}

/// Why an authorization was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthzError {
    /// No role held by the principal permits the action's class.
    #[error("principal {sub:?} (roles {roles:?}) may not perform a {class:?} action")]
    Forbidden {
        /// The principal's `sub`.
        sub: String,
        /// The roles considered.
        roles: Vec<Role>,
        /// The class that was denied.
        class: ActionClass,
    },
    /// A cross-residency action was requested — always denied.
    #[error("cross-residency action is structurally forbidden")]
    CrossResidency,
}

/// The central authorization gate. Returns a [`Capability`] iff some role held by the
/// principal permits the action's class.
///
/// # Errors
/// Returns [`AuthzError::CrossResidency`] for any cross-residency action, or
/// [`AuthzError::Forbidden`] when no role permits the action class.
pub fn authorize(principal: &Principal, spec: ActionSpec) -> Result<Capability, AuthzError> {
    if spec.class == ActionClass::CrossResidency {
        return Err(AuthzError::CrossResidency);
    }
    if principal.roles.iter().any(|r| r.permits(spec.class)) {
        Ok(Capability::mint(spec, principal.sub.clone()))
    } else {
        Err(AuthzError::Forbidden {
            sub: principal.sub.clone(),
            roles: principal.roles.clone(),
            class: spec.class,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(class: ActionClass) -> ActionSpec {
        ActionSpec::new("test.action", "host:1", class)
    }

    #[test]
    fn viewer_can_read_but_not_mutate() {
        let p = Principal::new("viewer@x", vec![Role::Viewer], Region::Eu);
        assert!(authorize(&p, spec(ActionClass::ReadOnly)).is_ok());
        assert_eq!(
            authorize(&p, spec(ActionClass::Mutating)).unwrap_err(),
            AuthzError::Forbidden {
                sub: "viewer@x".into(),
                roles: vec![Role::Viewer],
                class: ActionClass::Mutating,
            }
        );
    }

    #[test]
    fn senior_can_destruct_operator_cannot() {
        let op = Principal::new("op@x", vec![Role::Operator], Region::Eu);
        let sr = Principal::new("sr@x", vec![Role::Senior], Region::Uae);
        assert!(authorize(&op, spec(ActionClass::Destructive)).is_err());
        assert!(authorize(&sr, spec(ActionClass::Destructive)).is_ok());
        assert!(authorize(&sr, spec(ActionClass::CaUse)).is_ok());
    }

    #[test]
    fn cross_residency_denied_even_for_break_glass() {
        let bg = Principal::new("root@x", vec![Role::BreakGlass], Region::Eu);
        assert_eq!(
            authorize(&bg, spec(ActionClass::CrossResidency)).unwrap_err(),
            AuthzError::CrossResidency
        );
    }

    #[test]
    fn capability_records_grantee_and_spec() {
        let p = Principal::new("op@x", vec![Role::Operator], Region::Eu);
        let cap = authorize(&p, spec(ActionClass::Mutating)).unwrap();
        assert_eq!(cap.granted_to(), "op@x");
        assert_eq!(cap.spec().class, ActionClass::Mutating);
    }
}
