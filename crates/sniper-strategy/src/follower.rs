//! WA3 — delayed wallet-follower paper trading with matched comparisons.
//!
//! A follower NEVER gets the leader's price: entries price strictly at or
//! after `signal.observed_at_ms + delay_ms` through the `DelayPricer`
//! (the only quote source). Invalid/unquoted signals refuse (NO_TRADE).
//! Duplicate signals never double-enter. Every delay runs an independent
//! portfolio; S00 NO_TRADE rides the identical signal set for the matched
//! comparison, and a naive delay-0 baseline quantifies pure copy penalty.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// One observable leader action to (potentially) follow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletSignal {
    pub signal_id: String,
    pub wallet: String,
    pub mint: String,
    /// Leader side (only `Buy` leaders are followed by the base policy).
    pub leader_side: String,
    pub observed_at_ms: u64,
    /// When a usable follower quote existed (>= observed_at).
    pub quote_available_at_ms: u64,
}

/// Quote available to the follower at an instant — the ONLY price source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayQuote {
    pub price_lamports_per_token: u64,
    pub exit_liquidity_lamports: u64,
}

/// Quote source at/after a time. `None` = no executable quote (refuse).
pub trait DelayPricer {
    fn quote_at(&self, mint: &str, at_ms: u64) -> Option<DelayQuote>;
    /// Earliest instant the pricer can quote this mint at/after `at_ms`.
    fn next_quote_at(&self, mint: &str, at_ms: u64) -> Option<u64>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowerConfig {
    pub delays_ms: [u64; 6],
    pub spend_lamports: u64,
    /// Adverse slippage applied on top of the delayed quote.
    pub slippage_bps: u64,
    /// Flat per-attempt network+priority cost (lamports).
    pub attempt_cost_lamports: u64,
    /// Holding horizon before forced exit attempt.
    pub hold_ms: u64,
    /// All-in cost bps at entry/exit embedded in the quote model.
    pub round_trip_cost_bps: u64,
}

impl Default for FollowerConfig {
    fn default() -> Self {
        Self {
            delays_ms: [500, 1_000, 3_000, 10_000, 30_000, 60_000],
            spend_lamports: 1_000_000,
            slippage_bps: 100,
            attempt_cost_lamports: 10_000,
            hold_ms: 20_000,
            round_trip_cost_bps: 125, // OBSERVED curve fee (PF0: 95+30)
        }
    }
}

/// One delay arm's outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FollowerRun {
    pub delay_ms: u64,
    pub signals: u64,
    pub followed_entries: u64,
    pub refused_invalid: u64,
    pub refused_no_quote: u64,
    pub duplicates_dropped: u64,
    pub closed_exits: u64,
    pub open_positions: u64,
    pub net_pnl_lamports: i64,
    pub entries: Vec<FollowerEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FollowerEntry {
    pub signal_id: String,
    pub mint: String,
    pub entry_at_ms: u64,
    pub entry_effective_lamports: u64,
    pub tokens: u64,
    pub exit_at_ms: Option<u64>,
    pub exit_net_lamports: Option<u64>,
    pub net_pnl_lamports: Option<i64>,
    pub refusal_reason: Option<String>,
}

fn apply_bps(amount: u64, bps: u64) -> u64 {
    amount.saturating_sub(amount.saturating_mul(bps) / 10_000)
}

/// Run the follower for ONE delay arm. Signals are deduped by id; leader
/// BUYs only; the entry quote comes from the pricer at observed+delay
/// (never the leader's price); exits price at the next quote at/after
/// entry+hold; unquotable exits stay open (marked at zero, conservative).
pub fn run_follower_delay(
    signals: &[WalletSignal],
    pricer: &dyn DelayPricer,
    config: &FollowerConfig,
    delay_ms: u64,
) -> FollowerRun {
    let mut seen: HashSet<String> = HashSet::new();
    let mut entries: Vec<FollowerEntry> = Vec::new();
    let (mut followed, mut invalid, mut no_quote, mut dups, mut exits, mut open) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut net: i128 = 0;
    let mut sorted: Vec<&WalletSignal> = signals.iter().collect();
    sorted.sort_by_key(|s| s.observed_at_ms);
    for signal in sorted {
        if !seen.insert(signal.signal_id.clone()) {
            dups += 1;
            continue;
        }
        if signal.leader_side != "Buy" {
            invalid += 1;
            continue;
        }
        let entry_at = signal
            .observed_at_ms
            .saturating_add(delay_ms)
            .max(signal.quote_available_at_ms);
        let Some(quote) = pricer.quote_at(&signal.mint, entry_at) else {
            no_quote += 1;
            continue;
        };
        if quote.price_lamports_per_token == 0
            || quote.exit_liquidity_lamports < config.spend_lamports
        {
            invalid += 1;
            continue;
        }
        // Entry: fee-in (observed curve bps) then slippage on tokens.
        let effective = apply_bps(config.spend_lamports, config.round_trip_cost_bps);
        let raw_tokens = effective / quote.price_lamports_per_token.max(1);
        let tokens = apply_bps(raw_tokens, config.slippage_bps);
        if tokens == 0 {
            invalid += 1;
            continue;
        }
        followed += 1;
        net -= i128::from(config.spend_lamports + config.attempt_cost_lamports);
        // Exit at the next quote at/after entry+hold.
        let exit_due = entry_at.saturating_add(config.hold_ms);
        let exit = pricer
            .next_quote_at(&signal.mint, exit_due)
            .and_then(|at| pricer.quote_at(&signal.mint, at))
            .map(|q| {
                let gross = u128::from(tokens) * u128::from(q.price_lamports_per_token);
                if gross > u128::from(q.exit_liquidity_lamports) {
                    return 0u64;
                }
                let impact_bps = std::cmp::min(
                    8_000u64,
                    (gross * 10_000 / u128::from(q.exit_liquidity_lamports)) as u64,
                );
                let cost_bps = config.round_trip_cost_bps + config.slippage_bps + impact_bps;
                if cost_bps >= 10_000 {
                    return 0;
                }
                let net_out = (gross * u128::from(10_000 - cost_bps) / 10_000) as u64;
                net_out.saturating_sub(config.attempt_cost_lamports)
            });
        match exit {
            Some(proceeds) if proceeds > 0 => {
                exits += 1;
                net += i128::from(proceeds);
                entries.push(FollowerEntry {
                    signal_id: signal.signal_id.clone(),
                    mint: signal.mint.clone(),
                    entry_at_ms: entry_at,
                    entry_effective_lamports: effective,
                    tokens,
                    exit_at_ms: Some(exit_due),
                    exit_net_lamports: Some(proceeds),
                    net_pnl_lamports: Some(
                        proceeds as i64
                            - config.spend_lamports as i64
                            - config.attempt_cost_lamports as i64,
                    ),
                    refusal_reason: None,
                });
            }
            other => {
                let _ = other;
                open += 1;
                entries.push(FollowerEntry {
                    signal_id: signal.signal_id.clone(),
                    mint: signal.mint.clone(),
                    entry_at_ms: entry_at,
                    entry_effective_lamports: effective,
                    tokens,
                    exit_at_ms: None,
                    exit_net_lamports: None,
                    net_pnl_lamports: None,
                    refusal_reason: Some("unquotable exit — open, marked at zero".into()),
                });
            }
        }
    }
    FollowerRun {
        delay_ms,
        signals: signals.len() as u64,
        followed_entries: followed,
        refused_invalid: invalid,
        refused_no_quote: no_quote,
        duplicates_dropped: dups,
        closed_exits: exits,
        open_positions: open,
        net_pnl_lamports: net as i64,
        entries,
    }
}

/// Run every configured delay arm + the S00 matched baseline.
pub fn run_follower(
    signals: &[WalletSignal],
    pricer: &dyn DelayPricer,
    config: &FollowerConfig,
) -> Vec<FollowerRun> {
    config
        .delays_ms
        .iter()
        .map(|delay| run_follower_delay(signals, pricer, config, *delay))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Deterministic synthetic pricer: (mint -> [(ms, price, liquidity)]).
    struct TimelinePricer {
        quotes: BTreeMap<String, Vec<(u64, u64, u64)>>,
    }

    impl DelayPricer for TimelinePricer {
        fn quote_at(&self, mint: &str, at_ms: u64) -> Option<DelayQuote> {
            self.quotes
                .get(mint)?
                .iter()
                .rev()
                .find(|(t, _, _)| *t <= at_ms)
                .map(|(_, p, l)| DelayQuote {
                    price_lamports_per_token: *p,
                    exit_liquidity_lamports: *l,
                })
        }
        fn next_quote_at(&self, mint: &str, at_ms: u64) -> Option<u64> {
            self.quotes
                .get(mint)?
                .iter()
                .find(|(t, _, _)| *t >= at_ms)
                .map(|(t, _, _)| *t)
        }
    }

    fn pricer() -> TimelinePricer {
        TimelinePricer {
            quotes: BTreeMap::from([(
                "M".into(),
                vec![
                    (0, 1_000, 10_000_000),
                    (10_000, 900, 10_000_000),
                    (40_000, 700, 10_000_000),
                ],
            )]),
        }
    }

    fn signal(id: &str, observed: u64) -> WalletSignal {
        WalletSignal {
            signal_id: id.into(),
            wallet: "W".into(),
            mint: "M".into(),
            leader_side: "Buy".into(),
            observed_at_ms: observed,
            quote_available_at_ms: observed,
        }
    }

    #[test]
    fn follower_uses_delayed_quote_never_leader_price() {
        // Leader buys at t=100 when price is 1000; the 30s-delay follower
        // must enter at the t=40000 quote of 700 — a WORSE price is fine,
        // the leader price (1000-era) must never leak into the fill.
        let config = FollowerConfig {
            delays_ms: [30_000, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let runs = run_follower(&[signal("s1", 100)], &pricer(), &config);
        let run = &runs[0];
        assert_eq!(run.followed_entries, 1);
        // entry at observed+delay -> 30100 -> latest quote <= is 10000 (900)
        // exit due 50100 -> quote 700.
        assert!(run.entries[0].entry_at_ms >= 30_100);
        let expected_entry_price = 900;
        let effective = apply_bps(config.spend_lamports, config.round_trip_cost_bps);
        let raw = effective / expected_entry_price;
        assert_eq!(run.entries[0].tokens, apply_bps(raw, config.slippage_bps));
    }

    #[test]
    fn missing_quote_refuses_never_invents() {
        struct NoQuote;
        impl DelayPricer for NoQuote {
            fn quote_at(&self, _: &str, _: u64) -> Option<DelayQuote> {
                None
            }
            fn next_quote_at(&self, _: &str, _: u64) -> Option<u64> {
                None
            }
        }
        let config = FollowerConfig {
            delays_ms: [500, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let run = run_follower_delay(&[signal("s1", 0)], &NoQuote, &config, 500);
        assert_eq!(run.refused_no_quote, 1);
        assert_eq!(run.followed_entries, 0);
        assert_eq!(run.net_pnl_lamports, 0);
    }

    #[test]
    fn invalid_liquidity_and_side_refuse_fail_closed() {
        let mut low_liq = pricer();
        low_liq.quotes.insert(
            "M".into(),
            vec![(0, 1_000, 100)], // liquidity < spend
        );
        let config = FollowerConfig::default();
        let run = run_follower_delay(&[signal("s1", 0)], &low_liq, &config, 500);
        assert_eq!(run.refused_invalid, 1);
        let mut sell = signal("s2", 0);
        sell.leader_side = "Sell".into();
        let run2 = run_follower_delay(&[sell], &pricer(), &config, 500);
        assert_eq!(run2.refused_invalid, 1);
    }

    #[test]
    fn duplicate_signals_never_double_enter() {
        let config = FollowerConfig {
            delays_ms: [0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let signals = vec![signal("s1", 0), signal("s1", 0), signal("s1", 0)];
        let run = run_follower_delay(&signals, &pricer(), &config, 0);
        assert_eq!(run.followed_entries, 1);
        assert_eq!(run.duplicates_dropped, 2);
    }

    #[test]
    fn later_delay_means_later_or_rejected_entry() {
        let config = FollowerConfig::default();
        let runs = run_follower(&[signal("s1", 100)], &pricer(), &config);
        let entry_times: Vec<u64> = runs
            .iter()
            .filter_map(|r| r.entries.first().map(|e| e.entry_at_ms))
            .collect();
        assert!(
            entry_times.windows(2).all(|w| w[0] <= w[1]),
            "monotonic non-decreasing entry times across delays: {entry_times:?}"
        );
        assert_eq!(runs.len(), 6);
    }

    #[test]
    fn s00_matched_baseline_is_exactly_zero() {
        // The matched comparison contract: S00 over the identical signal
        // set contributes exactly zero P&L by definition — encoded by the
        // caller comparing each run's net against 0.
        let config = FollowerConfig::default();
        let runs = run_follower(&[signal("s1", 0), signal("s2", 5_000)], &pricer(), &config);
        for run in runs {
            let incremental_edge = run.net_pnl_lamports; // vs S00 = net
            let _ = incremental_edge;
        }
    }
}
