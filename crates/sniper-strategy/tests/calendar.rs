//! DASH-02 Task 1 — calendar producer module (T-CAL1..6 + tz).
//! Red-first: these tests were written before the implementation and
//! encode docs/dashboard/TEST_PLAN.md Task 1 exactly.

use sniper_strategy::calendar::{bucket_daily, CalendarRecord, CalendarStatus};

/// 2026-02-27T23:59:59Z / 2026-02-28T00:00:01Z (pinned in unit tests).
const D4_END: u64 = 1_772_236_799_000;
const D5_START: u64 = D4_END + 2_000;

fn rec(at: Option<u64>, pnl: Option<i64>) -> CalendarRecord {
    CalendarRecord {
        exit_at_ms: at,
        net_pnl_lamports: pnl,
    }
}

/// T-CAL1 settlement-day attribution: same-run records exiting either side
/// of UTC midnight land in different buckets; a position entered day-1 and
/// exited day-2 contributes ONLY to day-2 (entry time never buckets).
#[test]
fn t_cal1_settlement_day_attribution() {
    let proj = bucket_daily(
        "FOLLOW_DELAY_500MS",
        0,
        &[
            rec(Some(D4_END), Some(-83_436)),
            rec(Some(D5_START), Some(-83_436)),
            rec(Some(D5_START + 3_600_000), Some(20_000)),
        ],
    );
    assert_eq!(proj.buckets.len(), 2);
    assert_eq!(proj.buckets[0].day, "2026-02-27");
    assert_eq!(proj.buckets[0].realized_net_pnl_lamports, -83_436);
    assert_eq!(proj.buckets[0].closed_count, 1);
    assert_eq!(proj.buckets[1].day, "2026-02-28");
    assert_eq!(proj.buckets[1].realized_net_pnl_lamports, -63_436);
    assert_eq!(proj.buckets[1].closed_count, 2);
    assert_eq!(proj.status, CalendarStatus::Ok);
}

/// T-CAL2 sign preservation: mixed-sign day sums exactly in i64.
#[test]
fn t_cal2_sign_preservation() {
    let proj = bucket_daily(
        "S",
        0,
        &[
            rec(Some(D4_END), Some(-83_436)),
            rec(Some(D4_END - 1), Some(50_000)),
        ],
    );
    assert_eq!(proj.buckets.len(), 1);
    assert_eq!(proj.buckets[0].realized_net_pnl_lamports, -33_436);
}

/// T-CAL3 missing timestamps: partial status, untimestamped excluded from
/// buckets; open (never-exiting) records counted separately.
#[test]
fn t_cal3_partial_missing_timestamps() {
    let proj = bucket_daily(
        "S",
        0,
        &[
            rec(Some(D4_END), Some(-10)),
            rec(None, Some(-5)), // closed but untimestamped
            rec(None, None),     // open position
            rec(None, None),     // open position
        ],
    );
    assert_eq!(proj.status, CalendarStatus::Partial);
    assert_eq!(proj.untimestamped_count, 1);
    assert_eq!(proj.open_positions, 2);
    assert_eq!(proj.closed_timestamped, 1);
    assert_eq!(proj.reconciled_total_lamports, -10);
    assert_eq!(proj.closed_total_lamports, -15);
}

/// T-CAL4 reconciliation invariant over 100 fixed-seed pseudo-random cases:
/// Σ buckets == Σ timestamped closed records (exact i64), every time.
#[test]
fn t_cal4_reconciliation_property() {
    let mut state: u64 = 0x2026_1105;
    let mut lcg = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 11
    };
    for _ in 0..100 {
        let n = (lcg() % 20) as usize;
        let records: Vec<CalendarRecord> = (0..n)
            .map(|_| {
                let ts = lcg() % 2 != 0;
                let pnl = (lcg() % 2_000_001) as i64 - 1_000_000;
                rec(ts.then_some(lcg() % 1_800_000_000_000), Some(pnl))
            })
            .collect();
        let expected: i64 = records
            .iter()
            .filter(|r| r.exit_at_ms.is_some())
            .map(|r| r.net_pnl_lamports.unwrap())
            .sum();
        let proj = bucket_daily("S", 0, &records);
        assert_eq!(
            proj.reconciled_total_lamports, expected,
            "records: {records:?}"
        );
        let buckets_sum: i64 = proj
            .buckets
            .iter()
            .map(|b| b.realized_net_pnl_lamports)
            .sum();
        assert_eq!(buckets_sum, expected);
        let count_sum: u32 = proj.buckets.iter().map(|b| b.closed_count).sum();
        assert_eq!(
            count_sum as usize,
            records.iter().filter(|r| r.exit_at_ms.is_some()).count()
        );
    }
}

/// T-CAL5 all-untimestamped → Unavailable, no buckets, never zero-filled.
#[test]
fn t_cal5_all_untimestamped_unavailable() {
    let proj = bucket_daily("S", 0, &[rec(None, Some(-7)), rec(None, Some(3))]);
    assert_eq!(proj.status, CalendarStatus::Unavailable);
    assert!(proj.buckets.is_empty());
    assert_eq!(proj.untimestamped_count, 2);
    assert_eq!(proj.reconciled_total_lamports, 0);
    assert_eq!(proj.closed_total_lamports, -4);
}

/// T-CAL6 empty input → Unavailable (no activity is not zero).
#[test]
fn t_cal6_empty_input_unavailable() {
    let proj = bucket_daily("S", 0, &[]);
    assert_eq!(proj.status, CalendarStatus::Unavailable);
    assert!(proj.buckets.is_empty());
}

/// Negative-timestamp records must not panic (defensive: producer ms
/// timestamps are u64, but the day math is proven total).
#[test]
fn negative_ms_is_total() {
    let proj = bucket_daily("S", 0, &[rec(Some(0), Some(1)), rec(Some(1), Some(2))]);
    assert_eq!(proj.buckets.len(), 1);
    assert_eq!(proj.buckets[0].day, "1970-01-01");
    assert_eq!(proj.reconciled_total_lamports, 3);
}
