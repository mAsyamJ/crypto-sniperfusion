//! DASH-02 Task 3 — parsers + projection (T-IMP1, T-SHAPE1, T-REG1, T-HOLD1).

mod common;

use common::*;
use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};
use sniper_observability::queries;
use sniper_strategy::follower::{FollowerEntry, FollowerRun};
use std::fs;
use tempfile::tempdir;

/// T-IMP1: importing the same corpus twice changes nothing (idempotent by
/// content hash; run ids stable).
#[test]
fn t_imp1_duplicate_import_is_idempotent() {
    let dir = tempdir().unwrap();
    let oracle = sniper_strategy::oracle::AccountingOracle::new(50_000);
    write_research_bundle(
        &dir.path().join("a"),
        "a",
        "LegacySurrogateV1",
        &oracle,
        50_000,
        0,
        0,
        0,
        vec![s00_row()],
        &[],
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    let s1 = import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(s1.runs_imported, 1);
    let first_ids: Vec<String> = queries::list_runs(&db).into_iter().map(|r| r.id).collect();

    let s2 = import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(s2.runs_imported, 0);
    assert_eq!(s2.skipped_existing, 1);
    let second_ids: Vec<String> = queries::list_runs(&db).into_iter().map(|r| r.id).collect();
    assert_eq!(first_ids, second_ids);
}

/// T-SHAPE1: all three bundle families parse into normalized rows —
/// research JSONL (slot-only), wallet-follow JSON array (ms timestamps),
/// wallet-pnl empty siblings tolerated (D-4).
#[test]
fn t_shape1_three_bundle_shapes() {
    let dir = tempdir().unwrap();

    // Research family.
    let oracle = sniper_strategy::oracle::AccountingOracle::new(100);
    write_research_bundle(
        &dir.path().join("research1"),
        "r",
        "LegacySurrogateV1",
        &oracle,
        100,
        0,
        0,
        0,
        vec![s00_row()],
        &[
            buy("e1", "mintA", 465_000_001, 1_000, 900),
            sell("e2", "mintA", 465_000_002, 900, 1_100),
        ],
    );

    // Follower family: JSON array with timestamps + one open + one refused.
    let runs = vec![FollowerRun {
        delay_ms: 500,
        signals: 3,
        followed_entries: 3,
        refused_invalid: 1,
        refused_no_quote: 0,
        duplicates_dropped: 0,
        closed_exits: 2,
        open_positions: 1,
        net_pnl_lamports: -33_436,
        entries: vec![
            follower_entry(
                "s1",
                "mintX",
                1_772_236_000_000,
                987_500,
                Some(1_772_236_500_000),
                Some(-83_436),
            ),
            follower_entry(
                "s2",
                "mintY",
                1_772_245_000_000,
                500_000,
                Some(1_772_245_500_000),
                Some(50_000),
            ),
            follower_entry("s3", "mintZ", 1_772_250_000_000, 700_000, None, None),
            FollowerEntry {
                signal_id: "s4".into(),
                mint: "mintW".into(),
                entry_at_ms: 1_772_255_000_000,
                entry_effective_lamports: 0,
                tokens: 0,
                exit_at_ms: None,
                exit_net_lamports: None,
                net_pnl_lamports: None,
                refusal_reason: Some("unsupported route".into()),
            },
        ],
    }];
    write_follower_bundle(
        &dir.path().join("wa7twin"),
        &runs,
        &[("FOLLOW_DELAY_500MS", -33_436)],
    );

    // Wallet-pnl family: empty trades/equity.
    let oracle2 = sniper_strategy::oracle::AccountingOracle::new(0);
    write_research_bundle(
        &dir.path().join("wa2twin"),
        "w",
        "wallet_observer_v1",
        &oracle2,
        0,
        0,
        0,
        0,
        vec![s00_row(), arm_row("LEADER", -4_961_316_798)],
        &[],
    );

    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    let summary = import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(summary.runs_imported, 3);
    assert_eq!(summary.runs_quarantined, 0);

    let runs_list = queries::list_runs(&db);
    assert_eq!(runs_list.len(), 3);

    // Research rows carry slots, no timestamps.
    let research_id = runs_list
        .iter()
        .find(|r| r.engine == "LegacySurrogateV1")
        .unwrap()
        .id
        .clone();
    let page = queries::trades_page(&db, FIXED_BUILT_AT, &research_id, 0, 100).expect();
    assert_eq!(page.rows.len(), 2);
    assert_eq!(page.rows[0].slot, Some(465_000_001));
    assert_eq!(page.rows[0].at_ms, None);

    // Follower rows carry ms anchors; open + refused semantics preserved.
    let follow_id = runs_list
        .iter()
        .find(|r| r.engine == "wallet_follower_v1")
        .unwrap()
        .id
        .clone();
    let page = queries::trades_page(&db, FIXED_BUILT_AT, &follow_id, 0, 100).expect();
    assert_eq!(page.rows.len(), 4);
    let open_rows = page.rows.iter().filter(|r| r.open).count();
    assert_eq!(open_rows, 1);
    assert_eq!(page.rows.iter().filter(|r| r.kind == "REJECTED").count(), 1);
    assert!(page
        .rows
        .iter()
        .filter(|r| !r.open && r.kind == "FOLLOWER_ENTRY")
        .all(|r| r.at_ms.is_some() && r.net_pnl_lamports.is_some()));

    // Calendar reconciles for the twin: −83,436 + 50,000 = −33,436.
    let cal = queries::calendar(&db, FIXED_BUILT_AT, &follow_id, "FOLLOW_DELAY_500MS", 0).expect();
    assert_eq!(cal.projection.reconciled_total_lamports, -33_436);
    assert_eq!(cal.projection.open_positions, 1);
    assert_eq!(cal.comparison_row_net_pnl_lamports, Some(-33_436));
    assert_eq!(cal.reconciles_to_comparison, Some(true));

    // Wallet-pnl empty siblings: trade page is empty but the run serves.
    let obs_id = runs_list
        .iter()
        .find(|r| r.engine == "wallet_observer_v1")
        .unwrap()
        .id
        .clone();
    let page = queries::trades_page(&db, FIXED_BUILT_AT, &obs_id, 0, 100).expect();
    assert!(page.rows.is_empty());
}

/// T-REG1: a broken H8 chain is quarantined whole; a valid chain imports.
#[test]
fn t_reg1_chain_break_quarantined() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("h8-registry.jsonl"), registry_jsonl(false)).unwrap();
    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    let s = import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(s.registries_quarantined, 1);
    assert_eq!(s.registry_records, 0);

    let dir2 = tempdir().unwrap();
    fs::write(dir2.path().join("h8-registry.jsonl"), registry_jsonl(true)).unwrap();
    let db2 = open_db(&dir2.path().join("c.sqlite3")).unwrap();
    let s2 = import_corpus(&db2, &Roots::from_dirs([dir2.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(s2.registry_records, 2);
    assert_eq!(s2.registries_quarantined, 0);
}

/// T-HOLD1: holdout label material is stripped before storage — the
/// projection cannot serve what it never stores (DA-5).
#[test]
fn t_hold1_labels_never_stored() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("h8-registry.jsonl"), registry_jsonl(true)).unwrap();
    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let mut stmt = db
        .prepare("SELECT payload_json FROM registry_records")
        .unwrap();
    let payloads: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .flatten()
        .collect();
    assert!(!payloads.is_empty());
    for p in payloads {
        assert!(!p.contains("labels"), "label material stored: {p}");
        assert!(!p.contains("STRIPPED"), "label material stored: {p}");
    }
}
