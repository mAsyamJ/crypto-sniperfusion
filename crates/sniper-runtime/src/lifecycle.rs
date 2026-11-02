//! Order lifecycle: `INTENT → AUTHORIZED → SIGNED → SUBMITTED → OBSERVED →
//! FINALIZED | FAILED | UNKNOWN(→RECONCILING)`.
//!
//! Order lifecycle is separate from position lifecycle by contract. The
//! machine persists every transition through the journal BEFORE the stage it
//! describes takes effect (write-ahead), so recovery after a crash at any
//! point is exact: an order found in SIGNED/SUBMITTED/OBSERVED is restored
//! as UNKNOWN — never assumed failed, never blindly retried — and no fresh
//! buy may be issued while any order is UNKNOWN or RECONCILING.

use std::collections::HashMap;

use thiserror::Error;

use crate::journal::{FileJournal, LifecycleStage, TxJournal as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderState {
    Intent,
    Authorized,
    Signed,
    Submitted,
    Observed,
    Finalized,
    Failed,
    Unknown,
    Reconciling,
}

impl OrderState {
    pub fn as_stage(self) -> LifecycleStage {
        match self {
            OrderState::Intent => LifecycleStage::Intent,
            OrderState::Authorized => LifecycleStage::Authorized,
            OrderState::Signed => LifecycleStage::Signed,
            OrderState::Submitted => LifecycleStage::Submitted,
            OrderState::Observed => LifecycleStage::Observed,
            OrderState::Finalized => LifecycleStage::Finalized,
            OrderState::Failed => LifecycleStage::Failed,
            OrderState::Unknown => LifecycleStage::Unknown,
            OrderState::Reconciling => LifecycleStage::Reconciling,
        }
    }

    pub fn from_stage(stage: LifecycleStage) -> Self {
        match stage {
            LifecycleStage::Intent => OrderState::Intent,
            LifecycleStage::Authorized => OrderState::Authorized,
            LifecycleStage::Signed => OrderState::Signed,
            LifecycleStage::Submitted => OrderState::Submitted,
            LifecycleStage::Observed => OrderState::Observed,
            LifecycleStage::Finalized => OrderState::Finalized,
            LifecycleStage::Failed => OrderState::Failed,
            LifecycleStage::Unknown => OrderState::Unknown,
            LifecycleStage::Reconciling => OrderState::Reconciling,
        }
    }

    /// States that must not exist for any order before a fresh buy intent is
    /// allowed (reconciliation-first discipline).
    pub fn blocks_new_entries(self) -> bool {
        matches!(self, OrderState::Unknown | OrderState::Reconciling)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum LifecycleError {
    #[error("unknown order: {0}")]
    UnknownOrder(String),
    #[error("illegal transition {from:?} -> {to:?} for order {id}")]
    IllegalTransition {
        id: String,
        from: OrderState,
        to: OrderState,
    },
    #[error("duplicate submission for order {0} (already {1:?})")]
    DuplicateSubmission(String, OrderState),
    #[error("journal error: {0}")]
    Journal(String),
}

/// One order's durable record.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderRecord {
    pub intent_id: String,
    pub state: OrderState,
    /// Signature identity once signed — persisted before submission.
    pub signature: Option<String>,
    pub seq_last: u64,
}

/// The order lifecycle machine. Owns an append-only journal; every
/// transition appends BEFORE mutating in-memory state.
pub struct LifecycleMachine {
    journal: FileJournal,
    orders: HashMap<String, OrderRecord>,
}

impl LifecycleMachine {
    pub fn open(path: &std::path::Path) -> Result<Self, LifecycleError> {
        let journal =
            FileJournal::open(path).map_err(|e| LifecycleError::Journal(e.to_string()))?;
        let mut machine = Self {
            journal,
            orders: HashMap::new(),
        };
        machine.recover()?;
        Ok(machine)
    }

    /// Rebuild state from the journal after a crash/restart. Terminal states
    /// restore as-is; anything in-flight (SIGNED/SUBMITTED/OBSERVED) restores
    /// as UNKNOWN — the crash destroyed knowledge of the outcome.
    fn recover(&mut self) -> Result<(), LifecycleError> {
        let contents = self
            .journal
            .load()
            .map_err(|e| LifecycleError::Journal(e.to_string()))?;
        for rec in contents.records {
            let entry = self
                .orders
                .entry(rec.intent_id.clone())
                .or_insert_with(|| OrderRecord {
                    intent_id: rec.intent_id.clone(),
                    state: OrderState::Intent,
                    signature: None,
                    seq_last: 0,
                });
            entry.state = OrderState::from_stage(rec.stage);
            entry.seq_last = rec.seq;
            let sig = decode_signature(&rec.payload_json);
            if sig.is_some() {
                entry.signature = sig;
            }
        }
        // Crash mapping: in-flight states become UNKNOWN.
        for entry in self.orders.values_mut() {
            if matches!(
                entry.state,
                OrderState::Signed | OrderState::Submitted | OrderState::Observed
            ) {
                entry.state = OrderState::Unknown;
            }
        }
        Ok(())
    }

    fn persist(
        &mut self,
        id: &str,
        to: OrderState,
        payload_json: &str,
    ) -> Result<u64, LifecycleError> {
        self.journal
            .append(to.as_stage(), id, payload_json)
            .map_err(|e| LifecycleError::Journal(e.to_string()))
    }

    fn transition(
        &mut self,
        id: &str,
        to: OrderState,
        payload_json: &str,
    ) -> Result<(), LifecycleError> {
        let from = self
            .orders
            .get(id)
            .map(|r| r.state)
            .ok_or_else(|| LifecycleError::UnknownOrder(id.to_string()))?;
        let legal = matches!(
            (from, to),
            (OrderState::Intent, OrderState::Authorized)
                | (OrderState::Authorized, OrderState::Signed)
                | (OrderState::Signed, OrderState::Submitted)
                | (OrderState::Submitted, OrderState::Observed)
                | (OrderState::Observed, OrderState::Finalized)
                | (OrderState::Observed, OrderState::Failed)
                | (OrderState::Unknown, OrderState::Reconciling)
                | (OrderState::Reconciling, OrderState::Finalized)
                | (OrderState::Reconciling, OrderState::Failed)
        );
        if !legal {
            return Err(LifecycleError::IllegalTransition {
                id: id.to_string(),
                from,
                to,
            });
        }
        let seq = self.persist(id, to, payload_json)?;
        let entry = self.orders.get_mut(id).expect("checked above");
        entry.state = to;
        entry.seq_last = seq;
        Ok(())
    }

    pub fn create_intent(&mut self, id: &str) -> Result<(), LifecycleError> {
        if self.orders.contains_key(id) {
            return Err(LifecycleError::IllegalTransition {
                id: id.to_string(),
                from: self.orders[id].state,
                to: OrderState::Intent,
            });
        }
        let seq = self.persist(id, OrderState::Intent, "{}")?;
        self.orders.insert(
            id.to_string(),
            OrderRecord {
                intent_id: id.to_string(),
                state: OrderState::Intent,
                signature: None,
                seq_last: seq,
            },
        );
        Ok(())
    }

    pub fn authorize(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Authorized, "{}")
    }

    /// Record the signed identity BEFORE submission (crash between sign and
    /// submit leaves the signature recoverable for reconciliation).
    pub fn signed(&mut self, id: &str, signature: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Signed, &signature_json(signature))?;
        self.orders
            .get_mut(id)
            .expect("just transitioned")
            .signature = Some(signature.to_string());
        Ok(())
    }

    /// Submitting anything already in flight is rejected — a timeout is NOT
    /// a failure, and a duplicate submission risks a double fill.
    pub fn submitted(&mut self, id: &str) -> Result<(), LifecycleError> {
        if let Some(rec) = self.orders.get(id) {
            if matches!(
                rec.state,
                OrderState::Submitted | OrderState::Observed | OrderState::Unknown
            ) {
                return Err(LifecycleError::DuplicateSubmission(
                    id.to_string(),
                    rec.state,
                ));
            }
        }
        self.transition(id, OrderState::Submitted, "{}")
    }

    /// Confirmed observed on-chain (signature seen). Acknowledgment is not
    /// observation.
    pub fn observed(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Observed, "{}")
    }

    pub fn finalized(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Finalized, "{}")
    }

    pub fn failed(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Failed, "{}")
    }

    /// Outcome could not be determined (timeout, restart, provider loss).
    pub fn mark_unknown(&mut self, id: &str) -> Result<(), LifecycleError> {
        let from = self
            .orders
            .get(id)
            .map(|r| r.state)
            .ok_or_else(|| LifecycleError::UnknownOrder(id.to_string()))?;
        if from == OrderState::Unknown {
            return Ok(()); // idempotent
        }
        let seq = self.persist(id, OrderState::Unknown, "{}")?;
        let entry = self.orders.get_mut(id).expect("checked");
        entry.state = OrderState::Unknown;
        entry.seq_last = seq;
        Ok(())
    }

    pub fn reconcile(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(id, OrderState::Reconciling, "{}")
    }

    pub fn state_of(&self, id: &str) -> Option<OrderState> {
        self.orders.get(id).map(|r| r.state)
    }

    pub fn signature_of(&self, id: &str) -> Option<&str> {
        self.orders.get(id).and_then(|r| r.signature.as_deref())
    }

    /// Fresh buys are blocked while any order's outcome is unresolved.
    pub fn new_entries_allowed(&self) -> bool {
        !self.orders.values().any(|r| r.state.blocks_new_entries())
    }
}

fn signature_json(sig: &str) -> String {
    format!(r#"{{"signature":"{sig}"}}"#)
}

fn decode_signature(payload_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(payload_json)
        .ok()?
        .get("signature")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(dir: &tempfile::TempDir) -> LifecycleMachine {
        LifecycleMachine::open(&dir.path().join("orders.jsonl")).unwrap()
    }

    #[test]
    fn happy_path_through_finalized() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("o1").unwrap();
        m.authorize("o1").unwrap();
        m.signed("o1", "sigAAA").unwrap();
        m.submitted("o1").unwrap();
        m.observed("o1").unwrap();
        m.finalized("o1").unwrap();
        assert_eq!(m.state_of("o1"), Some(OrderState::Finalized));
        assert_eq!(m.signature_of("o1"), Some("sigAAA"));
        assert!(m.new_entries_allowed());
    }

    #[test]
    fn illegal_transitions_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("o1").unwrap();
        let e = m.submitted("o1").unwrap_err();
        assert!(
            matches!(e, LifecycleError::IllegalTransition { .. }),
            "got {e:?}"
        );
        m.authorize("o1").unwrap();
        let e = m.observed("o1").unwrap_err();
        assert!(
            matches!(e, LifecycleError::IllegalTransition { .. }),
            "got {e:?}"
        );
        let e = m.finalized("o1").unwrap_err();
        assert!(
            matches!(e, LifecycleError::IllegalTransition { .. }),
            "got {e:?}"
        );
    }

    #[test]
    fn duplicate_submission_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("o1").unwrap();
        m.authorize("o1").unwrap();
        m.signed("o1", "sigBBB").unwrap();
        m.submitted("o1").unwrap();
        let e = m.submitted("o1").unwrap_err();
        assert!(
            matches!(e, LifecycleError::DuplicateSubmission(ref id, OrderState::Submitted) if id == "o1"),
            "got {e:?}"
        );
        // Timeout → unknown → still no resubmission of the same order.
        m.mark_unknown("o1").unwrap();
        let e = m.submitted("o1").unwrap_err();
        assert!(
            matches!(e, LifecycleError::DuplicateSubmission(ref id, OrderState::Unknown) if id == "o1"),
            "got {e:?}"
        );
        assert!(!m.new_entries_allowed(), "unknown blocks fresh entries");
    }

    #[test]
    fn unknown_resolves_through_reconciling() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("o1").unwrap();
        m.authorize("o1").unwrap();
        m.signed("o1", "sigCCC").unwrap();
        m.submitted("o1").unwrap();
        m.mark_unknown("o1").unwrap();
        m.reconcile("o1").unwrap();
        m.failed("o1").unwrap();
        assert_eq!(m.state_of("o1"), Some(OrderState::Failed));
        assert!(m.new_entries_allowed());
    }

    #[test]
    fn crash_after_sign_recovers_unknown_with_signature() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut m = machine(&dir);
            m.create_intent("o1").unwrap();
            m.authorize("o1").unwrap();
            m.signed("o1", "sigDDD").unwrap();
            // "crash": no further transitions, drop without close semantics.
        }
        let m2 = machine(&dir);
        assert_eq!(m2.state_of("o1"), Some(OrderState::Unknown));
        assert_eq!(m2.signature_of("o1"), Some("sigDDD"));
        assert!(!m2.new_entries_allowed());
    }

    #[test]
    fn crash_after_observation_recovers_unknown_not_success() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut m = machine(&dir);
            m.create_intent("o1").unwrap();
            m.authorize("o1").unwrap();
            m.signed("o1", "sigEEE").unwrap();
            m.submitted("o1").unwrap();
            m.observed("o1").unwrap();
        }
        let m2 = machine(&dir);
        // Never assume success from a pre-crash OBSERVED: reconcile first.
        assert_eq!(m2.state_of("o1"), Some(OrderState::Unknown));
    }

    #[test]
    fn terminal_states_persist_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut m = machine(&dir);
            m.create_intent("done").unwrap();
            m.authorize("done").unwrap();
            m.signed("done", "sigFFF").unwrap();
            m.submitted("done").unwrap();
            m.observed("done").unwrap();
            m.finalized("done").unwrap();
        }
        let m2 = machine(&dir);
        assert_eq!(m2.state_of("done"), Some(OrderState::Finalized));
        assert!(m2.new_entries_allowed());
    }

    #[test]
    fn recovery_sequence_continues_in_reopened_machine() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("o1").unwrap();
        m.authorize("o1").unwrap();
        m.signed("o1", "sigGGG").unwrap();
        let seq_before = m.journal.load().unwrap().records.len();
        let mut m2 = machine(&dir); // reopen continues the file
        let seq_after = m2.journal.load().unwrap().records.len();
        assert_eq!(seq_before, seq_after);
        // The recovered machine can take a NEW intent and reach signed state.
        m2.create_intent("o2").unwrap();
        m2.authorize("o2").unwrap();
        m2.signed("o2", "sigHHH").unwrap();
        assert_eq!(m2.state_of("o2"), Some(OrderState::Signed));
    }

    #[test]
    fn duplicate_intent_id_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine(&dir);
        m.create_intent("dup").unwrap();
        let e = m.create_intent("dup").unwrap_err();
        assert!(
            matches!(e, LifecycleError::IllegalTransition { .. }),
            "got {e:?}"
        );
    }
}
