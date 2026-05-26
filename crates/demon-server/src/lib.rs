//! `demon-server` — the Axum REST + WebSocket surface.
//!
//! Phase 0 ships only the unauthenticated liveness surface (`/health`, `/version`).
//! The authed read API, WebSocket live-state stream, mTLS listener, and hardening
//! land in later phases. The daemon binds this router to its WireGuard address only.
#![forbid(unsafe_code)]

use axum::{extract::State, routing::get, Json, Router};
use demon_core::Region;

/// Shared, cheaply-cloneable server state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Build version (`CARGO_PKG_VERSION` of the daemon).
    pub version: &'static str,
    /// The residency group this daemon serves.
    pub region: Region,
}

/// Build the Phase 0 router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .with_state(state)
}

/// Liveness probe — always `200` while the process is up.
async fn health(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "region": s.region.as_str(),
    }))
}

/// Build/version information.
async fn version(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "proximiio.demon",
        "version": s.version,
        "region": s.region.as_str(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds() {
        let _ = router(AppState {
            version: "0.0.0",
            region: Region::Eu,
        });
    }
}
