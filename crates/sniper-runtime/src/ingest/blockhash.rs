//! Blockhash cache with explicit freshness bounds. A stale blockhash is
//! returned as `None` — callers must refresh rather than build with a hash
//! that will fail `BlockhashNotFound` at landing time (observed live on
//! LiteSVM during the M0 spike: consecutive sends need re-fetched hashes).

use std::time::{Duration, Instant};

use solana_sdk::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    hash: Hash,
    slot: u64,
    fetched_at: Instant,
}

#[derive(Debug)]
pub struct BlockhashCache {
    entry: Option<Entry>,
    max_age: Duration,
    max_slot_age: u64,
}

impl BlockhashCache {
    pub fn new(max_age: Duration, max_slot_age: u64) -> Self {
        Self {
            entry: None,
            max_age,
            max_slot_age,
        }
    }

    /// Record a freshly fetched blockhash.
    pub fn update(&mut self, hash: Hash, slot: u64, fetched_at: Instant) {
        // Only move forward: a younger observation never overwrites a newer one.
        let newer = match &self.entry {
            Some(e) => slot >= e.slot,
            None => true,
        };
        if newer {
            self.entry = Some(Entry {
                hash,
                slot,
                fetched_at,
            });
        }
    }

    /// Fresh blockhash for the given now/current-slot, or None when stale,
    /// absent, or the chain view is too old to trust.
    pub fn get(&self, now: Instant, current_slot: u64) -> Option<Hash> {
        let e = self.entry?;
        if now.duration_since(e.fetched_at) > self.max_age {
            return None;
        }
        if current_slot.saturating_sub(e.slot) > self.max_slot_age {
            return None;
        }
        Some(e.hash)
    }

    pub fn last_slot(&self) -> Option<u64> {
        self.entry.map(|e| e.slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(n: u8) -> Hash {
        Hash::from([n; 32])
    }

    #[test]
    fn fresh_within_bounds() {
        let mut c = BlockhashCache::new(Duration::from_secs(10), 150);
        let t0 = Instant::now();
        c.update(h(1), 1000, t0);
        assert_eq!(c.get(t0 + Duration::from_secs(5), 1100), Some(h(1)));
    }

    #[test]
    fn stale_by_time_returns_none() {
        let mut c = BlockhashCache::new(Duration::from_secs(10), 150);
        let t0 = Instant::now();
        c.update(h(1), 1000, t0);
        assert_eq!(c.get(t0 + Duration::from_secs(11), 1000), None);
    }

    #[test]
    fn stale_by_slot_distance_returns_none() {
        let mut c = BlockhashCache::new(Duration::from_secs(60), 150);
        let t0 = Instant::now();
        c.update(h(1), 1000, t0);
        assert_eq!(c.get(t0 + Duration::from_secs(1), 1200), None);
        assert_eq!(c.get(t0 + Duration::from_secs(1), 1150), Some(h(1)));
    }

    #[test]
    fn never_regresses_to_older_slots() {
        let mut c = BlockhashCache::new(Duration::from_secs(60), 10_000);
        let t0 = Instant::now();
        c.update(h(2), 2000, t0);
        c.update(h(1), 1500, t0 + Duration::from_secs(1)); // late, older view
        assert_eq!(c.get(t0 + Duration::from_secs(2), 2000), Some(h(2)));
        assert_eq!(c.last_slot(), Some(2000));
    }

    #[test]
    fn empty_cache_is_none() {
        let c = BlockhashCache::new(Duration::from_secs(10), 150);
        assert_eq!(c.get(Instant::now(), 100), None);
    }
}
