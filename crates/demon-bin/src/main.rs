//! proximiio.demon daemon entrypoint.
//!
//! Picks the residency group from config and monomorphises the whole daemon over the
//! corresponding [`demon_core::Residency`] marker, so EU and UAE run as distinct,
//! type-isolated instances of the same binary.
#![forbid(unsafe_code)]

mod config;

use anyhow::Context;
use config::Config;
use demon_core::{Eu, Region, Residency, Uae};
use demon_server::{router, AppState};
use demon_store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cfg = Config::from_env()?;
    tracing::info!(?cfg, "starting proximiio.demon");

    match cfg.region {
        Region::Eu => run::<Eu>(cfg).await,
        Region::Uae => run::<Uae>(cfg).await,
    }
}

/// Run the daemon for a single residency group `R`.
async fn run<R: Residency>(cfg: Config) -> anyhow::Result<()> {
    let _store = Store::<R>::open(&cfg.db_path)
        .await
        .with_context(|| format!("opening store at {}", cfg.db_path.display()))?;

    let state = AppState {
        version: env!("CARGO_PKG_VERSION"),
        region: R::REGION,
    };
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .with_context(|| format!("binding {}", cfg.bind))?;
    tracing::info!(addr = %cfg.bind, region = %R::REGION, "demon listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    Ok(())
}

/// Initialise structured logging (`RUST_LOG`, default `info`).
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

/// Resolve when the process receives Ctrl-C (SIGINT).
async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %e, "failed to install ctrl-c handler");
    }
    tracing::info!("shutdown signal received");
}
