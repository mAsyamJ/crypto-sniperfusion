//! Dual timestamps: wall-clock for correlation, monotonic for latency math.

use std::time::{Instant, SystemTime};

/// A point in time captured with both clocks. `mono_ns` is nanoseconds since
/// the owning process's epoch (or a synthetic epoch under replay).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Timestamps {
    pub wall: SystemTime,
    pub mono_ns: u64,
}

/// Process-wide monotonic reference. Replay uses a synthetic instance so runs
/// are deterministic; live uses the real clock.
#[derive(Debug)]
pub struct MonotonicClock {
    epoch: Instant,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }

    pub fn now(&self) -> Timestamps {
        Timestamps {
            wall: SystemTime::now(),
            mono_ns: self.epoch.elapsed().as_nanos() as u64,
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_ns_is_monotonic() {
        let clock = MonotonicClock::new();
        let a = clock.now().mono_ns;
        std::thread::sleep(std::time::Duration::from_micros(200));
        let b = clock.now().mono_ns;
        assert!(b > a, "monotonic clock must not go backwards: {b} <= {a}");
    }
}
