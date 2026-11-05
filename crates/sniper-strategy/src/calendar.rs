//! PF1 calendar extension (DASH-02, ADR-0013): producer-side daily
//! realized-PnL bucketing with an exact reconciliation invariant.
//!
//! This is the ONLY new financial aggregation in the Control Tower system
//! and it lives beside the accounting oracle, not in any server or browser.
//! Rules (docs/dashboard/REQUIREMENTS.md §6.3):
//! 1. Settlement-day attribution: a closed position's realized PnL lands on
//!    the day of its producer-emitted `exit_at_ms` (shifted by the requested
//!    fixed UTC offset — no DST zones allowed; the offset changes bucket
//!    labels only, never sums).
//! 2. Open positions are excluded from buckets (reported separately).
//! 3. Records lacking timestamps are never bucketed; if any exist the
//!    projection is `Partial`; if none are timestamped it is `Unavailable`.
//! 4. Reconciliation invariant: Σ buckets == Σ net PnL of timestamped closed
//!    records (exact i64 equality).
//! 5. No interpolation, no zero-filling of unknown days.
//!
//! Scope discipline: this calendar is REALIZED PnL of closed positions
//! only — it is explicitly NOT full economic PnL (no NAV marks, no
//! unrealized inventory, no fee re-attribution). Callers must surface the
//! scope label with every rendered calendar.

use std::collections::BTreeMap;

/// One producer-emitted position record, reduced to the calendar's needs.
/// Built by adapters (e.g. from `FollowerEntry`):
/// - `exit_at_ms=Some`, `net_pnl=Some` → closed + timestamped → bucketed
/// - closed but untimestamped → counted in `untimestamped_count`
/// - `exit_at_ms=None`, `net_pnl=None` → open position → `open_positions`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRecord {
    pub exit_at_ms: Option<u64>,
    pub net_pnl_lamports: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayBucket {
    /// `YYYY-MM-DD` in the requested offset (UTC when offset == 0).
    pub day: String,
    pub realized_net_pnl_lamports: i64,
    pub closed_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarStatus {
    /// Every closed record is timestamped and bucketed.
    Ok,
    /// Some closed records lack timestamps (excluded from buckets).
    Partial,
    /// No timestamped closed records exist: nothing may be shown.
    Unavailable,
}

pub const CALENDAR_SCOPE_LABEL: &str = "realized_pnl_closed_positions_only";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarProjection {
    pub strategy: String,
    pub tz_offset_secs: i64,
    /// Human label for the offset (e.g. "UTC", "UTC+07:00").
    pub tz_label: String,
    pub buckets: Vec<DayBucket>,
    pub closed_timestamped: u32,
    pub untimestamped_count: u32,
    pub open_positions: u32,
    /// Σ buckets (== Σ timestamped closed records; the invariant).
    pub reconciled_total_lamports: i64,
    /// Σ net PnL over ALL closed records (timestamped + untimestamped).
    pub closed_total_lamports: i64,
    pub status: CalendarStatus,
    /// Always [`CALENDAR_SCOPE_LABEL`]; callers must render it.
    pub scope: &'static str,
}

/// Bucket daily realized PnL. `tz_offset_secs` must be a whole-hour fixed
/// offset in `[-14h, +14h]` ( DST zones are rejected: they are not
/// deterministic day boundaries). Use `0` for canonical UTC bucketing.
pub fn bucket_daily(
    strategy: &str,
    tz_offset_secs: i64,
    records: &[CalendarRecord],
) -> CalendarProjection {
    assert_valid_offset(tz_offset_secs);
    let mut per_day: BTreeMap<String, (i64, u32)> = BTreeMap::new();
    let (mut closed_ts, mut untimestamped, mut open) = (0u32, 0u32, 0u32);
    let (mut reconciled, mut closed_total) = (0i64, 0i64);

    for r in records {
        match (r.exit_at_ms, r.net_pnl_lamports) {
            (Some(_), None) | (None, Some(_)) => {
                // Closed shape but a timestamp or a PnL is missing: never
                // bucket a number we cannot place in time.
                untimestamped += 1;
                if let Some(pnl) = r.net_pnl_lamports {
                    closed_total += pnl;
                }
            }
            (Some(at_ms), Some(pnl)) => {
                let day = day_string(at_ms as i64 + tz_offset_secs * 1000);
                let slot = per_day.entry(day).or_insert((0, 0));
                slot.0 += pnl;
                slot.1 += 1;
                closed_ts += 1;
                reconciled += pnl;
                closed_total += pnl;
            }
            (None, None) => open += 1,
        }
    }

    let buckets = per_day
        .into_iter()
        .map(
            |(day, (realized_net_pnl_lamports, closed_count))| DayBucket {
                day,
                realized_net_pnl_lamports,
                closed_count,
            },
        )
        .collect();

    let status = if closed_ts > 0 {
        if untimestamped > 0 {
            CalendarStatus::Partial
        } else {
            CalendarStatus::Ok
        }
    } else {
        CalendarStatus::Unavailable
    };

    CalendarProjection {
        strategy: strategy.to_string(),
        tz_offset_secs,
        tz_label: tz_label(tz_offset_secs),
        buckets,
        closed_timestamped: closed_ts,
        untimestamped_count: untimestamped,
        open_positions: open,
        reconciled_total_lamports: reconciled,
        closed_total_lamports: closed_total,
        status,
        scope: CALENDAR_SCOPE_LABEL,
    }
}

fn assert_valid_offset(secs: i64) {
    assert!(
        secs % 3600 == 0 && secs.abs() <= 14 * 3600,
        "calendar offset must be a whole-hour fixed offset in ±14h (got {secs}s)"
    );
}

fn tz_label(secs: i64) -> String {
    if secs == 0 {
        "UTC".to_string()
    } else {
        let sign = if secs < 0 { '-' } else { '+' };
        let h = secs.abs() / 3600;
        format!("UTC{sign}{h:02}:00")
    }
}

/// `YYYY-MM-DD` for a Unix-time-in-milliseconds value (can be negative).
fn day_string(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's `civil_from_days` (proleptic Gregorian, no chrono dep).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D4_END: u64 = 1_772_236_799_000; // 2026-02-27T23:59:59Z (checked via fixture below)
    const D5_START: u64 = D4_END + 2_000; // 2026-02-28T00:00:01Z

    #[test]
    fn day_boundaries_are_utc_midnights() {
        // Pin the fixtures: 2026-02-27T23:59:59Z and 2026-02-28T00:00:01Z.
        assert_eq!(day_string(D4_END as i64), "2026-02-27");
        assert_eq!(day_string(D5_START as i64), "2026-02-28");
    }

    #[test]
    fn settlement_day_attribution() {
        // One position entered day-1, exited day-2 → lands ONLY on day-2.
        let proj = bucket_daily(
            "S",
            0,
            &[CalendarRecord {
                exit_at_ms: Some(D5_START),
                net_pnl_lamports: Some(-100),
            }],
        );
        assert_eq!(proj.buckets.len(), 1);
        assert_eq!(proj.buckets[0].day, "2026-02-28");
        assert_eq!(proj.status, CalendarStatus::Ok);
    }

    #[test]
    fn tz_offset_shifts_labels_not_sums() {
        // 23:30Z on 2026-02-27 is already 2026-02-28 at UTC+7.
        let at = D4_END - 1_740_000; // 23:30:00Z
        let utc = bucket_daily(
            "S",
            0,
            &[CalendarRecord {
                exit_at_ms: Some(at),
                net_pnl_lamports: Some(-50),
            }],
        );
        let jkt = bucket_daily(
            "S",
            7 * 3600,
            &[CalendarRecord {
                exit_at_ms: Some(at),
                net_pnl_lamports: Some(-50),
            }],
        );
        assert_eq!(utc.buckets[0].day, "2026-02-27");
        assert_eq!(jkt.buckets[0].day, "2026-02-28");
        assert_eq!(utc.reconciled_total_lamports, jkt.reconciled_total_lamports);
        assert_eq!(jkt.tz_label, "UTC+07:00");
    }
}
