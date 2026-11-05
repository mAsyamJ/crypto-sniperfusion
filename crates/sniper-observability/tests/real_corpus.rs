//! DASH-02 — REAL-corpus proof against the on-disk wa7 wallet-follow run
//! (`harness/runs/wallet-alpha/wa7`). Skipped with a message when the
//! gitignored corpus is absent (CI); the synthetic twins in the other test
//! files carry the same invariants. This file is the source-to-dashboard
//! reconciliation evidence on actual SniperFusion output.

use sniper_observability::discovery::Roots;
use sniper_observability::project::{import_corpus, open_db};
use sniper_observability::queries::{self, DataStatus};

const WA7: &str = "harness/runs/wallet-alpha/wa7";

fn corpus_dir() -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(WA7);
    p.is_dir().then_some(p)
}

/// T-CAL7-real: the calendar for the real FOLLOW_DELAY_500MS arm reconciles
/// to the sealed comparison row: Σ entries == −250,308 == run.json value.
#[test]
fn t_cal7_real_wa7_calendar_reconciles() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP: real wa7 corpus absent ({WA7})");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let db = open_db(&tmp.path().join("c.sqlite3")).unwrap();
    let s = import_corpus(&db, &Roots::from_dirs([&dir]), "2026-11-05T00:00:00Z").unwrap();
    assert_eq!(s.runs_imported, 1, "real wa7 must import cleanly: {s:?}");
    let id = queries::list_runs(&db)[0].id.clone();

    for arm in [
        "FOLLOW_DELAY_500MS",
        "FOLLOW_DELAY_1000MS",
        "FOLLOW_DELAY_3000MS",
        "FOLLOW_DELAY_10000MS",
        "FOLLOW_DELAY_30000MS",
        "FOLLOW_DELAY_60000MS",
    ] {
        let cal = queries::calendar(&db, "t", &id, arm, 0).expect();
        assert_eq!(cal.projection.reconciled_total_lamports, -250_308, "{arm}");
        assert_eq!(
            cal.projection.status,
            sniper_strategy::calendar::CalendarStatus::Ok,
            "{arm}"
        );
        assert_eq!(cal.comparison_row_net_pnl_lamports, Some(-250_308), "{arm}");
        assert_eq!(cal.reconciles_to_comparison, Some(true), "{arm}");
    }
}

/// T-PNL1-real: served summary equals the sealed run.json values exactly
/// (spot KPIs + verdict + comparison row count).
#[test]
fn t_pnl1_real_wa7_summary_exact() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP: real wa7 corpus absent ({WA7})");
        return;
    };
    let raw = std::fs::read_to_string(dir.join("run.json")).unwrap();
    let receipt: sniper_strategy::receipt::PnlRunReceiptV2 = serde_json::from_str(&raw).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let db = open_db(&tmp.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([&dir]), "t").unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let s = queries::pnl_summary(&db, "t", &id).expect();

    assert_eq!(s.engine, receipt.engine);
    assert_eq!(s.verdict, receipt.verdict);
    assert_eq!(s.net_pnl_lamports, receipt.net_pnl_lamports);
    assert_eq!(
        s.external_deposits_lamports as u64,
        receipt.external_deposits_lamports
    );
    assert_eq!(s.comparison.len(), receipt.comparison.len());
    for (served, sealed) in s.comparison.iter().zip(receipt.comparison.iter()) {
        assert_eq!(served.strategy, sealed.strategy);
        assert_eq!(served.net_pnl_lamports, sealed.net_pnl_lamports);
        assert_eq!(served.status, sealed.status);
    }
    // S00 mandatory present with the baseline status.
    let s00 = s.comparison.iter().find(|c| c.strategy == "S00").unwrap();
    assert_eq!(s00.status, "NO_TRADE_BASELINE");
    assert_eq!(s00.net_pnl_lamports, 0);
}

/// S00-missing enforcement proven live: strip the S00 row from a COPY of
/// the real receipt (re-sealed via the producer rule) → SOURCE_MISMATCH.
#[test]
fn t_s001_real_baseline_gate_on_modified_copy() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP: real wa7 corpus absent ({WA7})");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let staged = tmp.path().join("nos00");
    std::fs::create_dir_all(&staged).unwrap();
    let raw = std::fs::read_to_string(dir.join("run.json")).unwrap();
    let mut receipt: sniper_strategy::receipt::PnlRunReceiptV2 =
        serde_json::from_str(&raw).unwrap();
    receipt.comparison.retain(|c| c.strategy != "S00");
    let receipt = sniper_strategy::receipt::PnlRunReceiptV2::seal(receipt);
    sniper_strategy::receipt::write_bundle(&staged, &receipt, "[]", "", "{}").unwrap();

    let db = open_db(&tmp.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([&staged]), "t").unwrap();
    let id = queries::list_runs(&db)[0].id.clone();
    let stamped = queries::pnl_summary(&db, "t", &id);
    assert_eq!(stamped.status, DataStatus::SourceMismatch);
}
