//! Full-portfolio evaluator (H7-4): metrics beyond closed trades, computed
//! over the engine trace (equity curve, per-phase fills/exits) with explicit
//! sampling units and safe handling of insufficient samples.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::engine::legacy::LegacyResult;
use crate::observation::Phase;

/// Execution trace collected beside the pinned `LegacyResult` (parity fields
/// stay untouched; the trace carries what evaluation needs).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineTrace {
    /// Full-NAV equity after every event.
    pub equity_curve: Vec<u64>,
    /// Events where at least one position was open.
    pub ticks_in_market: u64,
    /// Open position count after every event tick.
    pub open_count_curve: Vec<u64>,
    /// Entry phase per fill (phase of the entry event).
    pub fill_phases: Vec<Phase>,
    /// Exit phase + realized net per closed trade.
    pub exit_phases: Vec<(Phase, i64)>,
}

/// Metric bundle. Sampling units are declared per metric; unavailable
/// metrics are `None` with a reason, never zero-filled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    /// Net portfolio P&L after all modeled costs (lamports).
    pub net_pnl_lamports: i64,
    /// Max peak-to-trough drawdown of the FULL NAV curve.
    pub max_drawdown_lamports: u64,
    /// CVaR/expected shortfall of per-trade net P&L at 95% (sampling unit:
    /// CLOSED TRADE; None when fewer than 20 closed trades).
    pub cvar_95_per_trade: Option<i64>,
    /// Gross profit / gross loss over closed trades (None when no losses:
    /// infinite is NOT reported as a number).
    pub profit_factor: Option<f64>,
    /// Median per-trade net P&L (sampling unit: closed trade; None if none).
    pub median_trade_pnl: Option<i64>,
    /// Fraction of event ticks with at least one open position.
    pub time_in_market: f64,
    /// Max simultaneously open positions.
    pub max_concurrent_positions: u64,
    /// Filled entries / attempted entries (None when no attempts).
    pub fill_rate: Option<f64>,
    /// Rejected entries / attempted entries.
    pub rejection_rate: Option<f64>,
    /// Closed trades / filled entries (None when no fills).
    pub successful_exit_rate: Option<f64>,
    /// Positions still open at the end.
    pub open_positions: u64,
    /// Exit attempts that could not quote a positive value.
    pub unexitable_exits: u64,
    /// Realized net P&L by exit lifecycle phase.
    pub pnl_by_exit_phase: HashMap<String, i64>,
    /// Entries by entry lifecycle phase.
    pub fills_by_entry_phase: HashMap<String, u64>,
    /// Share of total absolute realized P&L from the single largest trade
    /// (None when no closed trades).
    pub largest_trade_dominance: Option<f64>,
    /// vs S00 NO_TRADE: net edge (positive only if the strategy beat doing
    /// nothing after costs).
    pub net_edge_vs_no_trade_lamports: i64,
}

/// Safe median over an even/odd slice.
fn median(sorted: &[i64]) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let mid = sorted.len() / 2;
    Some(if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2
    })
}

/// CVaR at `alpha` (0.95): mean of the worst (1-alpha) tail. Requires a
/// minimum sample; smaller samples return None (declared insufficient).
pub fn cvar_95(samples: &[i64], min_samples: usize) -> Option<i64> {
    if samples.len() < min_samples {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let tail_len = ((sorted.len() as f64) * 0.05).ceil() as usize;
    let tail_len = tail_len.max(1);
    let tail = &sorted[..tail_len.min(sorted.len())];
    Some(tail.iter().sum::<i64>() / tail.len() as i64)
}

/// Evaluate one strategy result against the S00 baseline.
pub fn evaluate(result: &LegacyResult, trace: &EngineTrace, no_trade: &LegacyResult) -> Metrics {
    let trade_pnls: Vec<i64> = result.trades.iter().map(|t| t.net_pnl).collect();
    let gross_profit: i64 = trade_pnls.iter().filter(|p| **p > 0).sum();
    let gross_loss: i64 = trade_pnls.iter().filter(|p| **p < 0).sum::<i64>().abs();

    let mut pnl_by_phase: HashMap<String, i64> = HashMap::new();
    for (phase, net) in &trace.exit_phases {
        *pnl_by_phase.entry(phase_name(phase)).or_insert(0) += net;
    }
    let mut fills_by_phase: HashMap<String, u64> = HashMap::new();
    for phase in &trace.fill_phases {
        *fills_by_phase.entry(phase_name(phase)).or_insert(0) += 1;
    }

    let max_concurrent = trace.open_count_curve.iter().copied().max().unwrap_or(0);

    Metrics {
        net_pnl_lamports: result.net_pnl_lamports,
        max_drawdown_lamports: result.max_drawdown_lamports,
        cvar_95_per_trade: cvar_95(&trade_pnls, 20),
        // No losses: profit factor is infinite; report None rather than a
        // fabricated number (and None when there is no profit either).
        profit_factor: if gross_loss == 0 {
            None
        } else {
            Some(gross_profit as f64 / gross_loss as f64)
        },
        median_trade_pnl: {
            let mut sorted = trade_pnls.clone();
            sorted.sort_unstable();
            median(&sorted)
        },
        time_in_market: if trace.equity_curve.is_empty() {
            0.0
        } else {
            trace.ticks_in_market as f64 / trace.equity_curve.len() as f64
        },
        max_concurrent_positions: max_concurrent.max(result.open_positions),
        fill_rate: if result.attempted_entries == 0 {
            None
        } else {
            Some(result.filled_entries as f64 / result.attempted_entries as f64)
        },
        rejection_rate: if result.attempted_entries == 0 {
            None
        } else {
            Some(result.rejected_entries as f64 / result.attempted_entries as f64)
        },
        successful_exit_rate: if result.filled_entries == 0 {
            None
        } else {
            Some(result.closed_trades as f64 / result.filled_entries as f64)
        },
        open_positions: result.open_positions,
        unexitable_exits: result.failed_exit_quotes,
        pnl_by_exit_phase: pnl_by_phase,
        fills_by_entry_phase: fills_by_phase,
        largest_trade_dominance: if trade_pnls.is_empty() {
            None
        } else {
            let total_abs: i64 = trade_pnls.iter().map(|p| p.abs()).sum();
            if total_abs == 0 {
                Some(0.0)
            } else {
                Some(
                    trade_pnls.iter().map(|p| p.abs()).max().unwrap_or(0) as f64 / total_abs as f64,
                )
            }
        },
        net_edge_vs_no_trade_lamports: result.net_pnl_lamports - no_trade.net_pnl_lamports,
    }
}

fn phase_name(phase: &Phase) -> String {
    serde_json::to_value(phase)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{phase:?}"))
}

/// Aggregate comparison table vs S00 for a whole batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchEvaluation {
    pub schema_version: u16,
    pub evidence_class: &'static str,
    pub promotable: bool,
    pub live_trading_enabled: bool,
    pub verdict: &'static str,
    pub metrics: Vec<(String, Metrics)>,
    pub limitations: Vec<&'static str>,
}

/// Shared model limitations carried onto every receipt.
pub const MODEL_LIMITATIONS: &[&str] = &[
    "synthetic or externally supplied observations, not a verified complete mainnet cohort",
    "Engine A is the frozen linear-price surrogate; protocol economics arrive with Engine B on real snapshots",
    "CVaR/profit factor sampling unit is the closed trade; open positions are marked conservatively, not sampled",
    "no out-of-sample evaluation, uncertainty intervals, or production promotion",
];

/// Build the batch evaluation for an 11-result run (S00 first).
pub fn evaluate_batch(results: &[LegacyResult], traces: &[EngineTrace]) -> BatchEvaluation {
    let no_trade = &results[0];
    let metrics = results
        .iter()
        .zip(traces)
        .map(|(result, trace)| (result.strategy.clone(), evaluate(result, trace, no_trade)))
        .collect();
    BatchEvaluation {
        schema_version: 1,
        evidence_class: crate::engine::legacy::EVIDENCE_CLASS,
        promotable: false,
        live_trading_enabled: false,
        verdict: "EDGE_NOT_DEMONSTRATED",
        metrics,
        limitations: MODEL_LIMITATIONS.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvar_requires_minimum_sample_and_averages_the_tail() {
        // 25 trades: worst 5% tail (ceil(1.25)=2 worst) averaged.
        let samples: Vec<i64> = (0..25).map(|i| i as i64 * 10 - 100).collect();
        let cvar = cvar_95(&samples, 20).unwrap();
        let mut sorted = samples.clone();
        sorted.sort_unstable();
        let expected = (sorted[0] + sorted[1]) / 2;
        assert_eq!(cvar, expected);
        assert!(
            cvar_95(&samples, 40).is_none(),
            "insufficient sample -> None"
        );
    }

    #[test]
    fn profit_factor_handles_zero_denominator_safely() {
        // Encoded through evaluate(): no losing trades -> None, not inf.
        let result = LegacyResult {
            strategy: "S09".into(),
            name: "post_migration".into(),
            starting_cash_lamports: 100,
            ending_cash_lamports: 110,
            ending_equity_lamports: 110,
            net_pnl_lamports: 10,
            max_drawdown_lamports: 0,
            attempted_entries: 1,
            filled_entries: 1,
            rejected_entries: 0,
            closed_trades: 1,
            open_positions: 0,
            failed_exit_quotes: 0,
            closed_net_pnl_lamports: 10,
            trades: vec![crate::engine::legacy::LegacyTrade {
                mint: "M".into(),
                entry_cost: 100,
                exit_value: 110,
                net_pnl: 10,
                entry_at_ms: 0,
                exit_at_ms: 1,
            }],
        };
        let no_trade = crate::engine::legacy::run_registry(
            &[],
            crate::StrategyKnobs::default(),
            100,
            &crate::RiskConstraints::default(),
            &crate::engine::legacy::LegacyParams::default(),
        )
        .remove(0);
        let trace = EngineTrace {
            equity_curve: vec![100, 110],
            ticks_in_market: 1,
            open_count_curve: vec![0, 0],
            fill_phases: vec![Phase::ActiveCurve],
            exit_phases: vec![(Phase::PumpswapOpen, 10)],
        };
        let metrics = evaluate(&result, &trace, &no_trade);
        assert_eq!(metrics.profit_factor, None);
        assert_eq!(metrics.median_trade_pnl, Some(10));
        assert_eq!(metrics.net_edge_vs_no_trade_lamports, 10);
        assert_eq!(metrics.pnl_by_exit_phase.get("PUMPSWAP_OPEN"), Some(&10));
    }
}
