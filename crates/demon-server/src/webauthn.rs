//! WebAuthn step-up — platform authenticator (Touch ID / Windows Hello) providing
//! **touch-per-op** confirmation for destructive/secret/CA actions.
//!
//! An operator registers a passkey once; then, to *apply* a job whose action class
//! demands step-up ([`FactorPolicy`](demon_core::FactorPolicy)), they perform a fresh
//! WebAuthn assertion scoped to **that job** — not a cached session bit. A successful
//! assertion records [`FactorLevel::WebAuthnPlatform`] on the job, which the apply gate
//! then accepts.
//!
//! Passkeys + in-flight ceremony state are in-memory for now (per the security doc's
//! interim posture); durable storage lands with the persistent session work.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use uuid::Uuid;
use webauthn_rs::prelude::{
    CredentialID, Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Url, Webauthn, WebauthnBuilder,
};

use demon_core::{FactorLevel, Residency};

use crate::session::AuthCtx;
use crate::{AppState, ErrorBody};

/// WebAuthn relying-party + in-memory credential/ceremony stores.
pub struct WebauthnCtx {
    webauthn: Webauthn,
    passkeys: Mutex<HashMap<String, Vec<Passkey>>>,
    reg: Mutex<HashMap<String, PasskeyRegistration>>,
    auth: Mutex<HashMap<String, (String, PasskeyAuthentication)>>,
}

impl WebauthnCtx {
    /// Build for a relying-party id + origin (dev: `localhost` + `http://localhost:5179`;
    /// prod: the demon domain over HTTPS).
    ///
    /// # Errors
    /// Returns a message if the origin is invalid or the builder rejects the config.
    pub fn new(rp_id: &str, origin: &str) -> Result<Self, String> {
        let url = Url::parse(origin).map_err(|e| e.to_string())?;
        let webauthn = WebauthnBuilder::new(rp_id, &url)
            .map_err(|e| e.to_string())?
            .rp_name("proximiio.demon")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            webauthn,
            passkeys: Mutex::new(HashMap::new()),
            reg: Mutex::new(HashMap::new()),
            auth: Mutex::new(HashMap::new()),
        })
    }

    fn passkeys_for(&self, sub: &str) -> Vec<Passkey> {
        self.passkeys
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(sub)
            .cloned()
            .unwrap_or_default()
    }
}

fn err(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, Json(ErrorBody { error: msg.into() })).into_response()
}

/// `POST /api/v1/webauthn/register/start` — begin passkey registration.
pub(crate) async fn register_start<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
) -> Response {
    let Some(wa) = s.webauthn.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "WebAuthn is not configured");
    };
    let sub = ctx.principal.sub.clone();
    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, sub.as_bytes());
    let exclude: Vec<CredentialID> = wa.passkeys_for(&sub).iter().map(|p| p.cred_id().clone()).collect();
    match wa
        .webauthn
        .start_passkey_registration(user_id, &sub, &sub, Some(exclude))
    {
        Ok((ccr, reg)) => {
            wa.reg.lock().unwrap_or_else(PoisonError::into_inner).insert(sub, reg);
            Json(ccr).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// `POST /api/v1/webauthn/register/finish` — complete passkey registration.
pub(crate) async fn register_finish<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Json(cred): Json<RegisterPublicKeyCredential>,
) -> Response {
    let Some(wa) = s.webauthn.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "WebAuthn is not configured");
    };
    let sub = ctx.principal.sub.clone();
    let reg = wa.reg.lock().unwrap_or_else(PoisonError::into_inner).remove(&sub);
    let Some(reg) = reg else {
        return err(StatusCode::BAD_REQUEST, "no registration in progress");
    };
    match wa.webauthn.finish_passkey_registration(&cred, &reg) {
        Ok(passkey) => {
            wa.passkeys
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(sub)
                .or_default()
                .push(passkey);
            Json(serde_json::json!({ "registered": true })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

/// `POST /api/v1/jobs/{id}/stepup/start` — begin a per-job assertion.
pub(crate) async fn stepup_start<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(job_id): Path<String>,
) -> Response {
    let Some(wa) = s.webauthn.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "WebAuthn is not configured");
    };
    let passkeys = wa.passkeys_for(&ctx.principal.sub);
    if passkeys.is_empty() {
        return err(StatusCode::BAD_REQUEST, "no passkey registered — register one first");
    }
    match wa.webauthn.start_passkey_authentication(&passkeys) {
        Ok((rcr, auth)) => {
            wa.auth
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(job_id, (ctx.principal.sub.clone(), auth));
            Json(rcr).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// `POST /api/v1/jobs/{id}/stepup/finish` — complete the assertion; on success record
/// platform-WebAuthn step-up on the job.
pub(crate) async fn stepup_finish<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(job_id): Path<String>,
    Json(cred): Json<PublicKeyCredential>,
) -> Response {
    let Some(wa) = s.webauthn.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "WebAuthn is not configured");
    };
    let entry = wa.auth.lock().unwrap_or_else(PoisonError::into_inner).remove(&job_id);
    let Some((sub, auth)) = entry else {
        return err(StatusCode::BAD_REQUEST, "no step-up in progress for this job");
    };
    if sub != ctx.principal.sub {
        return err(StatusCode::FORBIDDEN, "step-up belongs to a different operator");
    }
    match wa.webauthn.finish_passkey_authentication(&cred, &auth) {
        Ok(_result) => {
            if s.jobs.mark_stepped_up(&job_id, FactorLevel::WebAuthnPlatform) {
                Json(serde_json::json!({ "stepped_up": "webauthn_platform" })).into_response()
            } else {
                err(StatusCode::NOT_FOUND, "job disappeared")
            }
        }
        Err(e) => err(StatusCode::FORBIDDEN, format!("assertion failed: {e}")),
    }
}
