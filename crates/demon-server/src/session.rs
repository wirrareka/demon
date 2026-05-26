//! In-memory server-side sessions and pending-auth (PKCE) state.
//!
//! A [`Session`] is created after a successful OIDC login and referenced by an opaque
//! id in a `HttpOnly` cookie. Phase 2 binds the session to the OIDC subject, residency,
//! and step-up [`FactorLevel`]; later phases additionally bind the mTLS cert thumbprint
//! and WG peer key (security doc §1.3).
//!
//! Both stores recover from a poisoned mutex (a panicked holder) instead of
//! propagating the panic, so one bad request can never wedge auth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use demon_core::{FactorLevel, Principal};

/// An authenticated operator session.
#[derive(Debug, Clone)]
pub struct Session {
    /// The authorized principal.
    pub principal: Principal,
    /// Highest step-up factor presented this session.
    pub factor: FactorLevel,
    /// Expiry (epoch ms).
    pub expires_at: i64,
}

/// Opaque, cheaply-cloneable session store.
#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<String, Session>>>,
}

impl SessionStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a session under `id`.
    pub fn insert(&self, id: String, session: Session) {
        let mut g = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        g.insert(id, session);
    }

    /// Fetch a session if present and not expired (expired sessions are evicted).
    #[must_use]
    pub fn get(&self, id: &str, now_ms: i64) -> Option<Session> {
        let mut g = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match g.get(id) {
            Some(s) if s.expires_at > now_ms => Some(s.clone()),
            Some(_) => {
                g.remove(id);
                None
            }
            None => None,
        }
    }

    /// Drop a session (logout).
    pub fn remove(&self, id: &str) {
        let mut g = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        g.remove(id);
    }
}

/// PKCE verifier awaiting the authorization-code callback, keyed by OAuth `state`.
#[derive(Debug, Clone)]
pub struct Pending {
    /// The PKCE code verifier to present at token exchange.
    pub verifier: String,
    /// Creation time (epoch ms) — for TTL eviction.
    pub created_at: i64,
}

/// Store of in-flight authorization requests.
#[derive(Clone, Default)]
pub struct PendingStore {
    inner: Arc<Mutex<HashMap<String, Pending>>>,
}

impl PendingStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a pending auth under its `state`.
    pub fn insert(&self, state: String, pending: Pending) {
        let mut g = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        g.insert(state, pending);
    }

    /// Consume (remove and return) the pending auth for `state`, if any.
    #[must_use]
    pub fn take(&self, state: &str) -> Option<Pending> {
        let mut g = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        g.remove(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::{Region, Role};

    fn principal() -> Principal {
        Principal::new("op@x", vec![Role::Operator], Region::Eu)
    }

    #[test]
    fn session_lifecycle() {
        let store = SessionStore::new();
        store.insert(
            "sid".into(),
            Session {
                principal: principal(),
                factor: FactorLevel::None,
                expires_at: 1000,
            },
        );
        assert!(store.get("sid", 500).is_some());
        // expired -> evicted
        assert!(store.get("sid", 2000).is_none());
        assert!(store.get("sid", 500).is_none());
    }

    #[test]
    fn logout_removes_session() {
        let store = SessionStore::new();
        store.insert(
            "sid".into(),
            Session {
                principal: principal(),
                factor: FactorLevel::None,
                expires_at: 9999,
            },
        );
        store.remove("sid");
        assert!(store.get("sid", 0).is_none());
    }

    #[test]
    fn pending_take_is_one_shot() {
        let store = PendingStore::new();
        store.insert(
            "state1".into(),
            Pending {
                verifier: "v".into(),
                created_at: 0,
            },
        );
        assert!(store.take("state1").is_some());
        assert!(store.take("state1").is_none());
    }
}
