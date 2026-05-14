pub mod bundles;
pub mod delegate;
pub mod diff;
pub mod doctor;
pub mod export;
pub mod mutate;
pub mod next;
pub mod next_bundle;
pub mod paths;
pub mod query;
pub mod render;
pub mod schema;
pub mod schema_json;
pub mod scoring;
pub mod stale;
pub mod validate;
pub mod watch;

pub use stale::{find_stale, parse_duration};

/// Convert epoch days (days since 1970-01-01) to a (year, month, day) triple.
/// Adapted from Howard Hinnant's civil_from_days — public domain.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
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
