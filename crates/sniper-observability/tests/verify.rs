//! DASH-02 Task 2 — verification + discovery (T-VER1, T-VER2, T-DISC1).

mod common;

use common::*;
use sniper_observability::discovery::{discover, ArtifactKind, Roots};
use sniper_observability::project::{import_corpus, open_db};
use sniper_observability::queries;
use sniper_observability::verify::{dataset_sha_is_hash, verify_run_receipt};
use sniper_strategy::oracle::AccountingOracle;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::tempdir;

fn one_run_corpus(dir: &std::path::Path) {
    let oracle = AccountingOracle::new(100_000_000);
    write_research_bundle(
        &dir.join("r1"),
        "r1",
        "LegacySurrogateV1",
        &oracle,
        100_000_000,
        0,
        0,
        0,
        vec![s00_row()],
        &[],
    );
}

/// T-VER1: a tampered receipt must fail hash verification and be
/// quarantined — never served as data. The untouched twin imports OK.
#[test]
fn t_ver1_tamper_is_quarantined_not_served() {
    let good = tempdir().unwrap();
    let bad = tempdir().unwrap();
    one_run_corpus(good.path());
    one_run_corpus(bad.path());

    // Tamper: flip one lamport digit in the bad copy.
    let run_path = bad.path().join("r1/run.json");
    let raw = fs::read_to_string(&run_path).unwrap();
    let tampered = raw.replace(
        "\"ending_nav_lamports\": 100000000",
        "\"ending_nav_lamports\": 900000000",
    );
    assert_ne!(&raw, &tampered, "fixture must actually change");
    // The producer chmods bundle files 0444 (write-once); simulate an
    // after-the-fact tamper by replacing the file, not writing in place.
    fs::remove_file(&run_path).unwrap();
    fs::write(&run_path, &tampered).unwrap();

    // Unit level: verifier catches it.
    assert!(verify_run_receipt("bad/run.json", &tampered).is_err());
    assert!(verify_run_receipt("good/run.json", &raw).is_ok());

    // Corpus level: quarantine, and the numbers are absent from `runs`.
    let db = open_db(&bad.path().join("cache.sqlite3")).unwrap();
    let summary = import_corpus(&db, &Roots::from_dirs([bad.path()]), FIXED_BUILT_AT).unwrap();
    assert_eq!(summary.runs_quarantined, 1);
    assert_eq!(summary.runs_imported, 0);
    assert_eq!(queries::list_runs(&db).len(), 0);
    assert_eq!(queries::quarantined(&db).len(), 1);
}

/// T-VER2: `dataset_sha256` free-text labels pass through as provenance
/// labels (D-2); 64-hex values are recognized as hashes.
#[test]
fn t_ver2_dataset_sha_label_passthrough() {
    let dir = tempdir().unwrap();
    one_run_corpus(dir.path());
    let db = open_db(&dir.path().join("c.sqlite3")).unwrap();
    import_corpus(&db, &Roots::from_dirs([dir.path()]), FIXED_BUILT_AT).unwrap();
    let runs = queries::list_runs(&db);
    assert_eq!(runs.len(), 1);
    let detail = queries::run_detail(&db, FIXED_BUILT_AT, &runs[0].id).expect();
    // "deadbeef"*8 is 64 hex chars → hash-like in this fixture.
    assert!(detail.dataset_sha_is_hash);

    let oracle = AccountingOracle::new(0);
    let receipt = {
        use sniper_strategy::receipt::build_receipt;
        build_receipt(
            "e",
            "SYNTHETIC",
            "QUOTES_AS_PROVIDED",
            "signals+quotes",
            0,
            &oracle,
            &Default::default(),
            0,
            0,
            0,
            0,
            vec![s00_row()],
            vec![],
        )
    };
    assert!(!dataset_sha_is_hash(&receipt));
}

/// T-DISC1: symlinked roots are refused; allowlisted discovery only.
#[test]
fn t_disc1_symlinked_root_refused() {
    let real = tempdir().unwrap();
    one_run_corpus(real.path());
    let link_parent = tempdir().unwrap();
    symlink(real.path(), link_parent.path().join("evil-link")).unwrap();

    let err = discover(&Roots::from_dirs([link_parent.path().join("evil-link")])).unwrap_err();
    assert!(err.to_string().contains("symlink"), "got: {err}");

    // The real root discovers exactly the bundle (and nothing outside it).
    let report = discover(&Roots::from_dirs([real.path()])).unwrap();
    let kinds: Vec<_> = report.artifacts.iter().map(|a| a.kind.clone()).collect();
    assert!(kinds.contains(&ArtifactKind::PnlRunBundle {
        dir: real.path().join("r1")
    }));
}
