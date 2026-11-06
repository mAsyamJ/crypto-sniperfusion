//! Allowlisted-root artifact discovery. Read-only, symlink-refusing,
//! size-capped. Classification is shape-driven (JSON key sniffing), never
//! filename-only.

use std::fs;
use std::path::{Path, PathBuf};

use crate::VerifyError;

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DEPTH: usize = 3;

#[derive(Debug, Clone)]
pub struct Roots {
    /// Directories allowed to be scanned. Each must exist and be a real
    /// directory (no symlinks — T-DISC1).
    pub dirs: Vec<PathBuf>,
}

impl Roots {
    pub fn from_dirs(dirs: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            dirs: dirs.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactKind {
    /// A PnL V2 bundle: `run.json` plus sibling files inside `dir`.
    PnlRunBundle { dir: PathBuf },
    /// `harness/launch-cohort_capture` window manifest.
    G0Manifest,
    /// snf-harness command manifest.
    HarnessManifest,
    /// H8 hash-chained registry JSONL.
    Registry,
    /// Harness² run summary.
    Harness2Summary,
}

#[derive(Debug, Clone)]
pub struct ArtifactRef {
    pub path: PathBuf,
    pub kind: ArtifactKind,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct DiscoveryReport {
    pub artifacts: Vec<ArtifactRef>,
    /// Files seen but not recognized (recorded, not imported).
    pub ignored: Vec<String>,
}

/// Walk every allowlisted root (depth-capped, symlink-refusing) and return
/// classified artifacts with their bytes and content hashes.
pub fn discover(roots: &Roots) -> Result<DiscoveryReport, VerifyError> {
    let mut report = DiscoveryReport::default();
    for root in &roots.dirs {
        let meta = fs::symlink_metadata(root).map_err(|e| VerifyError::Parse {
            path: root.display().to_string(),
            reason: format!("root unreadable: {e}"),
        })?;
        if meta.file_type().is_symlink() {
            return Err(VerifyError::Unsupported {
                path: root.display().to_string(),
                reason: "symlinked roots are refused (path-escape defense)".into(),
            });
        }
        if !meta.is_dir() {
            return Err(VerifyError::Parse {
                path: root.display().to_string(),
                reason: "root is not a directory".into(),
            });
        }
        walk(root, 0, &mut report)?;
    }
    report.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

fn walk(dir: &Path, depth: usize, report: &mut DiscoveryReport) -> Result<(), VerifyError> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // unreadable subdir: skip, never fail the corpus
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            // Inner symlinks: skip silently-refused (recorded as ignored).
            report
                .ignored
                .push(format!("{}: symlink refused", path.display()));
            continue;
        }
        if ft.is_dir() {
            walk(&path, depth + 1, report)?;
            continue;
        }
        if !ft.is_file() || meta.len() > MAX_FILE_BYTES {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !name.ends_with(".json") && !name.ends_with(".jsonl") {
                continue;
            }
        } else {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            report
                .ignored
                .push(format!("{}: unreadable", path.display()));
            continue;
        };
        match classify(&path, &bytes) {
            Some(kind) => {
                let sha256 = crate::verify::sha256_hex(&bytes);
                report.artifacts.push(ArtifactRef {
                    path: path.clone(),
                    kind,
                    sha256,
                    bytes,
                });
            }
            None => report
                .ignored
                .push(format!("{}: unrecognized shape", path.display())),
        }
    }
    Ok(())
}

/// Shape-driven classification (T-SHAPE family). A file is what its JSON
/// keys say it is; unknown shapes are ignored, not guessed.
fn classify(path: &Path, bytes: &[u8]) -> Option<ArtifactKind> {
    let text = std::str::from_utf8(bytes).ok()?;
    let name = path.file_name()?.to_str()?;
    if name == "summary.json" {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("evidence_class").and_then(|c| c.as_str())
            == Some("DETERMINISTIC_RESEARCH_HARNESS")
        {
            return Some(ArtifactKind::Harness2Summary);
        }
        return None;
    }
    if name == "run.json" {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("schema_version").and_then(|s| s.as_u64()) == Some(2)
            && v.get("receipt_sha256").is_some()
            && v.get("engine").is_some()
        {
            return Some(ArtifactKind::PnlRunBundle {
                dir: path.parent()?.to_path_buf(),
            });
        }
        return None;
    }
    if name.starts_with("manifest-") && name.ends_with(".json") {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("coverage_state").is_some() && v.get("window").is_some() {
            return Some(ArtifactKind::G0Manifest);
        }
        return None;
    }
    // snf-harness manifests: <stamp>-<command>-<pid>.json (any command).
    if name.ends_with(".json") {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("command").and_then(|c| c.as_str()).is_some()
            && v.get("exit_code").is_some()
            && v.get("git_sha").is_some()
            && v.get("inputs").is_some()
        {
            return Some(ArtifactKind::HarnessManifest);
        }
        return None;
    }
    if name.ends_with(".jsonl") {
        // H8 registry: every line carries kind + prev_hash + record_sha256.
        let first = text.lines().find(|l| !l.trim().is_empty())?;
        let v: serde_json::Value = serde_json::from_str(first).ok()?;
        if v.get("kind").is_some()
            && v.get("prev_hash").is_some()
            && v.get("record_sha256").is_some()
        {
            return Some(ArtifactKind::Registry);
        }
    }
    None
}
