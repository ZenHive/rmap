use crate::schema::Task;

pub fn efficiency(task: &Task) -> f64 {
    f64::from(task.scores.b + task.scores.u) / (2.0 * f64::from(task.scores.d))
}

pub fn format_efficiency(value: f64) -> String {
    let formatted = format!("{value:.2}");
    let trimmed = formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();

    if trimmed.contains('.') {
        trimmed
    } else {
        format!("{trimmed}.0")
    }
}

/// Tier glyph for a computed efficiency value. Single source of truth for the
/// D/B/U rubric — `>= 2.0` is 🎯 (highest priority), `>= 1.5` is 🚀,
/// `>= 1.0` is 📋, anything below is ⚠️. The thresholds and glyphs are part of
/// the agent-grep contract; renaming or shifting them is a breaking change.
pub fn tier_glyph(efficiency: f64) -> &'static str {
    if efficiency >= 2.0 {
        "🎯"
    } else if efficiency >= 1.5 {
        "🚀"
    } else if efficiency >= 1.0 {
        "📋"
    } else {
        "⚠️"
    }
}

/// Round an efficiency value to 2 decimals for emission. `data.json`'s `eff`
/// field, `rmap bundles` summaries, and the HTML `data-eff` attribute all agree
/// because they share this single rounding.
pub fn round_eff(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// `round_eff(efficiency(task))` — the common case for emitting a task's
/// efficiency.
pub fn rounded_efficiency(task: &Task) -> f64 {
    round_eff(efficiency(task))
}

/// Score-decay threshold in days. A `scored_at` older than this — or missing —
/// flags the task as having a stale score in `render` and `doctor`.
pub const SCORE_DECAY_DAYS: i64 = 30;

// ---------------------------------------------------------------------------
// Date math — Howard Hinnant days_from_civil (public domain)
// ---------------------------------------------------------------------------

/// Days since 1970-01-01 (epoch) for a proleptic Gregorian date.
pub(crate) fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy: u32 = (153 * mp + 2) / 5 + d - 1;
    let doe: u32 = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

fn parse_iso(s: &str) -> Option<(i32, u32, u32)> {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let y: i32 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    Some((y, m, d))
}

/// Returns `Some(days)` where `days = today - then` (positive when `then` is in the past).
/// Returns `None` if either string fails to parse as YYYY-MM-DD.
pub fn days_since(today: &str, then: &str) -> Option<i64> {
    let (ty, tm, td) = parse_iso(today)?;
    let (ay, am, ad) = parse_iso(then)?;
    Some(days_from_civil(ty, tm, td) - days_from_civil(ay, am, ad))
}

/// Returns `"?"` if the task's score is decayed (older than `SCORE_DECAY_DAYS` or `scored_at` missing).
/// Returns `""` otherwise. If `today` is empty, decay check is disabled and always returns `""`.
pub fn score_decay_suffix(task: &Task, today: &str) -> &'static str {
    if today.is_empty() {
        return "";
    }
    match task.scored_at.as_deref() {
        None => "?",
        Some(scored) => match days_since(today, scored) {
            Some(d) if d > SCORE_DECAY_DAYS => "?",
            _ => "",
        },
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_since_same_day() {
        assert_eq!(days_since("2026-05-11", "2026-05-11"), Some(0));
    }

    #[test]
    fn days_since_one_day() {
        assert_eq!(days_since("2026-05-11", "2026-05-10"), Some(1));
    }

    #[test]
    fn days_since_31_days() {
        assert_eq!(days_since("2026-05-11", "2026-04-10"), Some(31));
    }

    #[test]
    fn days_since_across_year_boundary() {
        // 2026-01-01 to 2025-12-01 = 31 days
        assert_eq!(days_since("2026-01-01", "2025-12-01"), Some(31));
    }

    #[test]
    fn days_since_malformed_input() {
        assert_eq!(days_since("not-a-date", "2026-05-11"), None);
        assert_eq!(days_since("2026-05-11", "bad"), None);
        assert_eq!(days_since("", "2026-05-11"), None);
    }

    #[test]
    fn tier_glyph_boundaries() {
        assert_eq!(tier_glyph(2.5), "🎯");
        assert_eq!(tier_glyph(2.0), "🎯");
        assert_eq!(tier_glyph(1.99), "🚀");
        assert_eq!(tier_glyph(1.5), "🚀");
        assert_eq!(tier_glyph(1.49), "📋");
        assert_eq!(tier_glyph(1.0), "📋");
        assert_eq!(tier_glyph(0.99), "⚠️");
        assert_eq!(tier_glyph(0.0), "⚠️");
    }
}
