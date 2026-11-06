//! Shared fixture builders for DASH-02 tests. Fixtures are built through
//! the REAL producer paths (`AccountingOracle` + `build_receipt` +
//! `write_bundle`) — hand-written PnL numbers are forbidden here (TEST_PLAN
//! Global Constraints).

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;

use sniper_strategy::engine::protocol::ExecutionRecord;
use sniper_strategy::follower::FollowerEntry;
use sniper_strategy::oracle::AccountingOracle;
use sniper_strategy::receipt::{build_receipt, write_bundle, ComparisonRow, PnlRunReceiptV2};

pub const FIXED_BUILT_AT: &str = "2026-11-05T00:00:00Z";

pub fn s00_row() -> ComparisonRow {
    ComparisonRow {
        strategy: "S00".into(),
        net_pnl_lamports: 0,
        incremental_net_edge_vs_no_trade: 0,
        max_drawdown_lamports: 0,
        open_positions: 0,
        status: "NO_TRADE_BASELINE".into(),
    }
}

pub fn arm_row(strategy: &str, net: i64) -> ComparisonRow {
    ComparisonRow {
        strategy: strategy.into(),
        net_pnl_lamports: net,
        incremental_net_edge_vs_no_trade: net,
        max_drawdown_lamports: 0,
        open_positions: 0,
        status: if net <= 0 {
            "LOSING".into()
        } else {
            "CANDIDATE_FOR_INDEPENDENT_REVIEW".into()
        },
    }
}

pub fn buy(event_id: &str, mint: &str, slot: u64, spend: u64, tokens: u64) -> ExecutionRecord {
    ExecutionRecord::BuyFill {
        event_id: event_id.into(),
        mint: mint.into(),
        slot,
        spend_lamports: spend,
        tokens_out: tokens,
        protocol_fee_lamports: 0,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 5_000,
        priority_fee_lamports: 1_000,
        tip_lamports: 0,
        rent_lamports: 1_500_000,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    }
}

pub fn sell(event_id: &str, mint: &str, slot: u64, tokens: u64, quote_net: u64) -> ExecutionRecord {
    ExecutionRecord::SellFill {
        event_id: event_id.into(),
        mint: mint.into(),
        slot,
        tokens_in: tokens,
        quote_out_net: quote_net,
        protocol_fee_lamports: 0,
        creator_fee_lamports: 0,
        lp_fee_lamports: 0,
        network_fee_lamports: 5_000,
        priority_fee_lamports: 1_000,
        tip_lamports: 0,
        rent_recovered_lamports: 0,
        slippage_lamports: 0,
        fee_synthetic_fallback: false,
    }
}

/// Write a research-style bundle (JSONL trades with embedded records).
#[allow(clippy::too_many_arguments)]
pub fn write_research_bundle(
    dir: &Path,
    _name: &str,
    engine: &str,
    oracle: &AccountingOracle,
    starting_nav: u64,
    realized: i64,
    max_drawdown: u64,
    open_positions: u64,
    comparison: Vec<ComparisonRow>,
    records: &[ExecutionRecord],
) -> PnlRunReceiptV2 {
    let receipt = build_receipt(
        engine,
        "SYNTHETIC",
        "SYNTHETIC_COMPLETE_FIXTURE",
        &"deadbeef".repeat(8),
        starting_nav,
        oracle,
        &HashMap::new(),
        realized,
        max_drawdown,
        open_positions,
        0,
        comparison,
        vec!["synthetic test fixture; not market evidence".into()],
    );
    let mut trades = String::new();
    for (i, rec) in records.iter().enumerate() {
        trades
            .push_str(&serde_json::json!({"strategy": format!("T{i}"), "record": rec}).to_string());
        trades.push('\n');
    }
    write_bundle(dir, &receipt, &trades, "", "{}").expect("bundle writes");
    receipt
}

/// Write a wallet-follow-style bundle: run.json + a JSON ARRAY trades file.
pub fn write_follower_bundle(
    dir: &Path,
    runs: &[sniper_strategy::follower::FollowerRun],
    arm_nets: &[(&str, i64)],
) -> PnlRunReceiptV2 {
    let oracle = AccountingOracle::new(0);
    let mut comparison: Vec<ComparisonRow> = arm_nets.iter().map(|(s, n)| arm_row(s, *n)).collect();
    comparison.push(s00_row());
    let receipt = build_receipt(
        "wallet_follower_v1",
        "SYNTHETIC_OR_OBSERVED_QUOTES",
        "QUOTES_AS_PROVIDED",
        "signals+quotes",
        0,
        &oracle,
        &HashMap::new(),
        0,
        0,
        0,
        0,
        comparison,
        vec!["fixture follower bundle".into()],
    );
    let trades_json = serde_json::to_string_pretty(runs).unwrap();
    write_bundle(dir, &receipt, &trades_json, "", "{}").expect("bundle writes");
    receipt
}

pub fn follower_entry(
    signal_id: &str,
    mint: &str,
    entry_at_ms: u64,
    entry_lamports: u64,
    exit_at_ms: Option<u64>,
    net: Option<i64>,
) -> FollowerEntry {
    FollowerEntry {
        signal_id: signal_id.into(),
        mint: mint.into(),
        entry_at_ms,
        entry_effective_lamports: entry_lamports,
        tokens: 1_000,
        exit_at_ms,
        exit_net_lamports: exit_at_ms
            .map(|_| entry_lamports.saturating_add_signed(net.unwrap_or(0))),
        net_pnl_lamports: net,
        refusal_reason: None,
    }
}

/// Minimal H8 registry JSONL with a valid hash-shaped chain (values are
/// fixture hex; linkage is what T-REG1 exercises).
pub fn registry_jsonl(valid: bool) -> String {
    let r0 = serde_json::json!({
        "kind": "hypothesis",
        "payload": {"hypothesis_id": "H-0001", "claim": "fixture", "status": "PREREGISTERED"},
        "prev_hash": "GENESIS",
        "record_sha256": "aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111",
        "seq": 0
    });
    let prev = if valid {
        "aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111".to_string()
    } else {
        "broken0000000000000000000000000000000000000000000000000000000000".to_string()
    };
    let r1 = serde_json::json!({
        "kind": "trial",
        "payload": {"hypothesis_id": "H-0001", "result": "ADOPTED", "dev_metrics": {"labels": "STRIPPED"}},
        "prev_hash": prev,
        "record_sha256": "bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222",
        "seq": 1
    });
    format!("{r0}\n{r1}\n")
}
