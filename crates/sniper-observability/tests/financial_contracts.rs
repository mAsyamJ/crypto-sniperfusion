//! DASH-02 Task 4 — financial acceptance cases (T-DEP1, T-POS1, T-PART1,
//! T-MON1, T-S001). Every fixture is built through the REAL oracle and
//! receipt builder; the assertions check what the SERVED values are after
//! import (oracle → sealed receipt → projection → query).

mod common;

use common::*;
use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};
use sniper_observability::queries::{self, DataStatus};
use sniper_strategy::oracle::AccountingOracle;
use sniper_strategy::receipt::ComparisonRow;
use std::collections::HashMap;
use tempfile::tempdir;

/// T-DEP1: a 1 SOL external deposit with zero trades is NOT profit —
/// served net PnL is exactly 0 and the deposit is a separate field.
#[test]
fn t_dep1_deposits_are_not_profit() {
    let dir = tempdir().unwrap();
    let mut oracle = AccountingOracle::new(0);
    oracle
        .post_deposit(1_000_000_000, "operator top-up")
        .unwrap();
    write_research_bundle(
        &dir.path().join("dep"),
        "dep",
        "LegacySurrogateV1",
        &oracle,
        0,
        0,
        0,
        0,
        vec![s00_row()],
        &[],
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let s = queries::pnl_summary(&db, FIXED_BUILT_AT, &id).expect();

    assert_eq!(s.net_pnl_lamports, 0, "deposit inflated net PnL");
    assert_eq!(s.external_deposits_lamports, 1_000_000_000);
    assert_eq!(s.ending_nav_lamports, 1_000_000_000);
    assert_eq!(s.realized_pnl_lamports, 0);
}

/// T-POS1: unexitable/unknown inventory is surfaced as at-risk, never as
/// cash or profit; open-lot basis is exact.
#[test]
fn t_pos1_unexitable_is_not_cash() {
    let dir = tempdir().unwrap();
    let mut oracle = AccountingOracle::new(200_000_000);
    // Open position: buy 50M (fees on top), never sell.
    oracle
        .post_buy_fill("T0", &buy("b1", "mintOpen", 1, 50_000_000, 700_000))
        .unwrap();
    // Unknown hold on a second mint: cash left, outcome unknown.
    oracle
        .post_unknown_hold("T0", "hold-1", 10_000_000)
        .unwrap();
    let nav = oracle.nav(&HashMap::new());
    write_research_bundle(
        &dir.path().join("pos"),
        "pos",
        "LegacySurrogateV1",
        &oracle,
        200_000_000,
        0,
        0,
        1,
        vec![s00_row()],
        &[],
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let s = queries::pnl_summary(&db, FIXED_BUILT_AT, &id).expect();
    let p = queries::positions(&db, FIXED_BUILT_AT, &id).expect();

    assert_eq!(
        s.open_inventory_cost_basis_lamports,
        nav.open_inventory_cost_basis as i64
    );
    assert_eq!(s.unknown_at_risk_lamports, nav.unresolved_at_risk as i64);
    assert_eq!(p.open_positions, 1);
    assert_eq!(p.nav_series, "UNAVAILABLE");
    assert_eq!(p.mark_model, "UNAVAILABLE");
    // The identity: NAV excludes the unresolved hold (producer rule mirrored).
    assert_eq!(s.ending_nav_lamports, nav.nav as i64);
}

/// T-PART1: one lot, two partial exits — FIFO basis consumption and exact
/// per-sell realized sums to the run total.
#[test]
fn t_part1_partial_exits_fifo() {
    let dir = tempdir().unwrap();
    let mut oracle = AccountingOracle::new(100_000_000);
    let records = [
        buy("b1", "mintP", 10, 40_000_000, 1_000_000),
        sell("s1", "mintP", 11, 400_000, 20_000_000),
        sell("s2", "mintP", 12, 600_000, 19_500_000),
    ];
    for r in &records {
        match r {
            sniper_strategy::engine::protocol::ExecutionRecord::BuyFill { .. } => {
                oracle.post_buy_fill("T0", r).unwrap()
            }
            sniper_strategy::engine::protocol::ExecutionRecord::SellFill { .. } => {
                oracle.post_sell_fill("T0", r).unwrap()
            }
            _ => unreachable!(),
        }
    }
    // Realized from the oracle's own attribution (single strategy T0).
    let realized = oracle
        .attributed()
        .get("T0")
        .map(|a| a.realized_pnl)
        .unwrap_or(0);
    write_research_bundle(
        &dir.path().join("part"),
        "part",
        "LegacySurrogateV1",
        &oracle,
        100_000_000,
        realized,
        0,
        0,
        vec![s00_row()],
        &records,
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let s = queries::pnl_summary(&db, FIXED_BUILT_AT, &id).expect();
    assert_eq!(s.realized_pnl_lamports, realized);
    // The full lot is exited: no open inventory remains.
    assert_eq!(s.open_inventory_cost_basis_lamports, 0);
    assert_eq!(s.open_positions, 0);
}

/// T-MON1: 64-bit money survives SQLite + query round-trips exactly (2^53+1
/// and near-i64::MAX values, hand-sealed via the real builder).
#[test]
fn t_mon1_64bit_precision() {
    let dir = tempdir().unwrap();
    // Construct a receipt with extreme values directly, then SEAL it with
    // the producer's own rule (values this extreme cannot arise from a
    // small oracle scenario; the builder is still the producer's).
    let oracle = AccountingOracle::new(0);
    let big = 9_007_199_254_740_993i64; // 2^53 + 1
    let near_max = i64::MAX - 7;
    let mut receipt = {
        let mut r = sniper_strategy::receipt::build_receipt(
            "precision-probe",
            "SYNTHETIC",
            "SYNTHETIC_COMPLETE_FIXTURE",
            &"a".repeat(64),
            0,
            &oracle,
            &HashMap::new(),
            big,
            near_max.unsigned_abs(),
            0,
            0,
            vec![
                ComparisonRow {
                    strategy: "S00".into(),
                    net_pnl_lamports: 0,
                    incremental_net_edge_vs_no_trade: 0,
                    max_drawdown_lamports: 0,
                    open_positions: 0,
                    status: "NO_TRADE_BASELINE".into(),
                },
                ComparisonRow {
                    strategy: "BIG".into(),
                    net_pnl_lamports: near_max,
                    incremental_net_edge_vs_no_trade: near_max,
                    max_drawdown_lamports: 0,
                    open_positions: 0,
                    status: "LOSING".into(),
                },
            ],
            vec![],
        );
        r.net_pnl_lamports = big;
        r.realized_pnl_lamports = near_max;
        r.unknown_at_risk_lamports = 9_223_372_036_854_775_800u64;
        r
    };
    // Re-seal after the mutations: same rule the producer uses.
    receipt = sniper_strategy::receipt::PnlRunReceiptV2::seal(receipt);
    sniper_strategy::receipt::write_bundle(&dir.path().join("mon"), &receipt, "", "", "{}")
        .unwrap();

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let s = queries::pnl_summary(&db, FIXED_BUILT_AT, &id).expect();
    assert_eq!(s.net_pnl_lamports, big, "2^53+1 corrupted in round-trip");
    assert_eq!(s.realized_pnl_lamports, near_max);
    assert_eq!(s.unknown_at_risk_lamports, 9_223_372_036_854_775_800i64);
    let big_row = s.comparison.iter().find(|c| c.strategy == "BIG").unwrap();
    assert_eq!(big_row.net_pnl_lamports, near_max);
}

/// T-S001: a receipt whose comparison lacks the S00 baseline row serves
/// SOURCE_MISMATCH — a missing baseline voids the view (SOUL Law 5).
#[test]
fn t_s001_missing_baseline_fails_closed() {
    let dir = tempdir().unwrap();
    let oracle = AccountingOracle::new(1_000);
    // Deliberately baseline-free comparison (a broken producer output).
    write_research_bundle(
        &dir.path().join("nos00"),
        "x",
        "LegacySurrogateV1",
        &oracle,
        1_000,
        0,
        0,
        0,
        vec![arm_row("S04", -1)],
        &[],
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let stamped = queries::pnl_summary(&db, FIXED_BUILT_AT, &id);
    assert_eq!(stamped.status, DataStatus::SourceMismatch);
    assert!(stamped.reason.unwrap().contains("S00"));
}
