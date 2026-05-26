//! `demon-server` — the Axum REST (+ later WebSocket) surface.
//!
//! Phase 1 ships the unauthenticated liveness probes plus the **read-only** inventory
//! and health API, backed by [`Store<R>`]. Responses are HATEOAS-lite: each carries an
//! `available_actions` list (empty in Phase 1 — actions arrive with the gated mutation
//! pipeline). The authed write API, the WebSocket live-state stream, the mTLS listener,
//! and hardening land in later phases. The daemon binds this router to its WireGuard
//! address only.
#![forbid(unsafe_code)]

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use demon_core::Residency;
use demon_store::{Store, StoreError};

/// Shared, cheaply-cloneable server state, scoped to one residency group.
#[derive(Debug, Clone)]
pub struct AppState<R: Residency> {
    /// Build version (`CARGO_PKG_VERSION` of the daemon).
    pub version: &'static str,
    /// The residency-scoped store.
    pub store: Store<R>,
}

/// Build the Phase 1 router for residency group `R`.
pub fn router<R: Residency>(state: AppState<R>) -> Router {
    Router::new()
        .route("/health", get(health::<R>))
        .route("/version", get(version::<R>))
        .route("/api/v1/residency-groups", get(residency_groups::<R>))
        .route("/api/v1/hosts", get(hosts::<R>))
        .route("/api/v1/hosts/{id}", get(host_detail::<R>))
        .route("/api/v1/hosts/{id}/health", get(host_health::<R>))
        .route("/api/v1/services", get(services::<R>))
        .route("/api/v1/tenants", get(tenants::<R>))
        .with_state(state)
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

impl<T> ItemResponse<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            available_actions: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
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
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    match s.store.get_host(&id).await? {
        Some(h) => Ok(Json(ItemResponse::new(h)).into_response()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::Eu;

    #[tokio::test]
    async fn router_builds_with_store() {
        let store = Store::<Eu>::open_in_memory().await.unwrap();
        let _ = router(AppState {
            version: "0.0.0",
            store,
        });
    }
}
