//! `demon-clients` — typed API clients for the services demon controls and depends on.
//!
//! Phase 2 adds the [`identity`] OIDC client (operator login). The kalista / vulture /
//! vault clients (adapted from proximiio-tui's `*_api.rs`) and the OpenSearch audit
//! shipper land alongside the phases that need them.
#![forbid(unsafe_code)]

pub mod identity;
pub mod opensearch;

pub use identity::{
    authorize_url, decode_claims, pkce, random_state, Discovery, IdentityClient, IdentityError,
    OidcConfig, Pkce, TokenResponse,
};
pub use opensearch::{index_for, AuditShipError, OpenSearchAudit};
