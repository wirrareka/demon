//! The guarded mutation pipeline (pure core).
//!
//! Every mutation follows the non-negotiable path:
//! `authorize → plan → typed-confirm → dry-run → apply → check-*.sh verify → audit`,
//! with the worst classes requiring two-person dual-control. This module models the
//! pure pieces — the action catalog, the [`Plan`] (incl. the typed-confirm phrase),
//! [`DualControl`] accounting (no self-approval), and the [`JobState`] transition
//! rules. Execution I/O (SSH apply, `check-*.sh` verify) lives behind a trait in a
//! driver crate; only a [`Capability`] (mintable solely by `authorize`) can start a plan.

use std::collections::BTreeSet;

use crate::action::{ActionClass, Capability};
use crate::authorize::Role;

/// A known, guided guarded action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuardedAction {
    /// Restart a service.
    ServiceRestart,
    /// Rotate a certificate (uses the signing CA).
    CertRotate,
    /// Run a backup now.
    BackupRun,
    /// Drain a host (move load off it).
    HostDrain,
    /// Update packages + refresh integrity checksums.
    PkgUpdate,
    /// Horizontally scale a service out (spawn node(s) per the capacity model).
    ScaleOut,
}

impl GuardedAction {
    /// Every action in the catalog.
    pub const ALL: [GuardedAction; 6] = [
        GuardedAction::ServiceRestart,
        GuardedAction::CertRotate,
        GuardedAction::BackupRun,
        GuardedAction::HostDrain,
        GuardedAction::PkgUpdate,
        GuardedAction::ScaleOut,
    ];

    /// Stable `<noun>.<verb>` id (also the audit `action`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            GuardedAction::ServiceRestart => "service.restart",
            GuardedAction::CertRotate => "cert.rotate",
            GuardedAction::BackupRun => "backup.run",
            GuardedAction::HostDrain => "host.drain",
            GuardedAction::PkgUpdate => "pkg.update",
            GuardedAction::ScaleOut => "service.scale-out",
        }
    }

    /// Danger classification driving authz + step-up + dual-control.
    #[must_use]
    pub const fn class(self) -> ActionClass {
        match self {
            GuardedAction::ServiceRestart | GuardedAction::BackupRun | GuardedAction::PkgUpdate => {
                ActionClass::Mutating
            }
            GuardedAction::HostDrain | GuardedAction::ScaleOut => ActionClass::Destructive,
            GuardedAction::CertRotate => ActionClass::CaUse,
        }
    }

    /// The `check-*.sh` area that verifies success, if any.
    #[must_use]
    pub const fn verify_area(self) -> Option<&'static str> {
        match self {
            GuardedAction::BackupRun => Some("backup"),
            GuardedAction::PkgUpdate => Some("fim"),
            GuardedAction::CertRotate => Some("access"),
            GuardedAction::ServiceRestart | GuardedAction::HostDrain | GuardedAction::ScaleOut => {
                None
            }
        }
    }

    /// Whether two-person dual-control is required (the worst classes).
    #[must_use]
    pub const fn requires_dual_control(self) -> bool {
        self.class().requires_dual_control()
    }

    /// Parse an action id.
    #[must_use]
    pub fn from_id(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == s)
    }

    /// Whether a role may initiate this action.
    #[must_use]
    pub const fn permitted_for(self, role: Role) -> bool {
        role.permits(self.class())
    }
}

/// The actions a set of roles may initiate — drives the API's `available_actions[]`.
#[must_use]
pub fn available_actions(roles: &[Role]) -> Vec<GuardedAction> {
    GuardedAction::ALL
        .into_iter()
        .filter(|a| roles.iter().any(|r| a.permitted_for(*r)))
        .collect()
}

/// A concrete plan for one action against one target, with the exact phrase the
/// operator must type to confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The action.
    pub action: GuardedAction,
    /// The target identifier.
    pub target: String,
    /// The phrase the operator must type verbatim to confirm.
    pub confirm_phrase: String,
    /// Whether dual-control is required.
    pub dual_control: bool,
}

impl Plan {
    /// Build a plan from a [`Capability`] (proof the action was authorized). Returns
    /// `None` if the capability's action is not a known guarded action.
    #[must_use]
    pub fn from_capability(cap: &Capability) -> Option<Self> {
        let action = GuardedAction::from_id(&cap.spec().action)?;
        let target = cap.spec().target.clone();
        Some(Self {
            confirm_phrase: format!("{} {target}", action.id()),
            dual_control: action.requires_dual_control(),
            action,
            target,
        })
    }

    /// Whether the typed input exactly matches the required confirm phrase.
    #[must_use]
    pub fn confirm_matches(&self, typed: &str) -> bool {
        typed == self.confirm_phrase
    }
}

/// Why an approval was rejected.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApprovalError {
    /// The initiator may not approve their own action (two-person rule).
    #[error("self-approval is forbidden")]
    SelfApproval,
    /// This principal already approved.
    #[error("principal already approved")]
    AlreadyApproved,
}

/// Two-person dual-control accounting: the initiator plus at least one distinct approver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualControl {
    initiator: String,
    approvers: BTreeSet<String>,
    required: usize,
}

impl DualControl {
    /// New control requiring `required` distinct approvers (≥1) besides the initiator.
    #[must_use]
    pub fn new(initiator: impl Into<String>, required: usize) -> Self {
        Self {
            initiator: initiator.into(),
            approvers: BTreeSet::new(),
            required: required.max(1),
        }
    }

    /// Record an approval.
    ///
    /// # Errors
    /// [`ApprovalError::SelfApproval`] if `who` is the initiator, or
    /// [`ApprovalError::AlreadyApproved`] if already counted.
    pub fn approve(&mut self, who: &str) -> Result<(), ApprovalError> {
        if who == self.initiator {
            return Err(ApprovalError::SelfApproval);
        }
        if !self.approvers.insert(who.to_owned()) {
            return Err(ApprovalError::AlreadyApproved);
        }
        Ok(())
    }

    /// Whether enough distinct approvers have signed off.
    #[must_use]
    pub fn is_satisfied(&self) -> bool {
        self.approvers.len() >= self.required
    }
}

/// The lifecycle state of a mutation job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Planned, not yet confirmed.
    Planned,
    /// Awaiting dual-control approvals.
    AwaitingApproval,
    /// Typed-confirm (+ approvals) done; ready to dry-run.
    Confirmed,
    /// Dry-run succeeded.
    DryRunOk,
    /// Apply in progress.
    Applying,
    /// Applied and verified (terminal success).
    Verified,
    /// Apply or verify failed (terminal failure).
    Failed,
    /// Failed apply rolled back via the inverse plan (terminal).
    RolledBack,
}

impl JobState {
    /// Whether a direct transition `self → next` is allowed.
    #[must_use]
    pub fn can_advance_to(self, next: JobState) -> bool {
        use JobState::{
            Applying, AwaitingApproval, Confirmed, DryRunOk, Failed, Planned, RolledBack, Verified,
        };
        matches!(
            (self, next),
            (Planned, AwaitingApproval | Confirmed)
                | (AwaitingApproval, Confirmed)
                | (Confirmed, DryRunOk)
                | (DryRunOk, Applying)
                | (Applying, Verified | Failed)
                | (Failed, RolledBack)
        )
    }

    /// Terminal states accept no further transitions.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobState::Verified | JobState::Failed | JobState::RolledBack
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionSpec;
    use crate::authorize::{authorize, Principal};
    use crate::residency::Region;

    fn cap(action: &str, class: ActionClass, role: Role) -> Capability {
        let p = Principal::new("op@x", vec![role], Region::Eu);
        authorize(&p, ActionSpec::new(action, "host:core-1/opensearch", class)).unwrap()
    }

    #[test]
    fn catalog_classes_and_dual_control() {
        assert_eq!(GuardedAction::ServiceRestart.class(), ActionClass::Mutating);
        assert!(!GuardedAction::ServiceRestart.requires_dual_control());
        assert!(GuardedAction::HostDrain.requires_dual_control()); // Destructive
        assert!(GuardedAction::CertRotate.requires_dual_control()); // CaUse
        assert_eq!(
            GuardedAction::from_id("backup.run"),
            Some(GuardedAction::BackupRun)
        );
        assert_eq!(GuardedAction::from_id("nope"), None);
    }

    #[test]
    fn available_actions_are_role_scoped() {
        let viewer = available_actions(&[Role::Viewer]);
        assert!(viewer.is_empty()); // no mutating actions

        let operator = available_actions(&[Role::Operator]);
        assert!(operator.contains(&GuardedAction::ServiceRestart));
        assert!(operator.contains(&GuardedAction::BackupRun));
        assert!(!operator.contains(&GuardedAction::HostDrain)); // Destructive
        assert!(!operator.contains(&GuardedAction::CertRotate)); // CaUse

        let senior = available_actions(&[Role::Senior]);
        assert!(senior.contains(&GuardedAction::HostDrain));
        assert!(senior.contains(&GuardedAction::CertRotate));
        assert!(senior.contains(&GuardedAction::ScaleOut));
        assert!(!operator.contains(&GuardedAction::ScaleOut)); // Destructive
    }

    #[test]
    fn plan_confirm_phrase_must_match_exactly() {
        let plan = Plan::from_capability(&cap(
            "service.restart",
            ActionClass::Mutating,
            Role::Operator,
        ))
        .unwrap();
        assert_eq!(
            plan.confirm_phrase,
            "service.restart host:core-1/opensearch"
        );
        assert!(plan.confirm_matches("service.restart host:core-1/opensearch"));
        assert!(!plan.confirm_matches("service.restart host:core-1"));
        assert!(!plan.dual_control);
    }

    #[test]
    fn plan_flags_dual_control_for_destructive() {
        let plan =
            Plan::from_capability(&cap("host.drain", ActionClass::Destructive, Role::Senior))
                .unwrap();
        assert!(plan.dual_control);
    }

    #[test]
    fn unknown_action_has_no_plan() {
        assert!(
            Plan::from_capability(&cap("frobnicate", ActionClass::Mutating, Role::Operator))
                .is_none()
        );
    }

    #[test]
    fn dual_control_no_self_approve_and_needs_distinct() {
        let mut dc = DualControl::new("op@x", 1);
        assert_eq!(dc.approve("op@x"), Err(ApprovalError::SelfApproval));
        assert!(!dc.is_satisfied());
        dc.approve("sr@y").unwrap();
        assert!(dc.is_satisfied());
        assert_eq!(dc.approve("sr@y"), Err(ApprovalError::AlreadyApproved));
    }

    #[test]
    fn job_transitions_follow_the_pipeline() {
        use JobState::{
            Applying, AwaitingApproval, Confirmed, DryRunOk, Failed, Planned, RolledBack, Verified,
        };
        assert!(Planned.can_advance_to(AwaitingApproval));
        assert!(Planned.can_advance_to(Confirmed));
        assert!(Confirmed.can_advance_to(DryRunOk));
        assert!(DryRunOk.can_advance_to(Applying));
        assert!(Applying.can_advance_to(Verified));
        assert!(Applying.can_advance_to(Failed));
        assert!(Failed.can_advance_to(RolledBack));
        // illegal jumps
        assert!(!Planned.can_advance_to(Applying));
        assert!(!Confirmed.can_advance_to(Verified));
        assert!(Verified.is_terminal());
        assert!(!Verified.can_advance_to(Applying));
    }
}
