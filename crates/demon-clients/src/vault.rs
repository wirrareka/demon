//! terrapi-vault **secrets broker** client (v1 contract).
//!
//! Typed against `terrapi-vault/spec/broker-openapi.yaml` v1.0.0 (the frozen "Secrets
//! broker" contract). Request/response shapes are stable even where the broker handler
//! is still a stub (`501`), so this module is wired now and works unchanged when the
//! SSH-CA / creds engines land.
//!
//! Auth is **mTLS over WireGuard** (client cert signed by the fleet Root CA) — the
//! daemon's single host-bound secret. Build the mTLS-configured `reqwest::Client` at
//! deploy and inject it via [`VaultClient::with_client`]; [`VaultClient::new`] is for
//! tests / a pre-configured client. The broker is per residency group (one instance,
//! no cross-group). Mutating ops never log/audit the returned secret material.

use serde::{Deserialize, Serialize};

/// Operational + transport errors from the broker.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// Transport/HTTP error.
    #[error("vault http error: {0}")]
    Http(#[from] reqwest::Error),
    /// Broker is sealed (`503`) — poll `seal-status` until an operator unseals.
    #[error("vault is sealed")]
    Sealed,
    /// Lease/session not renewable / already revoked / unknown (`409`).
    #[error("vault conflict: {0}")]
    Conflict(String),
    /// Engine not implemented yet (`501`) — shape is final, handler is a stub.
    #[error("vault op not implemented yet")]
    NotImplemented,
    /// mTLS client cert missing/invalid (`401`).
    #[error("vault unauthorized")]
    Unauthorized,
    /// Cert SAN role not permitted for this op (`403`).
    #[error("vault forbidden")]
    Forbidden,
    /// `404` — group mismatch, tenant not in group, or unknown role (body has the code).
    #[error("vault not found: {0}")]
    NotFound(String),
    /// `502` — backend (e.g. OpenSearch/RethinkDB) failed to create/drop the cred.
    #[error("vault backend error: {0}")]
    Backend(String),
}

/// Common lease envelope shared by every issued credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub lease_id: String,
    pub ttl_secs: u32,
    pub renewable: bool,
    pub max_ttl_secs: u32,
}

/// `GET /v1/sys/seal-status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealStatus {
    pub sealed: bool,
}

/// `GET /v1/{group}/ssh/ca`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshCaResponse {
    pub ca_public_key: String,
}

/// `POST /v1/{group}/ssh/sign` request.
#[derive(Debug, Clone, Serialize)]
pub struct SshSignRequest {
    pub public_key: String,
    /// `user` or `host` (host certs are group-scoped: `tenant_id` MUST be null).
    pub cert_type: String,
    pub principals: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// `POST /v1/{group}/ssh/sign` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshSignResponse {
    pub signed_certificate: String,
    pub serial: i64,
    #[serde(default)]
    pub valid_after: Option<String>,
    pub valid_before: String,
    pub lease_id: String,
}

/// `POST /v1/{group}/{tenant_id}/creds/{role}` response (lease + ephemeral creds).
#[derive(Clone, Deserialize)]
pub struct CredsResponse {
    #[serde(flatten)]
    pub lease: Lease,
    pub username: String,
    /// Generated secret — NEVER logged or audited (redacted in `Debug`).
    pub password: String,
}

impl std::fmt::Debug for CredsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredsResponse")
            .field("lease", &self.lease)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// `POST /v1/sys/session` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOpenResponse {
    pub session_id: String,
    pub ttl_secs: u32,
    pub idle_timeout_secs: u32,
}

/// `DELETE /v1/sys/session/{id}` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEndResponse {
    pub session_id: String,
    pub revoked_leases: Vec<String>,
}

/// `POST /v1/sys/leases/renew` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRenewResponse {
    pub lease_id: String,
    pub ttl_secs: u32,
}

/// `POST /v1/sys/leases/revoke` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub ok: bool,
}

/// A per-residency-group secrets-broker client.
#[derive(Debug, Clone)]
pub struct VaultClient {
    base: String,
    http: reqwest::Client,
}

impl VaultClient {
    /// Build with a default client (tests / a pre-configured client). Production should
    /// use [`with_client`](Self::with_client) with an mTLS-over-WG client.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self::with_client(base, reqwest::Client::new())
    }

    /// Build with a caller-provided (mTLS-configured) client.
    #[must_use]
    pub fn with_client(base: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            http,
        }
    }

    /// Map broker status codes to [`VaultError`] before decoding.
    async fn check(resp: reqwest::Response) -> Result<reqwest::Response, VaultError> {
        match resp.status().as_u16() {
            200..=299 => Ok(resp),
            401 => Err(VaultError::Unauthorized),
            403 => Err(VaultError::Forbidden),
            404 => Err(VaultError::NotFound(resp.text().await.unwrap_or_default())),
            409 => Err(VaultError::Conflict(resp.text().await.unwrap_or_default())),
            501 => Err(VaultError::NotImplemented),
            502 => Err(VaultError::Backend(resp.text().await.unwrap_or_default())),
            503 => Err(VaultError::Sealed),
            _ => {
                resp.error_for_status()?;
                Err(VaultError::NotImplemented)
            }
        }
    }

    /// `GET /healthz` — process liveness.
    ///
    /// # Errors
    /// [`VaultError`] on transport/status failure.
    pub async fn healthz(&self) -> Result<bool, VaultError> {
        Ok(self.http.get(format!("{}/healthz", self.base)).send().await?.status().is_success())
    }

    /// `GET /v1/sys/seal-status` — readiness (poll while sealed).
    ///
    /// # Errors
    /// [`VaultError`] on transport/status failure.
    pub async fn seal_status(&self) -> Result<SealStatus, VaultError> {
        let r = self.http.get(format!("{}/v1/sys/seal-status", self.base)).send().await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `POST /v1/sys/session` — open a session whose lifetime bounds child leases.
    ///
    /// # Errors
    /// [`VaultError`] (incl. `Sealed`) on failure.
    pub async fn open_session(
        &self,
        ttl_secs: Option<u32>,
        idle_timeout_secs: Option<u32>,
    ) -> Result<SessionOpenResponse, VaultError> {
        let r = self
            .http
            .post(format!("{}/v1/sys/session", self.base))
            .json(&serde_json::json!({ "ttl_secs": ttl_secs, "idle_timeout_secs": idle_timeout_secs }))
            .send()
            .await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `DELETE /v1/sys/session/{id}` — end a session, cascade-revoking child leases.
    ///
    /// # Errors
    /// [`VaultError`] on failure.
    pub async fn end_session(&self, id: &str) -> Result<SessionEndResponse, VaultError> {
        let r = self.http.delete(format!("{}/v1/sys/session/{id}", self.base)).send().await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `POST /v1/sys/leases/renew`.
    ///
    /// # Errors
    /// [`VaultError::Conflict`] if not renewable, or transport failure.
    pub async fn renew_lease(
        &self,
        lease_id: &str,
        increment_secs: u32,
    ) -> Result<LeaseRenewResponse, VaultError> {
        let r = self
            .http
            .post(format!("{}/v1/sys/leases/renew", self.base))
            .json(&serde_json::json!({ "lease_id": lease_id, "increment_secs": increment_secs }))
            .send()
            .await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `POST /v1/sys/leases/revoke`.
    ///
    /// # Errors
    /// [`VaultError`] on failure.
    pub async fn revoke_lease(&self, lease_id: &str) -> Result<Ack, VaultError> {
        let r = self
            .http
            .post(format!("{}/v1/sys/leases/revoke", self.base))
            .json(&serde_json::json!({ "lease_id": lease_id }))
            .send()
            .await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `GET /v1/{group}/ssh/ca` — the group-scoped SSH CA trust anchor. (Stub: `501`.)
    ///
    /// # Errors
    /// [`VaultError::NotImplemented`] until the SSH-CA engine lands, or transport failure.
    pub async fn ssh_ca(&self, group: &str) -> Result<SshCaResponse, VaultError> {
        let r = self.http.get(format!("{}/v1/{group}/ssh/ca", self.base)).send().await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `POST /v1/{group}/ssh/sign` — sign an SSH public key. (Stub: `501`.)
    ///
    /// # Errors
    /// [`VaultError::NotImplemented`] until the SSH-CA engine lands, or transport failure.
    pub async fn ssh_sign(
        &self,
        group: &str,
        req: &SshSignRequest,
    ) -> Result<SshSignResponse, VaultError> {
        let r = self
            .http
            .post(format!("{}/v1/{group}/ssh/sign", self.base))
            .json(req)
            .send()
            .await?;
        Ok(Self::check(r).await?.json().await?)
    }

    /// `POST /v1/{group}/{tenant_id}/creds/{role}` — issue ephemeral backend creds.
    /// (Stub: `501`.)
    ///
    /// # Errors
    /// [`VaultError::NotImplemented`] until the creds engine lands, or transport failure.
    pub async fn issue_creds(
        &self,
        group: &str,
        tenant_id: &str,
        role: &str,
        ttl_secs: Option<u32>,
    ) -> Result<CredsResponse, VaultError> {
        let r = self
            .http
            .post(format!("{}/v1/{group}/{tenant_id}/creds/{role}", self.base))
            .json(&serde_json::json!({ "ttl_secs": ttl_secs }))
            .send()
            .await?;
        Ok(Self::check(r).await?.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creds_response_flattens_lease_and_redacts_password() {
        let json = r#"{"lease_id":"l1","ttl_secs":900,"renewable":true,"max_ttl_secs":1800,
            "username":"demon_audit_x","password":"s3cr3t"}"#;
        let c: CredsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(c.lease.lease_id, "l1");
        assert_eq!(c.lease.ttl_secs, 900);
        assert_eq!(c.username, "demon_audit_x");
        assert_eq!(c.password, "s3cr3t");
        // Debug must not leak the secret.
        assert!(format!("{c:?}").contains("<redacted>"));
        assert!(!format!("{c:?}").contains("s3cr3t"));
    }

    #[test]
    fn seal_status_and_session_parse() {
        let s: SealStatus = serde_json::from_str(r#"{"sealed":true}"#).unwrap();
        assert!(s.sealed);
        let so: SessionOpenResponse =
            serde_json::from_str(r#"{"session_id":"sess1","ttl_secs":28800,"idle_timeout_secs":1800}"#)
                .unwrap();
        assert_eq!(so.session_id, "sess1");
        assert_eq!(so.ttl_secs, 28800);
    }
}
