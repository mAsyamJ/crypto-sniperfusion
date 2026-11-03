//! Exit policy, deliberately separate from entry strategies: batch-1
//! experiments isolate entry selection under one shared exit rule.

use crate::observation::{Observation, Phase};

/// Shared exit rule (mirrors the surrogate: max-hold horizon OR terminal
/// decay phases trigger an exit attempt; exit feasibility is priced by the
/// engine, an unexecutable exit leaves the position open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitPolicy {
    pub max_hold_ms: u64,
}

impl Default for ExitPolicy {
    fn default() -> Self {
        Self {
            max_hold_ms: 20_000,
        }
    }
}

impl ExitPolicy {
    /// Whether an exit is DUE for a position entered at
    /// `entry_at_ms` (the feature-available time of the entry event),
    /// evaluated at this observation.
    pub fn due(&self, entry_at_ms: u64, obs: &Observation) -> bool {
        obs.feature_available_at_ms.saturating_sub(entry_at_ms) >= self.max_hold_ms
            || matches!(obs.phase, Phase::Decaying | Phase::Inactive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Safety;

    fn obs(phase: Phase, feature_available_at_ms: u64) -> Observation {
        Observation {
            event_id: "e".into(),
            mint: "M".into(),
            slot: 1,
            chain_at_ms: 0,
            observed_at_ms: 0,
            feature_available_at_ms,
            phase,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: 1,
            exit_liquidity_lamports: 1,
            net_flow_lamports: 0,
            unique_buyers: 0,
            creator_sell_lamports: 0,
            curve_progress_bps: 0,
            fee_bps: 0,
        }
    }

    #[test]
    fn max_hold_triggers_exit() {
        let policy = ExitPolicy {
            max_hold_ms: 20_000,
        };
        assert!(!policy.due(100, &obs(Phase::ActiveCurve, 20_099)));
        assert!(policy.due(100, &obs(Phase::ActiveCurve, 20_100)));
    }

    #[test]
    fn decay_phases_trigger_exit_regardless_of_hold() {
        let policy = ExitPolicy {
            max_hold_ms: 1_000_000,
        };
        assert!(policy.due(100, &obs(Phase::Decaying, 200)));
        assert!(policy.due(100, &obs(Phase::Inactive, 200)));
    }
}
