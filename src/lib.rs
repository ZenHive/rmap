pub mod bundles;
pub mod critical_path;
pub mod delegate;
pub mod diff;
pub mod doctor;
pub mod export;
pub mod graph_export;
pub mod import;
pub mod milestones;
pub mod mutate;
pub mod next;
pub mod next_bundle;
pub mod paths;
pub mod query;
pub mod render;
pub mod render_html;
pub mod schema;
pub mod schema_json;
pub mod scoring;
pub mod stale;
pub mod topo;
pub mod validate;
pub mod watch;

pub use stale::{find_awaiting_landing, find_stale, parse_duration};

/// Convert epoch days (days since 1970-01-01) to a (year, month, day) triple.
///
/// Adapted from Howard Hinnant's `civil_from_days` (public domain).
/// See https://howardhinnant.github.io/date_algorithms.html
///
/// This implementation is deliberately dependency-free (no `chrono`, `time`,
/// or similar crates) to keep cold-start latency under the ~10 ms target
/// documented in DESIGN.md. All date math used by rmap (both directions)
/// lives in this crate so it can be tested and evolved together.
pub(crate) fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i32 + (era as i32) * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Returns today's date as a `YYYY-MM-DD` string.
/// If the `RMAP_TODAY` environment variable is set and non-empty, its value is returned
/// directly (useful for deterministic tests and golden fixtures).
pub fn today_iso() -> String {
    if let Ok(v) = std::env::var("RMAP_TODAY")
        && !v.is_empty()
    {
        return v;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let days_since_epoch = (now.as_secs() / 86_400) as i64;
    let (y, m, d) = civil_from_days(days_since_epoch);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// Unit tests for the real-clock date conversion path (civil_from_days).
// The env-var fast path of today_iso() is exercised by virtually every
// integration test via RMAP_TODAY; these tests focus on the pure conversion
// and its roundtrip relationship with scoring::days_from_civil.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::days_from_civil;

    // Representative dates chosen for coverage: epoch, leaps, pre-epoch,
    // year boundaries, a recent dev date, and a far-future non-leap century.
    const DATE_CASES: &[(i32, u32, u32)] = &[
        (1970, 1, 1), // Unix epoch day 0
        (1970, 1, 2),
        (1969, 12, 31), // day before epoch
        (1969, 7, 20),  // Apollo 11 (pre-epoch)
        (2000, 2, 29),  // leap day (Y2K)
        (2024, 2, 29),  // recent leap day
        (2026, 5, 11),  // score_decay golden fixture date
        (2026, 5, 19),
        (2100, 1, 1), // non-leap century year
    ];

    #[test]
    fn civil_from_days_roundtrips_through_days_from_civil() {
        for &(y, m, d) in DATE_CASES {
            let z = days_from_civil(y, m, d);
            let (y2, m2, d2) = civil_from_days(z);
            assert_eq!(
                (y2, m2, d2),
                (y, m, d),
                "civil_from_days(days_from_civil) roundtrip failed for {y:04}-{m:02}-{d:02}"
            );
        }
    }

    #[test]
    fn days_from_civil_roundtrips_through_civil_from_days() {
        for &(y, m, d) in DATE_CASES {
            let z = days_from_civil(y, m, d);
            let (y2, m2, d2) = civil_from_days(z);
            let z2 = days_from_civil(y2, m2, d2);
            assert_eq!(
                z2, z,
                "days_from_civil(civil_from_days) roundtrip failed for z={z} ({y:04}-{m:02}-{d:02})"
            );
        }
    }

    // Direct known-value checks for the most critical points (no reliance on
    // the inverse function for these assertions).
    #[test]
    fn civil_from_days_known_epoch_values() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn civil_from_days_leap_day_2000() {
        let z = days_from_civil(2000, 2, 29);
        assert_eq!(civil_from_days(z), (2000, 2, 29));
    }

    #[test]
    fn civil_from_days_far_future() {
        // 2100-01-01 is not a leap year (century rule)
        let z = days_from_civil(2100, 1, 1);
        assert_eq!(civil_from_days(z), (2100, 1, 1));
        // 9999-12-31 — stress the era math path with a large positive z
        let z = days_from_civil(9999, 12, 31);
        assert_eq!(civil_from_days(z), (9999, 12, 31));
    }

    /// today_iso() must always return a well-formed YYYY-MM-DD string,
    /// regardless of whether RMAP_TODAY is set (fast path) or the real
    /// SystemTime path is taken. We assert shape + a plausible year range.
    #[test]
    fn today_iso_always_produces_well_formed_date() {
        let s = today_iso();
        assert!(
            s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-',
            "today_iso() produced malformed date: {s:?}"
        );
        // Basic sanity: year is plausible for any machine running the suite
        // in the 2026+ timeframe (covers both env and real-clock paths).
        let y: i32 = s[0..4].parse().expect("year");
        assert!(
            (1970..=2100).contains(&y),
            "implausible year from today_iso: {y}"
        );
    }
}
