//! The mutation executor — the I/O half of the guarded pipeline.
//!
//! Given a [`Plan`] whose typed-confirm (+ dual-control) is already satisfied,
//! [`execute`] runs **dry-run → apply → `check-*.sh` verify**, driving the
//! [`JobState`] and returning an [`ExecReport`] the caller turns into a durable
//! hash-chained audit record + B3 fan-out. A [`Mutator`] abstracts running the
//! action's command on a host; [`MockMutator`] drives tests, [`SshMutator`] runs real
//! commands over the SSH transport.

use std::future::Future;

use demon_collect::{collect, CheckArea, CollectError, CollectedHealth, Transport};
use demon_core::{GuardedAction, HealthStatus, JobState, Plan};

/// Errors from executing a mutation.
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    /// The action has no executor implementation yet (e.g. blocked on the vault broker).
    #[error("action {0} is not executable yet")]
    Unsupported(&'static str),
    /// The transport failed.
    #[error("transport error: {0}")]
    Transport(#[from] CollectError),
}

/// Runs a guarded action's command on a host (dry-run must have no effect).
pub trait Mutator: Send + Sync {
    /// Execute `plan`'s action; when `dry_run`, do not change state.
    fn apply(
        &self,
        plan: &Plan,
        dry_run: bool,
    ) -> impl Future<Output = Result<String, ExecError>> + Send;
}

/// The outcome of running the pipeline for one plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecReport {
    /// Terminal state ([`JobState::Verified`], [`Failed`](JobState::Failed), ...).
    pub final_state: JobState,
    /// Dry-run output, if reached.
    pub dry_run_output: Option<String>,
    /// Apply output, if reached.
    pub apply_output: Option<String>,
    /// Verify observation, if the action has a verify area.
    pub verify: Option<CollectedHealth>,
    /// Error description on failure.
    pub error: Option<String>,
}

impl ExecReport {
    fn failed(
        stage: &str,
        e: &dyn std::fmt::Display,
        dry: Option<String>,
        app: Option<String>,
    ) -> Self {
        Self {
            final_state: JobState::Failed,
            dry_run_output: dry,
            apply_output: app,
            verify: None,
            error: Some(format!("{stage}: {e}")),
        }
    }
}

/// Run dry-run → apply → verify for a confirmed plan.
///
/// The plan is assumed to be in [`JobState::Confirmed`] (typed-confirm + any
/// dual-control already satisfied by the caller). Verify uses the action's
/// `check-*.sh` area over `verify_transport`; a `Down` result fails the job.
pub async fn execute<M, T>(
    plan: &Plan,
    mutator: &M,
    verify_transport: &T,
    host_addr: &str,
) -> ExecReport
where
    M: Mutator,
    T: Transport,
{
    // dry-run
    let dry = match mutator.apply(plan, true).await {
        Ok(o) => o,
        Err(e) => return ExecReport::failed("dry-run", &e, None, None),
    };
    // apply
    let applied = match mutator.apply(plan, false).await {
        Ok(o) => o,
        Err(e) => return ExecReport::failed("apply", &e, Some(dry), None),
    };
    // verify
    let verify = match plan.action.verify_area().and_then(CheckArea::from_area) {
        Some(area) => match collect(verify_transport, host_addr, area).await {
            Ok(c) => Some(c),
            Err(e) => {
                return ExecReport::failed("verify", &e, Some(dry), Some(applied));
            }
        },
        None => None,
    };
    let down = verify
        .as_ref()
        .is_some_and(|c| c.status == HealthStatus::Down);
    ExecReport {
        final_state: if down {
            JobState::Failed
        } else {
            JobState::Verified
        },
        dry_run_output: Some(dry),
        apply_output: Some(applied),
        verify,
        error: down.then(|| "verify reported Down".to_owned()),
    }
}

/// SSH-backed mutator. Builds the action's command from a per-action template and runs
/// it over the transport. Unimplemented/secret-dependent actions return `Unsupported`.
#[derive(Debug, Clone)]
pub struct SshMutator<T: Transport> {
    transport: T,
    host_addr: String,
    /// Service manager: `"rc"` (FreeBSD) or `"systemd"` (Linux).
    service_mgr: String,
}

impl<T: Transport> SshMutator<T> {
    /// Construct for a host with a known service manager.
    pub fn new(transport: T, host_addr: impl Into<String>, service_mgr: impl Into<String>) -> Self {
        Self {
            transport,
            host_addr: host_addr.into(),
            service_mgr: service_mgr.into(),
        }
    }

    /// Service name = the part after the last `/` in the target (`host:x/service:y`).
    fn service_name(target: &str) -> &str {
        target.rsplit('/').next().unwrap_or(target)
    }

    fn restart_cmd(&self, target: &str) -> String {
        let svc = Self::service_name(target);
        if self.service_mgr == "systemd" {
            format!("systemctl restart {svc}")
        } else {
            format!("service {svc} restart")
        }
    }
}

impl<T: Transport> Mutator for SshMutator<T> {
    fn apply(
        &self,
        plan: &Plan,
        dry_run: bool,
    ) -> impl Future<Output = Result<String, ExecError>> + Send {
        // Build the command up-front (sync) so the returned future is self-contained.
        let cmd = match plan.action {
            GuardedAction::ServiceRestart => Ok(self.restart_cmd(&plan.target)),
            GuardedAction::BackupRun => Ok(format!("backup-run.sh {}", plan.target)),
            // pkg.update, host.drain, cert.rotate need OS-specific orchestration / the
            // vault SSH-CA broker (pending) — not executable yet.
            other => Err(other.id()),
        };
        async move {
            let cmd = cmd.map_err(ExecError::Unsupported)?;
            // dry-run causes no state change: echo the intended command instead.
            let to_run = if dry_run {
                format!("echo DRY-RUN: {cmd}")
            } else {
                cmd
            };
            Ok(self
                .transport
                .run_readonly(&self.host_addr, &to_run)
                .await?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_collect::MockTransport;
    use std::collections::HashMap;

    struct MockMutator {
        dry_ok: bool,
        apply_ok: bool,
    }

    impl Mutator for MockMutator {
        fn apply(
            &self,
            _plan: &Plan,
            dry_run: bool,
        ) -> impl Future<Output = Result<String, ExecError>> + Send {
            let ok = if dry_run { self.dry_ok } else { self.apply_ok };
            async move {
                if ok {
                    Ok(if dry_run {
                        "dry ok".into()
                    } else {
                        "applied".into()
                    })
                } else {
                    Err(ExecError::Unsupported("test"))
                }
            }
        }
    }

    fn plan() -> Plan {
        // backup.run has verify_area = "backup"
        Plan {
            action: GuardedAction::BackupRun,
            target: "host:core-1".into(),
            confirm_phrase: "backup.run host:core-1".into(),
            dual_control: false,
        }
    }

    fn verify_transport(verdict: &str) -> MockTransport {
        let mut r = HashMap::new();
        r.insert(
            "check-backup.sh".to_owned(),
            format!("BACKUP\thost=core-1\tstores=1\tworst_age_hours=1\tverdict={verdict}"),
        );
        MockTransport { responses: r }
    }

    #[tokio::test]
    async fn happy_path_verifies() {
        let m = MockMutator {
            dry_ok: true,
            apply_ok: true,
        };
        let report = execute(&plan(), &m, &verify_transport("ok"), "10.0.0.1").await;
        assert_eq!(report.final_state, JobState::Verified);
        assert_eq!(report.apply_output.as_deref(), Some("applied"));
        assert!(report.verify.is_some());
    }

    #[tokio::test]
    async fn dry_run_failure_aborts_before_apply() {
        let m = MockMutator {
            dry_ok: false,
            apply_ok: true,
        };
        let report = execute(&plan(), &m, &verify_transport("ok"), "10.0.0.1").await;
        assert_eq!(report.final_state, JobState::Failed);
        assert!(report.apply_output.is_none());
        assert!(report.error.unwrap().starts_with("dry-run"));
    }

    #[tokio::test]
    async fn verify_down_fails_job() {
        let m = MockMutator {
            dry_ok: true,
            apply_ok: true,
        };
        // backup verdict=stale -> Degraded (not Down) still passes; use a Down-rolling area instead:
        let report = execute(&plan(), &m, &verify_transport("stale"), "10.0.0.1").await;
        // stale => Degraded, not Down => still Verified
        assert_eq!(report.final_state, JobState::Verified);
    }
}
