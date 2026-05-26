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
    /// OIDC client config (set `DEMON_OIDC_ISSUER` to enable operator login). When
    /// `None`, the daemon runs but the API stays closed (no way to authenticate).
    pub oidc: Option<demon_clients::OidcConfig>,
    /// mTLS material (set all of `DEMON_TLS_CERT`/`_KEY`/`_CLIENT_CA` to enable the
    /// mTLS listener). When `None`, the daemon serves plain HTTP (dev only).
    pub tls: Option<TlsPaths>,
    /// **DEV ONLY**: bypass the auth gate (`DEMON_DEV_NO_AUTH=1`). Never in production.
    pub dev_no_auth: bool,
}

/// Paths to the mTLS server cert, key, and the client-CA bundle (fleet Root CA).
#[derive(Debug, Clone)]
pub struct TlsPaths {
    /// Server certificate chain (PEM).
    pub cert: PathBuf,
    /// Server private key (PEM).
    pub key: PathBuf,
    /// Client-CA bundle operator certs must chain to (PEM).
    pub client_ca: PathBuf,
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
        let oidc =
            std::env::var("DEMON_OIDC_ISSUER")
                .ok()
                .map(|issuer| demon_clients::OidcConfig {
                    issuer,
                    client_id: std::env::var("DEMON_OIDC_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("DEMON_OIDC_CLIENT_SECRET").unwrap_or_default(),
                    redirect_uri: std::env::var("DEMON_OIDC_REDIRECT_URI").unwrap_or_default(),
                });
        let tls = match (
            std::env::var("DEMON_TLS_CERT"),
            std::env::var("DEMON_TLS_KEY"),
            std::env::var("DEMON_TLS_CLIENT_CA"),
        ) {
            (Ok(cert), Ok(key), Ok(client_ca)) => Some(TlsPaths {
                cert: cert.into(),
                key: key.into(),
                client_ca: client_ca.into(),
            }),
            _ => None,
        };
        let dev_no_auth = matches!(
            std::env::var("DEMON_DEV_NO_AUTH").as_deref(),
            Ok("1" | "true")
        );
        Ok(Self {
            region,
            bind,
            db_path,
            ssh_user,
            poll_interval,
            oidc,
            tls,
            dev_no_auth,
        })
    }
}
