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
    demon_server::tls::install_crypto_provider();
    let cfg = Config::from_env()?;
    tracing::info!(?cfg, "starting proximiio.demon");

    let seed = std::env::args().nth(1).as_deref() == Some("seed-demo");
    match cfg.region {
        Region::Eu => run::<Eu>(cfg, seed).await,
        Region::Uae => run::<Uae>(cfg, seed).await,
    }
}

/// Run the daemon for a single residency group `R` (or seed demo data and exit).
async fn run<R: Residency>(cfg: Config, seed: bool) -> anyhow::Result<()> {
    let store = Store::<R>::open(&cfg.db_path)
        .await
        .with_context(|| format!("opening store at {}", cfg.db_path.display()))?;

    if seed {
        return seed_demo(&store).await;
    }

    if cfg.dev_no_auth {
        tracing::warn!(
            "DEMON_DEV_NO_AUTH is set — AUTH GATE BYPASSED. DEV ONLY, never use in production."
        );
    }

    // Live health feed: poll worker publishes, the WS stream subscribes.
    let (events, _rx) = tokio::sync::broadcast::channel(demon_workers::EVENT_CHANNEL_CAPACITY);
    let transport = demon_collect::SshTransport::new(cfg.ssh_user.clone());
    tokio::spawn(demon_workers::run(
        store.clone(),
        transport.clone(),
        events.clone(),
        cfg.poll_interval,
    ));
    tracing::info!(
        poll_secs = cfg.poll_interval.as_secs(),
        "health poller started"
    );

    let identity = cfg.oidc.clone().map(demon_clients::IdentityClient::new);
    if identity.is_none() {
        tracing::warn!("OIDC not configured (DEMON_OIDC_ISSUER unset) — API stays closed");
    }

    let audit = std::env::var("DEMON_OPENSEARCH_URL")
        .ok()
        .map(demon_clients::OpenSearchAudit::new);
    if audit.is_none() {
        tracing::warn!("DEMON_OPENSEARCH_URL unset — audit fan-out to OpenSearch disabled");
    }
    let node = std::env::var("DEMON_NODE").unwrap_or_else(|_| format!("demon-{}", R::REGION));

    let state = AppState {
        version: env!("CARGO_PKG_VERSION"),
        store,
        events,
        identity,
        sessions: demon_server::SessionStore::new(),
        pending: demon_server::PendingStore::new(),
        audit,
        node,
        dev_no_auth: cfg.dev_no_auth,
        jobs: demon_server::JobStore::new(),
        runbooks: demon_server::RunbookStore::new(),
        transport,
        webauthn: build_webauthn(),
        metrics: std::env::var("DEMON_PROMETHEUS_URL")
            .ok()
            .map(demon_clients::PrometheusClient::new),
    };
    let app = router(state);

    if let Some(tls) = &cfg.tls {
        let cert =
            std::fs::read(&tls.cert).with_context(|| format!("reading {}", tls.cert.display()))?;
        let key =
            std::fs::read(&tls.key).with_context(|| format!("reading {}", tls.key.display()))?;
        let ca = std::fs::read(&tls.client_ca)
            .with_context(|| format!("reading {}", tls.client_ca.display()))?;
        let server_config =
            demon_server::tls::server_config(&cert, &key, &ca).context("building mTLS config")?;
        let rustls_config =
            axum_server::tls_rustls::RustlsConfig::from_config(std::sync::Arc::new(server_config));
        tracing::info!(addr = %cfg.bind, region = %R::REGION, "demon listening (mTLS)");
        axum_server::bind_rustls(cfg.bind, rustls_config)
            .serve(app.into_make_service())
            .await
            .context("mTLS server error")?;
    } else {
        let listener = tokio::net::TcpListener::bind(cfg.bind)
            .await
            .with_context(|| format!("binding {}", cfg.bind))?;
        tracing::warn!(addr = %cfg.bind, region = %R::REGION, "demon listening WITHOUT TLS (dev) — set DEMON_TLS_* for mTLS");
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("server error")?;
    }
    Ok(())
}

/// Seed a small demo fleet (hosts, a tenant, a service, varied health) so the API/TUI
/// can be exercised locally without identity or real SSH. Idempotent (upserts).
async fn seed_demo<R: Residency>(store: &Store<R>) -> anyhow::Result<()> {
    use demon_core::{Fleet, HealthSnapshot, HealthStatus, Host, Service, TargetKind, Tenant};

    let now = demon_workers::now_ms();
    let region = R::REGION;
    let tenant_id = "00000000-0000-4000-8000-000000000abc";

    let hosts = [
        Host {
            id: "core-1".into(),
            fqdn: format!("core-1.{region}.demon"),
            fleet: Fleet::Core,
            os: "freebsd".into(),
            residency_group: region,
            wg_ip: Some("10.200.0.2".into()),
            tenant_id: None,
            enrolled_at: now,
            last_seen: Some(now),
        },
        Host {
            id: "tnt-acme-1".into(),
            fqdn: format!("acme-1.{region}.demon"),
            fleet: Fleet::Tenant,
            os: "linux".into(),
            residency_group: region,
            wg_ip: Some("10.200.0.50".into()),
            tenant_id: Some(tenant_id.into()),
            enrolled_at: now,
            last_seen: Some(now),
        },
    ];
    for h in &hosts {
        store.upsert_host(h).await?;
    }

    store
        .upsert_tenant(&Tenant {
            id: tenant_id.into(),
            name: "Acme Corp".into(),
            residency_group: region,
            lifecycle_state: "active".into(),
            plan: Some("enterprise".into()),
            created_at: now,
        })
        .await?;

    store
        .upsert_service(&Service {
            id: "core-1/opensearch".into(),
            host_id: "core-1".into(),
            kind: "opensearch".into(),
            version: Some("2.13".into()),
            residency_group: region,
            desired_state: None,
            observed_state: Some("green".into()),
            updated_at: now,
        })
        .await?;

    let demo = [
        ("core-1", "os", HealthStatus::Up),
        ("core-1", "backup", HealthStatus::Up),
        ("core-1", "fim", HealthStatus::Degraded),
        ("core-1", "residency", HealthStatus::Up),
        ("core-1", "access", HealthStatus::Up),
        ("tnt-acme-1", "os", HealthStatus::Up),
        ("tnt-acme-1", "backup", HealthStatus::Degraded),
        ("tnt-acme-1", "compliance", HealthStatus::Degraded),
        ("tnt-acme-1", "access", HealthStatus::Down),
    ];
    for (host, area, status) in demo {
        store
            .insert_health(&HealthSnapshot {
                target_id: host.into(),
                target_kind: TargetKind::Host,
                area: area.into(),
                status,
                raw_json: "{\"demo\":true}".into(),
                observed_at: now,
            })
            .await?;
    }

    println!(
        "seeded {} hosts + 1 tenant + 1 service + {} health snapshots into region {region}",
        hosts.len(),
        demo.len()
    );
    Ok(())
}

/// Build the WebAuthn relying party from `DEMON_WEBAUTHN_RP_ID` + `_ORIGIN`
/// (e.g. `localhost` + `http://localhost:5179` in dev). `None` when unset.
fn build_webauthn() -> Option<std::sync::Arc<demon_server::webauthn::WebauthnCtx>> {
    let rp_id = std::env::var("DEMON_WEBAUTHN_RP_ID").ok()?;
    let origin = std::env::var("DEMON_WEBAUTHN_ORIGIN").ok()?;
    match demon_server::webauthn::WebauthnCtx::new(&rp_id, &origin) {
        Ok(ctx) => Some(std::sync::Arc::new(ctx)),
        Err(e) => {
            tracing::error!(error = %e, "WebAuthn config invalid — step-up disabled");
            None
        }
    }
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
