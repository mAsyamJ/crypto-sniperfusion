//! H6-1 coverage contracts: honest completeness accounting for a bounded
//! observation window.
//!
//! Coverage is a DATA-completeness concept only. It is deliberately
//! independent of token lifecycle state: a mint whose data is missing is
//! `Censored`/`Incomplete` regardless of whether it later graduated, died, or
//! never traded — and absence of trading inside an unobserved window is never
//! evidence that a token is dead.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Completeness of the collected evidence for the window (or one mint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageState {
    /// Every slot in the window has block evidence, every discovered record
    /// reconciled, no gaps. The only state that may back completeness claims.
    Complete,
    /// Collection ran but evidence has known holes (gaps recorded).
    Incomplete,
    /// The window closed while data was still missing: right-censored. Any
    /// downstream "no event observed" conclusion is censored, not negative.
    Censored,
    /// No block evidence at all: nothing is provable about this window.
    Unknown,
}

/// The specific shape of a hole in the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapKind {
    /// Feed/provider down during part of the window.
    ProviderUnavailable,
    /// Provider up but refused work (429/limit).
    RateLimited,
    /// Transaction no longer served by the RPC node.
    PrunedTransaction { signature: String },
    /// Transaction seen but its metadata/decoding inputs were missing.
    MissingMetadata { signature: String },
    /// Record present but undecodable; detail carries the reason.
    MalformedRecord { detail: String },
    /// Slots the chain skipped (no block produced).
    SkippedSlots { from_slot: u64, to_slot: u64 },
    /// A fork previously observed on was reorged away.
    Reorg { slot: u64 },
    /// Commitment moved backwards for a previously promoted observation.
    CommitmentRollback { slot: u64 },
}

/// One recorded gap with its evidence trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapRecord {
    pub kind: GapKind,
    pub observed_at_wall_ns: u64,
    pub evidence: String,
}

/// Inclusive slot bounds of the collection attempt. Bounded by contract:
/// completeness checks iterate the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageWindow {
    pub from_slot: u64,
    pub to_slot: u64,
}

impl CoverageWindow {
    pub fn contains(&self, slot: u64) -> bool {
        self.from_slot <= slot && slot <= self.to_slot
    }

    fn every_slot_verified(&self, verified: &BTreeSet<u64>) -> bool {
        verified.range(self.from_slot..=self.to_slot).count() as u64
            == self.to_slot - self.from_slot + 1
    }
}

/// Per-mint coverage entry. Records that a creation was observed and how
/// complete this mint's evidence is — never what the token later did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintCoverage {
    pub mint: Pubkey,
    pub created_observed: bool,
    pub state: CoverageState,
}

/// Completeness ledger for one bounded window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageLedger {
    window: CoverageWindow,
    mints: BTreeMap<Pubkey, MintCoverage>,
    gaps: Vec<GapRecord>,
    verified_blocks: BTreeSet<u64>,
    right_censored: Option<String>,
}

impl CoverageLedger {
    pub fn new(window: CoverageWindow) -> Self {
        Self {
            window,
            mints: BTreeMap::new(),
            gaps: Vec::new(),
            verified_blocks: BTreeSet::new(),
            right_censored: None,
        }
    }

    /// Record a gap in the evidence. Any gap degrades window completeness.
    pub fn record_gap(&mut self, record: GapRecord) {
        self.gaps.push(record);
    }

    /// Record that a slot's block was actually retrieved and reconciled —
    /// the only input that can support a `Complete` verdict.
    pub fn record_verified_block(&mut self, slot: u64) {
        self.verified_blocks.insert(slot);
    }

    /// Record an observed creation. Every observed launch enters the
    /// denominator — including mints that never trade, never graduate, or
    /// fail. (No success-only sampling.)
    pub fn record_create(&mut self, mint: Pubkey) {
        self.mints.entry(mint).or_insert(MintCoverage {
            mint,
            created_observed: false,
            state: CoverageState::Unknown,
        });
        let entry = self.mints.get_mut(&mint).expect("just inserted");
        entry.created_observed = true;
        if entry.state == CoverageState::Unknown {
            entry.state = CoverageState::Incomplete;
        }
    }

    /// Mark the window right-censored: observation stopped before evidence
    /// could complete. Downstream absence-of-event conclusions for this
    /// window are censored, not negative findings.
    pub fn mark_right_censored(&mut self, reason: &str) {
        self.right_censored = Some(reason.to_string());
        for entry in self.mints.values_mut() {
            entry.state = CoverageState::Censored;
        }
    }

    /// Overall window completeness.
    pub fn window_state(&self) -> CoverageState {
        if self.right_censored.is_some() {
            return CoverageState::Censored;
        }
        // Recorded gaps are positive evidence of holes: they make the window
        // Incomplete even when nothing else was collected.
        if !self.gaps.is_empty() {
            return CoverageState::Incomplete;
        }
        if self.verified_blocks.is_empty() && self.mints.is_empty() {
            return CoverageState::Unknown;
        }
        if self.window.every_slot_verified(&self.verified_blocks) {
            CoverageState::Complete
        } else {
            CoverageState::Incomplete
        }
    }

    /// Per-mint coverage state.
    pub fn coverage_for(&self, mint: &Pubkey) -> CoverageState {
        self.mints
            .get(mint)
            .map(|m| m.state)
            .unwrap_or(CoverageState::Unknown)
    }

    pub fn gap_count(&self) -> usize {
        self.gaps.len()
    }

    /// The full creation denominator observed in this window.
    pub fn mints(&self) -> Vec<Pubkey> {
        self.mints.keys().copied().collect()
    }

    pub fn mint_count(&self) -> usize {
        self.mints.len()
    }

    pub fn creates_observed(&self) -> usize {
        self.mints.values().filter(|m| m.created_observed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_contains_is_inclusive() {
        let window = CoverageWindow {
            from_slot: 5,
            to_slot: 7,
        };
        assert!(window.contains(5) && window.contains(7));
        assert!(!window.contains(4) && !window.contains(8));
    }

    #[test]
    fn create_then_censor_overwrites_incomplete_only() {
        let mut ledger = CoverageLedger::new(CoverageWindow {
            from_slot: 0,
            to_slot: 1,
        });
        let mint = Pubkey::new_from_array([1; 32]);
        ledger.record_create(mint);
        assert_eq!(ledger.coverage_for(&mint), CoverageState::Incomplete);
        ledger.mark_right_censored("window closed");
        assert_eq!(ledger.coverage_for(&mint), CoverageState::Censored);
    }
}
