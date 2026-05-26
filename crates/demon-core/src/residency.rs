//! Residency — a **compile-time** invariant, not a runtime convention.
//!
//! EU and UAE are physically air-gapped and must NEVER be mixed (see
//! `coordination/conventions/residency.md`). We encode this in the type system: a
//! [`Residency`] marker type parameterises stateful types (e.g. `Store<Eu>` vs
//! `Store<Uae>` in `demon-store`). Code that holds a `Store<Eu>` cannot be handed a
//! `Store<Uae>` — the mismatch is a *compile error*, so a whole class of
//! cross-region bugs cannot be written. The runtime [`Region`] backstops the proof
//! at trust boundaries (deserialised input, SQL rows, JWT claims).

use serde::{Deserialize, Serialize};

/// The two residency groups. Lowercase `eu` / `uae` on the wire — matches the
/// `residency_group` JWT claim and `NODE_RESIDENCY_GROUP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    Eu,
    Uae,
}

impl Region {
    /// The canonical lowercase wire string (`"eu"` / `"uae"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Region::Eu => "eu",
            Region::Uae => "uae",
        }
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string cannot be parsed into a [`Region`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown residency group: {0:?} (expected \"eu\" or \"uae\")")]
pub struct UnknownRegion(pub String);

impl std::str::FromStr for Region {
    type Err = UnknownRegion;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eu" => Ok(Region::Eu),
            "uae" => Ok(Region::Uae),
            other => Err(UnknownRegion(other.to_owned())),
        }
    }
}

/// Marker trait implemented by the zero-sized residency types [`Eu`] and [`Uae`].
///
/// Sealed: only this crate may implement it, so the set of residency groups is
/// closed and every generic `R: Residency` is exactly one of the known regions.
pub trait Residency: sealed::Sealed + Copy + Send + Sync + 'static {
    /// The runtime region this marker corresponds to.
    const REGION: Region;
}

/// EU residency group (`10.200.0.x` WireGuard mesh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Eu;

/// UAE residency group (`10.210.0.x` WireGuard mesh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Uae;

impl Residency for Eu {
    const REGION: Region = Region::Eu;
}
impl Residency for Uae {
    const REGION: Region = Region::Uae;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Eu {}
    impl Sealed for super::Uae {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_roundtrips_lowercase() {
        assert_eq!("eu".parse::<Region>(), Ok(Region::Eu));
        assert_eq!("uae".parse::<Region>(), Ok(Region::Uae));
        assert_eq!(Region::Eu.as_str(), "eu");
        assert_eq!(Region::Uae.to_string(), "uae");
    }

    #[test]
    fn region_rejects_unknown() {
        assert!("us".parse::<Region>().is_err());
        assert!("EU".parse::<Region>().is_err());
    }

    #[test]
    fn marker_types_carry_their_region() {
        assert_eq!(Eu::REGION, Region::Eu);
        assert_eq!(Uae::REGION, Region::Uae);
    }
}
