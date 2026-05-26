//! mTLS server configuration.
//!
//! Builds a rustls [`ServerConfig`] that **requires** a client certificate chaining to
//! the fleet Root CA (`WebPkiClientVerifier`, no anonymous clients). Combined with the
//! WireGuard-only bind, this is the defense-in-depth from §5 of the security doc: even
//! an intra-WG attacker needs a valid operator client cert.
//!
//! The process crypto provider must be installed once at startup (the daemon does this
//! before serving).

use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};

/// Install the process-wide rustls crypto provider (ring). Idempotent; call once at
/// startup before building any TLS config. A second call is a no-op.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Errors building the TLS config.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    /// Reading/parsing PEM failed.
    #[error("pem io error: {0}")]
    Io(#[from] std::io::Error),
    /// No private key found in the key PEM.
    #[error("no private key found in key PEM")]
    NoPrivateKey,
    /// Building the client-cert verifier failed (e.g. empty/invalid CA set).
    #[error("client verifier error: {0}")]
    Verifier(#[from] rustls::server::VerifierBuilderError),
    /// rustls configuration error.
    #[error("rustls error: {0}")]
    Rustls(#[from] rustls::Error),
}

/// Build a mutual-TLS [`ServerConfig`]: presents `server_cert`/`server_key` and
/// **requires** a client cert verifiable against `client_ca` (the fleet Root CA).
///
/// # Errors
/// [`TlsError`] if any PEM is malformed, the key is missing, the CA set is empty/invalid,
/// or rustls rejects the cert/key.
pub fn server_config(
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
    client_ca_pem: &[u8],
) -> Result<ServerConfig, TlsError> {
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut &server_cert_pem[..]).collect::<Result<_, _>>()?;
    let key: PrivateKeyDer<'static> =
        rustls_pemfile::private_key(&mut &server_key_pem[..])?.ok_or(TlsError::NoPrivateKey)?;

    let mut roots = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut &client_ca_pem[..]) {
        roots.add(cert?)?;
    }
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots)).build()?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)?;
    Ok(config)
}
