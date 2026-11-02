//! sniper-core: canonical domain contracts shared by every other crate.
//!
//! Layering rule: core depends on no internal crate. It owns
//! - dual event identity (chain truth vs provider observation),
//! - commitment handling and dedup,
//! - integer money primitives with checked math,
//! - the typed configuration model (safe defaults),
//! - risk authorization (the only door into any money-moving path),
//! - the error taxonomy.

pub mod authz;
pub mod config;
pub mod error;
pub mod event;
pub mod primitives;
pub mod serde_keys;
pub mod time;
pub mod venue;

pub use authz::{
    AuthorizedOrder, AuthzError, AuthzRequest, Chain, OrderFacts, RiskAuthorization, RiskEngine,
    StrategyId,
};
pub use config::{
    AppConfig, ConfigError, ExitStrategy, GeyserAuth, IngestConfig, IngestProviderKind, RunMode,
};
pub use error::SniperError;
pub use event::{
    ChainEvent, ChainEventId, Commitment, DedupOutcome, DedupRegistry, EventPayload, ProviderKind,
};
pub use primitives::{BasisPoints, Lamports, Microlamports, TokenAmount};
pub use time::Timestamps;
pub use venue::Venue;
