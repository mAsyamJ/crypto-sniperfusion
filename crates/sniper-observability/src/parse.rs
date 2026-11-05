//! Shape-aware trade-record parsers (T-SHAPE1). SniperFusion emits THREE
//! different `trades.jsonl` shapes by subcommand family (D-3/D-4):
//!
//! - research: JSONL rows `{"strategy", "record": <ExecutionRecord>}` where
//!   fills carry `slot` but NO wall-clock timestamp;
//! - wallet-follow: a pretty JSON ARRAY of `FollowerRun` whose entries carry
//!   `entry_at_ms` / `exit_at_ms` (the calendar anchor);
//! - anything else (wallet-pnl observed rows): generic passthrough rows.
//!
//! The parser sniffs; it never assumes JSONL.

use serde_json::Value;
use sniper_strategy::follower::FollowerRun;

use crate::VerifyError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeKind {
    BuyFill,
    SellFill,
    Rejected,
    FailedLanding,
    ExitBlocked,
    FollowerEntry,
    ObservedTrade,
}

impl TradeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeKind::BuyFill => "BUY_FILL",
            TradeKind::SellFill => "SELL_FILL",
            TradeKind::Rejected => "REJECTED",
            TradeKind::FailedLanding => "FAILED_LANDING",
            TradeKind::ExitBlocked => "EXIT_BLOCKED",
            TradeKind::FollowerEntry => "FOLLOWER_ENTRY",
            TradeKind::ObservedTrade => "OBSERVED_TRADE",
        }
    }
}

/// Normalized trade row stored in the projection. `net_pnl_lamports` is
/// ONLY set where the producer emits a per-record net (follower family);
/// research per-record nets are NOT emitted and are never computed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRow {
    pub ord: u32,
    pub strategy: String,
    pub kind: TradeKind,
    pub mint: String,
    pub slot: Option<u64>,
    /// Producer wall-clock anchor (follower `exit_at_ms`; refused follower
    /// entries use `entry_at_ms`).
    pub at_ms: Option<u64>,
    pub net_pnl_lamports: Option<i64>,
    pub open: bool,
    pub raw: String,
}

/// Sniff and parse a `trades.jsonl` payload for the given engine family.
pub fn parse_trades(engine: &str, path: &str, raw: &str) -> Result<Vec<TradeRow>, VerifyError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new()); // wallet-* emits empty files by design (D-4)
    }
    if engine == "wallet_follower_v1" {
        return parse_follower(path, trimmed);
    }
    if engine == "wallet_observer_v1" {
        // wallet-pnl observed rows (empty files are common — D-4).
        return parse_observed(path, trimmed);
    }
    if trimmed.starts_with('[') {
        // A JSON array under a research engine is unexpected: fail closed.
        return Err(VerifyError::Unsupported {
            path: path.to_string(),
            reason: format!("engine {engine} produced a JSON-array trades file"),
        });
    }
    parse_research(path, trimmed)
}

/// Follower family: pretty JSON array of `FollowerRun` (serde structs from
/// the accounting crate — the producer's own types).
fn parse_follower(path: &str, raw: &str) -> Result<Vec<TradeRow>, VerifyError> {
    let runs: Vec<FollowerRun> = serde_json::from_str(raw).map_err(|e| VerifyError::Parse {
        path: path.to_string(),
        reason: format!("follower trades array failed to parse: {e}"),
    })?;
    let mut rows = Vec::new();
    for run in &runs {
        let strategy = format!("FOLLOW_DELAY_{}MS", run.delay_ms);
        for entry in &run.entries {
            let (kind, at_ms, net, open) = if entry.refusal_reason.is_some() {
                (TradeKind::Rejected, Some(entry.entry_at_ms), None, false)
            } else if entry.exit_at_ms.is_some() {
                (
                    TradeKind::FollowerEntry,
                    entry.exit_at_ms,
                    entry.net_pnl_lamports,
                    false,
                )
            } else {
                (TradeKind::FollowerEntry, None, None, true)
            };
            rows.push(TradeRow {
                ord: rows.len() as u32,
                strategy: strategy.clone(),
                kind,
                mint: entry.mint.clone(),
                slot: None,
                at_ms,
                net_pnl_lamports: net,
                open,
                raw: serde_json::to_string(entry).unwrap_or_default(),
            });
        }
    }
    Ok(rows)
}

/// Research family: JSONL `{"strategy", "record": {"<Variant>": {…}}}`.
/// Externally tagged `ExecutionRecord`; only fills carry `slot`; the
/// rejected/failed/exit-blocked variants carry none (audited fact).
fn parse_research(path: &str, raw: &str) -> Result<Vec<TradeRow>, VerifyError> {
    let mut rows = Vec::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).map_err(|e| VerifyError::Parse {
            path: path.to_string(),
            reason: format!("research trades line failed to parse: {e}"),
        })?;
        let strategy = v
            .get("strategy")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let Some(record) = v.get("record").and_then(|r| r.as_object()) else {
            return Err(VerifyError::Parse {
                path: path.to_string(),
                reason: "research row lacks `record` object".into(),
            });
        };
        let (variant, inner) = record.iter().next().ok_or_else(|| VerifyError::Parse {
            path: path.to_string(),
            reason: "research record has no variant".into(),
        })?;
        let kind = match variant.as_str() {
            "BuyFill" => TradeKind::BuyFill,
            "SellFill" => TradeKind::SellFill,
            "AttemptRejected" => TradeKind::Rejected,
            "AttemptFailedLanding" => TradeKind::FailedLanding,
            "ExitBlocked" => TradeKind::ExitBlocked,
            other => {
                return Err(VerifyError::Unsupported {
                    path: path.to_string(),
                    reason: format!("unknown ExecutionRecord variant {other}"),
                })
            }
        };
        rows.push(TradeRow {
            ord: rows.len() as u32,
            strategy,
            kind,
            mint: inner
                .get("mint")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string(),
            slot: inner.get("slot").and_then(|s| s.as_u64()),
            at_ms: None, // research fills carry slots only (audited)
            net_pnl_lamports: None,
            open: false,
            raw: line.to_string(),
        });
    }
    Ok(rows)
}

/// Generic observed rows (wallet-pnl engine): mint/slot passthrough when
/// present; never any computed value.
pub fn parse_observed(path: &str, raw: &str) -> Result<Vec<TradeRow>, VerifyError> {
    let mut rows = Vec::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Err(VerifyError::Parse {
                    path: path.to_string(),
                    reason: format!("observed trades line failed to parse: {e}"),
                })
            }
        };
        rows.push(TradeRow {
            ord: rows.len() as u32,
            strategy: v
                .get("strategy")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            kind: TradeKind::ObservedTrade,
            mint: v
                .get("mint")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string(),
            slot: v.get("slot").and_then(|s| s.as_u64()),
            at_ms: v.get("exit_at_ms").and_then(|t| t.as_u64()),
            net_pnl_lamports: v.get("net_pnl_lamports").and_then(|n| n.as_i64()),
            open: false,
            raw: line.to_string(),
        });
    }
    Ok(rows)
}
