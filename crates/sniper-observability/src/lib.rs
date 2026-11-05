//! sniper-observability: read-only projection of SniperFusion's own
//! write-once artifacts into a disposable SQLite cache (ADR-0013).
//!
//! Boundary rules (tested by T-SEC5):
//! - Imports only from allowlisted local roots; no network, ever.
//! - Verifies hash-sealed receipts before trusting them; mismatches are
//!   quarantined, never served as data.
//! - Performs no financial arithmetic; the calendar lives in
//!   `sniper-strategy::calendar` (the accounting crate) and this crate only
//!   calls it.
//! - Never depends on `sniper-runtime` (signers/authz live there).

pub mod discovery;
pub mod parse;
pub mod project;
pub mod queries;
pub mod verify;

use thiserror::Error;

/// Typed import/verification failure. Fail-closed: the artifact is
/// quarantined with its status; nothing is guessed.
#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("source mismatch at {path}: expected receipt_sha256 {expected}, computed {actual}")]
    SourceMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("unsupported artifact at {path}: {reason}")]
    Unsupported { path: String, reason: String },
    #[error("parse failure at {path}: {reason}")]
    Parse { path: String, reason: String },
}
