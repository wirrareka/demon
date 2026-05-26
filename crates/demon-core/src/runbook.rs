//! First-class guided runbooks (pure core).
//!
//! A runbook is an **ordered sequence of guarded actions** — each step still goes
//! through the full plan → confirm → (dual-control) → apply → verify → audit pipeline,
//! so a runbook is "a guided checklist of guarded mutations", not a privileged bypass.
//! This module is the pure catalog + the [`RunbookRun`] progress state machine; the
//! server instantiates a runbook into one [`crate::mutation::Plan`]-backed job per step.

use crate::mutation::GuardedAction;

/// The seeded runbook catalog (security doc §6 / BUILD-PROMPT Phase 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunbookId {
    /// Restart a service.
    RestartService,
    /// Rotate a certificate, then restart to load it.
    RotateCert,
    /// Run a fresh backup.
    RunBackup,
    /// Drain load off a host.
    DrainHost,
    /// Update packages and refresh integrity checksums.
    PkgUpdate,
}

/// One step of a runbook: a guarded action against a (resolved) target + a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunbookStep {
    /// The guarded action this step performs.
    pub action: GuardedAction,
    /// The target the action applies to.
    pub target: String,
    /// Operator-facing description.
    pub description: String,
}

impl RunbookId {
    /// Every runbook in the catalog.
    pub const ALL: [RunbookId; 5] = [
        RunbookId::RestartService,
        RunbookId::RotateCert,
        RunbookId::RunBackup,
        RunbookId::DrainHost,
        RunbookId::PkgUpdate,
    ];

    /// Stable kebab-case id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            RunbookId::RestartService => "restart-service",
            RunbookId::RotateCert => "rotate-cert",
            RunbookId::RunBackup => "run-backup",
            RunbookId::DrainHost => "drain-host",
            RunbookId::PkgUpdate => "pkg-update",
        }
    }

    /// Human title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            RunbookId::RestartService => "Restart a service",
            RunbookId::RotateCert => "Rotate a certificate",
            RunbookId::RunBackup => "Run a backup",
            RunbookId::DrainHost => "Drain a host",
            RunbookId::PkgUpdate => "Update packages",
        }
    }

    /// Parse a runbook id.
    #[must_use]
    pub fn from_id(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == s)
    }

    /// The ordered steps for this runbook against `target`.
    #[must_use]
    pub fn steps(self, target: &str) -> Vec<RunbookStep> {
        let step = |action, description: &str| RunbookStep {
            action,
            target: target.to_owned(),
            description: description.to_owned(),
        };
        match self {
            RunbookId::RestartService => {
                vec![step(GuardedAction::ServiceRestart, "Restart the service")]
            }
            RunbookId::RotateCert => vec![
                step(
                    GuardedAction::CertRotate,
                    "Rotate the certificate (uses the signing CA)",
                ),
                step(
                    GuardedAction::ServiceRestart,
                    "Restart the service to load the new certificate",
                ),
            ],
            RunbookId::RunBackup => vec![step(
                GuardedAction::BackupRun,
                "Run a fresh backup and verify recency",
            )],
            RunbookId::DrainHost => vec![step(GuardedAction::HostDrain, "Drain load off the host")],
            RunbookId::PkgUpdate => vec![step(
                GuardedAction::PkgUpdate,
                "Update packages and refresh integrity checksums",
            )],
        }
    }
}

/// Progress status of a runbook run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Not yet started.
    Pending,
    /// Steps remain.
    InProgress,
    /// All steps succeeded.
    Completed,
    /// A step failed — the run stops (no further steps run).
    Failed,
}

/// The pure progress state of executing a runbook's steps in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunbookRun {
    /// Which runbook.
    pub runbook: RunbookId,
    /// The target.
    pub target: String,
    /// Total step count.
    pub total_steps: usize,
    /// Index of the current (next-to-run) step.
    pub current: usize,
    /// Status.
    pub status: RunStatus,
}

impl RunbookRun {
    /// Start a run for `runbook` against `target`.
    #[must_use]
    pub fn new(runbook: RunbookId, target: &str) -> Self {
        let total_steps = runbook.steps(target).len();
        Self {
            runbook,
            target: target.to_owned(),
            total_steps,
            current: 0,
            status: if total_steps == 0 {
                RunStatus::Completed
            } else {
                RunStatus::Pending
            },
        }
    }

    /// Record the outcome of the current step and advance. A failed step stops the run
    /// (remaining steps do not run); the last successful step completes it.
    pub fn advance(&mut self, step_ok: bool) {
        if matches!(self.status, RunStatus::Completed | RunStatus::Failed) {
            return;
        }
        if !step_ok {
            self.status = RunStatus::Failed;
            return;
        }
        self.current += 1;
        self.status = if self.current >= self.total_steps {
            RunStatus::Completed
        } else {
            RunStatus::InProgress
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_and_steps() {
        assert_eq!(
            RunbookId::from_id("rotate-cert"),
            Some(RunbookId::RotateCert)
        );
        assert_eq!(RunbookId::from_id("nope"), None);
        let steps = RunbookId::RotateCert.steps("host:core-1/service:kalista");
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].action, GuardedAction::CertRotate);
        assert_eq!(steps[1].action, GuardedAction::ServiceRestart);
        assert_eq!(steps[0].target, "host:core-1/service:kalista");
    }

    #[test]
    fn run_completes_on_all_success() {
        let mut run = RunbookRun::new(RunbookId::RotateCert, "host:c1/service:k");
        assert_eq!(run.total_steps, 2);
        assert_eq!(run.status, RunStatus::Pending);
        run.advance(true);
        assert_eq!(run.status, RunStatus::InProgress);
        assert_eq!(run.current, 1);
        run.advance(true);
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.current, 2);
    }

    #[test]
    fn run_stops_on_step_failure() {
        let mut run = RunbookRun::new(RunbookId::RotateCert, "host:c1/service:k");
        run.advance(false);
        assert_eq!(run.status, RunStatus::Failed);
        assert_eq!(run.current, 0); // did not advance past the failed step
                                    // further advances are no-ops
        run.advance(true);
        assert_eq!(run.status, RunStatus::Failed);
    }

    #[test]
    fn single_step_runbook() {
        let mut run = RunbookRun::new(RunbookId::RestartService, "host:c1/service:k");
        assert_eq!(run.total_steps, 1);
        run.advance(true);
        assert_eq!(run.status, RunStatus::Completed);
    }
}
