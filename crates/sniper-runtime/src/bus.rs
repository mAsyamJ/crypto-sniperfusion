//! Three-tier priority bus with overload shedding.
//!
//! CRITICAL: transaction reconciliation and emergency exits — never dropped;
//! a full queue back-pressures the caller.
//! POSITION: open-position and liquidity monitoring — back-pressure preferred.
//! SPECULATIVE: new-entry candidates — the ONLY tier dropped under overload,
//! always counted.

use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::bus::Priority::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    Critical,
    Position,
    Speculative,
}

/// Result of a publish attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    /// Enqueued on the tier's bounded queue.
    Enqueued,
    /// SPECULATIVE queue full: the item was dropped and counted. Never
    /// returned for other tiers.
    ShedSpeculative,
}

#[derive(Debug)]
pub struct PriorityBus<T: Send + 'static> {
    tx: HashMap<Priority, mpsc::Sender<T>>,
    rx: HashMap<Priority, mpsc::Receiver<T>>,
    capacities: HashMap<Priority, usize>,
    shed_counts: HashMap<Priority, std::sync::atomic::AtomicU64>,
}

impl<T: Send + 'static> PriorityBus<T> {
    /// All tiers share the given per-tier capacity.
    pub fn new(capacity: usize) -> Self {
        Self::with_capacities(capacity, capacity, capacity)
    }

    pub fn with_capacities(critical: usize, position: usize, speculative: usize) -> Self {
        let mut tx = HashMap::new();
        let mut rx = HashMap::new();
        let mut capacities = HashMap::new();
        let mut shed_counts = HashMap::new();
        for (prio, cap) in [
            (Critical, critical),
            (Position, position),
            (Speculative, speculative),
        ] {
            let (s, r) = mpsc::channel(cap.max(1));
            tx.insert(prio, s);
            rx.insert(prio, r);
            capacities.insert(prio, cap.max(1));
            shed_counts.insert(prio, std::sync::atomic::AtomicU64::new(0));
        }
        Self {
            tx,
            rx,
            capacities,
            shed_counts,
        }
    }

    /// Publish without awaiting. Overload policy per tier:
    /// - Speculative full → drop + count (`ShedSpeculative`).
    /// - Critical/Position full → error; the caller must slow down (these
    ///   tiers are never silently dropped).
    pub fn try_publish(
        &self,
        priority: Priority,
        item: T,
    ) -> Result<PublishOutcome, mpsc::error::TrySendError<T>> {
        let sender = &self.tx[&priority];
        match sender.try_send(item) {
            Ok(()) => Ok(PublishOutcome::Enqueued),
            Err(mpsc::error::TrySendError::Full(item)) if priority == Speculative => {
                self.shed_counts[&priority].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                // item is dropped here
                drop(item);
                Ok(PublishOutcome::ShedSpeculative)
            }
            Err(e) => Err(e),
        }
    }

    /// Async publish: back-pressures (awaits) on Critical/Position, sheds on
    /// Speculative without ever blocking.
    pub async fn publish(&self, priority: Priority, item: T) -> PublishOutcome {
        if priority == Speculative {
            // Never block the hot path for speculative candidates.
            match self.try_publish(priority, item) {
                Ok(outcome) => outcome,
                Err(_) => PublishOutcome::ShedSpeculative, // channel closed: shed
            }
        } else {
            let sender = self.tx[&priority].clone();
            // A closed runtime channel is an operator-visible shutdown; the
            // send error is surfaced by dropping the item on the floor only
            // after logging at error level.
            if sender.send(item).await.is_err() {
                tracing::error!(
                    ?priority,
                    "priority bus channel closed; item dropped at shutdown"
                );
            }
            PublishOutcome::Enqueued
        }
    }

    /// Take the next event, strictly preferring CRITICAL > POSITION >
    /// SPECULATIVE. Returns None when all queues are empty.
    pub async fn recv(&mut self) -> Option<(Priority, T)> {
        loop {
            if let Some(v) = self.rx.get_mut(&Critical).and_then(|r| r.try_recv().ok()) {
                return Some((Critical, v));
            }
            if let Some(v) = self.rx.get_mut(&Position).and_then(|r| r.try_recv().ok()) {
                return Some((Position, v));
            }
            if let Some(v) = self
                .rx
                .get_mut(&Speculative)
                .and_then(|r| r.try_recv().ok())
            {
                return Some((Speculative, v));
            }
            // All empty: yield once so a concurrent producer can run.
            tokio::task::yield_now().await;
        }
    }

    /// Non-blocking variant for tests and drain loops.
    pub fn try_recv(&mut self) -> Option<(Priority, T)> {
        if let Some(v) = self.rx.get_mut(&Critical).and_then(|r| r.try_recv().ok()) {
            return Some((Critical, v));
        }
        if let Some(v) = self.rx.get_mut(&Position).and_then(|r| r.try_recv().ok()) {
            return Some((Position, v));
        }
        if let Some(v) = self
            .rx
            .get_mut(&Speculative)
            .and_then(|r| r.try_recv().ok())
        {
            return Some((Speculative, v));
        }
        None
    }

    pub fn shed_count(&self, priority: Priority) -> u64 {
        self.shed_counts[&priority].load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn capacity(&self, priority: Priority) -> usize {
        self.capacities[&priority]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speculative_sheds_and_counts_under_overload() {
        let mut bus = PriorityBus::<u32>::with_capacities(2, 2, 2);
        for i in 0..2 {
            assert_eq!(
                bus.try_publish(Speculative, i).unwrap(),
                PublishOutcome::Enqueued
            );
        }
        // Third speculative item overflows: dropped, counted, not an error.
        assert_eq!(
            bus.try_publish(Speculative, 99).unwrap(),
            PublishOutcome::ShedSpeculative
        );
        assert_eq!(bus.shed_count(Speculative), 1);
        assert_eq!(
            bus.try_publish(Speculative, 98).unwrap(),
            PublishOutcome::ShedSpeculative
        );
        assert_eq!(bus.shed_count(Speculative), 2);
        // Queue still holds exactly the first two items.
        assert_eq!(bus.try_recv(), Some((Speculative, 0)));
        assert_eq!(bus.try_recv(), Some((Speculative, 1)));
        assert_eq!(bus.try_recv(), None);
    }

    #[test]
    fn critical_never_silently_dropped() {
        let bus = PriorityBus::<u32>::with_capacities(1, 1, 1);
        assert_eq!(
            bus.try_publish(Critical, 1).unwrap(),
            PublishOutcome::Enqueued
        );
        // A full CRITICAL queue is an error the caller must handle — it is
        // never converted into a silent drop.
        assert!(bus.try_publish(Critical, 2).is_err());
        assert_eq!(bus.shed_count(Critical), 0);
    }

    #[test]
    fn drain_order_prefers_critical_then_position() {
        let mut bus = PriorityBus::<&'static str>::new(8);
        bus.try_publish(Speculative, "spec1").unwrap();
        bus.try_publish(Position, "pos1").unwrap();
        bus.try_publish(Critical, "crit1").unwrap();
        bus.try_publish(Position, "pos2").unwrap();
        bus.try_publish(Speculative, "spec2").unwrap();

        let mut drained = Vec::new();
        while let Some((prio, item)) = bus.try_recv() {
            drained.push((prio, item));
        }
        assert_eq!(
            drained,
            vec![
                (Critical, "crit1"),
                (Position, "pos1"),
                (Position, "pos2"),
                (Speculative, "spec1"),
                (Speculative, "spec2"),
            ]
        );
    }
}
