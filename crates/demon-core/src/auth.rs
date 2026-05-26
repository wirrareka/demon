//! Authentication domain (pure): the verified OIDC claim set demon relies on, the
//! claims→[`Principal`] mapping (with the residency backstop), and the step-up
//! [`FactorLevel`] / [`FactorPolicy`].
//!
//! Token *verification* (signature, JWKS, issuer/audience) is I/O and lives in
//! `demon-clients`; this module only interprets an already-verified claim set.
//!
//! Per the identity contract (`coordination/conventions/jwt-claims.md`, answered
//! 2026-05-26): claims carry `sub / tenant_id / residency_group / roles / scope` —
//! **no `acr`/`amr`** — so demon owns its own per-op step-up via [`FactorPolicy`].

use serde::{Deserialize, Serialize};

use crate::action::ActionClass;
use crate::authorize::{Principal, Role};
use crate::residency::Region;

/// The verified access-token claims demon consumes (subset of the minted set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the operator's stable id.
    pub sub: String,
    /// Tenant/org id (Vulture `organization_id`, UUID v4). Present for all tokens.
    #[serde(default)]
    pub tenant_id: String,
    /// Residency group the token is scoped to.
    pub residency_group: Region,
    /// Role strings (free-form; mapped via [`Role::from_claim`]).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Space-delimited scopes.
    #[serde(default)]
    pub scope: String,
}

/// Why authentication mapping failed (after signature/issuer verification succeeded).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthnError {
    /// The token's residency group does not match the daemon's. Hard backstop on the
    /// compile-time residency invariant for data crossing the auth boundary.
    #[error("token residency {token} does not match daemon residency {daemon}")]
    ResidencyMismatch {
        /// Region asserted by the token.
        token: Region,
        /// Region this daemon serves.
        daemon: Region,
    },
    /// The token granted no role demon recognises.
    #[error("token grants no recognised role (roles claim: {0:?})")]
    NoRecognisedRole(Vec<String>),
}

/// Map a verified claim set to a [`Principal`], enforcing the residency backstop.
///
/// # Errors
/// [`AuthnError::ResidencyMismatch`] if `claims.residency_group != daemon`, or
/// [`AuthnError::NoRecognisedRole`] if no role string maps to a known [`Role`].
pub fn principal_from_claims(claims: &Claims, daemon: Region) -> Result<Principal, AuthnError> {
    if claims.residency_group != daemon {
        return Err(AuthnError::ResidencyMismatch {
            token: claims.residency_group,
            daemon,
        });
    }
    let roles: Vec<Role> = claims
        .roles
        .iter()
        .filter_map(|r| Role::from_claim(r))
        .collect();
    if roles.is_empty() {
        return Err(AuthnError::NoRecognisedRole(claims.roles.clone()));
    }
    Ok(Principal::new(
        claims.sub.clone(),
        roles,
        claims.residency_group,
    ))
}

/// Strength of a second-factor assertion, ascending. `Ord` ⇒ a stronger factor
/// satisfies a weaker requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorLevel {
    /// No step-up performed.
    None,
    /// TOTP (bootstrap only; disabled for senior/break-glass in prod).
    Totp,
    /// Platform WebAuthn (Touch ID / Windows Hello) — available day one.
    WebAuthnPlatform,
    /// Roaming WebAuthn / FIDO2 security key (the YubiKey slot) — strongest.
    WebAuthnRoaming,
}

/// Policy mapping action classes to the step-up factor demon requires.
///
/// Policy is **data, not code**: the interim default reflects "YubiKeys not yet in
/// hand" — destructive/secret/CA actions require a fresh platform WebAuthn assertion;
/// when roaming hardware is enforced, bump those to [`FactorLevel::WebAuthnRoaming`]
/// without touching call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorPolicy {
    destructive: FactorLevel,
    secret: FactorLevel,
    ca: FactorLevel,
}

impl FactorPolicy {
    /// The interim policy: platform WebAuthn for the dangerous classes (hardware off).
    #[must_use]
    pub const fn interim() -> Self {
        Self {
            destructive: FactorLevel::WebAuthnPlatform,
            secret: FactorLevel::WebAuthnPlatform,
            ca: FactorLevel::WebAuthnPlatform,
        }
    }

    /// The hardened policy: roaming hardware for the dangerous classes. Flip to this
    /// once YubiKeys are agreed — no rebuild of call sites.
    #[must_use]
    pub const fn hardware_enforced() -> Self {
        Self {
            destructive: FactorLevel::WebAuthnRoaming,
            secret: FactorLevel::WebAuthnRoaming,
            ca: FactorLevel::WebAuthnRoaming,
        }
    }

    /// The factor level required for an action class.
    #[must_use]
    pub const fn required(&self, class: ActionClass) -> FactorLevel {
        match class {
            ActionClass::ReadOnly | ActionClass::Mutating => FactorLevel::None,
            ActionClass::Destructive => self.destructive,
            ActionClass::SecretAccess => self.secret,
            ActionClass::CaUse => self.ca,
            // Cross-residency is denied outright by `authorize`; require the strongest.
            ActionClass::CrossResidency => FactorLevel::WebAuthnRoaming,
        }
    }

    /// Whether a presented factor satisfies the requirement for `class`.
    #[must_use]
    pub fn satisfied(&self, class: ActionClass, presented: FactorLevel) -> bool {
        presented >= self.required(class)
    }
}

impl Default for FactorPolicy {
    fn default() -> Self {
        Self::interim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(region: Region, roles: &[&str]) -> Claims {
        Claims {
            sub: "op@x".into(),
            tenant_id: "00000000-0000-4000-8000-000000000000".into(),
            residency_group: region,
            roles: roles.iter().map(|s| (*s).to_owned()).collect(),
            scope: "openid".into(),
        }
    }

    #[test]
    fn maps_roles_and_subject() {
        let p =
            principal_from_claims(&claims(Region::Eu, &["operator", "bogus"]), Region::Eu).unwrap();
        assert_eq!(p.sub, "op@x");
        assert_eq!(p.roles, vec![Role::Operator]); // bogus dropped
        assert_eq!(p.residency, Region::Eu);
    }

    #[test]
    fn rejects_cross_residency_token() {
        let err =
            principal_from_claims(&claims(Region::Uae, &["operator"]), Region::Eu).unwrap_err();
        assert_eq!(
            err,
            AuthnError::ResidencyMismatch {
                token: Region::Uae,
                daemon: Region::Eu
            }
        );
    }

    #[test]
    fn rejects_no_recognised_role() {
        let err = principal_from_claims(&claims(Region::Eu, &["nobody"]), Region::Eu).unwrap_err();
        assert!(matches!(err, AuthnError::NoRecognisedRole(_)));
    }

    #[test]
    fn factor_policy_interim_requires_webauthn_for_dangerous() {
        let p = FactorPolicy::interim();
        assert_eq!(p.required(ActionClass::ReadOnly), FactorLevel::None);
        assert_eq!(p.required(ActionClass::Mutating), FactorLevel::None);
        assert_eq!(
            p.required(ActionClass::Destructive),
            FactorLevel::WebAuthnPlatform
        );
        assert!(p.satisfied(ActionClass::Destructive, FactorLevel::WebAuthnRoaming));
        assert!(!p.satisfied(ActionClass::Destructive, FactorLevel::Totp));
        assert!(p.satisfied(ActionClass::Mutating, FactorLevel::None));
    }

    #[test]
    fn hardware_policy_requires_roaming() {
        let p = FactorPolicy::hardware_enforced();
        assert!(!p.satisfied(ActionClass::SecretAccess, FactorLevel::WebAuthnPlatform));
        assert!(p.satisfied(ActionClass::SecretAccess, FactorLevel::WebAuthnRoaming));
    }

    #[test]
    fn factor_levels_ordered_by_strength() {
        assert!(FactorLevel::WebAuthnRoaming > FactorLevel::WebAuthnPlatform);
        assert!(FactorLevel::WebAuthnPlatform > FactorLevel::Totp);
        assert!(FactorLevel::Totp > FactorLevel::None);
    }
}
