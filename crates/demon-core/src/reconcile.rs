//! Drift reconciliation (pure core).
//!
//! Compares **desired** vs **observed** state for a target and reports drift. The
//! non-negotiable: the reconciler only *detects and proposes* — applying a fix always
//! goes through a typed-confirm-gated job. An explicit **auto-apply whitelist**
//! (default empty) governs the narrow set of keys an operator has opted into
//! auto-applying; everything else is human-gated. No silent 3am changes.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// A single drifted key: desired ≠ observed (or observed absent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drift {
    /// The desired-state key.
    pub key: String,
    /// The desired value.
    pub desired: String,
    /// The observed value, if any.
    pub observed: Option<String>,
}

/// The outcome of reconciling one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReconcilePlan {
    /// Every drifted key.
    pub drifts: Vec<Drift>,
    /// Drifts whose key is on the opt-in auto-apply whitelist.
    pub auto_appliable: Vec<Drift>,
    /// Drifts that require a human-gated job (the default for everything).
    pub gated: Vec<Drift>,
}

impl ReconcilePlan {
    /// Whether any drift was detected.
    #[must_use]
    pub fn has_drift(&self) -> bool {
        !self.drifts.is_empty()
    }
}

/// Reconcile `observed` toward `desired`. A key present in `desired` whose value
/// differs from `observed` (or is missing there) is a drift. Keys present only in
/// `observed` are ignored (we converge toward desired, we do not prune unknowns here).
///
/// `auto_whitelist` is the opt-in set of keys that may be auto-applied; every other
/// drift is gated. An empty whitelist (the default) ⇒ everything is human-gated.
#[must_use]
pub fn reconcile(
    desired: &BTreeMap<String, String>,
    observed: &BTreeMap<String, String>,
    auto_whitelist: &BTreeSet<String>,
) -> ReconcilePlan {
    let mut plan = ReconcilePlan::default();
    for (key, want) in desired {
        let have = observed.get(key);
        if have == Some(want) {
            continue; // in sync
        }
        let drift = Drift {
            key: key.clone(),
            desired: want.clone(),
            observed: have.cloned(),
        };
        if auto_whitelist.contains(key) {
            plan.auto_appliable.push(drift.clone());
        } else {
            plan.gated.push(drift.clone());
        }
        plan.drifts.push(drift);
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    #[test]
    fn no_drift_when_in_sync() {
        let d = map(&[("version", "2.13"), ("replicas", "3")]);
        let o = map(&[("version", "2.13"), ("replicas", "3"), ("extra", "x")]);
        let plan = reconcile(&d, &o, &BTreeSet::new());
        assert!(!plan.has_drift());
    }

    #[test]
    fn detects_changed_and_missing_keys() {
        let d = map(&[("version", "2.13"), ("replicas", "5")]);
        let o = map(&[("version", "2.11")]); // replicas missing, version differs
        let plan = reconcile(&d, &o, &BTreeSet::new());
        assert_eq!(plan.drifts.len(), 2);
        let repl = plan.drifts.iter().find(|x| x.key == "replicas").unwrap();
        assert_eq!(repl.observed, None);
        let ver = plan.drifts.iter().find(|x| x.key == "version").unwrap();
        assert_eq!(ver.observed.as_deref(), Some("2.11"));
    }

    #[test]
    fn everything_gated_by_default() {
        let d = map(&[("version", "2.13"), ("replicas", "5")]);
        let o = map(&[]);
        let plan = reconcile(&d, &o, &BTreeSet::new());
        assert_eq!(plan.gated.len(), 2);
        assert!(plan.auto_appliable.is_empty());
    }

    #[test]
    fn whitelist_partitions_auto_vs_gated() {
        let d = map(&[("version", "2.13"), ("replicas", "5")]);
        let o = map(&[]);
        let mut wl = BTreeSet::new();
        wl.insert("replicas".to_owned()); // operator opted replicas into auto-apply
        let plan = reconcile(&d, &o, &wl);
        assert_eq!(plan.auto_appliable.len(), 1);
        assert_eq!(plan.auto_appliable[0].key, "replicas");
        assert_eq!(plan.gated.len(), 1);
        assert_eq!(plan.gated[0].key, "version"); // not whitelisted -> human-gated
    }
}
