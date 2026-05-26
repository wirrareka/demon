//! Daemon configuration, loaded from the environment.
//!
//! One daemon serves exactly one residency group, so [`Config::region`] is required
//! and chosen at startup; the rest have safe local-dev defaults.

use std::net::SocketAddr;
use std::path::PathBuf;

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
        Ok(Self {
            region,
            bind,
            db_path,
        })
    }
}
