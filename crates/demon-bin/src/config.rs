//! Daemon configuration, loaded from the environment.
//!
//! One daemon serves exactly one residency group, so [`Config::region`] is required
//! and chosen at startup; the rest have safe local-dev defaults.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use demon_core::Region;

/// Resolved daemon configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// The residency group this daemon serves (`DEMON_RESIDENCY`, required).
    pub region: Region,
    /// Listen address (`DEMON_BIND`, default `127.0.0.1:8787`). In production this is
    /// the WireGuard interface address only.
    pub bind: SocketAddr,
    /// SQLite database path (`DEMON_DB_PATH`, default `demon-<region>.db`).
    pub db_path: PathBuf,
    /// SSH login user for collectors (`DEMON_SSH_USER`, default `ops`).
    pub ssh_user: String,
    /// Health poll interval (`DEMON_POLL_SECS`, default `60`).
    pub poll_interval: Duration,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns an error if `DEMON_RESIDENCY` is unset/invalid or `DEMON_BIND` is not a
    /// valid `host:port`.
    pub fn from_env() -> anyhow::Result<Self> {
        let region: Region = std::env::var("DEMON_RESIDENCY")
            .context("DEMON_RESIDENCY must be set to \"eu\" or \"uae\"")?
            .parse()?;
        let bind: SocketAddr = std::env::var("DEMON_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_owned())
            .parse()
            .context("DEMON_BIND must be a valid host:port")?;
        let db_path = std::env::var("DEMON_DB_PATH")
            .unwrap_or_else(|_| format!("demon-{region}.db"))
            .into();
        let ssh_user = std::env::var("DEMON_SSH_USER").unwrap_or_else(|_| "ops".to_owned());
        let poll_secs: u64 = std::env::var("DEMON_POLL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        let poll_interval = Duration::from_secs(poll_secs.max(1));
        Ok(Self {
            region,
            bind,
            db_path,
            ssh_user,
            poll_interval,
        })
    }
}
