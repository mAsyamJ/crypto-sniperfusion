//! PnlRunReceiptV2 (PF1): the immutable, deterministic P&L receipt bundle.
//!
//! One receipt per run, derived ONLY from oracle + engine outputs; same
//! inputs MUST produce an identical `receipt_sha256` (no clock, no RNG in
//! this module). Bundle files: run.json, trades.jsonl, equity.jsonl,
//! fees.json, comparison.json, audit.json — all write-once 0444.
//! Synthetic runs are permanently `promotable: false`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::oracle::{AccountingOracle, StrategyAttribution};

pub const PNL_SCHEMA_VERSION: u16 = 2;
pub const PROOF_STATE_SYNTHETIC: &str = "SYNTHETIC";

/// Per-strategy comparison row (identical cohort/capital/fee model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonRow {
    pub strategy: String,
    pub net_pnl_lamports: i64,
    /// challenger − S00 NO_TRADE on identical inputs.
    pub incremental_net_edge_vs_no_trade: i64,
    pub max_drawdown_lamports: u64,
    pub open_positions: u64,
    pub status: String,
}

/// The V2 receipt (run.json). Field order is stable for hashing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PnlRunReceiptV2 {
    pub schema_version: u16,
    pub engine: String,
    pub evidence_class: String,
    pub proof_state: String,
    pub promotable: bool,
    pub live_trading_enabled: bool,
    pub currency: String,
    pub completeness: String,
    pub dataset_sha256: String,
    pub starting_nav_lamports: u64,
    pub external_deposits_lamports: u64,
    pub external_withdrawals_lamports: u64,
    pub ending_nav_lamports: u64,
    pub realized_pnl_lamports: i64,
    pub unrealized_pnl_lamports: i64,
    pub net_pnl_lamports: i64,
    pub recoverable_rent_lamports: u64,
    pub open_inventory_cost_basis_lamports: u64,
    pub open_positions: u64,
    pub unexitable_exits: u64,
    pub unknown_at_risk_lamports: u64,
    pub max_drawdown_lamports: u64,
    pub fees_by_kind_lamports: BTreeMap<String, u64>,
    pub attribution: BTreeMap<String, StrategyAttribution>,
    pub comparison: Vec<ComparisonRow>,
    pub verdict: String,
    pub limitations: Vec<String>,
    /// sha256 of the canonical JSON of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

impl PnlRunReceiptV2 {
    /// Deterministic identity: canonical-JSON sha256 with the hash field
    /// itself zeroed (self-reference excluded).
    pub fn compute_hash(&self) -> String {
        let mut copy = self.clone();
        copy.receipt_sha256 = String::new();
        let canonical = serde_json::to_string(&copy).expect("receipt serializes");
        let digest = Sha256::digest(canonical.as_bytes());
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Seal: compute and store the hash.
    pub fn seal(mut self) -> Self {
        let hash = self.compute_hash();
        self.receipt_sha256 = hash;
        self
    }
}

/// Build a receipt from one strategy's oracle + engine summary numbers.
#[allow(clippy::too_many_arguments)]
pub fn build_receipt(
    engine: &str,
    evidence_class: &str,
    completeness: &str,
    dataset_sha256: &str,
    starting_nav: u64,
    oracle: &AccountingOracle,
    exit_marks: &std::collections::HashMap<String, u64>,
    realized_pnl: i64,
    max_drawdown: u64,
    open_positions: u64,
    unexitable_exits: u64,
    comparison: Vec<ComparisonRow>,
    limitations: Vec<String>,
) -> PnlRunReceiptV2 {
    let nav = oracle.nav(exit_marks);
    let net_pnl = oracle.net_pnl(starting_nav, exit_marks);
    let mut fees = BTreeMap::new();
    for (kind, amount) in oracle.fee_totals() {
        fees.insert(format!("{kind:?}").to_lowercase(), amount);
    }
    let verdict = if comparison
        .iter()
        .all(|r| r.incremental_net_edge_vs_no_trade <= 0)
    {
        "EDGE_NOT_DEMONSTRATED".to_string()
    } else {
        "CANDIDATE_FOR_INDEPENDENT_REVIEW".to_string()
    };
    PnlRunReceiptV2 {
        schema_version: PNL_SCHEMA_VERSION,
        engine: engine.to_string(),
        evidence_class: evidence_class.to_string(),
        proof_state: PROOF_STATE_SYNTHETIC.to_string(),
        promotable: false,
        live_trading_enabled: false,
        currency: "SOL-lamports".to_string(),
        completeness: completeness.to_string(),
        dataset_sha256: dataset_sha256.to_string(),
        starting_nav_lamports: starting_nav,
        external_deposits_lamports: oracle.external_deposits(),
        external_withdrawals_lamports: oracle.external_withdrawals(),
        ending_nav_lamports: nav.nav,
        realized_pnl_lamports: realized_pnl,
        unrealized_pnl_lamports: nav.unrealized_pnl,
        net_pnl_lamports: net_pnl,
        recoverable_rent_lamports: nav.recoverable_rent,
        open_inventory_cost_basis_lamports: nav.open_inventory_cost_basis,
        open_positions,
        unexitable_exits,
        unknown_at_risk_lamports: nav.unresolved_at_risk,
        max_drawdown_lamports: max_drawdown,
        fees_by_kind_lamports: fees,
        attribution: oracle.attributed(),
        comparison,
        verdict,
        limitations,
        receipt_sha256: String::new(),
    }
    .seal()
}

/// Write-once bundle file (exclusive create, fsync, chmod 0444 on unix).
pub fn write_bundle_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    file.flush()?;
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444))?;
    }
    Ok(())
}

/// Write the full V2 bundle into `dir`; returns run.json path.
pub fn write_bundle(
    dir: &Path,
    receipt: &PnlRunReceiptV2,
    trades_jsonl: &str,
    equity_jsonl: &str,
    audit_json: &str,
) -> std::io::Result<std::path::PathBuf> {
    let run_path = dir.join("run.json");
    write_bundle_file(&run_path, &serde_json::to_string_pretty(receipt).unwrap())?;
    write_bundle_file(&dir.join("trades.jsonl"), trades_jsonl)?;
    write_bundle_file(&dir.join("equity.jsonl"), equity_jsonl)?;
    write_bundle_file(
        &dir.join("fees.json"),
        &serde_json::to_string_pretty(&receipt.fees_by_kind_lamports).unwrap(),
    )?;
    write_bundle_file(
        &dir.join("comparison.json"),
        &serde_json::to_string_pretty(&receipt.comparison).unwrap(),
    )?;
    write_bundle_file(&dir.join("audit.json"), audit_json)?;
    Ok(run_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::AccountingOracle;

    fn sample_receipt() -> PnlRunReceiptV2 {
        let oracle = AccountingOracle::new(100_000_000);
        build_receipt(
            "ProtocolEconomicV1",
            "SYNTHETIC",
            "SYNTHETIC_COMPLETE_FIXTURE",
            "deadbeef",
            100_000_000,
            &oracle,
            &Default::default(),
            0,
            0,
            0,
            0,
            vec![ComparisonRow {
                strategy: "S00".into(),
                net_pnl_lamports: 0,
                incremental_net_edge_vs_no_trade: 0,
                max_drawdown_lamports: 0,
                open_positions: 0,
                status: "NO_TRADE_BASELINE".into(),
            }],
            vec!["synthetic fixture; not market evidence".into()],
        )
    }

    #[test]
    fn identical_inputs_yield_identical_receipt_hash() {
        let a = sample_receipt();
        let b = sample_receipt();
        assert_eq!(a.receipt_sha256, b.receipt_sha256);
        assert!(!a.receipt_sha256.is_empty());
        // Hash covers the content: any field change changes the hash.
        let mut c = a.clone();
        c.net_pnl_lamports += 1;
        assert_ne!(a.receipt_sha256, c.compute_hash());
    }

    #[test]
    fn receipt_is_nonpromotable_and_sol_denominated() {
        let receipt = sample_receipt();
        assert!(!receipt.promotable);
        assert_eq!(receipt.proof_state, "SYNTHETIC");
        assert_eq!(receipt.currency, "SOL-lamports");
        assert_eq!(receipt.schema_version, 2);
    }

    #[test]
    fn bundle_writes_once_and_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let receipt = sample_receipt();
        let run = write_bundle(dir.path(), &receipt, "", "", "{}").unwrap();
        assert!(run.exists());
        assert!(write_bundle(dir.path(), &receipt, "", "", "{}").is_err());
    }
}
