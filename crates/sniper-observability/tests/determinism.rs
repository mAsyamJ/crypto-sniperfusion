//! DASH-02 Task 7 — determinism and rebuild (T-DET1, T-DET2).

mod common;

use common::*;
use sniper_observability::discovery::Roots;
use sniper_observability::project::{open_db, rebuild};
use sniper_observability::queries;
use sniper_strategy::follower::FollowerRun;
use std::fs;
use tempfile::tempdir;

fn build_corpus(dir: &std::path::Path) {
    let oracle = sniper_strategy::oracle::AccountingOracle::new(100);
    write_research_bundle(
        &dir.join("r1"),
        "r1",
        "LegacySurrogateV1",
        &oracle,
        100,
        0,
        0,
        0,
        vec![s00_row()],
        &[],
    );
    let runs = vec![FollowerRun {
        delay_ms: 1000,
        signals: 2,
        followed_entries: 2,
        refused_invalid: 0,
        refused_no_quote: 0,
        duplicates_dropped: 0,
        closed_exits: 1,
        open_positions: 1,
        net_pnl_lamports: -10_000,
        entries: vec![
            follower_entry(
                "s1",
                "mintX",
                1_772_236_000_000,
                500_000,
                Some(1_772_236_500_000),
                Some(-10_000),
            ),
            follower_entry("s2", "mintY", 1_772_240_000_000, 400_000, None, None),
        ],
    }];
    write_follower_bundle(&dir.join("f1"), &runs, &[("FOLLOW_DELAY_1000MS", -10_000)]);
    fs::write(dir.join("h8-registry.jsonl"), registry_jsonl(true)).unwrap();
}

/// T-DET1: two full rebuilds of the identical corpus (same fixed built_at)
/// yield identical serialized query output — the projection is a
/// deterministic function of its inputs.
#[test]
fn t_det1_rebuild_is_deterministic() {
    let corpus = tempdir().unwrap();
    build_corpus(corpus.path());

    let dump = |tag: &str| -> String {
        let db = open_db(corpus.path().join(format!("cache-{tag}.sqlite3")).as_path()).unwrap();
        rebuild(&db, &Roots::from_dirs([corpus.path()]), FIXED_BUILT_AT).unwrap();
        let mut out = String::new();
        for run in queries::list_runs(&db) {
            out.push_str(&format!("{run:?}\n"));
            out.push_str(&format!(
                "{:?}\n",
                queries::run_detail(&db, FIXED_BUILT_AT, &run.id)
            ));
            out.push_str(&format!(
                "{:?}\n",
                queries::pnl_summary(&db, FIXED_BUILT_AT, &run.id)
            ));
            out.push_str(&format!(
                "{:?}\n",
                queries::positions(&db, FIXED_BUILT_AT, &run.id)
            ));
            out.push_str(&format!(
                "{:?}\n",
                queries::trades_page(&db, FIXED_BUILT_AT, &run.id, 0, 500)
            ));
            for c in queries::pnl_summary(&db, FIXED_BUILT_AT, &run.id)
                .expect()
                .comparison
            {
                out.push_str(&format!(
                    "{:?}\n",
                    queries::calendar(&db, FIXED_BUILT_AT, &run.id, &c.strategy, 0)
                ));
            }
        }
        out.push_str(&format!("{:?}\n", queries::strategies(&db)));
        out.push_str(&format!("{:?}\n", queries::corpus_status(&db)));
        out
    };

    let a = dump("a");
    let b = dump("b");
    assert_eq!(a, b, "rebuild of identical corpus diverged");
    assert!(a.contains("FOLLOW_DELAY_1000MS"));
}

/// T-DET2: corrupting ONE artifact after an import quarantines exactly that
/// artifact on rebuild — blast radius 1, everything else unchanged.
#[test]
fn t_det2_quarantine_blast_radius_is_one() {
    let corpus = tempdir().unwrap();
    build_corpus(corpus.path());

    let db = open_db(&corpus.path().join("c.sqlite3")).unwrap();
    rebuild(&db, &Roots::from_dirs([corpus.path()]), FIXED_BUILT_AT).unwrap();
    let before = queries::list_runs(&db);
    assert_eq!(before.len(), 2);

    // Tamper one bundle (replace the 0444 file wholesale).
    let run_path = corpus.path().join("r1/run.json");
    let raw = fs::read_to_string(&run_path).unwrap();
    let tampered = raw.replace(
        "\"ending_nav_lamports\": 100",
        "\"ending_nav_lamports\": 999",
    );
    fs::remove_file(&run_path).unwrap();
    fs::write(&run_path, tampered).unwrap();

    let s = rebuild(&db, &Roots::from_dirs([corpus.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(s.runs_quarantined, 1);
    assert_eq!(
        s.runs_imported, 1,
        "the untouched follower bundle must still import"
    );
    let after = queries::list_runs(&db);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].engine, "wallet_follower_v1");
    assert_eq!(queries::quarantined(&db).len(), 1);
}
