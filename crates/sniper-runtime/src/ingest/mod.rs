//! Ingestion plane: provider adapters, stream supervision, gap detection,
//! commitment handling, blockhash cache.
//!
//! Network-facing adapters (WSS pubsub, Yellowstone gRPC) compile in the
//! default build but their connection tests are opt-in: without
//! `SNF_NETWORK_TESTS=1` and endpoint credentials they print a skip reason
//! and pass — never silently pretending to have talked to a provider.

pub mod blockhash;
pub mod frame;
pub mod geyser;
pub mod provider;
pub mod supervisor;
pub mod wss;

pub use blockhash::BlockhashCache;
pub use frame::{RawObservation, RawPayload};
pub use provider::{EventProvider, ProviderError, ReconnectPolicy};
pub use supervisor::{
    BackfillReceipt, EntryGate, GapTracker, SlotCursor, StreamEvent, StreamSupervisor,
};
