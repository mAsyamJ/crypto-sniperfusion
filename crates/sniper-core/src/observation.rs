//! H6-1 canonical provenance contracts: dataset-layer ordering, observation
//! provenance, causality clocks, and persistent historical deduplication.
//!
//! Layer distinction: `event::ChainEvent` is the decoded event every subsystem
//! reasons over; the types here are the DATASET envelope around it — where the
//! event sat on chain (canonical order), who saw it and when (provenance),
//! which clocks govern decidability (causality), and what was verifiably
//! collected (coverage, in `crate::coverage`).

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::event::{ChainEventId, Commitment, DedupOutcome, ProviderKind};
use crate::time::Timestamps;

/// SHA-256 of the exact source payload bytes this observation was decoded
/// from. Content-addressing key for the raw evidence store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadHash(pub [u8; 32]);

/// Full canonical on-chain position of one event.
///
/// `transaction_index` is ONLY ever sourced from the containing block
/// (getBlock evidence) — never from RPC signature pagination order, which is
/// not a chain-order guarantee. `None` means the coordinate has not been
/// established; such records are observation-order-only and MUST NOT be
/// silently ordered against block-verified records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalOrder {
    pub slot: u64,
    pub transaction_index: Option<u64>,
    pub outer_ix_index: u32,
    pub inner_ix_index: Option<u32>,
    pub event_ordinal: u16,
}

impl CanonicalOrder {
    /// Key for the instruction position inside one transaction.
    ///
    /// `inner_ix_index == None` denotes the top-level instruction itself,
    /// which executes BEFORE the CPI calls it invokes: it therefore sorts
    /// ahead of its inner instructions. This is protocol semantics (CPI
    /// parent-first), not a fabricated coordinate.
    fn instruction_position(&self) -> (u32, u32, u16) {
        let inner_key = self.inner_ix_index.map_or(0, |i| i + 1);
        (self.outer_ix_index, inner_key, self.event_ordinal)
    }

    /// Total chain-order key, or an error when the transaction index is not
    /// established. Ordering records with unknown coordinates would fabricate
    /// a chain position, so it is refused.
    pub fn chain_order_key(&self) -> Result<(u64, u64, u32, u32, u16), NotChainOrderable> {
        let transaction_index = self
            .transaction_index
            .ok_or(NotChainOrderable { slot: self.slot })?;
        let (outer, inner_key, ordinal) = self.instruction_position();
        Ok((self.slot, transaction_index, outer, inner_key, ordinal))
    }
}

/// The record's coordinates do not support chain-order replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("transaction index unknown at slot {slot}; observation-order only")]
pub struct NotChainOrderable {
    pub slot: u64,
}

/// Why a record is not (yet) chain-orderable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionIndexUnknown {
    /// Log-derived observation; the containing block has not been fetched.
    LogDerivedWithoutBlock,
    /// Block evidence requested but unavailable (pruned, rate-limited, ...).
    BlockUnavailable,
    /// The provider surface does not carry block coordinates.
    ProviderDidNotSupply,
}

/// Canonicalization state of one observed record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalStatus {
    /// Full position verified from block evidence; chain-order replay allowed.
    ChainOrderable,
    /// Observation-order replay only.
    ObservationOrderOnly { reason: TransactionIndexUnknown },
}

impl CanonicalStatus {
    pub fn is_chain_orderable(&self) -> bool {
        matches!(self, CanonicalStatus::ChainOrderable)
    }
}

/// One commitment observation of the same chain event, with the wall time it
/// was made. History is append-only: commitment evolves one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentObservation {
    pub commitment: Commitment,
    pub observed_at_wall_ns: u64,
}

/// The fork (slot + block hash) an observation was made on, and whether that
/// fork was later abandoned by a reorganization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkIdentity {
    pub slot: u64,
    pub block_hash: [u8; 32],
    pub status: ForkStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkStatus {
    /// Part of the canonical chain as far as observed.
    Live,
    /// Reorged away. Observations on abandoned forks are never canonical
    /// facts; they are retained as raw evidence only.
    Abandoned,
}

/// Provider-side provenance of one observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationProvenance {
    pub provider: ProviderKind,
    pub commitment: Commitment,
    /// Append-only commitment evolution (processed → confirmed → finalized).
    pub commitment_history: Vec<CommitmentObservation>,
    /// Receipt time at the observing process.
    pub received: Timestamps,
    /// Process restart counter: `mono_ns` is only comparable within one
    /// epoch, so latency math across restarts needs this pairing.
    pub process_epoch: u64,
    /// Hash of the exact raw payload this observation was decoded from.
    pub payload_hash: PayloadHash,
    /// Program (IDL/account-layout) version the decoder assumed, if known.
    pub program_version: Option<u32>,
    pub fork: ForkIdentity,
}

/// Commitment tried to move backwards; history is retained unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("commitment downgrade from {from:?} to {to:?} refused")]
pub struct CommitmentDowngrade {
    pub from: Commitment,
    pub to: Commitment,
}

impl ObservationProvenance {
    /// Upgrade commitment one way (same level is an idempotent no-op). Each
    /// change appends to the history; downgrades are refused.
    pub fn observe_commitment(
        &mut self,
        new: Commitment,
        at_wall_ns: u64,
    ) -> Result<(), CommitmentDowngrade> {
        if new == self.commitment {
            return Ok(());
        }
        if !self.commitment.can_upgrade(new) {
            return Err(CommitmentDowngrade {
                from: self.commitment,
                to: new,
            });
        }
        self.commitment = new;
        self.commitment_history.push(CommitmentObservation {
            commitment: new,
            observed_at_wall_ns: at_wall_ns,
        });
        Ok(())
    }

    /// Record that this observation's fork lost to a reorganization.
    pub fn mark_fork_abandoned(&mut self) {
        self.fork.status = ForkStatus::Abandoned;
    }

    /// A canonical fact requires a live fork AND finalized commitment.
    /// Anything less is provisional raw evidence.
    pub fn is_canonical_fact(&self) -> bool {
        self.fork.status == ForkStatus::Live && self.commitment == Commitment::Finalized
    }
}

/// The three clocks attached to every observation.
///
/// `chain_at_ms` is Solana's estimated block timestamp — an ESTIMATE. It is
/// deliberately NOT required to precede `observed_at_ms`: estimated block
/// times can sit after receipt times without implying time travel, and
/// enforcing that inequality would silently drop real observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalityClocks {
    pub chain_at_ms: Option<u64>,
    pub observed_at_ms: u64,
    pub feature_available_at_ms: u64,
}

/// A decision would have used information from the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("feature available at {feature_available_at_ms} ms but decision at {decision_at_ms} ms")]
pub enum CausalityError {
    FeatureNotYetAvailable {
        feature_available_at_ms: u64,
        decision_at_ms: u64,
    },
}

/// The ONLY enforced causality rule: features must be available at or before
/// the decision instant. Chain-time vs receipt-time relationships are
/// recorded uncertainty, not enforced ordering.
pub fn ensure_decidable(
    clocks: &CausalityClocks,
    decision_at_ms: u64,
) -> Result<(), CausalityError> {
    if clocks.feature_available_at_ms > decision_at_ms {
        return Err(CausalityError::FeatureNotYetAvailable {
            feature_available_at_ms: clocks.feature_available_at_ms,
            decision_at_ms,
        });
    }
    Ok(())
}

/// One dataset-layer record: chain identity + canonical position +
/// provenance + causality clocks. The decoded payload lives in
/// `event::ChainEvent`; records re-attach to it via `event_id` /
/// `payload_hash`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedRecord {
    pub event_id: ChainEventId,
    pub order: CanonicalOrder,
    pub provenance: ObservationProvenance,
    pub clocks: CausalityClocks,
    /// Convenience cohort key; redundant with the payload but stable here.
    pub mint: solana_sdk::pubkey::Pubkey,
}

impl ObservedRecord {
    pub fn canonical_status(&self) -> CanonicalStatus {
        if self.order.transaction_index.is_some() {
            CanonicalStatus::ChainOrderable
        } else {
            CanonicalStatus::ObservationOrderOnly {
                reason: TransactionIndexUnknown::LogDerivedWithoutBlock,
            }
        }
    }
}

/// Sorting failed because at least one record lacks chain coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChainSortError {
    #[error("transaction index unknown at slot {slot}")]
    UnknownTransactionIndex { slot: u64 },
}

/// Sort records into canonical chain order. REFUSES (never silently orders)
/// records whose transaction index is not established from block evidence —
/// log-derived data is observation-order-only by construction.
pub fn chain_sort_records(records: &mut [ObservedRecord]) -> Result<(), ChainSortError> {
    for record in records.iter() {
        if record.order.transaction_index.is_none() {
            return Err(ChainSortError::UnknownTransactionIndex {
                slot: record.order.slot,
            });
        }
    }
    records.sort_by_key(|r| r.order.chain_order_key().expect("validated above"));
    Ok(())
}

/// Persistent (or in-memory) historical deduplication across process
/// restarts, keyed by chain-truth event identity. The persistent journal is
/// append-only JSONL; a corrupt line is a hard error (never silently
/// forgotten history).
#[derive(Debug)]
pub struct HistoricalDedup {
    seen: HashSet<ChainEventId>,
    journal: Option<std::path::PathBuf>,
    duplicates_observed: u64,
}

impl HistoricalDedup {
    pub fn in_memory() -> Self {
        Self {
            seen: HashSet::new(),
            journal: None,
            duplicates_observed: 0,
        }
    }

    /// Open (and load) an append-only dedup journal. Existing ids are
    /// remembered; malformed lines abort loading.
    pub fn persistent(journal: &Path) -> std::io::Result<Self> {
        let mut seen = HashSet::new();
        if journal.exists() {
            let text = std::fs::read_to_string(journal)?;
            for (lineno, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let id: ChainEventId = serde_json::from_str(line).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("dedup journal line {}: {e}", lineno + 1),
                    )
                })?;
                seen.insert(id);
            }
        }
        Ok(Self {
            seen,
            journal: Some(journal.to_path_buf()),
            duplicates_observed: 0,
        })
    }

    /// Register an id. First-seen ids are appended to the journal (and
    /// flushed) when persistent; duplicates are only counted.
    pub fn register(&mut self, id: ChainEventId) -> std::io::Result<DedupOutcome> {
        if self.seen.contains(&id) {
            self.duplicates_observed += 1;
            return Ok(DedupOutcome::Duplicate);
        }
        if let Some(path) = &self.journal {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            let line = serde_json::to_string(&id)
                .map_err(|e| std::io::Error::other(format!("serialize id: {e}")))?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
            file.flush()?;
        }
        self.seen.insert(id);
        Ok(DedupOutcome::FirstSeen)
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

    #[test]
    fn instruction_position_orders_parent_before_children() {
        let parent = CanonicalOrder {
            slot: 1,
            transaction_index: Some(0),
            outer_ix_index: 2,
            inner_ix_index: None,
            event_ordinal: 0,
        };
        let child0 = CanonicalOrder {
            inner_ix_index: Some(0),
            ..parent
        };
        let child1 = CanonicalOrder {
            inner_ix_index: Some(1),
            ..parent
        };
        assert!(parent.chain_order_key().unwrap() < child0.chain_order_key().unwrap());
        assert!(child0.chain_order_key().unwrap() < child1.chain_order_key().unwrap());
    }

    #[test]
    fn unknown_transaction_index_is_not_chain_orderable() {
        let order = CanonicalOrder {
            slot: 7,
            transaction_index: None,
            outer_ix_index: 0,
            inner_ix_index: None,
            event_ordinal: 0,
        };
        assert!(order.chain_order_key().is_err());
    }
}
