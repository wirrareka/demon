//! OIDC login routes and the fail-closed auth middleware.
//!
//! `/auth/login` starts Authorization Code + PKCE-S256 against the residency group's
//! identity issuer; `/auth/callback` exchanges the code, verifies the token, maps it
//! to a [`Principal`](demon_core::authorize::Principal) (residency-checked), and opens
//! a session cookie; `/auth/logout` drops it. [`require_auth`] gates the API: no valid
//! session ⇒ `401` (fail closed).
//!
//! Cookie is `HttpOnly; SameSite=Strict` today; `Secure` + the `__Host-` prefix are
//! added with the TLS/mTLS listener.

use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use serde::Deserialize;

use demon_core::{principal_from_claims, FactorLevel, Residency};
use demon_clients::{authorize_url, pkce, random_state};

use crate::session::{Pending, Session};
use crate::{now_ms, AppState, ErrorBody};

/// Session cookie name. Prod adds the `__Host-` prefix + `Secure` over TLS.
const COOKIE: &str = "demon_session";
/// OIDC scopes requested.
const SCOPE: &str = "openid";
/// Operator session lifetime (8 h hard cap — matches the vault session contract).
const SESSION_TTL_SECS: i64 = 28_800;

fn bad_gateway(error: String) -> Response {
    (StatusCode::BAD_GATEWAY, Json(ErrorBody { error })).into_response()
}

/// Extract the session id from the `Cookie` header.
fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .map(|(_, v)| v.to_owned())
}

fn set_cookie(value: &str, max_age: i64) -> String {
    format!("{COOKIE}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}")
}

/// `GET /auth/login` — redirect to the identity authorize endpoint with PKCE.
pub(crate) async fn login<R: Residency>(State(s): State<AppState<R>>) -> Response {
    let Some(idc) = s.identity.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "OIDC is not configured on this daemon".into(),
            }),
        )
            .into_response();
    };
    let disc = match idc.discover().await {
        Ok(d) => d,
        Err(e) => return bad_gateway(e.to_string()),
    };
    let p = pkce();
    let state = random_state();
    s.pending.insert(
        state.clone(),
        Pending {
            verifier: p.verifier,
            created_at: now_ms(),
        },
    );
    match authorize_url(
        &disc.authorization_endpoint,
        idc.client_id(),
        idc.redirect_uri(),
        &state,
        &p.challenge,
        SCOPE,
    ) {
        Ok(url) => Redirect::to(&url).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub(crate) struct Callback {
    code: String,
    state: String,
}

/// `GET /auth/callback` — exchange the code, verify the token, open a session.
pub(crate) async fn callback<R: Residency>(
    State(s): State<AppState<R>>,
    Query(q): Query<Callback>,
) -> Response {
    let Some(idc) = s.identity.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "OIDC not configured").into_response();
    };
    let Some(pending) = s.pending.take(&q.state) else {
        return (StatusCode::BAD_REQUEST, "unknown or replayed state").into_response();
    };
    let disc = match idc.discover().await {
        Ok(d) => d,
        Err(e) => return bad_gateway(e.to_string()),
    };
    let tokens = match idc.exchange_code(&disc.token_endpoint, &q.code, &pending.verifier).await {
        Ok(t) => t,
        Err(e) => return bad_gateway(e.to_string()),
    };
    let claims = match idc
        .verify_token(&tokens.access_token, &disc.jwks_uri, idc.client_id())
        .await
    {
        Ok(c) => c,
        Err(e) => return bad_gateway(e.to_string()),
    };
    let principal = match principal_from_claims(&claims, R::REGION) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: e.to_string(),
                }),
            )
                .into_response()
        }
    };
    let sid = random_state();
    s.sessions.insert(
        sid.clone(),
        Session {
            principal,
            factor: FactorLevel::None,
            expires_at: now_ms() + SESSION_TTL_SECS * 1000,
        },
    );
    tracing::info!(region = %R::REGION, "operator session opened");
    (
        [(header::SET_COOKIE, set_cookie(&sid, SESSION_TTL_SECS))],
        Redirect::to("/"),
    )
        .into_response()
}

/// `GET /auth/logout` — drop the session and clear the cookie.
pub(crate) async fn logout<R: Residency>(
    State(s): State<AppState<R>>,
    headers: HeaderMap,
) -> Response {
    if let Some(id) = session_cookie(&headers) {
        s.sessions.remove(&id);
    }
    (
        [(header::SET_COOKIE, set_cookie("", 0))],
        Redirect::to("/"),
    )
        .into_response()
}

/// Fail-closed auth gate: requires a valid, unexpired session cookie.
pub(crate) async fn require_auth<R: Residency>(
    State(s): State<AppState<R>>,
    req: Request,
    next: Next,
) -> Response {
    let authed = session_cookie(req.headers())
        .and_then(|id| s.sessions.get(&id, now_ms()))
        .is_some();
    if authed {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: "authentication required".into(),
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_cookie_among_others() {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, "foo=1; demon_session=abc123; bar=2".parse().unwrap());
        assert_eq!(session_cookie(&h).as_deref(), Some("abc123"));
        assert!(session_cookie(&HeaderMap::new()).is_none());
    }

    #[test]
    fn clear_cookie_has_zero_max_age() {
        assert!(set_cookie("", 0).contains("Max-Age=0"));
        assert!(set_cookie("x", 10).contains("HttpOnly"));
        assert!(set_cookie("x", 10).contains("SameSite=Strict"));
    }
}
