//! `demon-server` — the Axum REST (+ later WebSocket) surface.
//!
//! Phase 1 ships the unauthenticated liveness probes plus the **read-only** inventory
//! and health API, backed by [`Store<R>`]. Responses are HATEOAS-lite: each carries an
//! `available_actions` list (empty in Phase 1 — actions arrive with the gated mutation
//! pipeline). The authed write API, the WebSocket live-state stream, the mTLS listener,
//! and hardening land in later phases. The daemon binds this router to its WireGuard
//! address only.
#![forbid(unsafe_code)]

mod auth;
mod jobs;
mod runbooks;
pub mod session;
pub mod tls;
pub mod webauthn;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{middleware, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use demon_clients::{IdentityClient, OpenSearchAudit, PrometheusClient, VaultClient, VaultError};
use demon_collect::SshTransport;
use demon_core::{available_actions, GuardedAction, HealthSnapshot, Residency};
use demon_store::{Store, StoreError};

pub use jobs::JobStore;
pub use runbooks::RunbookStore;
pub use session::{AuthCtx, PendingStore, SessionStore};

/// Current wall-clock time in epoch milliseconds.
pub(crate) fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(i64::MAX)
}

/// Current UTC time as an RFC3339 string (for B3 audit-event `ts`).
pub(crate) fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Shared, cheaply-cloneable server state, scoped to one residency group.
#[derive(Clone)]
pub struct AppState<R: Residency> {
    /// Build version (`CARGO_PKG_VERSION` of the daemon).
    pub version: &'static str,
    /// The residency-scoped store.
    pub store: Store<R>,
    /// Live health-snapshot feed (fanned out by the poll worker).
    pub events: broadcast::Sender<HealthSnapshot>,
    /// OIDC client for operator login (`None` ⇒ auth unconfigured ⇒ API stays closed).
    pub identity: Option<IdentityClient>,
    /// Active operator sessions.
    pub sessions: SessionStore,
    /// In-flight PKCE auth state.
    pub pending: PendingStore,
    /// Group-local OpenSearch audit shipper (`None` ⇒ audit fan-out disabled).
    pub audit: Option<OpenSearchAudit>,
    /// This daemon's node hostname (B3 `node` field).
    pub node: String,
    /// **DEV ONLY**: bypass the auth gate (`DEMON_DEV_NO_AUTH`). Must never be set in
    /// production — the daemon warns loudly at startup when it is.
    pub dev_no_auth: bool,
    /// In-memory guarded-mutation job store.
    pub jobs: JobStore,
    /// In-memory runbook-run store.
    pub runbooks: RunbookStore,
    /// SSH transport used to execute mutations + verify (shared with the poller).
    pub transport: SshTransport,
    /// WebAuthn relying party for step-up (`None` ⇒ step-up unavailable; destructive/
    /// secret/CA actions then cannot meet the factor gate unless dev-bypassed).
    pub webauthn: Option<std::sync::Arc<webauthn::WebauthnCtx>>,
    /// Group-local Prometheus client for the bottleneck load feed (`None` ⇒ disabled).
    pub metrics: Option<PrometheusClient>,
    /// terrapi-vault secrets-broker client (`None` ⇒ secrets layer not configured).
    pub vault: Option<VaultClient>,
}

/// Build the router for residency group `R`. Liveness (`/health`, `/version`) and the
/// `/auth/*` login routes are public; everything under `/api/v1` is gated by
/// [`auth::require_auth`] (fail closed).
pub fn router<R: Residency>(state: AppState<R>) -> Router {
    let protected = Router::new()
        .route("/api/v1/me", get(me::<R>))
        .route("/api/v1/residency-groups", get(residency_groups::<R>))
        .route("/api/v1/hosts", get(hosts::<R>))
        .route("/api/v1/hosts/{id}", get(host_detail::<R>))
        .route("/api/v1/hosts/{id}/health", get(host_health::<R>))
        .route("/api/v1/hosts/{id}/load", get(host_load::<R>))
        .route("/api/v1/services", get(services::<R>))
        .route("/api/v1/tenants", get(tenants::<R>))
        .route("/api/v1/audit", get(audit_list::<R>))
        .route("/api/v1/audit/verify", get(audit_verify::<R>))
        .route("/api/v1/secrets/status", get(secrets_status::<R>))
        .route("/api/v1/stream", get(stream::<R>))
        .route("/api/v1/jobs", get(jobs::list::<R>).post(jobs::create::<R>))
        .route("/api/v1/jobs/{id}", get(jobs::get::<R>))
        .route("/api/v1/jobs/{id}/approve", post(jobs::approve::<R>))
        .route("/api/v1/jobs/{id}/confirm", post(jobs::confirm::<R>))
        .route("/api/v1/jobs/{id}/apply", post(jobs::apply::<R>))
        .route("/api/v1/runbooks", get(runbooks::catalog::<R>))
        .route("/api/v1/runbooks/{id}/runs", post(runbooks::start::<R>))
        .route(
            "/api/v1/runbooks/runs/{run_id}",
            get(runbooks::get_run::<R>),
        )
        .route(
            "/api/v1/webauthn/register/start",
            post(webauthn::register_start::<R>),
        )
        .route(
            "/api/v1/webauthn/register/finish",
            post(webauthn::register_finish::<R>),
        )
        .route(
            "/api/v1/jobs/{id}/stepup/start",
            post(webauthn::stepup_start::<R>),
        )
        .route(
            "/api/v1/jobs/{id}/stepup/finish",
            post(webauthn::stepup_finish::<R>),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth::<R>,
        ));

    Router::new()
        .route("/health", get(health::<R>))
        .route("/version", get(version::<R>))
        .route("/auth/login", get(auth::login::<R>))
        .route("/auth/callback", get(auth::callback::<R>))
        .route("/auth/logout", get(auth::logout::<R>))
        .merge(protected)
        .with_state(state)
}

// ---- WebSocket live-state stream -------------------------------------------

async fn stream<R: Residency>(State(s): State<AppState<R>>, ws: WebSocketUpgrade) -> Response {
    let rx = s.events.subscribe();
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

/// Forward broadcast health snapshots to one WebSocket client until it disconnects.
async fn pump_events(mut socket: WebSocket, mut rx: broadcast::Receiver<HealthSnapshot>) {
    loop {
        match rx.recv().await {
            Ok(snapshot) => {
                let Ok(text) = serde_json::to_string(&snapshot) else {
                    continue;
                };
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break; // client gone
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "ws subscriber lagged");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

// ---- response envelopes (HATEOAS-lite) -------------------------------------

#[derive(Serialize)]
struct ListResponse<T> {
    data: Vec<T>,
    available_actions: Vec<String>,
}

impl<T> ListResponse<T> {
    fn new(data: Vec<T>) -> Self {
        Self {
            data,
            available_actions: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct ItemResponse<T> {
    data: T,
    available_actions: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    pub(crate) error: String,
}

/// Wraps a [`StoreError`] as a `500` JSON response.
struct AppError(StoreError);

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        AppError(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "request failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: self.0.to_string(),
            }),
        )
            .into_response()
    }
}

// ---- liveness --------------------------------------------------------------

#[derive(Serialize)]
struct Health {
    status: &'static str,
    region: &'static str,
}

async fn health<R: Residency>(State(_s): State<AppState<R>>) -> Json<Health> {
    Json(Health {
        status: "ok",
        region: R::REGION.as_str(),
    })
}

#[derive(Serialize)]
struct VersionInfo {
    service: &'static str,
    version: &'static str,
    region: &'static str,
}

async fn version<R: Residency>(State(s): State<AppState<R>>) -> Json<VersionInfo> {
    Json(VersionInfo {
        service: "proximiio.demon",
        version: s.version,
        region: R::REGION.as_str(),
    })
}

// ---- read API --------------------------------------------------------------

#[derive(Serialize)]
struct MeDto {
    sub: String,
    roles: Vec<&'static str>,
    residency: &'static str,
    factor: demon_core::FactorLevel,
}

async fn me<R: Residency>(Extension(ctx): Extension<AuthCtx>) -> Json<MeDto> {
    let roles = ctx
        .principal
        .roles
        .iter()
        .map(|r| match r {
            demon_core::Role::Viewer => "viewer",
            demon_core::Role::Operator => "operator",
            demon_core::Role::Senior => "senior",
            demon_core::Role::BreakGlass => "break_glass",
        })
        .collect();
    Json(MeDto {
        sub: ctx.principal.sub.clone(),
        roles,
        residency: R::REGION.as_str(),
        factor: ctx.factor,
    })
}

async fn residency_groups<R: Residency>(
    State(_s): State<AppState<R>>,
) -> Json<ListResponse<&'static str>> {
    // One daemon serves exactly one group; it only knows its own.
    Json(ListResponse::new(vec![R::REGION.as_str()]))
}

async fn hosts<R: Residency>(
    State(s): State<AppState<R>>,
) -> Result<Json<ListResponse<demon_core::Host>>, AppError> {
    Ok(Json(ListResponse::new(s.store.list_hosts().await?)))
}

async fn host_detail<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    match s.store.get_host(&id).await? {
        Some(h) => {
            let actions = available_actions(&ctx.principal.roles)
                .into_iter()
                .map(|a: GuardedAction| a.id().to_owned())
                .collect();
            Ok(Json(ItemResponse {
                data: h,
                available_actions: actions,
            })
            .into_response())
        }
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: format!("host {id} not found"),
            }),
        )
            .into_response()),
    }
}

async fn host_health<R: Residency>(
    State(s): State<AppState<R>>,
    Path(id): Path<String>,
) -> Result<Json<ListResponse<demon_core::HealthSnapshot>>, AppError> {
    Ok(Json(ListResponse::new(s.store.latest_health(&id).await?)))
}

#[derive(Deserialize)]
struct ServiceQuery {
    host_id: Option<String>,
}

async fn services<R: Residency>(
    State(s): State<AppState<R>>,
    Query(q): Query<ServiceQuery>,
) -> Result<Json<ListResponse<demon_core::Service>>, AppError> {
    Ok(Json(ListResponse::new(
        s.store.list_services(q.host_id.as_deref()).await?,
    )))
}

async fn tenants<R: Residency>(
    State(s): State<AppState<R>>,
) -> Result<Json<ListResponse<demon_core::Tenant>>, AppError> {
    Ok(Json(ListResponse::new(s.store.list_tenants().await?)))
}

#[derive(Serialize)]
struct LoadReport {
    load: demon_core::ServiceLoad,
    findings: Vec<demon_core::Finding>,
    confidence: Option<demon_core::Confidence>,
}

async fn host_load<R: Residency>(State(s): State<AppState<R>>, Path(id): Path<String>) -> Response {
    let Some(prom) = s.metrics.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "metrics (Prometheus) not configured".into(),
            }),
        )
            .into_response();
    };
    let host = match s.store.get_host(&id).await {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: format!("host {id} not found"),
                }),
            )
                .into_response()
        }
        Err(e) => return AppError(e).into_response(),
    };
    let selector = format!("instance=~\"{}.*\"", host.fqdn);
    match prom.host_load(&selector).await {
        Ok(load) => {
            let findings = demon_core::detect(&load);
            let confidence = demon_core::bottleneck::overall_confidence(&findings);
            Json(LoadReport {
                load,
                findings,
                confidence,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
}

async fn audit_list<R: Residency>(
    State(s): State<AppState<R>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<ListResponse<demon_core::AuditRecord>>, AppError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    Ok(Json(ListResponse::new(s.store.list_audit(limit).await?)))
}

#[derive(Serialize)]
struct AuditVerify {
    intact: bool,
}

async fn audit_verify<R: Residency>(
    State(s): State<AppState<R>>,
) -> Result<Json<AuditVerify>, AppError> {
    Ok(Json(AuditVerify {
        intact: s.store.verify_audit().await?,
    }))
}

#[derive(Serialize)]
struct SecretsStatus {
    configured: bool,
    sealed: Option<bool>,
    error: Option<String>,
}

/// Readiness of the vault secrets broker (configured? sealed?).
async fn secrets_status<R: Residency>(State(s): State<AppState<R>>) -> Json<SecretsStatus> {
    let Some(vault) = s.vault.as_ref() else {
        return Json(SecretsStatus {
            configured: false,
            sealed: None,
            error: None,
        });
    };
    match vault.seal_status().await {
        Ok(st) => Json(SecretsStatus {
            configured: true,
            sealed: Some(st.sealed),
            error: None,
        }),
        Err(VaultError::Sealed) => Json(SecretsStatus {
            configured: true,
            sealed: Some(true),
            error: None,
        }),
        Err(e) => Json(SecretsStatus {
            configured: true,
            sealed: None,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::Eu;

    #[tokio::test]
    async fn router_builds_with_store() {
        let store = Store::<Eu>::open_in_memory().await.unwrap();
        let (events, _rx) = broadcast::channel(16);
        let _ = router(AppState {
            version: "0.0.0",
            store,
            events,
            identity: None,
            sessions: SessionStore::new(),
            pending: PendingStore::new(),
            audit: None,
            node: "test".into(),
            dev_no_auth: false,
            jobs: JobStore::new(),
            runbooks: RunbookStore::new(),
            transport: SshTransport::new("ops"),
            webauthn: None,
            metrics: None,
            vault: None,
        });
    }
}
