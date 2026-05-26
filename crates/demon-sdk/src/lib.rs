//! `demon-sdk` — the single typed client for the demon API.
//!
//! The TUI (and, later, the OpenAPI-generated web types) consume this so there is one
//! source of truth for request/response shapes. Models re-export `demon-core` view
//! types; envelopes mirror `demon-server`'s HATEOAS-lite responses.
#![forbid(unsafe_code)]

use serde::Deserialize;

pub use demon_core::{
    Fleet, HealthSnapshot, HealthStatus, Host, Region, Service, TargetKind, Tenant,
};

/// A collection response: `{ "data": [...], "available_actions": [...] }`.
#[derive(Debug, Clone, Deserialize)]
pub struct ListResponse<T> {
    /// The items.
    pub data: Vec<T>,
    /// Discoverable actions (empty until the mutation pipeline lands).
    #[serde(default)]
    pub available_actions: Vec<String>,
}

/// `/version` payload.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionInfo {
    /// Service name (`proximiio.demon`).
    pub service: String,
    /// Build version.
    pub version: String,
    /// Residency group served.
    pub region: String,
}

/// Errors talking to the demon API.
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    /// Transport/HTTP error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// A typed client for one demon instance.
#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    http: reqwest::Client,
}

impl Client {
    /// Build a client for `base` (e.g. `http://10.200.0.2:8787`). Trailing slash is
    /// trimmed.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        let base = base.into().trim_end_matches('/').to_owned();
        Self {
            base,
            http: reqwest::Client::new(),
        }
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, SdkError> {
        Ok(self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }

    /// `GET /version`.
    ///
    /// # Errors
    /// [`SdkError`] on transport/decoding failure.
    pub async fn version(&self) -> Result<VersionInfo, SdkError> {
        self.get_json("/version").await
    }

    /// `GET /api/v1/hosts`.
    ///
    /// # Errors
    /// [`SdkError`] on transport/decoding failure.
    pub async fn hosts(&self) -> Result<Vec<Host>, SdkError> {
        let r: ListResponse<Host> = self.get_json("/api/v1/hosts").await?;
        Ok(r.data)
    }

    /// `GET /api/v1/hosts/{id}/health`.
    ///
    /// # Errors
    /// [`SdkError`] on transport/decoding failure.
    pub async fn host_health(&self, id: &str) -> Result<Vec<HealthSnapshot>, SdkError> {
        let r: ListResponse<HealthSnapshot> =
            self.get_json(&format!("/api/v1/hosts/{id}/health")).await?;
        Ok(r.data)
    }

    /// `GET /api/v1/tenants`.
    ///
    /// # Errors
    /// [`SdkError`] on transport/decoding failure.
    pub async fn tenants(&self) -> Result<Vec<Tenant>, SdkError> {
        let r: ListResponse<Tenant> = self.get_json("/api/v1/tenants").await?;
        Ok(r.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_response_deserializes_with_default_actions() {
        let json = r#"{"data":[]}"#;
        let r: ListResponse<Host> = serde_json::from_str(json).unwrap();
        assert!(r.data.is_empty());
        assert!(r.available_actions.is_empty());
    }

    #[test]
    fn base_trailing_slash_trimmed() {
        let c = Client::new("http://x:8787/");
        assert_eq!(c.base, "http://x:8787");
    }
}
