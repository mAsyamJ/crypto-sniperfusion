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

/// Proof that the missing slot range was backfilled from a trusted source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackfillReceipt {
    pub gap_from: u64,
    pub gap_to: u64,
    /// How many distinct missing slots the backfill actually recovered.
    pub slots_backfilled: u64,
    /// Optional root/bank hash of the highest backfilled slot, for
    /// continuity verification against the live stream's root.
    pub root: Option<[u8; 32]>,
}

impl BackfillReceipt {
    /// True iff this receipt fully covers the given gap window.
    fn covers(&self, from: u64, to: u64) -> bool {
        self.gap_from == from
            && self.gap_to == to
            && self.slots_backfilled == to.saturating_sub(from) + 1
    }
}

/// Entry gate (H0.3): halts SPECULATIVE entries on a critical stream gap and
/// resumes ONLY when (a) the exact missing-slot window has been backfilled
/// with a covering receipt, AND (b) position/account reconciliation over the
/// recovered data has completed. Contiguous observations alone NEVER resume
/// the gate — that was the unsafe behavior this replaces.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryGate {
    halted: bool,
    open_gap: Option<(u64, u64)>,
    reconciled: bool,
    /// Largest (from,to) window ever recorded — for diagnostics.
    last_gap: Option<(u64, u64)>,
}

impl Default for EntryGate {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryGate {
    pub fn new() -> Self {
        Self {
            halted: false,
            open_gap: None,
            reconciled: true,
            last_gap: None,
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn open_gap(&self) -> Option<(u64, u64)> {
        self.open_gap
    }

    /// A gap was detected: halt immediately and open the reconciliation
    /// requirement (a fresh gap invalidates a prior reconciliation).
    pub fn note_gap(&mut self, from: u64, to: u64) {
        self.halted = true;
        self.reconciled = false;
        self.open_gap = Some((from, to));
        self.last_gap = Some((from, to));
    }

    /// Operator/reconciler asserts account and position state over the
    /// backfilled window is consistent. Necessary but not sufficient.
    pub fn complete_reconciliation(&mut self) {
        self.reconciled = true;
        self.try_resume();
    }

    /// Backfill evidence arrived: resume only when it fully covers the open
    /// gap window AND reconciliation has completed. Mismatched, partial, or
    /// unsolicited receipts are ignored (and never un-halt).
    pub fn note_backfill(&mut self, receipt: BackfillReceipt) -> bool {
        if let Some((from, to)) = self.open_gap {
            if receipt.covers(from, to) {
                self.open_gap = None;
                self.try_resume();
                return true;
            }
        }
        false
    }

    fn try_resume(&mut self) {
        if self.halted && self.open_gap.is_none() && self.reconciled {
            self.halted = false;
        }
    }
}

/// Slot-coverage cursor driven by the SLOT stream (blockMeta/slot updates),
/// not by transaction heuristics: the mission's "root/slot source for
/// continuous slot coverage". Reports gaps with parent linkage for backfill.
#[derive(Debug, Default)]
pub struct SlotCursor {
    last_slot: Option<u64>,
}

impl SlotCursor {
    /// Observe a slot notification with its parent (blockMeta carries it).
    /// Returns the missing-window when continuity broke: (last+1, slot-1).
    /// A parent that does not match our last slot is treated as a gap even
    /// when numerically adjacent — fork/reorg safe.
    pub fn observe(&mut self, slot: u64, parent: Option<u64>) -> Option<(u64, u64)> {
        // Replayed/duplicate older slots are gap-irrelevant noise.
        if let Some(last) = self.last_slot {
            if slot <= last {
                return None;
            }
        }
        let gap = match (self.last_slot, parent) {
            (Some(last), Some(p)) if p == last && slot > last + 1 => Some((last + 1, slot - 1)),
            (Some(last), Some(p)) if p != last => {
                // Parent linkage broke: treat everything between as missing.
                let from = last.saturating_add(1).min(slot.saturating_sub(1));
                Some((from, slot.saturating_sub(1)))
            }
            (Some(last), None) if slot > last + 1 => Some((last + 1, slot - 1)),
            _ => None,
        };
        if self.last_slot.is_none_or(|l| slot > l) {
            self.last_slot = Some(slot);
        }
        gap
    }

    pub fn last_slot(&self) -> Option<u64> {
        self.last_slot
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
    /// Entry gate driven by gap events (H0.3). Speculative entries must
    /// consult `gate().is_halted()`; the supervisor no longer auto-resumes.
    gate: EntryGate,
}

impl<P: EventProvider> StreamSupervisor<P> {
    pub fn new(provider: P, policy: ReconnectPolicy, max_failures: u32, contiguous: bool) -> Self {
        Self {
            provider,
            policy,
            max_failures,
            contiguous,
            gap_tracker: GapTracker::default(),
            gate: EntryGate::new(),
        }
    }

    /// The entry gate this supervisor drives on gap events.
    pub fn gate(&self) -> &EntryGate {
        &self.gate
    }

    pub fn gate_mut(&mut self) -> &mut EntryGate {
        &mut self.gate
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
                    // A gap halts the entry gate; only a covering backfill
                    // receipt + completed reconciliation will resume it.
                    self.gate.note_gap(from, to);
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
    fn gate_halts_on_gap_and_never_auto_resumes_on_contiguity() {
        let mut gate = EntryGate::new();
        assert!(!gate.is_halted());
        gate.note_gap(101, 104);
        assert!(gate.is_halted());
        assert_eq!(gate.open_gap(), Some((101, 104)));
        // The OLD unsafe behavior (auto-resume after N contiguous
        // observations) is gone: there is no observe_contiguous path, and
        // reconciliation alone or backfill alone must not resume either.
        gate.complete_reconciliation();
        assert!(gate.is_halted(), "no backfill receipt -> still halted");
    }

    #[test]
    fn gate_resumes_only_on_covering_receipt_plus_reconciliation() {
        let mut gate = EntryGate::new();
        gate.note_gap(101, 104); // 4 missing slots

        // Partial receipt (3 of 4 slots) — rejected.
        assert!(!gate.note_backfill(BackfillReceipt {
            gap_from: 101,
            gap_to: 104,
            slots_backfilled: 3,
            root: None,
        }));
        assert!(gate.is_halted(), "partial backfill never resumes");

        // Wrong window — rejected.
        assert!(!gate.note_backfill(BackfillReceipt {
            gap_from: 102,
            gap_to: 105,
            slots_backfilled: 4,
            root: None,
        }));
        assert!(gate.is_halted());

        // Full receipt clears the gap but not the halt (not reconciled yet).
        assert!(gate.note_backfill(BackfillReceipt {
            gap_from: 101,
            gap_to: 104,
            slots_backfilled: 4,
            root: Some([7u8; 32]),
        }));
        assert!(gate.is_halted(), "backfill without reconciliation holds");

        // Reconciliation after backfill resumes.
        gate.complete_reconciliation();
        assert!(!gate.is_halted());
    }

    #[test]
    fn gate_reconciliation_order_is_flexible_but_both_required() {
        // Reconcile first, then receipt — also resumes (both facts present).
        let mut gate = EntryGate::new();
        gate.note_gap(200, 202);
        gate.complete_reconciliation();
        assert!(gate.is_halted());
        assert!(gate.note_backfill(BackfillReceipt {
            gap_from: 200,
            gap_to: 202,
            slots_backfilled: 3,
            root: None,
        }));
        assert!(!gate.is_halted());
    }

    #[test]
    fn fresh_gap_invalidates_prior_reconciliation() {
        let mut gate = EntryGate::new();
        gate.note_gap(101, 104);
        gate.note_backfill(BackfillReceipt {
            gap_from: 101,
            gap_to: 104,
            slots_backfilled: 4,
            root: None,
        });
        gate.complete_reconciliation();
        assert!(!gate.is_halted());
        // A new gap must re-halt AND re-require reconciliation.
        gate.note_gap(110, 111);
        assert!(gate.is_halted());
        // Old receipt for the closed gap is now unsolicited — ignored.
        assert!(!gate.note_backfill(BackfillReceipt {
            gap_from: 101,
            gap_to: 104,
            slots_backfilled: 4,
            root: None,
        }));
        assert!(gate.is_halted());
    }

    #[test]
    fn slot_cursor_detects_numeric_and_parent_link_gaps() {
        let mut c = SlotCursor::default();
        // Pure numeric gap.
        assert_eq!(c.observe(100, None), None, "first observation seeds");
        assert_eq!(c.observe(101, Some(100)), None);
        assert_eq!(c.observe(105, Some(101)), Some((102, 104)));
        // Adjacent numbers but broken parent linkage = fork-shaped gap.
        // (last=105; slot 106 claims parent 103 -> reorg around 104..105.)
        let gap = c.observe(106, Some(103));
        assert!(gap.is_some(), "parent mismatch must be treated as a gap");
        // Older/replayed slots never move the cursor back.
        assert_eq!(c.observe(90, Some(89)), None);
        assert_eq!(c.last_slot(), Some(106));
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
        // H0.3 wiring: the supervisor's entry gate is halted by the gap and
        // stays halted without a covering backfill + reconciliation.
        assert!(sup.gate().is_halted());
        assert_eq!(sup.gate().open_gap(), Some((101, 104)));
        // A full receipt + reconciliation resumes it.
        sup.gate_mut().note_backfill(BackfillReceipt {
            gap_from: 101,
            gap_to: 104,
            slots_backfilled: 4,
            root: None,
        });
        sup.gate_mut().complete_reconciliation();
        assert!(!sup.gate().is_halted());
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
