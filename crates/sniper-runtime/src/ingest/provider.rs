//! Provider abstraction: one trait over every ingestion surface, with a
//! bounded reconnection policy.

use std::time::Duration;

use sniper_core::event::ProviderKind;

use crate::ingest::frame::RawObservation;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("provider not configured: {0}")]
    NotConfigured(String),
    #[error("subscription rejected: {0}")]
    Rejected(String),
}

/// A streaming source of raw observations. Implementations: WSS pubsub
/// (logs/blocks), Yellowstone gRPC, fixture replay. Static dispatch; the
/// supervisor drives any implementation.
/// Static-dispatch by design (the supervisor is generic); the RPITIT futures
/// are `Send` for the concrete providers used here — enforced by the
/// supervisor's own `Send` bounds at spawn sites.
#[allow(async_fn_in_trait)]
pub trait EventProvider: Send {
    fn kind(&self) -> ProviderKind;

    /// (Re)establish the connection/subscription. Must be idempotent.
    async fn connect(&mut self) -> Result<(), ProviderError>;

    /// Next observation from the stream. An `Err` signals a broken stream —
    /// the supervisor reconnects with backoff; the provider must not retry
    /// internally (single owner of retry policy).
    async fn next_observation(&mut self) -> Result<RawObservation, ProviderError>;
}

/// Exponential backoff with a hard cap and a reset on sustained success.
#[derive(Debug, Clone, PartialEq)]
pub struct ReconnectPolicy {
    pub initial: Duration,
    pub max: Duration,
    pub multiplier: f32,
    /// Consecutive successes required before the delay resets to `initial`.
    pub reset_after_successes: u32,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_millis(500),
            max: Duration::from_secs(30),
            multiplier: 2.0,
            reset_after_successes: 10,
        }
    }
}

impl ReconnectPolicy {
    /// Next delay after the n-th consecutive failure (n starts at 1).
    pub fn delay_after_failures(&self, consecutive_failures: u32) -> Duration {
        let mut d = self.initial;
        for _ in 1..consecutive_failures.max(1) {
            let next = Duration::from_secs_f64(d.as_secs_f64() * self.multiplier as f64);
            if next >= self.max {
                return self.max;
            }
            d = next;
        }
        d.min(self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_until_cap() {
        let p = ReconnectPolicy {
            initial: Duration::from_millis(500),
            max: Duration::from_secs(2),
            multiplier: 2.0,
            reset_after_successes: 10,
        };
        assert_eq!(p.delay_after_failures(1), Duration::from_millis(500));
        assert_eq!(p.delay_after_failures(2), Duration::from_secs(1));
        assert_eq!(p.delay_after_failures(3), Duration::from_secs(2));
        assert_eq!(p.delay_after_failures(9), Duration::from_secs(2), "capped");
    }

    #[test]
    fn default_policy_bounds() {
        let p = ReconnectPolicy::default();
        assert!(p.delay_after_failures(1000) <= p.max);
        assert!(p.initial < p.max);
    }
}
