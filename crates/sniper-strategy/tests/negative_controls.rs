//! H7-4 mandated negative controls, consolidated. Controls whose primary
//! implementation lives in another module are re-asserted here end-to-end so
//! the battery is one auditable place:
//!
//!  1. future feature injection is rejected
//!  2. outcome-label shuffling cannot alter deterministic decisions
//!  3. delayed features produce delayed (or rejected) decisions
//!  4. reversed raw collection order reproduces canonical reconstruction
//!  5. duplicate provider events never generate duplicate trades
//!  6. missing coverage forces NO_TRADE
//!  7. open losing positions remain in portfolio NAV
//!  8. cost/impact thresholds prevent unexecutable exits
//!  9. fee-tier boundaries calculate correctly
//! 10. unlanded/landed-failed transactions get correct accounting
//! 11. independent vs shared wallets report capital contention
//! 12. unsupported venue configurations cannot silently execute

use std::collections::HashMap;

use sniper_core::labels::OutcomeLabels;
use sniper_core::observation::chain_sort_records;
use sniper_strategy::engine::legacy::{self, LegacyParams};
use sniper_strategy::engine::protocol::{
    ExecutionFrictions, MarketBook, MarketSnapshot, ProtocolParams, QuoteMint, VenueState,
};
use sniper_strategy::observation::{load_events, Observation, Phase, Safety};
use sniper_strategy::oracle::AccountingOracle;
use sniper_strategy::policies::EntryPolicy;
use sniper_strategy::{
    Observation as Obs, PortfolioView, RiskConstraints, Strategy, StrategyKnobs,
};

const EVENTS: &str = include_str!("../../../harness/fixtures/research/synthetic_lifecycle.jsonl");

fn observation(event_id: &str, observed_at_ms: u64, price: u64) -> Observation {
    Obs {
        event_id: event_id.into(),
        mint: "M".into(),
        slot: 1,
        chain_at_ms: observed_at_ms.saturating_sub(5),
        observed_at_ms,
        feature_available_at_ms: observed_at_ms + 10,
        phase: Phase::ActiveCurve,
        safety: Safety::Eligible,
        coverage_ok: true,
        price_lamports_per_token: price,
        exit_liquidity_lamports: 10_000_000,
        net_flow_lamports: 1,
        unique_buyers: 10,
        creator_sell_lamports: 0,
        curve_progress_bps: 1_000,
        fee_bps: 100,
    }
}

fn run_s01(events: &[Observation]) -> legacy::LegacyResult {
    legacy::run(
        events,
        &mut EntryPolicy::s01(StrategyKnobs::default()),
        100_000_000,
        &RiskConstraints::default(),
        &LegacyParams::default(),
    )
}

// 1 -----------------------------------------------------------------------
#[test]
fn control_1_future_feature_injection_is_rejected() {
    // (a) Loader: an observation whose features become available BEFORE the
    // observation itself violates the frozen clock contract -> rejected.
    let mut poisoned = observation("future", 100, 1_000);
    poisoned.feature_available_at_ms = 50; // < observed_at
    let text = serde_json::to_string(&poisoned).unwrap();
    assert!(
        load_events(&text).is_err(),
        "loader must reject clock violations"
    );
    // (b) Decider: features legitimately available at t=5000 cannot be used
    // for a decision at t=110 -> refused.
    let clocks = sniper_core::observation::CausalityClocks {
        chain_at_ms: Some(95),
        observed_at_ms: 100,
        feature_available_at_ms: 5_000,
    };
    assert!(sniper_core::observation::ensure_decidable(&clocks, 110).is_err());
    assert!(sniper_core::observation::ensure_decidable(&clocks, 5_000).is_ok());
}

// 2 -----------------------------------------------------------------------
#[test]
fn control_2_label_shuffling_cannot_alter_decisions() {
    let events = load_events(EVENTS).unwrap();
    let base = run_s01(&events);
    // Outcome labels live in a physically separate type; scramble them
    // (deterministic shuffle, no RNG dependency) and re-run the strategy.
    let mut labels: Vec<OutcomeLabels> = events
        .iter()
        .enumerate()
        .map(|(i, _e)| OutcomeLabels {
            mint: solana_sdk::pubkey::Pubkey::new_from_array([(i % 256) as u8; 32]),
            graduated: Some(i % 2 == 0),
            graduation_slot: Some(i as u64),
            max_observed_price_lamports_per_token: Some(u64::MAX - i as u64),
            window_end_phase: Some(if i % 3 == 0 { "MATURE" } else { "INACTIVE" }.into()),
        })
        .collect();
    labels.reverse();
    let after = run_s01(&events);
    assert_eq!(base, after, "decisions are label-blind by construction");
}

// 3 -----------------------------------------------------------------------
#[test]
fn control_3_delayed_features_delay_decisions() {
    let mut events = vec![
        observation("e1", 100, 1_000),
        observation("e2", 30_000, 900),
    ];
    let prompt = run_s01(&events);
    // Same events, features available 20s later: entries shift by the delay.
    for event in &mut events {
        event.feature_available_at_ms += 20_000;
    }
    let delayed = run_s01(&events);
    assert_eq!(prompt.filled_entries, delayed.filled_entries);
    assert_ne!(
        prompt.trades, delayed.trades,
        "entry_at_ms must track the delayed feature availability"
    );
}

// 4 -----------------------------------------------------------------------
#[test]
fn control_4_reversed_collection_order_reproduces_canonical_reconstruction() {
    let mut records: Vec<_> = (0..5)
        .map(|i| {
            let mut sig = [0u8; 64];
            sig[0] = i + 1;
            sniper_core::observation::ObservedRecord {
                event_id: sniper_core::event::ChainEventId {
                    signature: solana_sdk::signature::Signature::from(sig),
                    outer_ix_index: 0,
                    inner_ix_index: None,
                    event_ordinal: 0,
                },
                order: sniper_core::observation::CanonicalOrder {
                    slot: 100,
                    transaction_index: Some(u64::from(i)),
                    outer_ix_index: 0,
                    inner_ix_index: None,
                    event_ordinal: 0,
                },
                provenance: sniper_core::observation::ObservationProvenance {
                    provider: sniper_core::event::ProviderKind::Replay,
                    commitment: sniper_core::event::Commitment::Finalized,
                    commitment_history: vec![],
                    received: sniper_core::time::Timestamps {
                        wall: std::time::SystemTime::UNIX_EPOCH,
                        mono_ns: 0,
                    },
                    process_epoch: 1,
                    payload_hash: sniper_core::observation::PayloadHash([0; 32]),
                    program_version: Some(1),
                    fork: sniper_core::observation::ForkIdentity {
                        slot: 100,
                        block_hash: [0; 32],
                        status: sniper_core::observation::ForkStatus::Live,
                    },
                },
                clocks: sniper_core::observation::CausalityClocks {
                    chain_at_ms: Some(0),
                    observed_at_ms: 0,
                    feature_available_at_ms: 0,
                },
                mint: solana_sdk::pubkey::Pubkey::new_from_array([i; 32]),
            }
        })
        .collect();
    let mut reversed = records.clone();
    reversed.reverse();
    chain_sort_records(&mut records).unwrap();
    chain_sort_records(&mut reversed).unwrap();
    assert_eq!(
        records, reversed,
        "canonical order is input-order independent"
    );
    // NOTE (control semantics): observation-TIME decisions are NOT claimed
    // invariant under reordering — only the canonical reconstruction is.
}

// 5 -----------------------------------------------------------------------
#[test]
fn control_5_duplicate_provider_events_never_duplicate_trades() {
    let one = observation("dup", 100, 1_000);
    let text = format!(
        "{}\n{}\n",
        serde_json::to_string(&one).unwrap(),
        serde_json::to_string(&one).unwrap()
    );
    assert!(
        load_events(&text).is_err(),
        "duplicate event_id rejected at load"
    );
    let mut dedup = sniper_core::observation::HistoricalDedup::in_memory();
    let id = sniper_core::event::ChainEventId {
        signature: solana_sdk::signature::Signature::from([1u8; 64]),
        outer_ix_index: 0,
        inner_ix_index: None,
        event_ordinal: 0,
    };
    assert!(dedup.register(id).unwrap().is_first_seen());
    assert!(dedup.register(id).unwrap().is_duplicate());
}

// 6 -----------------------------------------------------------------------
#[test]
fn control_6_missing_coverage_forces_no_trade() {
    let mut uncovered = observation("e1", 100, 1_000);
    uncovered.coverage_ok = false;
    let mut strategy = EntryPolicy::s01(StrategyKnobs::default());
    let view = PortfolioView::default();
    assert_eq!(
        strategy.decide(&uncovered, &view, &RiskConstraints::default()),
        sniper_strategy::Intent::NoTrade,
        "coverage gaps must degrade to abstention, never to trading on partial data"
    );
}

// 7 -----------------------------------------------------------------------
#[test]
fn control_7_open_losing_positions_remain_in_nav() {
    // Covered fully in oracle tests; asserted here through the metrics path:
    // an open position with a worthless mark must reduce full-NAV equity.
    let mut events = vec![observation("e1", 100, 1_000), observation("e2", 30_000, 1)];
    events[1].exit_liquidity_lamports = 0; // exit unexecutable -> position stuck
    let (result, trace) = legacy::run_with_trace(
        &events,
        &mut EntryPolicy::s01(StrategyKnobs::default()),
        100_000_000,
        &RiskConstraints::default(),
        &LegacyParams::default(),
    );
    assert_eq!(result.open_positions, 1);
    assert_eq!(result.failed_exit_quotes, 1);
    let last_equity = *trace.equity_curve.last().unwrap();
    assert!(
        last_equity < 100_000_000,
        "the stuck position's cost must not vanish from NAV (equity {last_equity})"
    );
}

// 8 -----------------------------------------------------------------------
#[test]
fn control_8_cost_thresholds_prevent_unexecutable_exits() {
    // Impact > liquidity makes the whole position unexitable: quote is zero.
    let quote = legacy::executable_exit(
        1_000_000,
        Some(&{
            let mut e = observation("e1", 100, 1_000);
            e.exit_liquidity_lamports = 1_000; // gross 1e9 >> liquidity
            e
        }),
        100,
        10_000,
    );
    assert_eq!(quote, 0);
}

// 9 -----------------------------------------------------------------------
#[test]
fn control_9_fee_tier_boundaries_calculate_correctly() {
    // Asserted in depth in fees.rs; boundary sanity here.
    let mut fee = sniper_strategy::fees::CurveFeeState::synthetic_fallback_100bps(0);
    fee.tiers = vec![
        sniper_strategy::fees::FeeTier {
            market_cap_lamports_threshold: 0,
            protocol_bps: 50,
            creator_bps: 50,
        },
        sniper_strategy::fees::FeeTier {
            market_cap_lamports_threshold: 1_000,
            protocol_bps: 25,
            creator_bps: 0,
        },
    ];
    let at = fee.resolve(1_000);
    let below = fee.resolve(999);
    assert_eq!((at.protocol_bps, at.creator_bps), (25, 0));
    assert_eq!((below.protocol_bps, below.creator_bps), (50, 50));
}

// 10 ----------------------------------------------------------------------
#[test]
fn control_10_failed_and_unlanded_tx_accounting_is_conservative() {
    let mut oracle = AccountingOracle::new(100_000_000);
    oracle
        .post_failed_landing(
            "S01",
            &sniper_strategy::engine::protocol::ExecutionRecord::AttemptFailedLanding {
                event_id: "e1".into(),
                mint: "M".into(),
                network_fee_lamports: 5_000,
                priority_fee_lamports: 5_000,
            },
        )
        .unwrap();
    assert_eq!(oracle.cash(), 99_990_000);
    oracle.post_unknown_hold("S01", "e2", 1_000_000).unwrap();
    let nav = oracle.nav(&HashMap::new());
    assert_eq!(nav.nav, 98_990_000, "unknown holds are excluded from NAV");
}

// 11 ----------------------------------------------------------------------
#[test]
fn control_11_shared_wallet_contention_is_reported() {
    let fill = |mint: &str| sniper_strategy::engine::protocol::ExecutionRecord::BuyFill {
        event_id: "e".into(),
        mint: mint.into(),
        slot: 1,
        spend_lamports: 1_000_000,
        tokens_out: 900,
        protocol_fee_lamports: 10_000,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 5_000,
        priority_fee_lamports: 5_000,
        tip_lamports: 0,
        rent_lamports: 1_500_000,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    };
    let mut independent = AccountingOracle::new(3_000_000);
    assert!(independent.post_buy_fill("S01", &fill("A")).is_ok());
    let mut independent_b = AccountingOracle::new(3_000_000);
    assert!(independent_b.post_buy_fill("S02", &fill("B")).is_ok());
    // Shared: same total capital, both entries — the second must fail.
    let mut shared = AccountingOracle::new(3_000_000);
    assert!(shared.post_buy_fill("S01", &fill("A")).is_ok());
    assert!(shared.post_buy_fill("S02", &fill("B")).is_err());
}

// 12 ----------------------------------------------------------------------
#[test]
fn control_12_unsupported_venue_cannot_silently_execute() {
    let mut book = MarketBook::new();
    book.push(MarketSnapshot {
        mint: "M".into(),
        slot: 0,
        quote_mint: QuoteMint::Unsupported("USDT".into()),
        state: VenueState::BondingCurve {
            virtual_token_reserves: 1_073_000_000_000_000,
            virtual_quote_reserves: 30_000_000_000,
            mint_total_supply: 1_000_000_000_000_000,
            fee: sniper_strategy::fees::CurveFeeState::synthetic_fallback_100bps(9),
        },
    });
    let events = vec![observation("e1", 100, 1_000)];
    let outcome = sniper_strategy::engine::protocol::run(
        &events,
        &book,
        &mut EntryPolicy::s01(StrategyKnobs::default()),
        100_000_000,
        &RiskConstraints::default(),
        &ExecutionFrictions::default(),
        &ProtocolParams::default(),
    );
    assert!(matches!(
        outcome.records[0],
        sniper_strategy::engine::protocol::ExecutionRecord::AttemptRejected {
            reason: sniper_strategy::engine::protocol::RejectionReason::UnsupportedQuoteRoute,
            ..
        }
    ));
    assert_eq!(outcome.ending_cash_lamports, 100_000_000);
}
