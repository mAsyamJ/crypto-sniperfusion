//! Error taxonomy for the whole workspace. Structured, no secret material.

use thiserror::Error;

/// Top-level error taxonomy. Every subsystem error folds into this via
/// `#[from]`, so call sites match on domain reasons, not strings.
#[derive(Debug, Error)]
pub enum SniperError {
    #[error("configuration rejected: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("risk authorization failed: {0}")]
    Authz(#[from] crate::authz::AuthzError),

    #[error("checked math overflow: {op}({lhs}, {rhs})")]
    MathOverflow {
        op: &'static str,
        lhs: u128,
        rhs: u128,
    },

    #[error("unsupported or unregistered venue: {0:?}")]
    UnsupportedVenue(crate::venue::Venue),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("persistence error: {0}")]
    Persistence(String),

    #[error("runtime error: {0}")]
    Runtime(String),
}
