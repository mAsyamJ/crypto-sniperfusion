//! H6-1 contract tests: canonical provenance, causality, coverage, labels.
//!
//! These tests pin the DATASET-layer contracts required for dual-order replay
//! (chain order vs observation order), persistent deduplication, no-lookahead
//! causality, honest coverage states, and the physical separation of future
//! outcome labels from strategy-visible features.

use sniper_core::coverage::{CoverageLedger, CoverageState, CoverageWindow, GapKind, GapRecord};
use sniper_core::event::{ChainEventId, Commitment, ProviderKind};
use sniper_core::labels::{DatasetNamespace, OutcomeLabels};
use sniper_core::observation::{
    chain_sort_records, ensure_decidable, CanonicalOrder, CausalityClocks, CausalityError,
    ChainSortError, ForkStatus, HistoricalDedup, ObservationProvenance, ObservedRecord,
};
use sniper_core::time::Timestamps;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

fn sig(seed: u8) -> Signature {
    let mut bytes = [0u8; 64];
    bytes[0] = seed;
    Signature::from(bytes)
}

fn event_id(seed: u8, outer: u32, inner: Option<u32>, ordinal: u16) -> ChainEventId {
    ChainEventId {
        signature: sig(seed),
        outer_ix_index: outer,
        inner_ix_index: inner,
        event_ordinal: ordinal,
    }
}

fn provenance(seed: u8, commitment: Commitment) -> ObservationProvenance {
    ObservationProvenance {
        provider: ProviderKind::Logs,
        commitment,
        commitment_history: vec![sniper_core::observation::CommitmentObservation {
            commitment,
            observed_at_wall_ns: 1_000 + seed as u64,
        }],
        received: Timestamps {
            wall: std::time::SystemTime::UNIX_EPOCH,
            mono_ns: seed as u64,
        },
        process_epoch: 1,
        payload_hash: sniper_core::observation::PayloadHash([seed; 32]),
        program_version: Some(1),
        fork: sniper_core::observation::ForkIdentity {
            slot: 900,
            block_hash: [7; 32],
            status: ForkStatus::Live,
        },
    }
}

fn record(
    seed: u8,
    slot: u64,
    tx_index: Option<u64>,
    outer: u32,
    inner: Option<u32>,
    ordinal: u16,
) -> ObservedRecord {
    ObservedRecord {
        event_id: event_id(seed, outer, inner, ordinal),
        order: CanonicalOrder {
            slot,
            transaction_index: tx_index,
            outer_ix_index: outer,
            inner_ix_index: inner,
            event_ordinal: ordinal,
        },
        provenance: provenance(seed, Commitment::Processed),
        clocks: CausalityClocks {
            chain_at_ms: Some(slot * 400),
            observed_at_ms: slot * 400 + 5,
            feature_available_at_ms: slot * 400 + 10,
        },
        mint: Pubkey::new_from_array([seed; 32]),
    }
}

// ---------------------------------------------------------------- ordering --

#[test]
fn chain_sort_orders_by_full_canonical_key() {
    // Deliberately unordered: same slot, different transaction indexes; and
    // same instruction, different ordinals.
    let mut records = vec![
        record(1, 100, Some(2), 0, None, 0),
        record(2, 100, Some(1), 3, Some(0), 0),
        record(3, 101, Some(0), 0, None, 0),
        record(4, 100, Some(1), 3, Some(0), 1),
        record(5, 100, Some(1), 1, None, 0),
    ];
    chain_sort_records(&mut records).expect("all coordinates known");
    let ids: Vec<u8> = records
        .iter()
        .map(|r| r.event_id.signature.as_ref()[0])
        .collect();
    // slot 100 first (tx 1: ix1 -> ix3 inner 0 ord 0 -> ord 1, then tx 2), slot 101 last.
    assert_eq!(ids, vec![5, 2, 4, 1, 3]);
}

#[test]
fn chain_sort_rejects_unknown_transaction_index_rather_than_fabricating() {
    let mut records = vec![
        record(1, 100, Some(0), 0, None, 0),
        record(2, 100, None, 0, None, 0), // log-derived, no block evidence
    ];
    match chain_sort_records(&mut records) {
        Err(ChainSortError::UnknownTransactionIndex { slot, .. }) => assert_eq!(slot, 100),
        other => panic!("expected UnknownTransactionIndex, got {other:?}"),
    }
}

#[test]
fn instruction_itself_precedes_its_inner_instructions() {
    // outer 2 with inner None = the top-level instruction; it executes before
    // its CPI children (inner Some(0), Some(1)).
    let mut records = vec![
        record(1, 100, Some(0), 2, Some(1), 0),
        record(2, 100, Some(0), 2, None, 0),
        record(3, 100, Some(0), 2, Some(0), 0),
    ];
    chain_sort_records(&mut records).unwrap();
    let ids: Vec<u8> = records
        .iter()
        .map(|r| r.event_id.signature.as_ref()[0])
        .collect();
    assert_eq!(ids, vec![2, 3, 1]);
}

#[test]
fn canonical_status_distinguishes_chain_orderable_from_observation_only() {
    let chain_orderable = record(1, 100, Some(3), 0, None, 0);
    assert!(chain_orderable.canonical_status().is_chain_orderable());
    let observation_only = record(2, 100, None, 0, None, 0);
    assert!(!observation_only.canonical_status().is_chain_orderable());
}

// -------------------------------------------------------------- provenance --

#[test]
fn commitment_upgrades_one_way_and_keeps_history() {
    let mut prov = provenance(1, Commitment::Processed);
    prov.observe_commitment(Commitment::Confirmed, 2_000)
        .unwrap();
    prov.observe_commitment(Commitment::Finalized, 3_000)
        .unwrap();
    assert_eq!(prov.commitment, Commitment::Finalized);
    assert_eq!(prov.commitment_history.len(), 3);
    // Same level re-observation is idempotent, not a downgrade.
    prov.observe_commitment(Commitment::Finalized, 4_000)
        .unwrap();
    assert!(prov
        .observe_commitment(Commitment::Confirmed, 5_000)
        .is_err());
}

#[test]
fn abandoned_fork_observation_is_never_a_canonical_fact() {
    let mut prov = provenance(1, Commitment::Finalized);
    assert!(prov.is_canonical_fact());
    prov.mark_fork_abandoned();
    assert_eq!(prov.fork.status, ForkStatus::Abandoned);
    assert!(
        !prov.is_canonical_fact(),
        "an observation on an abandoned fork must not be treated as canonical truth"
    );
}

#[test]
fn non_finalized_commitment_is_not_yet_canonical() {
    let prov = provenance(1, Commitment::Confirmed);
    assert!(!prov.is_canonical_fact());
}

// ---------------------------------------------------------------- causality --

#[test]
fn no_lookahead_is_rejected() {
    let clocks = CausalityClocks {
        chain_at_ms: Some(1_000),
        observed_at_ms: 1_005,
        feature_available_at_ms: 2_000,
    };
    match ensure_decidable(&clocks, 1_500) {
        Err(CausalityError::FeatureNotYetAvailable {
            feature_available_at_ms,
            decision_at_ms,
        }) => {
            assert_eq!(feature_available_at_ms, 2_000);
            assert_eq!(decision_at_ms, 1_500);
        }
        other => panic!("expected FeatureNotYetAvailable, got {other:?}"),
    }
    assert!(ensure_decidable(&clocks, 2_000).is_ok());
}

#[test]
fn chain_time_after_receipt_time_is_allowed_clock_uncertainty() {
    // Solana block timestamps are estimates; a chain_at estimate LATER than
    // the receipt time must not invalidate the observation. Only
    // feature_available <= decision is enforced.
    let clocks = CausalityClocks {
        chain_at_ms: Some(9_999),
        observed_at_ms: 1_005,
        feature_available_at_ms: 1_010,
    };
    assert!(ensure_decidable(&clocks, 1_010).is_ok());
    // Absent chain time is also fine.
    let no_chain = CausalityClocks {
        chain_at_ms: None,
        observed_at_ms: 1_005,
        feature_available_at_ms: 1_010,
    };
    assert!(ensure_decidable(&no_chain, 1_010).is_ok());
}

// -------------------------------------------------------------------- dedup --

#[test]
fn historical_dedup_survives_process_restart() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("dedup.jsonl");
    let id = event_id(1, 0, None, 0);
    {
        let mut dedup = HistoricalDedup::persistent(&journal).unwrap();
        assert!(dedup.register(id).unwrap().is_first_seen());
        // Same process: duplicate detected.
        assert!(dedup.register(id).unwrap().is_duplicate());
    }
    // New process instance over the same journal: still a duplicate.
    let mut revived = HistoricalDedup::persistent(&journal).unwrap();
    assert!(
        revived.register(id).unwrap().is_duplicate(),
        "dedup state must persist across restarts"
    );
    assert_eq!(revived.duplicates_observed(), 1);
}

#[test]
fn corrupt_dedup_journal_is_a_hard_error() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("dedup.jsonl");
    std::fs::write(&journal, "not-json\n").unwrap();
    assert!(HistoricalDedup::persistent(&journal).is_err());
}

// ----------------------------------------------------------------- coverage --

fn window() -> CoverageWindow {
    CoverageWindow {
        from_slot: 100,
        to_slot: 102,
    }
}

fn ledger_with_all_blocks() -> CoverageLedger {
    let mut ledger = CoverageLedger::new(window());
    for slot in 100..=102 {
        ledger.record_verified_block(slot);
    }
    ledger
}

#[test]
fn coverage_complete_only_with_full_block_evidence_and_no_gaps() {
    let ledger = ledger_with_all_blocks();
    assert_eq!(ledger.window_state(), CoverageState::Complete);

    // One slot without block evidence: incomplete.
    let mut partial = CoverageLedger::new(window());
    partial.record_verified_block(100);
    partial.record_verified_block(102);
    assert_eq!(partial.window_state(), CoverageState::Incomplete);

    // Any gap degrades a fully verified window.
    let mut gapped = ledger_with_all_blocks();
    gapped.record_gap(GapRecord {
        kind: GapKind::RateLimited,
        observed_at_wall_ns: 1,
        evidence: "provider 429 during slot 101".into(),
    });
    assert_eq!(gapped.window_state(), CoverageState::Incomplete);
    assert_eq!(gapped.gap_count(), 1);
}

#[test]
fn coverage_unknown_when_there_is_no_evidence_at_all() {
    let ledger = CoverageLedger::new(window());
    assert_eq!(ledger.window_state(), CoverageState::Unknown);
}

#[test]
fn right_censoring_marks_window_and_mints_censored() {
    let mut ledger = ledger_with_all_blocks();
    let mint = Pubkey::new_from_array([9; 32]);
    ledger.record_create(mint);
    ledger.mark_right_censored("observation window closed before lifecycle resolution");
    assert_eq!(ledger.window_state(), CoverageState::Censored);
    assert_eq!(ledger.coverage_for(&mint), CoverageState::Censored);
    // Censoring is about DATA availability, never a claim the token died.
}

#[test]
fn pruned_and_missing_transactions_are_explicit_gap_kinds() {
    let mut ledger = CoverageLedger::new(window());
    ledger.record_gap(GapRecord {
        kind: GapKind::PrunedTransaction {
            signature: sig(3).to_string(),
        },
        observed_at_wall_ns: 5,
        evidence: "getTransaction returned null".into(),
    });
    ledger.record_gap(GapRecord {
        kind: GapKind::SkippedSlots {
            from_slot: 101,
            to_slot: 101,
        },
        observed_at_wall_ns: 6,
        evidence: "block skipped".into(),
    });
    assert_eq!(ledger.gap_count(), 2);
    assert_eq!(ledger.window_state(), CoverageState::Incomplete);
}

#[test]
fn denominator_keeps_never_traded_mints() {
    let mut ledger = ledger_with_all_blocks();
    let never_traded = Pubkey::new_from_array([1; 32]);
    let graduated = Pubkey::new_from_array([2; 32]);
    ledger.record_create(never_traded);
    ledger.record_create(graduated);
    // No lifecycle information is recorded or consulted: coverage is a
    // data-completeness concept only.
    assert_eq!(ledger.creates_observed(), 2);
    assert_eq!(ledger.mint_count(), 2);
    assert!(ledger.mints().contains(&never_traded));
    assert_eq!(ledger.window_state(), CoverageState::Complete);
}

// ------------------------------------------------------------------- labels --

#[test]
fn labels_live_in_a_separate_namespace_from_features() {
    assert_ne!(
        DatasetNamespace::Labels.directory(),
        DatasetNamespace::Features.directory()
    );
    let labels = OutcomeLabels {
        mint: Pubkey::new_from_array([3; 32]),
        graduated: Some(true),
        graduation_slot: Some(555),
        max_observed_price_lamports_per_token: Some(123),
        window_end_phase: Some("PUMPSWAP_OPEN".into()),
    };
    let json = serde_json::to_string(&labels).unwrap();
    assert!(json.contains("graduated"));
    assert!(
        !json.contains("unique_buyers"),
        "label records must not carry strategy-visible feature fields"
    );
    let back: OutcomeLabels = serde_json::from_str(&json).unwrap();
    assert_eq!(back.graduation_slot, Some(555));
}

#[test]
fn observed_records_roundtrip_through_jsonl() {
    let records = vec![
        record(1, 100, Some(0), 0, None, 0),
        record(2, 101, None, 1, Some(2), 3),
    ];
    let json: Vec<String> = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect();
    let reloaded: Vec<ObservedRecord> = json
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records, reloaded);
}
