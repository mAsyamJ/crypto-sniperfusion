//! Safe runtimes: replay (and paper) pipelines.
//!
//! M1 scope: deterministic event → dedup → priority bus → decision hook,
//! with observable counters. Live streaming arrives at M2 (observe), and
//! simulated fills at M3 (paper) — until then `paper` records planned
//! decisions without pretending fills happened.

use std::collections::HashMap;

use sniper_core::event::{ChainEvent, DedupOutcome, DedupRegistry};
use sniper_core::primitives::Lamports;
use sniper_core::venue::Venue;
use thiserror::Error;

use crate::bus::{Priority, PriorityBus, PublishOutcome};

/// What a strategy layer decided for one event. `PaperIntent` is recorded,
/// never executed — M3 turns these into authorized simulated orders.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub event_signature: solana_sdk::signature::Signature,
    pub venue: Venue,
    pub kind: DecisionKind,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionKind {
    /// Wants to buy `spend` lamports of the event's mint (paper-only at M1).
    PaperIntentBuy { spend: Lamports },
    /// Logged an observation; no trade desire.
    ObservationOnly,
    /// Screening verdict pending/failed — recorded for replay attribution.
    ScreeningVerdict { verdict: ScreeningVerdict },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreeningVerdict {
    Eligible,
    Watch,
    Block,
}

/// Pluggable decision callback. M5 replaces the default no-op with the risk
/// engine; the contract is frozen now.
pub trait DecisionHook: Send {
    fn on_event(&mut self, event: &ChainEvent) -> Vec<Decision>;
}

/// Default hook: observes everything, decides nothing.
#[derive(Debug, Default)]
pub struct ObserveOnlyHook;

impl DecisionHook for ObserveOnlyHook {
    fn on_event(&mut self, _event: &ChainEvent) -> Vec<Decision> {
        Vec::new()
    }
}

#[derive(Debug, Error)]
pub enum RunError {
    #[error("fixture decode failed at record {index}: {message}")]
    FixtureDecode { index: usize, message: String },
}

/// Counters proving pipeline behavior in tests and evidence files.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunSummary {
    pub events_in: u64,
    pub duplicates: u64,
    pub published: u64,
    pub shed_speculative: u64,
    pub decisions: u64,
    pub by_priority: HashMap<Priority, u64>,
}

/// Replay/paper pipeline: load events → dedup → classify priority → bus →
/// drain in priority order → decision hook. Deterministic for a fixed input.
pub struct ReplayRunner {
    dedup_capacity: usize,
    bus_capacity: usize,
}

impl Default for ReplayRunner {
    fn default() -> Self {
        Self::new(8192, 1024)
    }
}

impl ReplayRunner {
    pub fn new(dedup_capacity: usize, bus_capacity: usize) -> Self {
        Self {
            dedup_capacity,
            bus_capacity,
        }
    }

    /// Map an event to its bus tier. Order lifecycle events and emergency
    /// exits are CRITICAL; open-position monitoring is POSITION; new-entry
    /// candidates are SPECULATIVE.
    fn classify(event: &ChainEvent) -> Priority {
        match event.payload {
            sniper_core::event::EventPayload::MigrationObserved { .. } => Priority::Position,
            sniper_core::event::EventPayload::TokenCreated { .. } => Priority::Speculative,
            sniper_core::event::EventPayload::TradeObserved { .. }
            | sniper_core::event::EventPayload::WalletActivity { .. } => Priority::Speculative,
            sniper_core::event::EventPayload::Unknown => Priority::Speculative,
        }
    }

    pub fn run<H: DecisionHook>(
        &self,
        events: &[ChainEvent],
        hook: &mut H,
    ) -> Result<RunSummary, RunError> {
        let mut dedup = DedupRegistry::new(self.dedup_capacity);
        let mut bus: PriorityBus<ChainEvent> = PriorityBus::new(self.bus_capacity);
        let mut summary = RunSummary::default();

        // Phase 1: ingest (dedup before decisioning).
        for event in events {
            summary.events_in += 1;
            match dedup.register(event.id) {
                DedupOutcome::FirstSeen => {}
                DedupOutcome::Duplicate => {
                    summary.duplicates += 1;
                    continue;
                }
            }
            let priority = Self::classify(event);
            match bus.try_publish(priority, event.clone()) {
                Ok(PublishOutcome::Enqueued) => summary.published += 1,
                Ok(PublishOutcome::ShedSpeculative) => summary.shed_speculative += 1,
                Err(_) => {
                    // Critical/Position overflow with a fully-buffered input
                    // cannot happen unless capacities are smaller than the
                    // input; surface as an error rather than dropping.
                    return Err(RunError::FixtureDecode {
                        index: summary.events_in as usize,
                        message: format!("bus overflow at priority {priority:?}"),
                    });
                }
            }
        }

        // Phase 2: drain in strict priority order and decide.
        while let Some((priority, event)) = bus.try_recv() {
            *summary.by_priority.entry(priority).or_insert(0) += 1;
            let decisions = hook.on_event(&event);
            summary.decisions += decisions.len() as u64;
        }
        Ok(summary)
    }

    /// Load fixtures from a JSONL file (one `ChainEvent` per line).
    pub fn load_fixtures(path: &std::path::Path) -> Result<Vec<ChainEvent>, RunError> {
        let text = std::fs::read_to_string(path).map_err(|e| RunError::FixtureDecode {
            index: 0,
            message: format!("read {path:?}: {e}"),
        })?;
        let mut events = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let event: ChainEvent =
                serde_json::from_str(line).map_err(|e| RunError::FixtureDecode {
                    index,
                    message: e.to_string(),
                })?;
            events.push(event);
        }
        Ok(events)
    }
}

/// Counting hook used by tests and the CLI's default paper mode.
#[derive(Debug, Default)]
pub struct CountingHook {
    pub observed: u64,
    pub buys_proposed: u64,
}

impl DecisionHook for CountingHook {
    fn on_event(&mut self, event: &ChainEvent) -> Vec<Decision> {
        self.observed += 1;
        use sniper_core::event::EventPayload;
        match &event.payload {
            EventPayload::TokenCreated { .. } => {
                // Paper mode at M1 records the desire, never a fill.
                self.buys_proposed += 1;
                vec![Decision {
                    event_signature: event.id.signature,
                    venue: event.venue,
                    kind: DecisionKind::PaperIntentBuy {
                        spend: Lamports::ZERO,
                    },
                    priority: Priority::Speculative,
                }]
            }
            _ => vec![Decision {
                event_signature: event.id.signature,
                venue: event.venue,
                kind: DecisionKind::ObservationOnly,
                priority: Priority::Position,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniper_core::event::{ChainEventId, Commitment, EventPayload, ProviderKind};
    use sniper_core::time::Timestamps;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signature;

    fn event(n: u8) -> ChainEvent {
        let mut sig = [0u8; 64];
        sig[0] = n;
        ChainEvent {
            id: ChainEventId {
                signature: Signature::from(sig),
                outer_ix_index: 0,
                inner_ix_index: None,
                event_ordinal: 0,
            },
            venue: Venue::PumpFun,
            slot: 1000 + n as u64,
            commitment: Commitment::Processed,
            received: Timestamps {
                wall: std::time::SystemTime::UNIX_EPOCH,
                mono_ns: n as u64,
            },
            source: ProviderKind::Replay,
            schema_version: 1,
            payload: EventPayload::TokenCreated {
                mint: Pubkey::new_unique(),
                creator: Pubkey::new_unique(),
                name: format!("coin{n}"),
                symbol: format!("C{n}"),
                uri: "https://example.com/m.json".into(),
            },
        }
    }

    #[test]
    fn duplicates_are_counted_not_reprocessed() {
        let runner = ReplayRunner::default();
        let mut hook = CountingHook::default();
        let events = vec![event(1), event(1), event(1), event(2)];
        let summary = runner.run(&events, &mut hook).unwrap();
        assert_eq!(summary.events_in, 4);
        assert_eq!(summary.duplicates, 2);
        assert_eq!(hook.observed, 2, "dedup happens before decisioning");
    }

    #[test]
    fn same_input_yields_same_summary() {
        let events: Vec<ChainEvent> = (0..50).map(|i| event(i as u8)).collect();
        let s1 = ReplayRunner::default()
            .run(&events, &mut CountingHook::default())
            .unwrap();
        let s2 = ReplayRunner::default()
            .run(&events, &mut CountingHook::default())
            .unwrap();
        assert_eq!(s1, s2, "replay must be deterministic");
    }

    #[test]
    fn paper_mode_proposes_but_never_fills() {
        let runner = ReplayRunner::default();
        let mut hook = CountingHook::default();
        let summary = runner.run(&[event(7)], &mut hook).unwrap();
        assert_eq!(hook.buys_proposed, 1);
        assert_eq!(summary.decisions, 1);
        // Spend is zero at M1: paper intents carry no money until M3 fills.
    }

    #[test]
    fn fixtures_roundtrip_through_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixtures.jsonl");
        let events: Vec<ChainEvent> = (0..3).map(|i| event(i as u8)).collect();
        let json: Vec<String> = events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        std::fs::write(&path, json.join("\n")).unwrap();

        let loaded = ReplayRunner::load_fixtures(&path).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0], events[0]);
        assert_eq!(loaded[2], events[2]);
    }
}
