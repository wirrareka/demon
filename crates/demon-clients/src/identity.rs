//! terrapi-identity OIDC client (operator login).
//!
//! Implements the contract answered in `coordination/inbox/identity/` (2026-05-26):
//! **Authorization Code + PKCE (S256, mandatory)**, token-endpoint auth
//! `client_secret_basic`, one confidential client **per residency group** against the
//! locked per-group issuer (`identity-eu.proximi.io` / `identity-uae.proximi.io`,
//! exact `iss` compare incl. trailing slash). Tokens are ES256, verified via JWKS.
//!
//! Pure helpers (PKCE, authorize-URL, claim decoding) are unit-tested; the
//! discovery / JWKS / token-exchange network calls are compile-checked.

use base64::Engine as _;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use demon_core::Claims;

/// Confidential-client configuration for one residency group.
///
/// `Debug` is hand-written to redact `client_secret` — the secret must never reach a
/// log line (non-negotiable).
#[derive(Clone)]
pub struct OidcConfig {
    /// Issuer URL (exact, incl. trailing slash), e.g. `https://identity-eu.proximi.io/`.
    pub issuer: String,
    /// Registered `client_id`.
    pub client_id: String,
    /// Client secret (operator-supplied at registration; sent via `client_secret_basic`).
    pub client_secret: String,
    /// Exact registered redirect URI.
    pub redirect_uri: String,
}

impl std::fmt::Debug for OidcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcConfig")
            .field("issuer", &self.issuer)
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("redirect_uri", &self.redirect_uri)
            .finish()
    }
}

/// `IdentityClient` debug never exposes the config secret (see [`OidcConfig`]).
impl std::fmt::Debug for IdentityClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityClient")
            .field("cfg", &self.cfg)
            .finish_non_exhaustive()
    }
}

/// OIDC discovery document (subset).
#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
    /// Issuer (must equal the configured issuer).
    pub issuer: String,
    /// Authorization endpoint.
    pub authorization_endpoint: String,
    /// Token endpoint.
    pub token_endpoint: String,
    /// JWKS URI.
    pub jwks_uri: String,
}

/// Token-endpoint response (subset).
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// The access token (JWT).
    pub access_token: String,
    /// ID token, if returned.
    #[serde(default)]
    pub id_token: Option<String>,
    /// Token type (`Bearer`).
    #[serde(default)]
    pub token_type: String,
    /// Lifetime in seconds.
    #[serde(default)]
    pub expires_in: Option<i64>,
}

/// Errors from the identity client.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// HTTP/transport error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// JWT verification/decoding error.
    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    /// The discovery document's issuer did not match the configured issuer.
    #[error("issuer mismatch: discovery {found:?} != configured {expected:?}")]
    IssuerMismatch {
        /// Issuer in the discovery doc.
        found: String,
        /// Configured issuer.
        expected: String,
    },
    /// No JWK matched the token's `kid`.
    #[error("no JWK matched key id {0:?}")]
    UnknownKid(String),
}

/// A PKCE verifier/challenge pair (S256).
#[derive(Debug, Clone)]
pub struct Pkce {
    /// The high-entropy verifier (kept secret until token exchange).
    pub verifier: String,
    /// The S256 challenge (sent on the authorize request).
    pub challenge: String,
}

/// Generate a fresh PKCE pair: 32 random bytes → base64url verifier, challenge =
/// base64url(SHA-256(verifier)).
#[must_use]
pub fn pkce() -> Pkce {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let verifier = b64.encode(bytes);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = b64.encode(digest);
    Pkce {
        verifier,
        challenge,
    }
}

/// A random opaque value for the `state`/`nonce` CSRF parameters.
#[must_use]
pub fn random_state() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Build the authorization-code + PKCE-S256 redirect URL.
///
/// # Errors
/// Returns [`IdentityError::Http`] if the endpoint is not a valid URL.
pub fn authorize_url(
    authorization_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
    scope: &str,
) -> Result<String, IdentityError> {
    let url = reqwest::Url::parse_with_params(
        authorization_endpoint,
        &[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("scope", scope),
            ("state", state),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(|_| IdentityError::IssuerMismatch {
        found: authorization_endpoint.to_owned(),
        expected: "valid URL".to_owned(),
    })?;
    Ok(url.to_string())
}

/// Build a [`Validation`] for ES256 tokens from this issuer/audience.
fn validation(issuer: &str, audience: &str) -> Validation {
    let mut v = Validation::new(Algorithm::ES256);
    v.set_issuer(&[issuer]);
    v.set_audience(&[audience]);
    v.validate_exp = true;
    v
}

/// Decode and validate a token's claims with an explicit key + validation. Kept
/// generic so it is unit-testable offline (any algorithm).
///
/// # Errors
/// [`IdentityError::Jwt`] if signature/claims validation fails.
pub fn decode_claims(
    token: &str,
    key: &DecodingKey,
    validation: &Validation,
) -> Result<Claims, IdentityError> {
    let data = decode::<Claims>(token, key, validation)?;
    Ok(data.claims)
}

/// The identity OIDC client for one residency group.
#[derive(Clone)]
pub struct IdentityClient {
    cfg: OidcConfig,
    http: reqwest::Client,
}

impl IdentityClient {
    /// Construct a client.
    #[must_use]
    pub fn new(cfg: OidcConfig) -> Self {
        Self {
            cfg,
            http: reqwest::Client::new(),
        }
    }

    /// Registered `client_id`.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.cfg.client_id
    }

    /// Registered redirect URI.
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.cfg.redirect_uri
    }

    /// Configured issuer.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.cfg.issuer
    }

    /// Fetch and validate the OIDC discovery document. Enforces exact issuer match.
    ///
    /// # Errors
    /// [`IdentityError`] on transport failure or issuer mismatch.
    pub async fn discover(&self) -> Result<Discovery, IdentityError> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            self.cfg.issuer.trim_end_matches('/')
        );
        let disc: Discovery = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if disc.issuer != self.cfg.issuer {
            return Err(IdentityError::IssuerMismatch {
                found: disc.issuer,
                expected: self.cfg.issuer.clone(),
            });
        }
        Ok(disc)
    }

    /// Exchange an authorization code for tokens (`client_secret_basic`).
    ///
    /// # Errors
    /// [`IdentityError::Http`] on transport/HTTP-status failure.
    pub async fn exchange_code(
        &self,
        token_endpoint: &str,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<TokenResponse, IdentityError> {
        let resp = self
            .http
            .post(token_endpoint)
            .basic_auth(&self.cfg.client_id, Some(&self.cfg.client_secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.cfg.redirect_uri.as_str()),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    /// Verify an ES256 access token against the issuer's JWKS and return its claims.
    ///
    /// # Errors
    /// [`IdentityError`] on transport, unknown `kid`, or signature/claims failure.
    pub async fn verify_token(
        &self,
        token: &str,
        jwks_uri: &str,
        audience: &str,
    ) -> Result<Claims, IdentityError> {
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header.kid.clone().unwrap_or_default();
        let jwks: jsonwebtoken::jwk::JwkSet = self
            .http
            .get(jwks_uri)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| IdentityError::UnknownKid(kid.clone()))?;
        let key = DecodingKey::from_jwk(jwk)?;
        decode_claims(token, &key, &validation(&self.cfg.issuer, audience))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::Region;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    #[test]
    fn pkce_pair_is_well_formed() {
        let p = pkce();
        // 32 bytes -> 43 base64url chars (no padding).
        assert_eq!(p.verifier.len(), 43);
        assert_eq!(p.challenge.len(), 43);
        assert!(
            !p.verifier.contains('=') && !p.verifier.contains('+') && !p.verifier.contains('/')
        );
        // distinct calls differ
        assert_ne!(pkce().verifier, p.verifier);
    }

    #[test]
    fn authorize_url_contains_pkce_s256() {
        let url = authorize_url(
            "https://identity-eu.proximi.io/authorize",
            "demon-eu",
            "https://demon-eu.wg/auth/callback",
            "st4te",
            "ch4llenge",
            "openid roles",
        )
        .unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("client_id=demon-eu"));
        assert!(url.contains("scope=openid+roles") || url.contains("scope=openid%20roles"));
    }

    #[test]
    fn decode_claims_validates_iss_aud_and_maps_fields() {
        // Offline: use HS256 so no key infra is needed; the claim/validation logic is
        // identical to the ES256 production path.
        let secret = b"test-secret";
        let claims = json!({
            "sub": "op@x",
            "tenant_id": "00000000-0000-4000-8000-000000000000",
            "residency_group": "eu",
            "roles": ["operator"],
            "scope": "openid",
            "iss": "https://identity-eu.proximi.io/",
            "aud": "demon-eu",
            "exp": 9_999_999_999i64,
        });
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        let mut v = Validation::new(Algorithm::HS256);
        v.set_issuer(&["https://identity-eu.proximi.io/"]);
        v.set_audience(&["demon-eu"]);
        let out = decode_claims(&token, &DecodingKey::from_secret(secret), &v).unwrap();
        assert_eq!(out.sub, "op@x");
        assert_eq!(out.residency_group, Region::Eu);
        assert_eq!(out.roles, vec!["operator".to_owned()]);
    }

    #[test]
    fn decode_claims_rejects_wrong_audience() {
        let secret = b"test-secret";
        let claims = json!({
            "sub": "op@x", "residency_group": "eu", "roles": ["operator"],
            "iss": "https://identity-eu.proximi.io/", "aud": "someone-else",
            "exp": 9_999_999_999i64,
        });
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();
        let mut v = Validation::new(Algorithm::HS256);
        v.set_issuer(&["https://identity-eu.proximi.io/"]);
        v.set_audience(&["demon-eu"]);
        assert!(decode_claims(&token, &DecodingKey::from_secret(secret), &v).is_err());
    }
}
