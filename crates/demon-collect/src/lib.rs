//! `demon-collect` — agentless SSH-over-WireGuard transport + `check-*.sh`
//! line-contract collectors.
//!
//! A [`Transport`] abstracts "run a read-only command on a host and return its
//! stdout". [`SshTransport`] shells out to `ssh` (over the WireGuard mesh);
//! [`MockTransport`] feeds canned output for tests. Collectors (e.g. [`collect_os`])
//! run a `check-*.sh` script through a transport and hand the output to the pure
//! parser in `demon-core` — no parsing logic lives here.
//!
//! Only **read-only** check scripts run through this path; mutations go through the
//! gated mutation pipeline (later phases), never here.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::future::Future;

use demon_core::{HealthStatus, OsStatus};

/// Command that emits the `check-os.sh` OS contract on a host.
pub const CHECK_OS_CMD: &str = "check-os.sh";

/// A read-only check area: which `check-*.sh` to run and how to roll its contract
/// into a [`HealthStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckArea {
    /// OS / platform detection (`check-os.sh`).
    Os,
    /// Backup recency (`check-backup.sh`).
    Backup,
    /// File integrity (`check-fim.sh`).
    Fim,
    /// SSH access posture (`check-access.sh`).
    Access,
    /// Audit/security posture (`check-audit.sh`).
    Audit,
    /// Config/git drift (`check-drift.sh`).
    Drift,
    /// WireGuard residency posture (`check-residency.sh`).
    Residency,
    /// CIS compliance (`check-compliance.sh`).
    Compliance,
}

impl CheckArea {
    /// All areas the Phase 1 poller collects.
    pub const ALL: [CheckArea; 8] = [
        CheckArea::Os,
        CheckArea::Backup,
        CheckArea::Fim,
        CheckArea::Access,
        CheckArea::Audit,
        CheckArea::Drift,
        CheckArea::Residency,
        CheckArea::Compliance,
    ];

    /// Canonical area name (matches the `area` column / contract tag, lowercased).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckArea::Os => "os",
            CheckArea::Backup => "backup",
            CheckArea::Fim => "fim",
            CheckArea::Access => "access",
            CheckArea::Audit => "audit",
            CheckArea::Drift => "drift",
            CheckArea::Residency => "residency",
            CheckArea::Compliance => "compliance",
        }
    }

    /// Parse a canonical area name back into a [`CheckArea`].
    #[must_use]
    pub fn from_area(area: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == area)
    }

    /// The host-side command that emits this area's contract.
    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            CheckArea::Os => "check-os.sh",
            CheckArea::Backup => "check-backup.sh",
            CheckArea::Fim => "check-fim.sh",
            CheckArea::Access => "check-access.sh",
            CheckArea::Audit => "check-audit.sh",
            CheckArea::Drift => "check-drift.sh",
            CheckArea::Residency => "check-residency.sh",
            CheckArea::Compliance => "check-compliance.sh",
        }
    }

    /// Parse raw contract output into a rolled-up status and a JSON detail blob.
    #[must_use]
    fn interpret(self, output: &str) -> (HealthStatus, String) {
        match self {
            CheckArea::Os => {
                let s = demon_core::parse_os(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Backup => {
                let s = demon_core::parse_backup(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Fim => {
                let s = demon_core::parse_fim(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Access => {
                let s = demon_core::parse_access(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Audit => {
                let s = demon_core::parse_audit(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Drift => {
                let s = demon_core::parse_drift(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Residency => {
                let s = demon_core::parse_residency(output);
                (s.health(), to_json(&s))
            }
            CheckArea::Compliance => {
                let s = demon_core::parse_compliance(output);
                (s.health(), to_json(&s))
            }
        }
    }
}

fn to_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_owned())
}

/// The outcome of collecting one area from one host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedHealth {
    /// Area name (`os`, `backup`, `fim`).
    pub area: String,
    /// Rolled-up health.
    pub status: HealthStatus,
    /// JSON detail of the parsed contract.
    pub raw_json: String,
}

/// Collect one [`CheckArea`] from a host: run its command over the transport and
/// interpret the contract. A transport failure is an `Err`; a malformed-but-returned
/// contract degrades to `Unknown` (never errors).
///
/// # Errors
/// Returns [`CollectError`] if the transport fails.
pub async fn collect<T: Transport>(
    transport: &T,
    host_addr: &str,
    area: CheckArea,
) -> Result<CollectedHealth, CollectError> {
    let output = transport.run_readonly(host_addr, area.command()).await?;
    let (status, raw_json) = area.interpret(&output);
    Ok(CollectedHealth {
        area: area.as_str().to_owned(),
        status,
        raw_json,
    })
}

/// Errors from collecting over a transport.
#[derive(Debug, thiserror::Error)]
pub enum CollectError {
    /// The transport command failed to spawn or its I/O errored.
    #[error("transport io error: {0}")]
    Io(#[from] std::io::Error),
    /// The remote command exited non-zero. (Note: well-behaved `check-*.sh` always
    /// exit 0 — status is in the line — so this signals a transport/SSH failure.)
    #[error("remote command exited with code {code:?}: {stderr}")]
    NonZeroExit {
        /// Exit code, if any.
        code: Option<i32>,
        /// Captured stderr (already size-bounded by the OS pipe).
        stderr: String,
    },
    /// A mock transport had no canned response for the command (test only).
    #[error("no mock response for command: {0}")]
    NoMock(String),
}

/// Run a read-only command on a host and return its stdout.
pub trait Transport: Send + Sync {
    /// Execute `command` on `host_addr` (read-only) and return captured stdout.
    fn run_readonly(
        &self,
        host_addr: &str,
        command: &str,
    ) -> impl Future<Output = Result<String, CollectError>> + Send;
}

/// SSH transport — shells out to the system `ssh` client over WireGuard.
#[derive(Debug, Clone)]
pub struct SshTransport {
    /// SSH login user.
    pub user: String,
    /// Extra `ssh` options (e.g. `-i key`, `-o BatchMode=yes`), prepended verbatim.
    pub opts: Vec<String>,
}

impl SshTransport {
    /// Construct a transport with batch-mode defaults.
    #[must_use]
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            opts: vec![
                "-o".into(),
                "BatchMode=yes".into(),
                "-o".into(),
                "StrictHostKeyChecking=accept-new".into(),
            ],
        }
    }
}

impl Transport for SshTransport {
    fn run_readonly(
        &self,
        host_addr: &str,
        command: &str,
    ) -> impl Future<Output = Result<String, CollectError>> + Send {
        let mut args: Vec<String> = self.opts.clone();
        args.push(format!("{}@{host_addr}", self.user));
        args.push(command.to_owned());
        async move {
            let out = tokio::process::Command::new("ssh")
                .args(&args)
                .output()
                .await?;
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            } else {
                Err(CollectError::NonZeroExit {
                    code: out.status.code(),
                    stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                })
            }
        }
    }
}

/// In-memory transport for tests: maps a command string to canned stdout.
#[derive(Debug, Clone, Default)]
pub struct MockTransport {
    /// command -> stdout.
    pub responses: HashMap<String, String>,
}

impl MockTransport {
    /// Build a mock with a single canned response.
    #[must_use]
    pub fn with(command: impl Into<String>, stdout: impl Into<String>) -> Self {
        let mut responses = HashMap::new();
        responses.insert(command.into(), stdout.into());
        Self { responses }
    }
}

impl Transport for MockTransport {
    fn run_readonly(
        &self,
        _host_addr: &str,
        command: &str,
    ) -> impl Future<Output = Result<String, CollectError>> + Send {
        let result = self
            .responses
            .get(command)
            .cloned()
            .ok_or_else(|| CollectError::NoMock(command.to_owned()));
        async move { result }
    }
}

/// Collect the OS/platform report from a host via `check-os.sh`.
///
/// # Errors
/// Returns [`CollectError`] if the transport fails. Malformed output does not error —
/// it degrades to an `Unknown` [`OsStatus`] in the pure parser.
pub async fn collect_os<T: Transport>(
    transport: &T,
    host_addr: &str,
) -> Result<OsStatus, CollectError> {
    let output = transport.run_readonly(host_addr, CHECK_OS_CMD).await?;
    Ok(demon_core::parse_os(&output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::OsFamily;

    #[tokio::test]
    async fn collect_os_parses_mock_output() {
        let line = "OS\thost=core-1\tfamily=freebsd\tid=freebsd\tversion=14.1-RELEASE\tpkg=pkg\tservice=rc\tfirewall=pf\tcontainer=jail";
        let t = MockTransport::with(CHECK_OS_CMD, line);
        let os = collect_os(&t, "10.200.0.5").await.unwrap();
        assert_eq!(os.family, OsFamily::FreeBsd);
        assert_eq!(os.host, "core-1");
    }

    #[tokio::test]
    async fn missing_mock_errors() {
        let t = MockTransport::default();
        let err = collect_os(&t, "10.200.0.5").await.unwrap_err();
        assert!(matches!(err, CollectError::NoMock(_)));
    }
}
