//! Engine A — `LegacySurrogateV1`.
//!
//! BYTE-EXACT reproduction of `harness/research_batch.py::simulate` (frozen
//! surrogate): same integer arithmetic, same loop ordering (state tracking,
//! exit-before-entry, elif semantics), same conservative single-tick exit
//! quote, same result fields. Differential parity is pinned against frozen
//! Python outputs in `tests/legacy_parity.rs`.
//!
//! Evidence class is permanently `SYNTHETIC_SURROGATE_UNCALIBRATED`:
//! linear price approximation, no landing model, no reserve math. It exists
//! so the migration to `ProtocolEconomicV1` can be diffed field-by-field —
//! never as promotion evidence.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::exit::ExitPolicy;
use crate::observation::{Observation, Safety};
use crate::{Intent, PortfolioView, RiskConstraints, Strategy};

pub const EVIDENCE_CLASS: &str = "SYNTHETIC_SURROGATE_UNCALIBRATED";

/// Frozen surrogate parameters (Python `run_batch` defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyParams {
    pub slip_bps: u64,
    pub tx_cost_lamports: u64,
    pub ata_rent_lamports: u64,
    pub max_hold_ms: u64,
}

impl Default for LegacyParams {
    fn default() -> Self {
        Self {
            slip_bps: 100,
            tx_cost_lamports: 10_000,
            ata_rent_lamports: 1_500_000,
            max_hold_ms: 20_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenPosition {
    tokens: u64,
    entry_at_ms: u64,
    entry_cost: u64,
}

/// One closed round trip (Python `trades` entry, identical keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyTrade {
    pub mint: String,
    pub entry_cost: u64,
    pub exit_value: u64,
    pub net_pnl: i64,
    pub entry_at_ms: u64,
    pub exit_at_ms: u64,
}

/// Per-strategy result (Python result dict, identical keys/order-free).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyResult {
    pub strategy: String,
    pub name: String,
    pub starting_cash_lamports: u64,
    pub ending_cash_lamports: u64,
    pub ending_equity_lamports: u64,
    pub net_pnl_lamports: i64,
    pub max_drawdown_lamports: u64,
    pub attempted_entries: u64,
    pub filled_entries: u64,
    pub rejected_entries: u64,
    pub closed_trades: u64,
    pub open_positions: u64,
    pub failed_exit_quotes: u64,
    pub closed_net_pnl_lamports: i64,
    pub trades: Vec<LegacyTrade>,
}

/// Conservative single-tick exit quote (Python `executable_exit` exactly).
/// Unknown/unsafe observations and liquidity-breach quote ZERO — an
/// unexitable position is marked worthless, never optimistically valued.
pub fn executable_exit(
    tokens: u64,
    event: Option<&Observation>,
    slip_bps: u64,
    tx_cost: u64,
) -> u64 {
    let Some(event) = event else {
        return 0;
    };
    if !event.coverage_ok || event.safety == Safety::Block {
        return 0;
    }
    let gross = tokens as u128 * event.price_lamports_per_token as u128;
    let liquidity = event.exit_liquidity_lamports as u128;
    if gross > liquidity || liquidity == 0 {
        return 0;
    }
    let impact_bps = std::cmp::min(8_000, (gross * 10_000 / liquidity) as u64);
    let cost_bps = event.fee_bps as u128 + slip_bps as u128 + impact_bps as u128;
    if cost_bps >= 10_000 {
        return 0;
    }
    let value = gross * (10_000 - cost_bps) / 10_000;
    (value as u64).saturating_sub(tx_cost)
}

/// Run one strategy over the observation stream under frozen surrogate
/// economics. Mirrors the Python loop exactly:
/// 1. per-mint state updates live inside the strategy (`decide` per event),
/// 2. exit handling BEFORE entry, and an exit event never re-enters same
///    event (Python `elif`),
/// 3. entry fill gated on cash, positive tokens, and exit-liquidity floor,
/// 4. equity marked at the conservative exit quote of every open position.
pub fn run(
    events: &[Observation],
    strategy: &mut dyn Strategy,
    cash: u64,
    risk: &RiskConstraints,
    params: &LegacyParams,
) -> LegacyResult {
    let exit_policy = ExitPolicy {
        max_hold_ms: params.max_hold_ms,
    };
    let mut balance = cash;
    let mut positions: HashMap<String, OpenPosition> = HashMap::new();
    let mut seen_accounts: HashSet<String> = HashSet::new();
    let mut latest: HashMap<String, Observation> = HashMap::new();
    let mut trades: Vec<LegacyTrade> = Vec::new();
    let mut equity_curve: Vec<u64> = Vec::with_capacity(events.len());
    let (mut attempts, mut fills, mut rejected, mut exits, mut no_exit) =
        (0u64, 0u64, 0u64, 0u64, 0u64);

    for event in events {
        let mint = event.mint.clone();
        let held = positions.contains_key(&mint);
        if held {
            let position = positions.get(&mint).expect("held");
            if exit_policy.due(position.entry_at_ms, event) {
                let value = executable_exit(
                    position.tokens,
                    Some(event),
                    params.slip_bps,
                    params.tx_cost_lamports,
                );
                if value > 0 {
                    balance += value;
                    trades.push(LegacyTrade {
                        mint: mint.clone(),
                        entry_cost: position.entry_cost,
                        exit_value: value,
                        net_pnl: value as i64 - position.entry_cost as i64,
                        entry_at_ms: position.entry_at_ms,
                        exit_at_ms: event.feature_available_at_ms,
                    });
                    positions.remove(&mint);
                    exits += 1;
                } else {
                    no_exit += 1;
                }
            }
        }
        latest.insert(mint.clone(), event.clone());
        let view = PortfolioView {
            cash_lamports: balance,
            open_mints: positions.keys().cloned().collect(),
        };
        // Python `elif`: entry is only considered when the mint was NOT held
        // at the start of this event (a same-event exit cannot re-enter).
        if let Intent::Buy { spend_lamports } = strategy.decide(event, &view, risk) {
            if !held {
                attempts += 1;
                let rent = if seen_accounts.contains(&mint) {
                    0
                } else {
                    params.ata_rent_lamports
                };
                let total_cost = spend_lamports + params.tx_cost_lamports + rent;
                let cost_bps = event.fee_bps + params.slip_bps;
                // Python: tokens = spend * max(0, 10000 - cost_bps) // 10000
                //          // price  (two successive floor divisions).
                let discounted = (spend_lamports as u128)
                    * (10_000u64.saturating_sub(cost_bps) as u128)
                    / 10_000;
                let tokens = (discounted / event.price_lamports_per_token.max(1) as u128) as u64;
                if total_cost <= balance
                    && tokens > 0
                    && event.exit_liquidity_lamports >= spend_lamports
                {
                    balance -= total_cost;
                    positions.insert(
                        mint.clone(),
                        OpenPosition {
                            tokens,
                            entry_at_ms: event.feature_available_at_ms,
                            entry_cost: total_cost,
                        },
                    );
                    seen_accounts.insert(mint.clone());
                    fills += 1;
                } else {
                    rejected += 1;
                }
            }
        }
        let equity = balance
            + positions
                .iter()
                .map(|(open_mint, position)| {
                    executable_exit(
                        position.tokens,
                        latest.get(open_mint),
                        params.slip_bps,
                        params.tx_cost_lamports,
                    )
                })
                .sum::<u64>();
        equity_curve.push(equity);
    }

    let final_equity = equity_curve.last().copied().unwrap_or(cash);
    let mut peak = cash;
    let mut max_drawdown = 0u64;
    for equity in &equity_curve {
        peak = peak.max(*equity);
        max_drawdown = max_drawdown.max(peak - equity);
    }
    LegacyResult {
        strategy: strategy.id().as_str().to_string(),
        name: strategy.name().to_string(),
        starting_cash_lamports: cash,
        ending_cash_lamports: balance,
        ending_equity_lamports: final_equity,
        net_pnl_lamports: final_equity as i64 - cash as i64,
        max_drawdown_lamports: max_drawdown,
        attempted_entries: attempts,
        filled_entries: fills,
        rejected_entries: rejected,
        closed_trades: exits,
        open_positions: positions.len() as u64,
        failed_exit_quotes: no_exit,
        closed_net_pnl_lamports: trades.iter().map(|t| t.net_pnl).sum(),
        trades,
    }
}

/// Convenience: run the full S00..S10 registry with identical inputs.
pub fn run_registry(
    events: &[Observation],
    knobs: crate::StrategyKnobs,
    cash: u64,
    risk: &RiskConstraints,
    params: &LegacyParams,
) -> Vec<LegacyResult> {
    crate::registry(knobs)
        .into_iter()
        .map(|mut strategy| run(events, strategy.as_mut(), cash, risk, params))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policies::EntryPolicy;

    fn event(observed_at_ms: u64, price: u64, liquidity: u64) -> Observation {
        Observation {
            event_id: format!("e{observed_at_ms}"),
            mint: "M".into(),
            slot: 1,
            chain_at_ms: observed_at_ms - 5,
            observed_at_ms,
            feature_available_at_ms: observed_at_ms + 10,
            phase: crate::observation::Phase::ActiveCurve,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: price,
            exit_liquidity_lamports: liquidity,
            net_flow_lamports: 1,
            unique_buyers: 10,
            creator_sell_lamports: 0,
            curve_progress_bps: 1_000,
            fee_bps: 100,
        }
    }

    #[test]
    fn s00_never_touches_the_portfolio() {
        let events = vec![
            event(100, 1_000, 10_000_000),
            event(20_000, 2_000, 10_000_000),
        ];
        let risk = RiskConstraints::default();
        let result = run(
            &events,
            &mut EntryPolicy::s00(crate::StrategyKnobs::default()),
            100_000_000,
            &risk,
            &LegacyParams::default(),
        );
        assert_eq!(result.net_pnl_lamports, 0);
        assert_eq!(result.ending_cash_lamports, 100_000_000);
        assert_eq!(result.attempted_entries, 0);
    }

    #[test]
    fn unexitable_position_marked_worthless_not_dropped() {
        // Enter at t=100, then liquidity collapses: the exit quote is zero,
        // the position stays open, and equity marks it at zero.
        let events = vec![
            event(100, 1_000, 50_000_000),
            event(25_000, 900, 0), // past max hold, zero exit liquidity
        ];
        let risk = RiskConstraints::default();
        let result = run(
            &events,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            100_000_000,
            &risk,
            &LegacyParams::default(),
        );
        assert_eq!(result.filled_entries, 1);
        assert_eq!(result.closed_trades, 0);
        assert_eq!(result.open_positions, 1);
        assert_eq!(result.failed_exit_quotes, 1);
        // Full entry cost shows up as drawdown; equity = cash after entry.
        assert_eq!(result.ending_equity_lamports, 100_000_000 - 2_510_000);
    }

    #[test]
    fn same_event_exit_never_reenters() {
        // S01 wants to enter on every eligible event; after the exit at
        // event 2 the Python `elif` forbids a same-event re-entry.
        let events = vec![
            event(100, 1_000, 50_000_000),
            event(25_000, 5_000, 50_000_000), // exit due (max hold), profitable
        ];
        let risk = RiskConstraints::default();
        let result = run(
            &events,
            &mut EntryPolicy::s01(crate::StrategyKnobs::default()),
            100_000_000,
            &risk,
            &LegacyParams::default(),
        );
        assert_eq!(result.attempted_entries, 1);
        assert_eq!(result.closed_trades, 1);
        assert_eq!(result.open_positions, 0);
    }

    #[test]
    fn deterministic_rerun_is_identical() {
        let events = vec![
            event(100, 1_000, 50_000_000),
            event(25_000, 5_000, 50_000_000),
            event(30_000, 4_000, 50_000_000),
        ];
        let risk = RiskConstraints::default();
        let a = run(
            &events,
            &mut EntryPolicy::s05(crate::StrategyKnobs::default()),
            100_000_000,
            &risk,
            &LegacyParams::default(),
        );
        let b = run(
            &events,
            &mut EntryPolicy::s05(crate::StrategyKnobs::default()),
            100_000_000,
            &risk,
            &LegacyParams::default(),
        );
        assert_eq!(a, b);
    }
}
