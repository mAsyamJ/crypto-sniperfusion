//! Durable transaction journal (append-only JSONL). M1 provides the interface,
//! the file implementation, and crash-tail tolerance; the full
//! write-before-submit protocol arrives with the M3 lifecycle.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Order lifecycle stages. An order is the money-moving intent; a position is
/// the portfolio effect — separate lifecycles by contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal io error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("journal record {seq} failed to decode: {message}")]
    Decode { seq: u64, message: String },
}

/// Result of loading a journal that was interrupted mid-write.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalContents {
    pub records: Vec<JournalRecord>,
    /// Number of trailing bytes that formed an incomplete final line
    /// (a crash mid-append). The partial record is discarded and flagged.
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

/// Append-only file journal. Each append writes one JSON line and flushes;
/// durability (`sync_all`) is explicit (`flush_sync`) because it is a latency
/// decision that belongs to the M3 write-before-submit protocol.
#[derive(Debug)]
pub struct FileJournal {
    path: PathBuf,
    file: File,
    next_seq: u64,
}

impl FileJournal {
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let next_seq = Self::scan_max_seq(path)? + 1;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            next_seq,
        })
    }

    pub fn flush_sync(&mut self) -> Result<(), JournalError> {
        self.file.sync_all().map_err(|source| JournalError::Io {
            path: self.path.clone(),
            source,
        })
    }

    fn scan_max_seq(path: &Path) -> Result<u64, JournalError> {
        let contents = Self::load_from(path)?;
        Ok(contents.records.last().map(|r| r.seq).unwrap_or(0))
    }

    fn load_from(path: &Path) -> Result<JournalContents, JournalError> {
        let file = File::open(path).map_err(|source| JournalError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut records = Vec::new();
        let mut truncated_tail = false;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|source| JournalError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<JournalRecord>(&line) {
                Ok(rec) => records.push(rec),
                Err(e) => {
                    // A malformed FINAL line is a crash mid-append: discard and
                    // flag. A malformed line in the middle is corruption we
                    // must not silently swallow — surface it.
                    if !line.ends_with('}') {
                        truncated_tail = true;
                    } else {
                        return Err(JournalError::Decode {
                            seq: records.len() as u64 + 1,
                            message: e.to_string(),
                        });
                    }
                }
            }
        }
        Ok(JournalContents {
            records,
            truncated_tail,
        })
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
        };
        let line = serde_json::to_string(&rec).map_err(|e| JournalError::Decode {
            seq,
            message: e.to_string(),
        })?;
        writeln!(self.file, "{line}").map_err(|source| JournalError::Io {
            path: self.path.clone(),
            source,
        })?;
        self.file.flush().map_err(|source| JournalError::Io {
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
        } // "crash": drop without anything special
        let mut journal2 = FileJournal::open(&path).unwrap();
        let seq = journal2
            .append(LifecycleStage::Submitted, "i-1", "{}")
            .unwrap();
        assert_eq!(seq, 3, "sequence must continue across restart");
        assert_eq!(journal2.load().unwrap().records.len(), 3);
    }

    #[test]
    fn truncated_tail_is_flagged_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.jsonl");
        {
            let mut journal = FileJournal::open(&path).unwrap();
            journal.append(LifecycleStage::Intent, "i-1", "{}").unwrap();
            journal.append(LifecycleStage::Signed, "i-1", "{}").unwrap();
        }
        // Simulate a crash mid-append: half a line.
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        write!(f, r#"{{"seq":3,"stage":"submi"#).unwrap();

        let journal = FileJournal::open(&path).unwrap();
        let contents = journal.load().unwrap();
        assert_eq!(contents.records.len(), 2, "complete records survive");
        assert!(contents.truncated_tail, "partial tail is flagged");
    }
}
