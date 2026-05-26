//! Append-only, tamper-evident audit chain (pure).
//!
//! Each [`AuditRecord`] commits to its predecessor via `hash = SHA-256(prev_hash ||
//! canonical-fields)`. Any edit, reorder, or deletion of a past record breaks the
//! chain, which [`AuditChain::verify`] detects. This is the pure core migrated from
//! the TUI's `audit.rs`/`seq.rs`; durable storage and OpenSearch shipping are the job
//! of driver crates.
//!
//! **Redaction is the caller's responsibility and is mandatory**: `redacted_payload`
//! must already be free of secrets, tokens, keys, and credentials before it reaches
//! this module. The chain only hashes what it is given.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The chain's genesis predecessor hash (64 hex zeroes).
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One link in the audit chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Monotonic sequence number (0-based).
    pub seq: u64,
    /// Hash of the previous record (or [`GENESIS_HASH`] for the first).
    pub prev_hash: String,
    /// `SHA-256` over `prev_hash` + this record's canonical fields.
    pub hash: String,
    /// Who performed the action (`sub`).
    pub actor: String,
    /// Action identifier.
    pub action: String,
    /// Target identifier.
    pub target: String,
    /// Whether this was a dry-run (no real effect).
    pub dry_run: bool,
    /// Already-redacted detail. NEVER contains secrets.
    pub redacted_payload: String,
    /// Event timestamp (epoch milliseconds). Passed in by the caller to keep this
    /// module pure (no clock read).
    pub ts: i64,
}

/// Hash a sequence of canonical fields. Each field is length-prefixed and
/// NUL-separated so distinct field boundaries cannot collide. The first field is the
/// predecessor hash, which is what chains records together.
fn chain_hash(fields: &[&str]) -> String {
    let mut h = Sha256::new();
    for field in fields {
        let len = u64::try_from(field.len()).expect("field length fits in u64");
        h.update(len.to_le_bytes());
        h.update(field.as_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// The boolean `dry_run` rendered as the canonical single-char field.
const fn dry_run_field(dry_run: bool) -> &'static str {
    if dry_run {
        "1"
    } else {
        "0"
    }
}

/// An in-memory audit chain. Records are appended in order; [`verify`](Self::verify)
/// re-derives every hash to detect tampering.
#[derive(Debug, Clone, Default)]
pub struct AuditChain {
    records: Vec<AuditRecord>,
}

impl AuditChain {
    /// A fresh, empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current head hash (genesis if empty).
    #[must_use]
    pub fn head(&self) -> &str {
        self.records.last().map_or(GENESIS_HASH, |r| r.hash.as_str())
    }

    /// All records, in order.
    #[must_use]
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    /// Append a new event, computing its sequence number and hash from the chain head.
    /// Returns the `seq` assigned to the new record.
    pub fn append(
        &mut self,
        actor: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        dry_run: bool,
        redacted_payload: impl Into<String>,
        ts: i64,
    ) -> u64 {
        let seq = self.records.len() as u64;
        let prev_hash = self.head().to_owned();
        let actor = actor.into();
        let action = action.into();
        let target = target.into();
        let redacted_payload = redacted_payload.into();
        let seq_s = seq.to_string();
        let ts_s = ts.to_string();
        let hash = chain_hash(&[
            &prev_hash,
            &seq_s,
            &actor,
            &action,
            &target,
            dry_run_field(dry_run),
            &redacted_payload,
            &ts_s,
        ]);
        self.records.push(AuditRecord {
            seq,
            prev_hash,
            hash,
            actor,
            action,
            target,
            dry_run,
            redacted_payload,
            ts,
        });
        seq
    }

    /// Verify the whole chain: sequence numbers, predecessor links, and every hash.
    /// Returns the seq of the first broken record, or `Ok(())` if intact.
    ///
    /// # Errors
    /// Returns [`ChainError`] describing the first inconsistency found.
    pub fn verify(&self) -> Result<(), ChainError> {
        let mut expected_prev = GENESIS_HASH.to_owned();
        for (i, r) in self.records.iter().enumerate() {
            let seq = i as u64;
            if r.seq != seq {
                return Err(ChainError::Sequence { at: i, found: r.seq });
            }
            if r.prev_hash != expected_prev {
                return Err(ChainError::BrokenLink { seq });
            }
            let seq_s = r.seq.to_string();
            let ts_s = r.ts.to_string();
            let recomputed = chain_hash(&[
                &r.prev_hash,
                &seq_s,
                &r.actor,
                &r.action,
                &r.target,
                dry_run_field(r.dry_run),
                &r.redacted_payload,
                &ts_s,
            ]);
            if recomputed != r.hash {
                return Err(ChainError::HashMismatch { seq });
            }
            expected_prev.clone_from(&r.hash);
        }
        Ok(())
    }
}

/// A detected inconsistency in an [`AuditChain`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ChainError {
    /// A record's `seq` did not match its position.
    #[error("audit record at index {at} has wrong seq {found}")]
    Sequence {
        /// Index in the record vector.
        at: usize,
        /// The (wrong) seq found.
        found: u64,
    },
    /// A record's `prev_hash` did not match the previous record's `hash`.
    #[error("audit record {seq} does not link to its predecessor")]
    BrokenLink {
        /// Seq of the broken record.
        seq: u64,
    },
    /// A record's stored `hash` did not match its recomputed hash (content tampered).
    #[error("audit record {seq} hash mismatch — content was altered")]
    HashMismatch {
        /// Seq of the tampered record.
        seq: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_record_chain() -> AuditChain {
        let mut c = AuditChain::new();
        c.append("op@x", "service.restart", "host:1/opensearch", false, "ok", 1_000);
        c.append("sr@x", "tenant.decommission", "tenant:42", true, "plan only", 2_000);
        c
    }

    #[test]
    fn empty_chain_head_is_genesis() {
        let c = AuditChain::new();
        assert_eq!(c.head(), GENESIS_HASH);
        assert!(c.verify().is_ok());
    }

    #[test]
    fn append_links_and_verifies() {
        let c = two_record_chain();
        assert_eq!(c.records().len(), 2);
        assert_eq!(c.records()[0].prev_hash, GENESIS_HASH);
        assert_eq!(c.records()[1].prev_hash, c.records()[0].hash);
        assert_eq!(c.head(), c.records()[1].hash);
        assert!(c.verify().is_ok());
    }

    #[test]
    fn tampering_payload_breaks_chain() {
        let mut c = two_record_chain();
        c.records[0].redacted_payload = "tampered".into();
        assert_eq!(c.verify(), Err(ChainError::HashMismatch { seq: 0 }));
    }

    #[test]
    fn reordering_breaks_chain() {
        let mut c = two_record_chain();
        c.records.swap(0, 1);
        assert!(c.verify().is_err());
    }

    #[test]
    fn deletion_breaks_chain() {
        let mut c = two_record_chain();
        c.records.remove(0);
        // record now at index 0 still has seq 1 -> sequence error
        assert_eq!(c.verify(), Err(ChainError::Sequence { at: 0, found: 1 }));
    }
}
