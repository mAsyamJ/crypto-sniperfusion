//! sniper-runtime: pipeline runtimes and durable plumbing.
//!
//! Capability separation is structural: the safe runtimes (`observe`,
//! `replay`, `paper`) have no function that returns a `Broadcaster` or loads a
//! funded `Signer`. Live capability exists only behind the non-default `live`
//! feature AND passes four independent runtime gates.

pub mod bus;
pub mod capability;
pub mod ingest;
pub mod journal;
pub mod runner;

pub use bus::{Priority, PriorityBus, PublishOutcome};
pub use capability::{acquire_live, LiveCapability, LiveGateError, LiveGateInputs};
pub use ingest::{
    BlockhashCache, EntryGate, EventProvider, GapTracker, ProviderError, RawObservation,
    ReconnectPolicy, StreamEvent, StreamSupervisor,
};
pub use journal::{FileJournal, JournalRecord, LifecycleStage, TxJournal};
pub use runner::{Decision, DecisionHook, ReplayRunner, RunSummary};

#[cfg(feature = "live")]
pub use capability::live_impl::{LiveBroadcaster, LiveSigner};
