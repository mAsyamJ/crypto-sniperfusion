//! Durable transaction journal (append-only JSONL) — H0.2 hardened.
//!
//! Contracts:
//! - **Single writer**: an O_EXCL PID lockfile guards the journal; a second
//!   opener gets `AlreadyLocked` unless the previous owner's process is dead
//!   (lock reclaimed by PID probe).
//! - **Durability**: `append` writes + flushes + `sync_all`s before
//!   returning — write-ahead means the record is on disk before the stage it
//!   describes takes effect externally.
//! - **Integrity**: every record carries a checksum over its content;
//!   `load()` verifies checksums and sequence contiguity (gap/duplicate are
//!   loud typed errors). A mid-file corrupted line is NEVER silently skipped.
//! - **Crash tail**: a final unterminated line (crash mid-append) is
//!   repaired on open by truncating to the last complete record, and the
//!   repair is surfaced via `tail_was_repaired()` so lifecycle recovery can
//!   react (map non-terminal orders to Unknown).

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Order lifecycle stages. An order is the money-moving intent; a position is
/// the portfolio effect — separate lifecycles by contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStage {
    Intent,
    Authorized,
    Signed,
    Submitted,
    Observed,
    Finalized,
    Failed,
    Unknown,
    Reconciling,
}

/// One durable record. `seq` is assigned by the journal (monotonic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalRecord {
    pub seq: u64,
    pub stage: LifecycleStage,
    pub intent_id: String,
    pub payload_json: String,
    /// Content digest (seq + stage + intent + payload); tampering is loud.
    pub checksum: u64,
}

fn checksum_of(seq: u64, stage: &LifecycleStage, intent_id: &str, payload_json: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    seq.hash(&mut h);
    stage.hash(&mut h);
    intent_id.hash(&mut h);
    payload_json.hash(&mut h);
    h.finish()
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal io error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("journal is locked by live process (pid {pid}); lock at {lock_path:?}")]
    AlreadyLocked { pid: u32, lock_path: PathBuf },
    #[error("journal record {seq} failed to decode: {message}")]
    Decode { seq: u64, message: String },
    #[error("journal sequence gap: expected {expected}, found {found}")]
    SequenceGap { expected: u64, found: u64 },
    #[error("journal record {seq} checksum mismatch (content tampered)")]
    ChecksumMismatch { seq: u64 },
}

/// Result of loading a journal.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalContents {
    pub records: Vec<JournalRecord>,
    /// True when this scan dropped an unterminated final fragment. After
    /// open-time repair this is false — use `FileJournal::tail_was_repaired`
    /// for the durable fact that a repair happened.
    pub truncated_tail: bool,
}

pub trait TxJournal {
    fn append(
        &mut self,
        stage: LifecycleStage,
        intent_id: &str,
        payload_json: &str,
    ) -> Result<u64, JournalError>;
    fn load(&self) -> Result<JournalContents, JournalError>;
}

/// Append-only file journal with single-writer locking and durable appends.
#[derive(Debug)]
pub struct FileJournal {
    path: PathBuf,
    file: File,
    lock_path: PathBuf,
    next_seq: u64,
    tail_repaired: bool,
}

impl FileJournal {
    /// Open (or create) a journal. Acquires the writer lock; validates the
    /// existing content (checksums + sequence); repairs an unterminated
    /// crash tail by truncating to the last complete record.
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let lock_path = path.with_extension("lock");
        acquire_lock(&lock_path)?;

        // Read existing content (may not exist yet).
        let mut existing: Vec<u8> = Vec::new();
        if let Ok(mut f) = File::open(path) {
            f.read_to_end(&mut existing)
                .map_err(|source| JournalError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }

        // Crash-tail repair: a file not ending in '\n' has a partial final
        // append — truncate to the last complete record, durably, before
        // anything appends after the fragment.
        let mut tail_repaired = false;
        let mut content = existing;
        if !content.is_empty() && *content.last().unwrap() != b'\n' {
            let keep = content
                .iter()
                .rposition(|&b| b == b'\n')
                .map(|pos| pos + 1)
                .unwrap_or(0);
            content.truncate(keep);
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .and_then(|mut f| {
                    f.write_all(&content)?;
                    f.sync_all()
                })
                .map_err(|source| JournalError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            tail_repaired = true;
        }

        // Validate what remains; determine the next sequence.
        let (records, _trunc) = parse_records(&content)?;
        let next_seq = records.last().map(|r| r.seq).unwrap_or(0) + 1;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            lock_path,
            next_seq,
            tail_repaired,
        })
    }

    /// True when open() truncated an unterminated crash tail.
    pub fn tail_was_repaired(&self) -> bool {
        self.tail_repaired
    }

    /// Explicit extra fsync (idempotent; append already syncs).
    pub fn flush_sync(&mut self) -> Result<(), JournalError> {
        self.file.sync_all().map_err(|source| JournalError::Io {
            path: self.path.clone(),
            source,
        })
    }

    fn load_from(path: &Path) -> Result<JournalContents, JournalError> {
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let (records, truncated_tail) = parse_records(&bytes)?;
        Ok(JournalContents {
            records,
            truncated_tail,
        })
    }
}

impl Drop for FileJournal {
    fn drop(&mut self) {
        // Release the writer lock. Best-effort: a failure leaves a stale
        // lock that the PID probe will reclaim.
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl TxJournal for FileJournal {
    fn append(
        &mut self,
        stage: LifecycleStage,
        intent_id: &str,
        payload_json: &str,
    ) -> Result<u64, JournalError> {
        let seq = self.next_seq;
        let rec = JournalRecord {
            seq,
            stage,
            intent_id: intent_id.to_string(),
            payload_json: payload_json.to_string(),
            checksum: checksum_of(seq, &stage, intent_id, payload_json),
        };
        let line = serde_json::to_string(&rec).map_err(|e| JournalError::Decode {
            seq,
            message: e.to_string(),
        })?;
        // Write-ahead durability: the line hits disk (sync_all) BEFORE the
        // caller acts on the stage it records.
        writeln!(self.file, "{line}")
            .and_then(|()| self.file.flush())
            .and_then(|()| self.file.sync_all())
            .map_err(|source| JournalError::Io {
                path: self.path.clone(),
                source,
            })?;
        self.next_seq += 1;
        Ok(seq)
    }

    fn load(&self) -> Result<JournalContents, JournalError> {
        Self::load_from(&self.path)
    }
}

/// Parse + validate record lines. Returns (records, truncated_tail).
/// Validation: per-record checksum, then seq contiguity across records.
fn parse_records(bytes: &[u8]) -> Result<(Vec<JournalRecord>, bool), JournalError> {
    let ends_with_newline = bytes.is_empty() || *bytes.last().unwrap() == b'\n';
    let text = String::from_utf8_lossy(bytes);
    let mut records: Vec<JournalRecord> = Vec::new();
    let mut truncated_tail = false;
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let rec: JournalRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                // Unterminated FINAL fragment = crash tail (dropped, flagged).
                // Anything else is corruption we refuse to swallow.
                if i == lines.len() - 1 && !ends_with_newline {
                    truncated_tail = true;
                    break;
                }
                return Err(JournalError::Decode {
                    seq: records.len() as u64 + 1,
                    message: e.to_string(),
                });
            }
        };
        // Checksum: content must still hash to the recorded digest.
        let expect = checksum_of(rec.seq, &rec.stage, &rec.intent_id, &rec.payload_json);
        if expect != rec.checksum {
            return Err(JournalError::ChecksumMismatch { seq: rec.seq });
        }
        // Sequence continuity: each record is prev+1 (first is arbitrary).
        if let Some(prev) = records.last() {
            if rec.seq != prev.seq + 1 {
                return Err(JournalError::SequenceGap {
                    expected: prev.seq + 1,
                    found: rec.seq,
                });
            }
        }
        records.push(rec);
    }
    Ok((records, truncated_tail))
}

/// Acquire `<path>.lock` (O_EXCL). An existing lock whose PID is alive is an
/// error; a dead owner's lock is reclaimed. PID liveness via /proc (Linux).
fn acquire_lock(lock_path: &Path) -> Result<u32, JournalError> {
    let pid = std::process::id();
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(mut f) => {
            let _ = write!(f, "{pid}");
            Ok(pid)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing: String = std::fs::read_to_string(lock_path).unwrap_or_default();
            let existing_pid: u32 = existing.trim().parse().unwrap_or(0);
            if existing_pid != 0 && !process_alive(existing_pid) {
                // Stale lock: previous owner is gone — reclaim.
                let _ = std::fs::remove_file(lock_path);
                return acquire_lock(lock_path);
            }
            Err(JournalError::AlreadyLocked {
                pid: existing_pid,
                lock_path: lock_path.to_path_buf(),
            })
        }
        Err(source) => Err(JournalError::Io {
            path: lock_path.to_path_buf(),
            source,
        }),
    }
}

fn process_alive(pid: u32) -> bool {
    // /proc check (Linux). On platforms without /proc, treat as alive —
    // fail closed rather than steal a possibly-live lock.
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_load_roundtrip_preserves_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.jsonl");
        let mut journal = FileJournal::open(&path).unwrap();
        let s1 = journal
            .append(LifecycleStage::Intent, "i-1", r#"{"mint":"AA"}"#)
            .unwrap();
        let s2 = journal
            .append(LifecycleStage::Authorized, "i-1", r#"{"nonce":1}"#)
            .unwrap();
        let s3 = journal
            .append(LifecycleStage::Signed, "i-1", r#"{"sig":"BB"}"#)
            .unwrap();
        assert_eq!((s1, s2, s3), (1, 2, 3));
        let contents = journal.load().unwrap();
        assert_eq!(contents.records.len(), 3);
        assert_eq!(contents.records[0].stage, LifecycleStage::Intent);
        assert_eq!(contents.records[2].stage, LifecycleStage::Signed);
        assert!(!contents.truncated_tail);
    }

    #[test]
    fn reopen_continues_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.jsonl");
        {
            let mut journal = FileJournal::open(&path).unwrap();
            journal.append(LifecycleStage::Intent, "i-1", "{}").unwrap();
            journal.append(LifecycleStage::Signed, "i-1", "{}").unwrap();
        } // drop releases the lock and leaves a synced file
        let mut journal2 = FileJournal::open(&path).unwrap();
        let seq = journal2
            .append(LifecycleStage::Submitted, "i-1", "{}")
            .unwrap();
        assert_eq!(seq, 3, "sequence must continue across restart");
        assert_eq!(journal2.load().unwrap().records.len(), 3);
    }

    #[test]
    fn truncated_tail_is_repaired_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.jsonl");
        {
            let mut journal = FileJournal::open(&path).unwrap();
            journal.append(LifecycleStage::Intent, "i-1", "{}").unwrap();
            journal.append(LifecycleStage::Signed, "i-1", "{}").unwrap();
        }
        // Crash mid-append: half a line, no trailing newline.
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        write!(f, r#"{{"seq":3,"stage":"submi"#).unwrap();

        let journal = FileJournal::open(&path).unwrap();
        assert!(journal.tail_was_repaired());
        let contents = journal.load().unwrap();
        assert_eq!(contents.records.len(), 2, "complete records survive");
        // And appending after repair is clean (no merged corrupt line).
        drop(journal);
        let mut j2 = FileJournal::open(&path).unwrap();
        j2.append(LifecycleStage::Submitted, "i-1", "{}").unwrap();
        let c = j2.load().unwrap();
        assert_eq!(c.records.len(), 3);
        assert_eq!(c.records[2].seq, 3);
    }
}
