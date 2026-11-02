//! Stream supervision: reconnect with backoff, slot-gap detection, entry
//! gating, and commitment upgrades.
//!
//! Gap policy (architecture contract): a gap on a CRITICAL stream halts new
//! entries and forces reconciliation; gaps on speculative streams only mark
//! data stale. Only the supervisor decides — providers stay dumb pipes.

use std::collections::VecDeque;

use sniper_core::event::{ChainEvent, Commitment};

use crate::ingest::frame::RawObservation;
use crate::ingest::provider::{EventProvider, ProviderError, ReconnectPolicy};

/// What the supervisor emits to the pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// A decoded set of canonical events from one observation.
    Events(Vec<ChainEvent>),
    /// Stream broke and reconnect succeeded after N failures.
    Reconnected { after_failures: u32 },
    /// A slot gap on a contiguous stream was detected.
    GapDetected { from: u64, to: u64 },
    /// The provider failed past `max_failures` — operator-visible stop.
    PermanentlyFailed { failures: u32 },
}

/// Slot-gap detector for contiguous streams (blocks/geyser). Logs streams are
/// not contiguous and must not use this.
#[derive(Debug, Default)]
pub struct GapTracker {
    last_slot: Option<u64>,
}

impl GapTracker {
    /// Observe a slot; returns Some((from, to)) when a gap was crossed.
    pub fn observe(&mut self, slot: u64) -> Option<(u64, u64)> {
        let gap = match self.last_slot {
            Some(last) if slot > last + 1 => Some((last + 1, slot - 1)),
            _ => None,
        };
        if self.last_slot.is_none_or(|l| slot > l) {
            self.last_slot = Some(slot);
        }
        gap
    }
}

/// Entry gate: halts SPECULATIVE entries while a critical gap is unresolved.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryGate {
    halted: bool,
    /// Contiguous post-gap slots required before auto-resume.
    resume_after: u32,
    contiguous_since_gap: u32,
}

impl EntryGate {
    pub fn new(resume_after: u32) -> Self {
        Self {
            halted: false,
            resume_after,
            contiguous_since_gap: 0,
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn halt(&mut self) {
        self.halted = true;
        self.contiguous_since_gap = 0;
    }

    /// Feed one gap-free observation; auto-resumes after the threshold.
    pub fn observe_contiguous(&mut self) {
        if !self.halted {
            return;
        }
        self.contiguous_since_gap += 1;
        if self.contiguous_since_gap >= self.resume_after {
            self.halted = false;
            self.contiguous_since_gap = 0;
        }
    }

    /// A further gap restarts the resume counter.
    pub fn observe_gap(&mut self) {
        self.halted = true;
        self.contiguous_since_gap = 0;
    }
}

/// Drives one provider: reconnect-with-backoff, gap detection, event decode,
/// commitment upgrade bookkeeping. `max_failures` consecutive failures (without
/// a successful reconnect) stop the stream loudly.
pub struct StreamSupervisor<P: EventProvider> {
    provider: P,
    policy: ReconnectPolicy,
    max_failures: u32,
    /// True for contiguous streams (gap-tracked): blocks/geyser.
    contiguous: bool,
    gap_tracker: GapTracker,
}

impl<P: EventProvider> StreamSupervisor<P> {
    pub fn new(provider: P, policy: ReconnectPolicy, max_failures: u32, contiguous: bool) -> Self {
        Self {
            provider,
            policy,
            max_failures,
            contiguous,
            gap_tracker: GapTracker::default(),
        }
    }

    /// Attempt to (re)connect with backoff up to `max_failures`.
    /// Returns the number of failures endured, or None when permanently failed.
    pub async fn ensure_connected(&mut self) -> Option<u32> {
        let mut failures: u32 = 0;
        loop {
            match self.provider.connect().await {
                Ok(()) => return Some(failures),
                Err(_) => {
                    failures += 1;
                    if failures >= self.max_failures {
                        return None;
                    }
                    let delay = self.policy.delay_after_failures(failures);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Pull exactly one observation (assuming a live connection), mapping to
    /// supervisor events including gap detection. `Err` propagates transport
    /// breakage — the caller reconnects via `ensure_connected`.
    pub async fn next(&mut self) -> Result<Vec<StreamEvent>, ProviderError> {
        let obs: RawObservation = self.provider.next_observation().await?;
        let mut out = Vec::new();
        if self.contiguous {
            if let Some(slot) = obs.slot {
                if let Some((from, to)) = self.gap_tracker.observe(slot) {
                    out.push(StreamEvent::GapDetected { from, to });
                }
            }
        }
        let events = obs.into_events();
        if !events.is_empty() {
            out.push(StreamEvent::Events(events));
        }
        Ok(out)
    }
}

/// Commitment ledger: tracks the highest commitment per event id and reports
/// upgrades (Processed→Confirmed→Finalized) exactly once per step.
#[derive(Debug, Default)]
pub struct CommitmentLedger {
    current: std::collections::HashMap<sniper_core::event::ChainEventId, Commitment>,
    /// Bounded: evict oldest when full (same discipline as dedup registry).
    order: VecDeque<sniper_core::event::ChainEventId>,
    capacity: usize,
}

impl CommitmentLedger {
    pub fn new(capacity: usize) -> Self {
        Self {
            current: std::collections::HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// Record an observation's commitment. Returns Some(new) when this
    /// observation upgraded the recorded level (downgrades return None and
    /// are ignored).
    pub fn observe(
        &mut self,
        id: sniper_core::event::ChainEventId,
        commitment: Commitment,
    ) -> Option<Commitment> {
        match self.current.get(&id) {
            Some(existing) => {
                if existing.can_upgrade(commitment) {
                    self.current.insert(id, commitment);
                    Some(commitment)
                } else {
                    None
                }
            }
            None => {
                if self.order.len() == self.capacity {
                    if let Some(evicted) = self.order.pop_front() {
                        self.current.remove(&evicted);
                    }
                }
                self.current.insert(id, commitment);
                self.order.push_back(id);
                None // first sight is not an upgrade
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn gap_tracker_reports_only_true_gaps() {
        let mut t = GapTracker::default();
        assert_eq!(t.observe(100), None, "first observation is not a gap");
        assert_eq!(t.observe(101), None, "contiguous");
        assert_eq!(t.observe(105), Some((102, 104)), "gap detected");
        assert_eq!(t.observe(103), None, "late older slot ignored");
        assert_eq!(t.observe(106), None);
    }

    #[test]
    fn entry_gate_halts_and_resumes_after_threshold() {
        let mut gate = EntryGate::new(3);
        assert!(!gate.is_halted());
        gate.halt();
        assert!(gate.is_halted());
        gate.observe_contiguous();
        gate.observe_contiguous();
        assert!(gate.is_halted(), "resume only after threshold");
        gate.observe_contiguous();
        assert!(!gate.is_halted());
        // A new gap during recovery restarts the count.
        gate.observe_gap();
        assert!(gate.is_halted());
        gate.observe_contiguous();
        gate.observe_gap();
        gate.observe_contiguous();
        gate.observe_contiguous();
        assert!(gate.is_halted(), "gap resets the resume counter");
    }

    // ---- mock provider + supervisor ----

    struct MockProvider {
        script: VecDeque<Result<crate::ingest::frame::RawObservation, ProviderError>>,
        connect_attempts: VecDeque<Result<(), ProviderError>>,
    }

    impl MockProvider {
        fn with_connect_failures(n: u32) -> Self {
            let mut connect_attempts = VecDeque::new();
            for _ in 0..n {
                connect_attempts.push_back(Err(ProviderError::Transport("down".into())));
            }
            connect_attempts.push_back(Ok(()));
            Self {
                script: VecDeque::new(),
                connect_attempts,
            }
        }
    }

    impl EventProvider for MockProvider {
        fn kind(&self) -> sniper_core::event::ProviderKind {
            sniper_core::event::ProviderKind::Geyser
        }

        async fn connect(&mut self) -> Result<(), ProviderError> {
            self.connect_attempts.pop_front().unwrap_or(Ok(()))
        }

        async fn next_observation(
            &mut self,
        ) -> Result<crate::ingest::frame::RawObservation, ProviderError> {
            self.script
                .pop_front()
                .unwrap_or(Err(ProviderError::Transport("exhausted".into())))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_reconnects_with_backoff_and_reports() {
        let policy = ReconnectPolicy {
            initial: Duration::from_millis(100),
            max: Duration::from_secs(1),
            multiplier: 2.0,
            reset_after_successes: 10,
        };
        let start = tokio::time::Instant::now();
        let mut sup =
            StreamSupervisor::new(MockProvider::with_connect_failures(3), policy, 10, true);
        let failures = sup.ensure_connected().await.expect("eventually connects");
        assert_eq!(failures, 3);
        // Backoff timeline: 100 + 200 + 400 = 700ms advanced by the paused clock.
        assert_eq!(
            tokio::time::Instant::now() - start,
            Duration::from_millis(700)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_permanent_failure_is_loud() {
        let mut connects = VecDeque::new();
        for _ in 0..5 {
            connects.push_back(Err(ProviderError::Transport("down".into())));
        }
        let provider = MockProvider {
            script: VecDeque::new(),
            connect_attempts: connects,
        };
        let mut sup = StreamSupervisor::new(provider, ReconnectPolicy::default(), 5, true);
        assert!(
            sup.ensure_connected().await.is_none(),
            "must give up loudly"
        );
    }

    #[tokio::test]
    async fn supervisor_emits_gap_events_on_contiguous_streams() {
        use crate::ingest::frame::{RawObservation, RawPayload};
        use sniper_core::time::Timestamps;
        use sniper_core::venue::Venue;
        use sniper_protocols::registry::program_id;
        use solana_sdk::signature::{Keypair, Signer};

        let pump = program_id(Venue::PumpFun).unwrap();
        let obs = |slot: u64| RawObservation {
            provider: sniper_core::event::ProviderKind::Geyser,
            signature: Some(Keypair::new().sign_message(b"x")),
            slot: Some(slot),
            commitment: Commitment::Processed,
            received: Timestamps {
                wall: std::time::SystemTime::UNIX_EPOCH,
                mono_ns: 0,
            },
            failed: false,
            payload: RawPayload::Logs {
                program: pump,
                logs: vec![],
            },
        };

        let mut script = VecDeque::new();
        script.push_back(Ok(obs(100)));
        script.push_back(Ok(obs(105)));
        let provider = MockProvider {
            script,
            connect_attempts: VecDeque::new(),
        };
        let mut sup = StreamSupervisor::new(provider, ReconnectPolicy::default(), 5, true);

        let first = sup.next().await.expect("first observation");
        assert!(
            first.is_empty(),
            "empty logs yield no events and no gap: {first:?}"
        );

        let second = sup.next().await.expect("second observation");
        assert!(
            matches!(
                second.as_slice(),
                [StreamEvent::GapDetected { from: 101, to: 104 }]
            ),
            "slot jump must report the exact gap: got {second:?}"
        );
    }

    #[test]
    fn commitment_ledger_reports_each_upgrade_once() {
        use sniper_core::event::ChainEventId;
        use solana_sdk::signature::Signature;

        let id = ChainEventId {
            signature: Signature::from([1u8; 64]),
            outer_ix_index: 0,
            inner_ix_index: None,
            event_ordinal: 0,
        };
        let mut ledger = CommitmentLedger::new(16);
        assert_eq!(ledger.observe(id, Commitment::Processed), None);
        assert_eq!(
            ledger.observe(id, Commitment::Processed),
            None,
            "same level is not an upgrade"
        );
        assert_eq!(
            ledger.observe(id, Commitment::Confirmed),
            Some(Commitment::Confirmed)
        );
        assert_eq!(ledger.observe(id, Commitment::Confirmed), None);
        assert_eq!(
            ledger.observe(id, Commitment::Finalized),
            Some(Commitment::Finalized)
        );
        assert_eq!(
            ledger.observe(id, Commitment::Processed),
            None,
            "downgrades ignored"
        );
    }
}
