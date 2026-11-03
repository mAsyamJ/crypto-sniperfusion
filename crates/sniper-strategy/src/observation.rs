//! Point-in-time research observation (frozen surrogate schema v1).
//!
//! Mirrors `harness/schemas/research_observation.v1.schema.json` exactly:
//! 16 required fields, strict validation (uniqueness, chronology, bps
//! bounds, phase/safety enums, tradeable-nonzero-price). This is the
//! strategy-visible FEATURE record only — future outcome labels live in
//! `sniper_core::labels` and can never appear here.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Lifecycle phase vocabulary (frozen surrogate schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    #[serde(rename = "CREATED")]
    Created,
    #[serde(rename = "EARLY_CURVE")]
    EarlyCurve,
    #[serde(rename = "ACTIVE_CURVE")]
    ActiveCurve,
    #[serde(rename = "NEAR_COMPLETION")]
    NearCompletion,
    #[serde(rename = "CURVE_COMPLETED")]
    CurveCompleted,
    #[serde(rename = "MIGRATION_PENDING")]
    MigrationPending,
    #[serde(rename = "PUMPSWAP_OPEN")]
    PumpswapOpen,
    #[serde(rename = "POST_MIGRATION")]
    PostMigration,
    #[serde(rename = "DECAYING")]
    Decaying,
    #[serde(rename = "INACTIVE")]
    Inactive,
}

impl Phase {
    pub fn is_tradeable(self) -> bool {
        matches!(
            self,
            Phase::EarlyCurve
                | Phase::ActiveCurve
                | Phase::NearCompletion
                | Phase::PumpswapOpen
                | Phase::PostMigration
        )
    }
}

/// Safety verdict at decision time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Safety {
    #[serde(rename = "ELIGIBLE")]
    Eligible,
    #[serde(rename = "WATCH")]
    Watch,
    #[serde(rename = "BLOCK")]
    Block,
}

/// One point-in-time observation. Every field was knowable at
/// `feature_available_at_ms`; nothing here is a future outcome label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub event_id: String,
    pub mint: String,
    pub slot: u64,
    pub chain_at_ms: u64,
    pub observed_at_ms: u64,
    pub feature_available_at_ms: u64,
    pub phase: Phase,
    pub safety: Safety,
    pub coverage_ok: bool,
    pub price_lamports_per_token: u64,
    pub exit_liquidity_lamports: u64,
    /// May be negative (net outflow).
    pub net_flow_lamports: i64,
    pub unique_buyers: u64,
    pub creator_sell_lamports: u64,
    pub curve_progress_bps: u64,
    pub fee_bps: u64,
}

/// Validation error for a dataset of observations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("line {line}: {message}")]
    Record { line: usize, message: String },
    #[error("dataset has no events")]
    Empty,
}

fn record(line: usize, message: impl Into<String>) -> ValidationError {
    ValidationError::Record {
        line,
        message: message.into(),
    }
}

impl Observation {
    /// Validate one parsed observation (field-level rules from the schema).
    fn validate(&self, line: usize) -> Result<(), ValidationError> {
        if self.curve_progress_bps > 10_000 {
            return Err(record(line, "curve_progress_bps exceeds 10000"));
        }
        if self.fee_bps >= 10_000 {
            return Err(record(line, "fee_bps must be < 10000"));
        }
        if self.phase.is_tradeable() && self.price_lamports_per_token == 0 {
            return Err(record(line, "tradeable phase has zero price"));
        }
        // Frozen surrogate contract keeps the strict three-clock ordering.
        // (The H6-1 dataset layer relaxes chain-vs-observed; this schema is
        // frozen for engine parity and may not drift.)
        if !(self.chain_at_ms <= self.observed_at_ms
            && self.observed_at_ms <= self.feature_available_at_ms)
        {
            return Err(record(
                line,
                "lookahead/clock-order violation (strict demo contract)",
            ));
        }
        Ok(())
    }
}

/// Load and validate a JSONL observation dataset with the full frozen-schema
/// contract: unique event ids, non-decreasing observation chronology (never
/// sort unseen data), no duplicate (slot, event_id) per mint.
pub fn load_events(text: &str) -> Result<Vec<Observation>, ValidationError> {
    let mut events = Vec::new();
    let mut ids: HashSet<String> = HashSet::new();
    let mut last_observed = 0u64;
    let mut first = true;
    let mut last_coordinate: Option<(u64, String)> = None;
    for (index, line) in text.lines().enumerate() {
        let lineno = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let event: Observation =
            serde_json::from_str(line).map_err(|e| record(lineno, format!("invalid JSON: {e}")))?;
        if event.event_id.is_empty() || !ids.insert(event.event_id.clone()) {
            return Err(record(lineno, "invalid/duplicate event_id"));
        }
        if event.mint.is_empty() {
            return Err(record(lineno, "invalid mint"));
        }
        event.validate(lineno)?;
        if !first && event.observed_at_ms < last_observed {
            return Err(record(
                lineno,
                "observation chronology violated; never sort unseen data",
            ));
        }
        first = false;
        last_observed = event.observed_at_ms;
        let coordinate = (event.slot, event.event_id.clone());
        if last_coordinate.as_ref() == Some(&coordinate) {
            return Err(record(lineno, "duplicate chain coordinate"));
        }
        last_coordinate = Some(coordinate);
        events.push(event);
    }
    if events.is_empty() {
        return Err(ValidationError::Empty);
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(event_id: &str, observed_at_ms: u64) -> Observation {
        Observation {
            event_id: event_id.into(),
            mint: "M".into(),
            slot: 1,
            chain_at_ms: observed_at_ms - 5,
            observed_at_ms,
            feature_available_at_ms: observed_at_ms + 5,
            phase: Phase::EarlyCurve,
            safety: Safety::Eligible,
            coverage_ok: true,
            price_lamports_per_token: 100,
            exit_liquidity_lamports: 10_000,
            net_flow_lamports: 1,
            unique_buyers: 2,
            creator_sell_lamports: 0,
            curve_progress_bps: 1_000,
            fee_bps: 100,
        }
    }

    #[test]
    fn valid_dataset_loads() {
        let text = format!(
            "{}\n{}\n",
            serde_json::to_string(&observation("e1", 100)).unwrap(),
            serde_json::to_string(&observation("e2", 200)).unwrap()
        );
        assert_eq!(load_events(&text).unwrap().len(), 2);
    }

    #[test]
    fn duplicate_event_id_rejected() {
        let one = serde_json::to_string(&observation("e1", 100)).unwrap();
        let text = format!("{one}\n{one}\n");
        assert!(load_events(&text).is_err());
    }

    #[test]
    fn chronology_violation_rejected() {
        let a = serde_json::to_string(&observation("e1", 200)).unwrap();
        let b = serde_json::to_string(&observation("e2", 100)).unwrap();
        assert!(load_events(&format!("{a}\n{b}\n")).is_err());
    }

    #[test]
    fn lookahead_rejected() {
        let mut e = observation("e1", 100);
        e.feature_available_at_ms = 50; // < observed
        let text = serde_json::to_string(&e).unwrap();
        assert!(load_events(&text).is_err());
    }

    #[test]
    fn tradeable_phase_zero_price_rejected() {
        let mut e = observation("e1", 100);
        e.price_lamports_per_token = 0;
        assert!(load_events(&serde_json::to_string(&e).unwrap()).is_err());
    }

    #[test]
    fn empty_dataset_rejected() {
        assert!(load_events("\n \n").is_err());
    }
}
