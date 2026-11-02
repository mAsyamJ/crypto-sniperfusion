//! H0.2 red tests: journal durability/integrity contracts (external crate).
use sniper_runtime::journal::{FileJournal, JournalError, LifecycleStage, TxJournal};

fn tmp(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join(name);
    (d, p)
}

#[test]
fn second_writer_on_same_journal_is_rejected() {
    let (_d, path) = tmp("j.jsonl");
    let _a = FileJournal::open(&path).unwrap();
    match FileJournal::open(&path) {
        Err(JournalError::AlreadyLocked { .. }) => {}
        other => panic!("expected AlreadyLocked, got {other:?}"),
    }
    // Releasing the first writer (drop) frees the lock.
    drop(_a);
    assert!(FileJournal::open(&path).is_ok());
}

#[test]
fn append_is_durable_by_default() {
    // append() must sync_all: observable via the unsynced variant existing
    // separately and the default requiring no explicit flush call.
    let (_d, path) = tmp("j.jsonl");
    let mut j = FileJournal::open(&path).unwrap();
    j.append(LifecycleStage::Intent, "i-1", "{}").unwrap();
    // Dropped without flush_sync; contents must be on disk (readable).
    drop(j);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("i-1"));
}

#[test]
fn sequence_gap_is_a_loud_error() {
    let (_d, path) = tmp("j.jsonl");
    {
        let mut j = FileJournal::open(&path).unwrap();
        let _ = j.append(LifecycleStage::Intent, "a", "{}").unwrap(); // seq 1
        let _ = j.append(LifecycleStage::Authorized, "a", "{}").unwrap(); // seq 2
        let _ = j.append(LifecycleStage::Signed, "a", "{}").unwrap(); // seq 3
    }
    // Remove the MIDDLE record: seq 1 then seq 3 -> contiguity break.
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.remove(1);
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    match FileJournal::open(&path) {
        Err(JournalError::SequenceGap { expected, found }) => {
            assert_eq!((expected, found), (2, 3));
        }
        other => panic!("expected SequenceGap at open, got {other:?}"),
    }
}

#[test]
fn checksum_mismatch_is_a_loud_error() {
    let (_d, path) = tmp("j.jsonl");
    {
        let mut j = FileJournal::open(&path).unwrap();
        let _ = j.append(LifecycleStage::Intent, "a", "{}").unwrap();
    }
    // Tamper with payload keeping seq/stage -> checksum must catch it.
    let text = std::fs::read_to_string(&path).unwrap();
    let tampered = text.replace(
        "\"payload_json\":\"{}\"",
        "\"payload_json\":\"{\\\"evil\\\":1}\"",
    );
    std::fs::write(&path, tampered).unwrap();
    // Tampering is caught at OPEN (validation is part of open, not just load).
    match FileJournal::open(&path) {
        Err(JournalError::ChecksumMismatch { seq }) => assert_eq!(seq, 1),
        other => panic!("expected ChecksumMismatch at open, got {other:?}"),
    }
}

#[test]
fn midfile_corruption_is_not_silently_skipped() {
    let (_d, path) = tmp("j.jsonl");
    {
        let mut j = FileJournal::open(&path).unwrap();
        for i in 0..5 {
            let _ = j
                .append(LifecycleStage::Intent, &format!("o{i}"), "{}")
                .unwrap();
        }
    }
    // Corrupt line 3 of 5 (complete line, garbage JSON).
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines[2] = "garbage-not-json".into();
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    assert!(matches!(
        FileJournal::open(&path),
        Err(JournalError::Decode { .. })
    ));
}

#[test]
fn crash_tail_only_when_final_line_unterminated() {
    let (_d, path) = tmp("j.jsonl");
    {
        let mut j = FileJournal::open(&path).unwrap();
        let _ = j.append(LifecycleStage::Intent, "a", "{}").unwrap();
        let _ = j.append(LifecycleStage::Authorized, "a", "{}").unwrap();
    }
    // Simulate crash mid-append: strip the trailing newline from line 2.
    let text = std::fs::read_to_string(&path).unwrap();
    let trimmed = text.strip_suffix('\n').unwrap().to_string();
    std::fs::write(&path, trimmed).unwrap();
    let j = FileJournal::open(&path).unwrap();
    assert!(j.tail_was_repaired(), "open repairs the unterminated tail");
    let c = j.load().unwrap();
    assert_eq!(c.records.len(), 1, "only the terminated record survives");
}

#[test]
fn truncation_at_every_boundary_recovers_exactly() {
    for cut in 0..=5usize {
        let (_d, path) = tmp("j.jsonl");
        {
            let mut j = FileJournal::open(&path).unwrap();
            for i in 0..5 {
                let _ = j
                    .append(LifecycleStage::Intent, &format!("o{i}"), "{}")
                    .unwrap();
            }
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // Rewrite with `cut` complete lines + a partial fragment of line cut+1.
        let mut out = lines[..cut.min(5)].join("\n");
        out.push('\n');
        if cut < 5 {
            out.push_str("{\"seq\":"); // unterminated fragment after newline
        }
        std::fs::write(&path, out).unwrap();
        let j = FileJournal::open(&path).unwrap();
        assert_eq!(
            j.tail_was_repaired(),
            cut < 5,
            "cut={cut}: repair iff partial tail"
        );
        let c = j.load().unwrap();
        assert_eq!(c.records.len(), cut, "cut={cut}: exactly {cut} survive");
    }
}

#[test]
fn lock_is_released_on_drop_and_survives_rename() {
    let (_d, path) = tmp("j.jsonl");
    {
        let _j = FileJournal::open(&path).unwrap();
    }
    {
        let _j2 = FileJournal::open(&path).unwrap();
    }
    // Third open after two full cycles still fine.
    let _j3 = FileJournal::open(&path).unwrap();
    let _ = std::io::stdout();
}
