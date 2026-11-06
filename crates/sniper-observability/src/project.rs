//! SQLite projection (schema v1): a disposable, deterministic cache of the
//! verified corpus. Import is idempotent (keyed by content hash); a changed
//! artifact appends a NEW run version rather than mutating history.
//! No SQL arithmetic on money ever happens — columns are copied values.

use std::path::Path;

use rusqlite::{params, Connection};
use serde_json::Value;

use crate::discovery::{discover, ArtifactKind, ArtifactRef, Roots};
use crate::parse::parse_trades;
use crate::verify::{dataset_sha_is_hash, sha256_hex, verify_registry_chain, verify_run_receipt};

pub const SCHEMA_VERSION: u32 = 1;

const SCHEMA_V1: &str = r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS projection_meta (
    key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS source_artifacts (
    id TEXT PRIMARY KEY, path TEXT NOT NULL, sha256 TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL, status TEXT NOT NULL, bytes INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY, artifact_sha256 TEXT NOT NULL, engine TEXT NOT NULL,
    evidence_class TEXT NOT NULL, proof_state TEXT NOT NULL, completeness TEXT NOT NULL,
    verdict TEXT NOT NULL, promotable INTEGER NOT NULL,
    starting_nav_lamports INTEGER NOT NULL, external_deposits_lamports INTEGER NOT NULL,
    external_withdrawals_lamports INTEGER NOT NULL, ending_nav_lamports INTEGER NOT NULL,
    realized_pnl_lamports INTEGER NOT NULL, unrealized_pnl_lamports INTEGER NOT NULL,
    net_pnl_lamports INTEGER NOT NULL, max_drawdown_lamports INTEGER NOT NULL,
    recoverable_rent_lamports INTEGER NOT NULL, open_inventory_cost_basis_lamports INTEGER NOT NULL,
    open_positions INTEGER NOT NULL, unexitable_exits INTEGER NOT NULL,
    unknown_at_risk_lamports INTEGER NOT NULL, dataset_sha256 TEXT NOT NULL,
    dataset_sha_is_hash INTEGER NOT NULL, receipt_sha256 TEXT NOT NULL,
    limitations_json TEXT NOT NULL, audit_json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS run_comparisons (
    run_id TEXT NOT NULL, ord INTEGER NOT NULL, strategy TEXT NOT NULL, net_pnl_lamports INTEGER NOT NULL,
    incremental_net_edge INTEGER NOT NULL, max_drawdown_lamports INTEGER NOT NULL,
    open_positions INTEGER NOT NULL, status TEXT NOT NULL,
    PRIMARY KEY (run_id, strategy));
CREATE TABLE IF NOT EXISTS run_fees (
    run_id TEXT NOT NULL, kind TEXT NOT NULL, lamports INTEGER NOT NULL,
    PRIMARY KEY (run_id, kind));
CREATE TABLE IF NOT EXISTS run_trades (
    run_id TEXT NOT NULL, ord INTEGER NOT NULL, strategy TEXT NOT NULL,
    record_kind TEXT NOT NULL, mint TEXT NOT NULL, slot INTEGER,
    at_ms INTEGER, net_pnl_lamports INTEGER, open INTEGER NOT NULL DEFAULT 0,
    raw TEXT NOT NULL, PRIMARY KEY (run_id, ord));
CREATE INDEX IF NOT EXISTS idx_run_trades_run ON run_trades(run_id, ord);
CREATE TABLE IF NOT EXISTS coverage_windows (
    id TEXT PRIMARY KEY, artifact_sha256 TEXT NOT NULL UNIQUE,
    from_slot INTEGER NOT NULL, to_slot INTEGER NOT NULL, coverage_state TEXT NOT NULL,
    gaps INTEGER NOT NULL, raw_creates_count INTEGER NOT NULL, unique_mints INTEGER NOT NULL,
    in_window_signatures INTEGER NOT NULL, dataset_sha256 TEXT NOT NULL,
    evidence_class TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS registry_records (
    seq INTEGER NOT NULL, registry_id TEXT NOT NULL, kind TEXT NOT NULL,
    payload_json TEXT NOT NULL, record_sha256 TEXT NOT NULL, prev_hash TEXT NOT NULL,
    PRIMARY KEY (registry_id, seq));
CREATE TABLE IF NOT EXISTS harness2_runs (
    id TEXT PRIMARY KEY, artifact_sha256 TEXT NOT NULL UNIQUE,
    done INTEGER NOT NULL, failed INTEGER NOT NULL, execution_count INTEGER NOT NULL,
    model_backed INTEGER NOT NULL, promotable INTEGER NOT NULL, evidence_class TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS harness_manifests (
    id TEXT PRIMARY KEY, artifact_sha256 TEXT NOT NULL UNIQUE,
    command TEXT NOT NULL, exit_code INTEGER NOT NULL, git_sha TEXT NOT NULL,
    recorded_at_unix INTEGER, summary_json TEXT NOT NULL);
"#;

#[derive(Debug, Default, PartialEq)]
pub struct ImportSummary {
    pub runs_imported: usize,
    pub runs_quarantined: usize,
    pub coverage_windows: usize,
    pub registry_records: usize,
    pub registries_quarantined: usize,
    pub harness2_runs: usize,
    pub harness_manifests: usize,
    pub skipped_existing: usize,
}

/// Open (creating if needed) the projection database with schema v1.
pub fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_V1)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(conn)
}

/// Wipe and rebuild the projection from the corpus (T-DET1).
pub fn rebuild(
    conn: &Connection,
    roots: &Roots,
    built_at_utc: &str,
) -> rusqlite::Result<ImportSummary> {
    conn.execute_batch(
        "DELETE FROM source_artifacts; DELETE FROM runs; DELETE FROM run_comparisons;
         DELETE FROM run_fees; DELETE FROM run_trades; DELETE FROM coverage_windows;
         DELETE FROM registry_records; DELETE FROM harness2_runs; DELETE FROM harness_manifests;
         DELETE FROM projection_meta;",
    )?;
    import_corpus(conn, roots, built_at_utc)
}

/// Idempotent import: artifacts already present (by content hash) are
/// skipped; verification failures quarantine the artifact only.
pub fn import_corpus(
    conn: &Connection,
    roots: &Roots,
    built_at_utc: &str,
) -> rusqlite::Result<ImportSummary> {
    let mut summary = ImportSummary::default();
    let report = match discover(roots) {
        Ok(r) => r,
        Err(e) => {
            // Discovery-level refusal (e.g. symlinked root): record and
            // fail closed for the whole import — do not serve a partial scan.
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e)));
        }
    };
    set_meta(conn, "schema_version", &SCHEMA_VERSION.to_string())?;
    set_meta(conn, "built_at", built_at_utc)?;
    set_meta(
        conn,
        "roots",
        &roots
            .dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(":"),
    )?;

    for artifact in &report.artifacts {
        if artifact_seen(conn, &artifact.sha256)? {
            summary.skipped_existing += 1;
            continue;
        }
        match &artifact.kind {
            ArtifactKind::PnlRunBundle { dir } => {
                import_run_bundle(conn, artifact, dir, &mut summary)?;
            }
            ArtifactKind::G0Manifest => {
                import_g0(conn, artifact, &mut summary)?;
            }
            ArtifactKind::Registry => {
                import_registry(conn, artifact, &mut summary)?;
            }
            ArtifactKind::Harness2Summary => {
                import_harness2(conn, artifact, &mut summary)?;
            }
            ArtifactKind::HarnessManifest => {
                import_harness_manifest(conn, artifact, &mut summary)?;
            }
        }
    }
    Ok(summary)
}

fn artifact_seen(conn: &Connection, sha256: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM source_artifacts WHERE sha256 = ?1",
        params![sha256],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

fn record_artifact(
    conn: &Connection,
    artifact: &ArtifactRef,
    status: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO source_artifacts (id, path, sha256, kind, status, bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            sha256_hex(format!("{}:{}", artifact.path.display(), artifact.sha256).as_bytes()),
            artifact.path.display().to_string(),
            artifact.sha256,
            kind_name(&artifact.kind),
            status,
            artifact.bytes.len() as i64
        ],
    )?;
    Ok(())
}

fn kind_name(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::PnlRunBundle { .. } => "pnl_run_bundle",
        ArtifactKind::G0Manifest => "g0_manifest",
        ArtifactKind::Registry => "h8_registry",
        ArtifactKind::Harness2Summary => "harness2_summary",
        ArtifactKind::HarnessManifest => "harness_manifest",
    }
}

fn import_run_bundle(
    conn: &Connection,
    artifact: &ArtifactRef,
    dir: &Path,
    summary: &mut ImportSummary,
) -> rusqlite::Result<()> {
    let path_str = artifact.path.display().to_string();
    let raw = String::from_utf8_lossy(&artifact.bytes).to_string();
    let receipt = match verify_run_receipt(&path_str, &raw) {
        Ok(r) => r,
        Err(crate::VerifyError::SourceMismatch { .. }) => {
            // Quarantine: record the artifact with its mismatch status; do
            // not import ANY of its numbers (T-VER1, T-DET2).
            record_artifact(conn, artifact, "SOURCE_MISMATCH")?;
            summary.runs_quarantined += 1;
            return Ok(());
        }
        Err(e) => {
            record_artifact(conn, artifact, "UNSUPPORTED")?;
            summary.runs_quarantined += 1;
            let _ = e; // status carries the class; path recorded above
            return Ok(());
        }
    };
    let run_id = artifact.sha256[..16].to_string();

    let trades_raw = std::fs::read_to_string(dir.join("trades.jsonl")).unwrap_or_default();
    let audit_raw = std::fs::read_to_string(dir.join("audit.json")).unwrap_or_default();
    let trade_rows = parse_trades(&receipt.engine, &path_str, &trades_raw)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT OR IGNORE INTO runs (id, artifact_sha256, engine, evidence_class, proof_state,
            completeness, verdict, promotable, starting_nav_lamports, external_deposits_lamports,
            external_withdrawals_lamports, ending_nav_lamports, realized_pnl_lamports,
            unrealized_pnl_lamports, net_pnl_lamports, max_drawdown_lamports,
            recoverable_rent_lamports, open_inventory_cost_basis_lamports, open_positions,
            unexitable_exits, unknown_at_risk_lamports, dataset_sha256, dataset_sha_is_hash,
            receipt_sha256, limitations_json, audit_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
        params![
            run_id,
            artifact.sha256,
            receipt.engine,
            receipt.evidence_class,
            receipt.proof_state,
            receipt.completeness,
            receipt.verdict,
            receipt.promotable as i64,
            receipt.starting_nav_lamports as i64,
            receipt.external_deposits_lamports as i64,
            receipt.external_withdrawals_lamports as i64,
            receipt.ending_nav_lamports as i64,
            receipt.realized_pnl_lamports,
            receipt.unrealized_pnl_lamports,
            receipt.net_pnl_lamports,
            receipt.max_drawdown_lamports as i64,
            receipt.recoverable_rent_lamports as i64,
            receipt.open_inventory_cost_basis_lamports as i64,
            receipt.open_positions as i64,
            receipt.unexitable_exits as i64,
            receipt.unknown_at_risk_lamports as i64,
            receipt.dataset_sha256,
            dataset_sha_is_hash(&receipt) as i64,
            receipt.receipt_sha256,
            serde_json::to_string(&receipt.limitations).unwrap_or_else(|_| "[]".into()),
            if audit_raw.trim().is_empty() { "" } else { audit_raw.trim() },
        ],
    )?;
    for (ord, row) in receipt.comparison.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO run_comparisons VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                run_id,
                ord as i64,
                row.strategy,
                row.net_pnl_lamports,
                row.incremental_net_edge_vs_no_trade,
                row.max_drawdown_lamports as i64,
                row.open_positions as i64,
                row.status
            ],
        )?;
    }
    for (kind, amount) in &receipt.fees_by_kind_lamports {
        conn.execute(
            "INSERT OR IGNORE INTO run_fees VALUES (?1,?2,?3)",
            params![run_id, kind, *amount as i64],
        )?;
    }
    for t in &trade_rows {
        conn.execute(
            "INSERT OR IGNORE INTO run_trades (run_id, ord, strategy, record_kind, mint, slot,
                at_ms, net_pnl_lamports, open, raw) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                run_id,
                t.ord as i64,
                t.strategy,
                t.kind.as_str(),
                t.mint,
                t.slot,
                t.at_ms,
                t.net_pnl_lamports,
                t.open as i64,
                t.raw
            ],
        )?;
    }
    record_artifact(conn, artifact, "OK")?;
    summary.runs_imported += 1;
    Ok(())
}

fn import_g0(
    conn: &Connection,
    artifact: &ArtifactRef,
    summary: &mut ImportSummary,
) -> rusqlite::Result<()> {
    let v: Value = match serde_json::from_slice(&artifact.bytes) {
        Ok(v) => v,
        Err(_) => {
            record_artifact(conn, artifact, "UNSUPPORTED")?;
            return Ok(());
        }
    };
    let window = v.get("window").cloned().unwrap_or(Value::Null);
    let gaps = v
        .get("gaps")
        .and_then(|g| g.as_array())
        .map(|a| a.len())
        .unwrap_or(0) as i64;
    conn.execute(
        "INSERT OR IGNORE INTO coverage_windows VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            &artifact.sha256[..16],
            artifact.sha256,
            window
                .get("from_slot")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            window.get("to_slot").and_then(|x| x.as_i64()).unwrap_or(0),
            v.get("coverage_state")
                .and_then(|x| x.as_str())
                .unwrap_or("UNKNOWN"),
            gaps,
            v.get("raw_creates_count")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            v.get("unique_mints").and_then(|x| x.as_i64()).unwrap_or(0),
            v.get("in_window_signatures")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            v.get("dataset_sha256")
                .and_then(|x| x.as_str())
                .unwrap_or(""),
            v.get("evidence_class")
                .and_then(|x| x.as_str())
                .unwrap_or(""),
        ],
    )?;
    record_artifact(conn, artifact, "OK")?;
    summary.coverage_windows += 1;
    Ok(())
}

fn import_registry(
    conn: &Connection,
    artifact: &ArtifactRef,
    summary: &mut ImportSummary,
) -> rusqlite::Result<()> {
    let text = String::from_utf8_lossy(&artifact.bytes);
    let records: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let registry_id = artifact.sha256[..16].to_string();
    if verify_registry_chain(&records).is_err() {
        record_artifact(conn, artifact, "SOURCE_MISMATCH")?;
        summary.registries_quarantined += 1;
        return Ok(());
    }
    for rec in &records {
        let seq = rec.get("seq").and_then(|s| s.as_i64()).unwrap_or(0);
        let kind = rec.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        // Holdout defense (T-HOLD1): label payloads are stripped server-side
        // before storage — recursively, at any depth; the projection cannot
        // serve what it never stores.
        let mut payload = rec.get("payload").cloned().unwrap_or(Value::Null);
        strip_labels(&mut payload);
        conn.execute(
            "INSERT OR IGNORE INTO registry_records VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                registry_id,
                seq,
                kind,
                serde_json::to_string(&payload).unwrap_or_default(),
                rec.get("record_sha256")
                    .and_then(|h| h.as_str())
                    .unwrap_or(""),
                rec.get("prev_hash").and_then(|h| h.as_str()).unwrap_or("")
            ],
        )?;
    }
    record_artifact(conn, artifact, "OK")?;
    summary.registry_records += records.len();
    Ok(())
}

fn import_harness2(
    conn: &Connection,
    artifact: &ArtifactRef,
    summary: &mut ImportSummary,
) -> rusqlite::Result<()> {
    let v: Value = match serde_json::from_slice(&artifact.bytes) {
        Ok(v) => v,
        Err(_) => {
            record_artifact(conn, artifact, "UNSUPPORTED")?;
            return Ok(());
        }
    };
    conn.execute(
        "INSERT OR IGNORE INTO harness2_runs VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            &artifact.sha256[..16],
            artifact.sha256,
            v.get("done").and_then(|x| x.as_i64()).unwrap_or(0),
            v.get("failed").and_then(|x| x.as_i64()).unwrap_or(0),
            v.get("execution_count")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            v.get("model_backed")
                .and_then(|x| x.as_bool())
                .unwrap_or(false) as i64,
            v.get("promotable")
                .and_then(|x| x.as_bool())
                .unwrap_or(true) as i64,
            v.get("evidence_class")
                .and_then(|x| x.as_str())
                .unwrap_or(""),
        ],
    )?;
    record_artifact(conn, artifact, "OK")?;
    summary.harness2_runs += 1;
    Ok(())
}

fn import_harness_manifest(
    conn: &Connection,
    artifact: &ArtifactRef,
    summary: &mut ImportSummary,
) -> rusqlite::Result<()> {
    let v: Value = match serde_json::from_slice(&artifact.bytes) {
        Ok(v) => v,
        Err(_) => {
            record_artifact(conn, artifact, "UNSUPPORTED")?;
            return Ok(());
        }
    };
    conn.execute(
        "INSERT OR IGNORE INTO harness_manifests VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            &artifact.sha256[..16],
            artifact.sha256,
            v.get("command").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("exit_code").and_then(|x| x.as_i64()).unwrap_or(-1),
            v.get("git_sha").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("recorded_at_unix").and_then(|x| x.as_i64()),
            serde_json::to_string(&v).unwrap_or_default(),
        ],
    )?;
    record_artifact(conn, artifact, "OK")?;
    summary.harness_manifests += 1;
    Ok(())
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO projection_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Recursively remove holdout-label material from a registry payload.
/// Keys: `labels`, `holdout_labels`, `label` (defensive breadth).
fn strip_labels(v: &mut Value) {
    match v {
        Value::Object(obj) => {
            obj.remove("labels");
            obj.remove("holdout_labels");
            obj.remove("label");
            for child in obj.values_mut() {
                strip_labels(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_labels(child);
            }
        }
        _ => {}
    }
}
