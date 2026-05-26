//! `demon-store` — SQLite (WAL) persistence for proximiio.demon.
//!
//! The store is generic over the residency marker `R` ([`demon_core::Residency`]):
//! a [`Store<Eu>`](Store) and a [`Store<Uae>`](Store) are **distinct types**, so code
//! cannot accidentally hand one region's store where the other is expected — the
//! mismatch is a compile error. At runtime, [`Store::check_region`] backstops the type
//! proof for data crossing a trust boundary (deserialised input, SQL rows).
//!
//! One database file per residency group; one `Store` per running daemon.
//!
//! # Residency is enforced at compile time
//!
//! Mixing regions does not type-check:
//!
//! ```compile_fail
//! use demon_store::Store;
//! use demon_core::{Eu, Uae};
//!
//! fn requires_same_region<R: demon_core::Residency>(_a: &Store<R>, _b: &Store<R>) {}
//!
//! fn demo(eu: &Store<Eu>, uae: &Store<Uae>) {
//!     // ERROR[E0308]: expected `&Store<Eu>`, found `&Store<Uae>`
//!     requires_same_region(eu, uae);
//! }
//! ```
//!
//! while a single region is fine:
//!
//! ```
//! use demon_store::Store;
//! use demon_core::Eu;
//!
//! fn requires_same_region<R: demon_core::Residency>(_a: &Store<R>, _b: &Store<R>) {}
//!
//! fn demo(a: &Store<Eu>, b: &Store<Eu>) {
//!     requires_same_region(a, b);
//! }
//! ```
#![forbid(unsafe_code)]

mod inventory;

use std::marker::PhantomData;

use demon_core::{Region, Residency};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

/// Errors from opening or using the store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Underlying sqlx/database error.
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    /// Migration error.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// A row/value from the wrong residency group was observed (runtime backstop).
    #[error("residency violation: store is {store}, value was {value}")]
    ResidencyViolation {
        /// The store's region.
        store: Region,
        /// The offending value's region.
        value: Region,
    },
    /// A stored value could not be decoded into its domain type (e.g. an unknown
    /// enum string). Indicates schema/data corruption.
    #[error("decode error: {0}")]
    Decode(String),
}

/// A residency-scoped SQLite store.
#[derive(Debug, Clone)]
pub struct Store<R: Residency> {
    pool: SqlitePool,
    _region: PhantomData<R>,
}

impl<R: Residency> Store<R> {
    /// Open (creating if missing) the SQLite database at `path`, enable WAL, and run
    /// migrations to the latest version.
    ///
    /// # Errors
    /// Returns [`StoreError`] if the database cannot be opened or migrations fail.
    pub async fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StoreError> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        tracing::info!(region = %R::REGION, "store opened and migrated");
        Ok(Self {
            pool,
            _region: PhantomData,
        })
    }

    /// Open an in-memory store (tests / ephemeral). Migrations are applied.
    ///
    /// # Errors
    /// Returns [`StoreError`] if the database cannot be created or migrations fail.
    pub async fn open_in_memory() -> Result<Self, StoreError> {
        let opts = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self {
            pool,
            _region: PhantomData,
        })
    }

    /// This store's residency group (a compile-time constant).
    #[must_use]
    pub const fn region(&self) -> Region {
        R::REGION
    }

    /// Access the underlying pool (for driver code in later phases).
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Runtime residency backstop: reject a value whose region differs from this
    /// store's. Use at trust boundaries where the type proof cannot reach.
    ///
    /// # Errors
    /// Returns [`StoreError::ResidencyViolation`] if `value` is not this store's region.
    pub fn check_region(&self, value: Region) -> Result<(), StoreError> {
        if value == R::REGION {
            Ok(())
        } else {
            Err(StoreError::ResidencyViolation {
                store: R::REGION,
                value,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_core::{Eu, Uae};

    #[tokio::test]
    async fn opens_and_migrates_in_memory() {
        let store = Store::<Eu>::open_in_memory().await.unwrap();
        assert_eq!(store.region(), Region::Eu);
        // schema_migrations + our seed should exist
        let groups: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM residency_groups")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(groups, 2);
    }

    #[tokio::test]
    async fn region_backstop_rejects_other_group() {
        let eu = Store::<Eu>::open_in_memory().await.unwrap();
        assert!(eu.check_region(Region::Eu).is_ok());
        assert!(matches!(
            eu.check_region(Region::Uae),
            Err(StoreError::ResidencyViolation { .. })
        ));
    }

    #[tokio::test]
    async fn uae_store_reports_uae() {
        let uae = Store::<Uae>::open_in_memory().await.unwrap();
        assert_eq!(uae.region(), Region::Uae);
    }
}
