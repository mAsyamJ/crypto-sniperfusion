//! sniper-strategy: pure deterministic strategy SDK (H7).
//!
//! Layering: this crate has NO network, clock, or signing dependencies.
//! A strategy is a pure function of the observation stream plus explicit
//! portfolio/risk inputs; engines turn intents into simulated economics.
//!
//! Two engines are deliberately separated:
//! - `engine::legacy` (`LegacySurrogateV1`): byte-exact reproduction of the
//!   frozen Python surrogate `harness/research_batch.py` — its numbers are
//!   for differential parity only, class `SYNTHETIC_SURROGATE_UNCALIBRATED`.
//! - `engine::protocol` (`ProtocolEconomicV1`): venue-aware reserve math and
//!   source-proven protocol fees; never compromises economic correctness for
//!   surrogate parity.

pub mod calendar;
pub mod engine;
pub mod eval;
pub mod exit;
pub mod fees;
pub mod follower;
pub mod observation;
pub mod oracle;
pub mod policies;
pub mod receipt;

pub use observation::{load_events, Observation, Phase, Safety, ValidationError};
pub use policies::{registry, StrategyKnobs};

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Identifier of one registered policy (S00 baseline + S01..S10 candidates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyId {
    S00,
    S01,
    S02,
    S03,
    S04,
    S05,
    S06,
    S07,
    S08,
    S09,
    S10,
    /// Deterministic RANDOM_ELIGIBLE baseline (PF1 comparison contract).
    #[serde(rename = "RX")]
    Rand,
}

impl StrategyId {
    pub const fn as_str(self) -> &'static str {
        match self {
            StrategyId::S00 => "S00",
            StrategyId::S01 => "S01",
            StrategyId::S02 => "S02",
            StrategyId::S03 => "S03",
            StrategyId::S04 => "S04",
            StrategyId::S05 => "S05",
            StrategyId::S06 => "S06",
            StrategyId::S07 => "S07",
            StrategyId::S08 => "S08",
            StrategyId::S09 => "S09",
            StrategyId::S10 => "S10",
            StrategyId::Rand => "RX",
        }
    }
}

/// What a strategy decided for one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Default, always-legitimate outcome.
    NoTrade,
    /// Wants to enter `spend_lamports` of the observation's mint. The engine
    /// still applies fill feasibility; an intent is not a fill.
    Buy { spend_lamports: u64 },
}

/// Read-only portfolio snapshot at decision time.
#[derive(Debug, Clone, Default)]
pub struct PortfolioView {
    pub cash_lamports: u64,
    /// Mints with an open position right now.
    pub open_mints: HashSet<String>,
}

impl PortfolioView {
    pub fn has_open_position(&self, mint: &str) -> bool {
        self.open_mints.contains(mint)
    }
}

/// Identical risk budget for every strategy in a batch (identical cohorts,
/// identical constraints — the batch isolates entry selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskConstraints {
    /// Lamports per entry attempt.
    pub spend_lamports: u64,
    /// Hard cap on simultaneously open positions (engine-enforced).
    pub max_open_positions: u64,
    /// Cash floor below which no new entries are attempted.
    pub cash_floor_lamports: u64,
}

impl Default for RiskConstraints {
    fn default() -> Self {
        Self {
            spend_lamports: 1_000_000,
            max_open_positions: 64,
            cash_floor_lamports: 0,
        }
    }
}

/// A deterministic entry policy.
///
/// `decide` is called for EVERY observation (in observation order) so
/// per-mint state tracking stays exact; the policy itself abstains when the
/// mint already has an open position. Future outcome labels are
/// structurally absent from [`Observation`] — a strategy cannot read them.
pub trait Strategy: Send {
    fn id(&self) -> StrategyId;
    fn name(&self) -> &'static str;
    fn decide(
        &mut self,
        observation: &Observation,
        portfolio: &PortfolioView,
        risk: &RiskConstraints,
    ) -> Intent;
}
