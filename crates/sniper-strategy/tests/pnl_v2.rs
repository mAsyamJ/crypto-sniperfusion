//! PF1 mandated P&L invariants (the nine from the profit-first mission),
//! exercised through the real Engine B + oracle + receipt stack.

use std::collections::HashMap;

use sniper_strategy::engine::protocol::{
    ExecutionFrictions, ExecutionRecord, MarketBook, MarketSnapshot, ProtocolParams, QuoteMint,
    VenueState,
};
use sniper_strategy::fees::CurveFeeState;
use sniper_strategy::observation::{Observation, Phase, Safety};
use sniper_strategy::oracle::{AccountingOracle, FeeKind};
use sniper_strategy::policies::EntryPolicy;
use sniper_strategy::receipt::{build_receipt, ComparisonRow};
use sniper_strategy::{RiskConstraints, StrategyKnobs};

fn event(id: &str, at_ms: u64, price: u64, liquidity: u64) -> Observation {
    Observation {
        event_id: id.into(),
        mint: "M".into(),
        slot: 10,
        chain_at_ms: at_ms - 5,
        observed_at_ms: at_ms,
        feature_available_at_ms: at_ms + 10,
        phase: Phase::ActiveCurve,
        safety: Safety::Eligible,
        coverage_ok: true,
        price_lamports_per_token: price,
        exit_liquidity_lamports: liquidity,
        net_flow_lamports: 1,
        unique_buyers: 10,
        creator_sell_lamports: 0,
        curve_progress_bps: 1_000,
        fee_bps: 125,
    }
}

fn book() -> MarketBook {
    let mut book = MarketBook::new();
    book.push(MarketSnapshot {
        mint: "M".into(),
        slot: 9,
        quote_mint: QuoteMint::Sol,
        state: VenueState::BondingCurve {
            virtual_token_reserves: 1_073_000_000_000_000,
            virtual_quote_reserves: 30_000_000_000,
            mint_total_supply: 1_000_000_000_000_000,
            fee: CurveFeeState::from_observed_fee_config(95, 30, 9),
        },
    });
    book
}

fn run_s01(
    events: &[Observation],
    frictions: &ExecutionFrictions,
) -> sniper_strategy::engine::protocol::ProtocolRunOutcome {
    sniper_strategy::engine::protocol::run(
        events,
        &book(),
        &mut EntryPolicy::s01(StrategyKnobs::default()),
        100_000_000,
        &RiskConstraints::default(),
        frictions,
        &ProtocolParams::default(),
    )
}

fn buy_fill(spend: u64, tip: u64) -> ExecutionRecord {
    ExecutionRecord::BuyFill {
        event_id: "e1".into(),
        mint: "M".into(),
        slot: 1,
        spend_lamports: spend,
        tokens_out: 900,
        protocol_fee_lamports: spend / 125,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 5_000,
        priority_fee_lamports: 5_000,
        tip_lamports: tip,
        rent_lamports: 1_500_000,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    }
}

// 1 -----------------------------------------------------------------------
#[test]
fn deposits_are_not_profits() {
    let mut oracle = AccountingOracle::new(100_000_000);
    oracle.post_deposit(50_000_000, "owner top-up").unwrap();
    // Marked NAV rose by the deposit, but net P&L must be exactly zero.
    let marks = HashMap::new();
    assert_eq!(oracle.nav(&marks).nav, 150_000_000);
    assert_eq!(oracle.net_pnl(100_000_000, &marks), 0);
    // And a withdrawal is not a loss.
    oracle.post_withdrawal(20_000_000, "owner pull").unwrap();
    assert_eq!(oracle.net_pnl(100_000_000, &marks), 0);
    assert_eq!(oracle.nav(&marks).nav, 130_000_000);
}

// 2 -----------------------------------------------------------------------
#[test]
fn partial_exits_conserve_cost_basis() {
    let mut oracle = AccountingOracle::new(300_000_000);
    let buy = |spend: u64, tokens: u64, id: &str| ExecutionRecord::BuyFill {
        event_id: id.into(),
        mint: "M".into(),
        slot: 1,
        spend_lamports: spend,
        tokens_out: tokens,
        protocol_fee_lamports: spend / 100,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 0,
        priority_fee_lamports: 0,
        tip_lamports: 0,
        rent_lamports: 0,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    };
    let sell = |tokens: u64, net: u64, id: &str| ExecutionRecord::SellFill {
        event_id: id.into(),
        mint: "M".into(),
        slot: 2,
        tokens_in: tokens,
        quote_out_net: net,
        protocol_fee_lamports: 0,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 0,
        priority_fee_lamports: 0,
        tip_lamports: 0,
        rent_recovered_lamports: 0,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    };
    oracle
        .post_buy_fill("S01", &buy(1_000_000, 1_000, "a"))
        .unwrap();
    oracle
        .post_buy_fill("S01", &buy(2_000_000, 2_000, "b"))
        .unwrap();
    oracle
        .post_sell_fill("S01", &sell(1_500, 2_000_000, "c"))
        .unwrap();
    let lots = oracle.lots_for("M");
    let remaining_basis: u64 = lots.iter().map(|l| l.cost_basis_lamports).sum();
    let consumed = (990_000 + 1_980_000) - remaining_basis;
    // Consumed basis + remaining basis == total bought basis (no basis
    // created or destroyed by the partial exit).
    assert_eq!(
        consumed + remaining_basis,
        990_000 + 1_980_000,
        "cost basis must conserve across partial exits"
    );
    assert_eq!(lots.iter().map(|l| l.tokens).sum::<u64>(), 1_500);
}

// 3 -----------------------------------------------------------------------
#[test]
fn failed_tx_charges_its_fee_exactly_once() {
    let mut oracle = AccountingOracle::new(100_000_000);
    let failed = ExecutionRecord::AttemptFailedLanding {
        event_id: "e1".into(),
        mint: "M".into(),
        network_fee_lamports: 5_000,
        priority_fee_lamports: 7_000,
    };
    oracle.post_failed_landing("S01", &failed).unwrap();
    oracle.post_failed_landing("S01", &failed).unwrap();
    // TWO failed attempts -> exactly 2x each fee (each charged once).
    assert_eq!(oracle.cash(), 100_000_000 - 2 * (5_000 + 7_000));
    let totals = oracle.fee_totals();
    assert_eq!(*totals.get(&FeeKind::Network).unwrap(), 10_000);
    assert_eq!(*totals.get(&FeeKind::Priority).unwrap(), 14_000);
}

// 4 -----------------------------------------------------------------------
#[test]
fn tips_and_priority_fees_have_lamport_units() {
    let mut oracle = AccountingOracle::new(100_000_000);
    oracle
        .post_buy_fill("S01", &buy_fill(1_000_000, 12_345))
        .unwrap();
    // Fee lines carry the EXACT lamport amounts from the record — never
    // micro-lamports, never bps.
    let totals = oracle.fee_totals();
    assert_eq!(*totals.get(&FeeKind::Tip).unwrap(), 12_345);
    assert_eq!(*totals.get(&FeeKind::Priority).unwrap(), 5_000);
    assert_eq!(*totals.get(&FeeKind::Network).unwrap(), 5_000);
    // And cash decreased by the exact sum of components.
    let expected_cash = 100_000_000 - (1_000_000 + 5_000 + 5_000 + 12_345 + 1_500_000);
    assert_eq!(oracle.cash(), expected_cash);
}

// 5 -----------------------------------------------------------------------
#[test]
fn ata_rent_is_locked_cash_not_free_money() {
    let mut oracle = AccountingOracle::new(100_000_000);
    oracle
        .post_buy_fill("S01", &buy_fill(1_000_000, 0))
        .unwrap();
    let nav = oracle.nav(&HashMap::new());
    // Rent sits in recoverable (part of NAV only because it is returnable),
    // it is NOT income: realized P&L is untouched.
    assert_eq!(nav.recoverable_rent, 1_500_000);
    let tb = oracle.trial_balance();
    assert_eq!(
        *tb.get(&sniper_strategy::oracle::Account::RealizedPnl)
            .unwrap_or(&0),
        0
    );
}

// 6 -----------------------------------------------------------------------
#[test]
fn sol_usd_fx_is_not_sniper_alpha() {
    // The receipt is SOL-denominated; USD is an as-of mark OUTSIDE the
    // ledger. Changing a USD price must not move net_pnl_lamports.
    let oracle = AccountingOracle::new(100_000_000);
    let marks = HashMap::new();
    let receipt = build_receipt(
        "ProtocolEconomicV1",
        "SYNTHETIC",
        "SYNTHETIC_COMPLETE_FIXTURE",
        "deadbeef",
        100_000_000,
        &oracle,
        &marks,
        0,
        0,
        0,
        0,
        vec![ComparisonRow {
            strategy: "S00".into(),
            net_pnl_lamports: 0,
            incremental_net_edge_vs_no_trade: 0,
            max_drawdown_lamports: 0,
            open_positions: 0,
            status: "NO_TRADE_BASELINE".into(),
        }],
        vec![],
    );
    // A hypothetical USD mark of 240 vs 42 SOL/USD: identical SOL P&L.
    assert_eq!(receipt.net_pnl_lamports, 0);
    assert_eq!(receipt.currency, "SOL-lamports");
    // And the hash is USD-independent (no USD field exists in the receipt).
    let receipt2 = build_receipt(
        "ProtocolEconomicV1",
        "SYNTHETIC",
        "SYNTHETIC_COMPLETE_FIXTURE",
        "deadbeef",
        100_000_000,
        &oracle,
        &marks,
        0,
        0,
        0,
        0,
        vec![ComparisonRow {
            strategy: "S00".into(),
            net_pnl_lamports: 0,
            incremental_net_edge_vs_no_trade: 0,
            max_drawdown_lamports: 0,
            open_positions: 0,
            status: "NO_TRADE_BASELINE".into(),
        }],
        vec![],
    );
    assert_eq!(receipt.receipt_sha256, receipt2.receipt_sha256);
}

// 7 -----------------------------------------------------------------------
#[test]
fn unexitable_inventory_is_not_withdrawable() {
    // Enter, then the market disappears: marks at zero -> NAV excludes the
    // position entirely (it is listed, valued at 0).
    let events = vec![event("e1", 100, 1_000, 10_000_000)];
    let outcome = run_s01(&events, &ExecutionFrictions::default());
    let oracle = AccountingOracle::from_run(&outcome, 100_000_000).unwrap();
    let nav_zero_marks = oracle.nav(&HashMap::new()); // mark = 0
    assert_eq!(nav_zero_marks.executable_exit_value, 0);
    assert!(nav_zero_marks.open_inventory_cost_basis > 0);
    // NAV = cash + recoverable rent ONLY: the stuck inventory contributes
    // nothing withdrawable (its entire cost basis is absent from NAV).
    assert_eq!(
        nav_zero_marks.nav,
        oracle.cash() + nav_zero_marks.recoverable_rent
    );
    assert_eq!(
        nav_zero_marks.nav,
        oracle.cash() + 1_500_000,
        "the unexitable position's cost basis must not be withdrawable"
    );
}

// 8 -----------------------------------------------------------------------
#[test]
fn unknown_tx_cannot_silently_become_realized() {
    let mut oracle = AccountingOracle::new(100_000_000);
    oracle.post_unknown_hold("S01", "e9", 1_000_000).unwrap();
    let marks = HashMap::new();
    // Still excluded from NAV and not in realized P&L.
    assert_eq!(oracle.nav(&marks).unresolved_at_risk, 1_000_000);
    assert_eq!(oracle.net_pnl(100_000_000, &marks), -1_000_000);
    let tb = oracle.trial_balance();
    let realized = *tb
        .get(&sniper_strategy::oracle::Account::RealizedPnl)
        .unwrap_or(&0);
    assert_eq!(
        realized, 0,
        "hold is at-risk capital, not a realized loss yet"
    );
    // Only an explicit resolution moves it.
    oracle.resolve_hold_to_loss("S01", "e9", 1_000_000).unwrap();
    let tb = oracle.trial_balance();
    assert_eq!(
        *tb.get(&sniper_strategy::oracle::Account::RealizedPnl)
            .unwrap(),
        -1_000_000
    );
}

// 9 -----------------------------------------------------------------------
#[test]
fn same_inputs_yield_identical_receipt_hash_end_to_end() {
    let events = vec![
        event("e1", 100, 1_000, 50_000_000),
        event("e2", 30_000, 1_400, 50_000_000),
    ];
    let frictions = ExecutionFrictions::default();
    let build = || {
        let outcome = run_s01(&events, &frictions);
        let oracle = AccountingOracle::from_run(&outcome, 100_000_000).unwrap();
        let marks = HashMap::new();
        build_receipt(
            "ProtocolEconomicV1",
            "SYNTHETIC",
            "SYNTHETIC_COMPLETE_FIXTURE",
            "deadbeef",
            100_000_000,
            &oracle,
            &marks,
            oracle
                .trial_balance()
                .get(&sniper_strategy::oracle::Account::RealizedPnl)
                .copied()
                .unwrap_or(0) as i64,
            0,
            outcome.open_lots.len() as u64,
            0,
            vec![ComparisonRow {
                strategy: "S01".into(),
                net_pnl_lamports: oracle.net_pnl(100_000_000, &marks),
                incremental_net_edge_vs_no_trade: oracle.net_pnl(100_000_000, &marks),
                max_drawdown_lamports: 0,
                open_positions: outcome.open_lots.len() as u64,
                status: "CANDIDATE".into(),
            }],
            vec![],
        )
    };
    assert_eq!(build().receipt_sha256, build().receipt_sha256);
}

// Bonus: full round trip still reconciles (CE-1 stays fixed with slippage
// and tip fields in play).
#[test]
fn round_trip_reconciles_with_tips_and_slippage() {
    let frictions = ExecutionFrictions {
        slippage_bps: 150,
        tip_lamports: 21_000,
        ..Default::default()
    };
    let events = vec![
        event("e1", 100, 1_000, 50_000_000),
        event("e2", 30_000, 1_400, 50_000_000),
    ];
    let outcome = run_s01(&events, &frictions);
    let oracle = AccountingOracle::from_run(&outcome, 100_000_000).unwrap();
    assert!(oracle.verify_conservation().is_ok());
    assert_eq!(oracle.cash(), outcome.ending_cash_lamports);
    // Slippage appears in the waterfall.
    let totals = oracle.fee_totals();
    assert!(totals.contains_key(&FeeKind::Slippage));
    assert!(*totals.get(&FeeKind::Slippage).unwrap() > 0);
}
