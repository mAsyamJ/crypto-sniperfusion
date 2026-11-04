//! S00 NO_TRADE baseline + S01..S10 entry policies.
//!
//! Eligibility mirrors `harness/research_batch.py::eligible` exactly
//! (same predicates, same field usage, same defaults: min_buyers=5,
//! liquidity_multiple=20). Per-mint state tracking (first observation,
//! running peak, previous price, seen-dip flag) mirrors the Python loop
//! ordering: seen_dip updates BEFORE eligibility; peak/previous_price update
//! AFTER — so eligibility always sees the previous event's price state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::observation::{Observation, Phase, Safety};
use crate::{Intent, PortfolioView, RiskConstraints, Strategy, StrategyId};

/// Preregistered policy knobs (validated bounds mirror the Python harness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyKnobs {
    pub min_buyers: u64,
    pub liquidity_multiple: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("knobs out of bounds: min_buyers {min_buyers} (1..=30), liquidity_multiple {liquidity_multiple} (1..=100)")]
pub struct InvalidKnobs {
    pub min_buyers: u64,
    pub liquidity_multiple: u64,
}

impl Default for StrategyKnobs {
    fn default() -> Self {
        Self {
            min_buyers: 5,
            liquidity_multiple: 20,
        }
    }
}

impl StrategyKnobs {
    pub fn new(min_buyers: u64, liquidity_multiple: u64) -> Result<Self, InvalidKnobs> {
        if !(1..=30).contains(&min_buyers) || !(1..=100).contains(&liquidity_multiple) {
            return Err(InvalidKnobs {
                min_buyers,
                liquidity_multiple,
            });
        }
        Ok(Self {
            min_buyers,
            liquidity_multiple,
        })
    }
}

/// Per-mint tracking state (mirrors the Python surrogate loop exactly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MintTrack {
    first_observed_ms: u64,
    peak: u64,
    previous_price: u64,
    seen_dip: bool,
}

impl MintTrack {
    fn new(obs: &Observation) -> Self {
        Self {
            first_observed_ms: obs.observed_at_ms,
            peak: obs.price_lamports_per_token,
            previous_price: obs.price_lamports_per_token,
            seen_dip: false,
        }
    }
}

/// The entry rule of one registered policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    NoTrade,
    Immediate,
    DelayMs(u64),
    OrganicFlow,
    DipRebound,
    FlowCreatorVeto,
    CurveRegime,
    PreGraduation,
    PostMigration,
    LiquidityFirst,
    /// Base eligibility (coverage+ELIGIBLE+tradeable) + deterministic
    /// 20% pseudo-random selection by FNV(event_id) — a chance baseline
    /// for the matched comparison contract.
    RandomEligible,
}

/// One registered strategy: identity + rule + knobs + per-mint tracking.
pub struct EntryPolicy {
    id: StrategyId,
    name: &'static str,
    rule: Rule,
    knobs: StrategyKnobs,
    tracks: HashMap<String, MintTrack>,
}

macro_rules! policy_ctor {
    ($ctor:ident, $id:expr, $name:expr, $rule:expr) => {
        pub fn $ctor(knobs: StrategyKnobs) -> Self {
            Self {
                id: $id,
                name: $name,
                rule: $rule,
                knobs,
                tracks: HashMap::new(),
            }
        }
    };
}

impl EntryPolicy {
    policy_ctor!(s00, StrategyId::S00, "no_trade", Rule::NoTrade);
    policy_ctor!(s01, StrategyId::S01, "immediate_safe", Rule::Immediate);
    policy_ctor!(s02, StrategyId::S02, "delay_5s", Rule::DelayMs(5_000));
    policy_ctor!(s03, StrategyId::S03, "delay_15s", Rule::DelayMs(15_000));
    policy_ctor!(s04, StrategyId::S04, "organic_flow", Rule::OrganicFlow);
    policy_ctor!(s05, StrategyId::S05, "dip_rebound", Rule::DipRebound);
    policy_ctor!(
        s06,
        StrategyId::S06,
        "flow_creator_veto",
        Rule::FlowCreatorVeto
    );
    policy_ctor!(s07, StrategyId::S07, "curve_regime", Rule::CurveRegime);
    policy_ctor!(s08, StrategyId::S08, "pre_graduation", Rule::PreGraduation);
    policy_ctor!(s09, StrategyId::S09, "post_migration", Rule::PostMigration);
    policy_ctor!(
        s10,
        StrategyId::S10,
        "liquidity_first",
        Rule::LiquidityFirst
    );
    policy_ctor!(
        random_eligible,
        StrategyId::Rand,
        "random_eligible",
        Rule::RandomEligible
    );

    /// Mirrors `research_batch.py::eligible` field-for-field.
    fn eligible(&self, obs: &Observation, track: &MintTrack, risk: &RiskConstraints) -> bool {
        if !obs.coverage_ok || obs.safety != Safety::Eligible {
            return false;
        }
        if !obs.phase.is_tradeable() {
            return false;
        }
        let age = obs.observed_at_ms.saturating_sub(track.first_observed_ms);
        let flow_positive = obs.net_flow_lamports > 0;
        let organic = obs.phase == Phase::ActiveCurve
            && obs.unique_buyers >= self.knobs.min_buyers
            && flow_positive;
        match self.rule {
            Rule::NoTrade => false,
            Rule::Immediate => true,
            Rule::DelayMs(ms) => age >= ms,
            Rule::OrganicFlow => organic,
            Rule::DipRebound => {
                obs.phase == Phase::ActiveCurve
                    && track.seen_dip
                    && obs.price_lamports_per_token > track.previous_price
            }
            Rule::FlowCreatorVeto => organic && obs.creator_sell_lamports == 0,
            Rule::CurveRegime => {
                obs.phase == Phase::ActiveCurve && (7_000..9_000).contains(&obs.curve_progress_bps)
            }
            Rule::PreGraduation => {
                obs.phase == Phase::NearCompletion && obs.curve_progress_bps >= 9_000
            }
            Rule::PostMigration => {
                matches!(obs.phase, Phase::PumpswapOpen | Phase::PostMigration)
                    && obs.unique_buyers >= 3
            }
            Rule::RandomEligible => {
                // Deterministic 2000/10000 selection; no RNG state.
                let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
                for byte in obs.event_id.bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x100_0000_01b3);
                }
                hash % 10_000 < 2_000
            }
            Rule::LiquidityFirst => {
                (obs.exit_liquidity_lamports as u128)
                    >= (self.knobs.liquidity_multiple as u128) * (risk.spend_lamports as u128)
                    && obs.unique_buyers >= 3
                    && flow_positive
            }
        }
    }
}

impl Strategy for EntryPolicy {
    fn id(&self) -> StrategyId {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn decide(
        &mut self,
        obs: &Observation,
        portfolio: &PortfolioView,
        risk: &RiskConstraints,
    ) -> Intent {
        // Python loop order: seen_dip updates from the PREVIOUS peak before
        // eligibility is evaluated; peak/previous_price update AFTER — so
        // eligibility must see the previous event's price state.
        let snapshot = {
            let track = self
                .tracks
                .entry(obs.mint.clone())
                .or_insert_with(|| MintTrack::new(obs));
            if track.peak > 0
                && (obs.price_lamports_per_token as u128) * 100 <= (track.peak as u128) * 95
            {
                track.seen_dip = true;
            }
            *track
        };
        let intent = if portfolio.has_open_position(&obs.mint) {
            Intent::NoTrade
        } else if self.eligible(obs, &snapshot, risk) {
            Intent::Buy {
                spend_lamports: risk.spend_lamports,
            }
        } else {
            Intent::NoTrade
        };
        let track = self
            .tracks
            .get_mut(&obs.mint)
            .expect("track inserted above");
        track.peak = track.peak.max(obs.price_lamports_per_token);
        track.previous_price = obs.price_lamports_per_token;
        intent
    }
}

/// All eleven policies in registry order (S00 baseline first).
pub fn registry(knobs: StrategyKnobs) -> Vec<Box<dyn Strategy>> {
    vec![
        Box::new(EntryPolicy::s00(knobs)),
        Box::new(EntryPolicy::s01(knobs)),
        Box::new(EntryPolicy::s02(knobs)),
        Box::new(EntryPolicy::s03(knobs)),
        Box::new(EntryPolicy::s04(knobs)),
        Box::new(EntryPolicy::s05(knobs)),
        Box::new(EntryPolicy::s06(knobs)),
        Box::new(EntryPolicy::s07(knobs)),
        Box::new(EntryPolicy::s08(knobs)),
        Box::new(EntryPolicy::s09(knobs)),
        Box::new(EntryPolicy::s10(knobs)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Phase;

    fn obs(phase: Phase, price: u64, buyers: u64, progress: u64, liquidity: u64) -> Observation {
        Observation {
            event_id: "e1".into(),
            mint: "M".into(),
            slot: 1,
            chain_at_ms: 95,
            observed_at_ms: 100,
            feature_available_at_ms: 110,
            phase,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: price,
            exit_liquidity_lamports: liquidity,
            net_flow_lamports: 1,
            unique_buyers: buyers,
            creator_sell_lamports: 0,
            curve_progress_bps: progress,
            fee_bps: 100,
        }
    }

    fn wants(policy: &mut EntryPolicy, obs: &Observation, risk: &RiskConstraints) -> bool {
        let view = PortfolioView::default();
        matches!(policy.decide(obs, &view, risk), Intent::Buy { .. })
    }

    #[test]
    fn s00_never_trades() {
        let risk = RiskConstraints::default();
        let mut s00 = EntryPolicy::s00(StrategyKnobs::default());
        assert!(!wants(
            &mut s00,
            &obs(Phase::ActiveCurve, 100, 50, 5_000, 10_000_000),
            &risk
        ));
    }

    #[test]
    fn s07_curve_regime_bounds() {
        let risk = RiskConstraints::default();
        let mut s07 = EntryPolicy::s07(StrategyKnobs::default());
        assert!(!wants(
            &mut s07,
            &obs(Phase::ActiveCurve, 100, 1, 6_999, 10),
            &risk
        ));
        assert!(wants(
            &mut s07,
            &obs(Phase::ActiveCurve, 100, 1, 7_000, 10),
            &risk
        ));
        assert!(wants(
            &mut s07,
            &obs(Phase::ActiveCurve, 100, 1, 8_999, 10),
            &risk
        ));
        assert!(!wants(
            &mut s07,
            &obs(Phase::ActiveCurve, 100, 1, 9_000, 10),
            &risk
        ));
    }

    #[test]
    fn s10_liquidity_multiple_threshold() {
        let risk = RiskConstraints {
            spend_lamports: 1_000_000,
            ..Default::default()
        };
        let mut s10 = EntryPolicy::s10(StrategyKnobs::default());
        // 20 * spend = 20_000_000 exactly passes; one lamport below fails.
        assert!(wants(
            &mut s10,
            &obs(Phase::ActiveCurve, 100, 3, 1, 20_000_000),
            &risk
        ));
        assert!(!wants(
            &mut s10,
            &obs(Phase::ActiveCurve, 100, 3, 1, 19_999_999),
            &risk
        ));
    }

    #[test]
    fn held_mint_never_reenters() {
        let risk = RiskConstraints::default();
        let mut s01 = EntryPolicy::s01(StrategyKnobs::default());
        let mut view = PortfolioView::default();
        view.open_mints.insert("M".into());
        assert_eq!(
            s01.decide(&obs(Phase::ActiveCurve, 100, 1, 1, 10), &view, &risk),
            Intent::NoTrade
        );
    }

    #[test]
    fn knob_bounds_enforced() {
        assert!(StrategyKnobs::new(0, 20).is_err());
        assert!(StrategyKnobs::new(5, 101).is_err());
        assert!(StrategyKnobs::new(30, 100).is_ok());
    }

    #[test]
    fn registry_has_eleven_policies_in_order() {
        let ids: Vec<&'static str> = registry(StrategyKnobs::default())
            .iter()
            .map(|s| s.id().as_str())
            .collect();
        assert_eq!(
            ids,
            vec!["S00", "S01", "S02", "S03", "S04", "S05", "S06", "S07", "S08", "S09", "S10"]
        );
    }
}
