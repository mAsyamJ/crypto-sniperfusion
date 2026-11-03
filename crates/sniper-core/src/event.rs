//! Canonical event model: chain-truth identity vs provider observation,
//! commitment handling, and deduplication.

use std::collections::{HashSet, VecDeque};

use solana_sdk::signature::Signature;

use crate::primitives::{Lamports, TokenAmount};
use crate::time::Timestamps;
use crate::venue::Venue;

/// Which provider surface produced an observation. Identity/dedup never
/// depends on this; latency attribution does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProviderKind {
    /// WSS `logsSubscribe` (processed commitment).
    Logs,
    /// WSS `blockSubscribe` (confirmed commitment).
    Blocks,
    /// Yellowstone gRPC `Subscribe`.
    Geyser,
    /// Geyser `SubscribeDeshred` pre-execution stream.
    Shreds,
    /// Offline fixture replay.
    Replay,
}

/// Commitment of the observation. Upgrades move one way only.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

impl Commitment {
    /// A later observation may upgrade an event's commitment, never downgrade.
    /// Re-observing at the same level is a no-op (idempotent).
    pub fn can_upgrade(self, new: Commitment) -> bool {
        (self as u8) < (new as u8)
    }
}

/// Chain-truth identity of one decoded event: a transaction signature plus the
/// fully-qualified instruction/event position inside it. Two providers seeing
/// the same on-chain event produce the same id — that is the dedup key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChainEventId {
    #[serde(with = "crate::serde_keys::signature_str")]
    pub signature: Signature,
    /// Top-level instruction index within the transaction.
    pub outer_ix_index: u32,
    /// Inner instruction index if the event fired inside a CPI, else None.
    pub inner_ix_index: Option<u32>,
    /// Ordinal when several logical events share one instruction (e.g. logs).
    pub event_ordinal: u16,
}

/// Minimal payload vocabulary for M1 (replay/bus contracts). M2 replaces the
/// population path with real decoders; the shapes are already final.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EventPayload {
    TokenCreated {
        #[serde(with = "crate::serde_keys::pubkey_str")]
        mint: solana_sdk::pubkey::Pubkey,
        #[serde(with = "crate::serde_keys::pubkey_str")]
        creator: solana_sdk::pubkey::Pubkey,
        name: String,
        symbol: String,
        uri: String,
    },
    TradeObserved {
        #[serde(with = "crate::serde_keys::pubkey_str")]
        mint: solana_sdk::pubkey::Pubkey,
        #[serde(with = "crate::serde_keys::pubkey_str")]
        trader: solana_sdk::pubkey::Pubkey,
        is_buy: bool,
        quote_amount: Lamports,
        token_amount: TokenAmount,
    },
    MigrationObserved {
        #[serde(with = "crate::serde_keys::pubkey_str")]
        mint: solana_sdk::pubkey::Pubkey,
        to: Venue,
    },
    WalletActivity {
        #[serde(with = "crate::serde_keys::pubkey_str")]
        wallet: solana_sdk::pubkey::Pubkey,
        #[serde(with = "crate::serde_keys::pubkey_str")]
        mint: solana_sdk::pubkey::Pubkey,
    },
    Unknown,
}

/// The canonical event every subsystem reasons over.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChainEvent {
    pub id: ChainEventId,
    pub venue: Venue,
    pub slot: u64,
    pub commitment: Commitment,
    pub received: Timestamps,
    pub source: ProviderKind,
    pub schema_version: u16,
    pub payload: EventPayload,
}

/// Outcome of registering an event id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupOutcome {
    FirstSeen,
    Duplicate,
}

impl DedupOutcome {
    pub fn is_first_seen(self) -> bool {
        matches!(self, DedupOutcome::FirstSeen)
    }

    pub fn is_duplicate(self) -> bool {
        matches!(self, DedupOutcome::Duplicate)
    }
}

/// Fixed-capacity registry of seen event ids. Evicts oldest-first; a duplicate
/// is only detectable while its id remains resident (capacity is therefore a
/// correctness parameter, documented per pipeline).
#[derive(Debug)]
pub struct DedupRegistry {
    seen: std::collections::HashSet<ChainEventId>,
    order: VecDeque<ChainEventId>,
    capacity: usize,
    duplicates_observed: u64,
}

impl DedupRegistry {
    pub fn new(capacity: usize) -> Self {
        Self {
            seen: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
            duplicates_observed: 0,
        }
    }

    pub fn register(&mut self, id: ChainEventId) -> DedupOutcome {
        if self.seen.contains(&id) {
            self.duplicates_observed += 1;
            return DedupOutcome::Duplicate;
        }
        if self.order.len() == self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.seen.remove(&evicted);
            }
        }
        self.seen.insert(id);
        self.order.push_back(id);
        DedupOutcome::FirstSeen
    }

    pub fn duplicates_observed(&self) -> u64 {
        self.duplicates_observed
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(n: u8) -> Signature {
        let mut b = [0u8; 64];
        b[0] = n;
        Signature::from(b)
    }

    fn id(n: u8, outer: u32, inner: Option<u32>, ord: u16) -> ChainEventId {
        ChainEventId {
            signature: sig(n),
            outer_ix_index: outer,
            inner_ix_index: inner,
            event_ordinal: ord,
        }
    }

    #[test]
    fn commitment_upgrades_one_way_only() {
        assert!(Commitment::Processed.can_upgrade(Commitment::Confirmed));
        assert!(Commitment::Confirmed.can_upgrade(Commitment::Finalized));
        assert!(Commitment::Processed.can_upgrade(Commitment::Finalized));
        assert!(!Commitment::Finalized.can_upgrade(Commitment::Processed));
        assert!(!Commitment::Confirmed.can_upgrade(Commitment::Processed));
        assert!(!Commitment::Processed.can_upgrade(Commitment::Processed));
    }

    #[test]
    fn dedup_same_id_two_providers() {
        let mut reg = DedupRegistry::new(128);
        assert_eq!(reg.register(id(1, 0, None, 0)), DedupOutcome::FirstSeen);
        // Same chain event arriving via a second provider → same id → duplicate.
        assert_eq!(reg.register(id(1, 0, None, 0)), DedupOutcome::Duplicate);
        assert_eq!(reg.duplicates_observed(), 1);
    }

    #[test]
    fn dedup_distinguishes_instruction_positions() {
        let mut reg = DedupRegistry::new(128);
        assert_eq!(reg.register(id(1, 0, None, 0)), DedupOutcome::FirstSeen);
        assert_eq!(reg.register(id(1, 1, None, 0)), DedupOutcome::FirstSeen);
        assert_eq!(reg.register(id(1, 0, Some(0), 0)), DedupOutcome::FirstSeen);
        assert_eq!(reg.register(id(1, 0, None, 1)), DedupOutcome::FirstSeen);
        assert_eq!(reg.len(), 4);
    }

    #[test]
    fn dedup_evicts_oldest_at_capacity() {
        let mut reg = DedupRegistry::new(2);
        reg.register(id(1, 0, None, 0));
        reg.register(id(2, 0, None, 0));
        reg.register(id(3, 0, None, 0)); // evicts id(1)
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.register(id(1, 0, None, 0)), DedupOutcome::FirstSeen);
    }
}
