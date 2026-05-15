use crate::schema::{Task, Tasks};
use crate::scoring::days_since;

#[derive(Debug, thiserror::Error)]
pub enum StaleError {
    #[error("invalid duration {0:?}: expected something like '7d', '2w', '6m', or '1y'")]
    InvalidDuration(String),
}

/// Parses durations of the form `<N><unit>` where unit ∈ {d, w, m, y}.
/// Units: d = 1 day, w = 7, m = 30, y = 365 (calendar-imprecise but fine for staleness).
pub fn parse_duration(input: &str) -> Result<u32, StaleError> {
    if input.len() < 2 {
        return Err(StaleError::InvalidDuration(input.into()));
    }
    let (num, unit) = input.split_at(input.len() - 1);
    let n: u32 = num
        .parse()
        .map_err(|_| StaleError::InvalidDuration(input.into()))?;
    let mult = match unit {
        "d" => 1u32,
        "w" => 7,
        "m" => 30,
        "y" => 365,
        _ => return Err(StaleError::InvalidDuration(input.into())),
    };
    n.checked_mul(mult)
        .ok_or_else(|| StaleError::InvalidDuration(input.into()))
}

/// Returns in-progress tasks whose `started_at` is older than `max_age_days` from `today`.
/// Tasks without `started_at`, or with malformed dates, are skipped (validation rejects malformed
/// dates upstream — defensive skip rather than panic).
pub fn find_stale<'a>(tasks: &'a Tasks, max_age_days: u32, today: &str) -> Vec<&'a Task> {
    tasks
        .task
        .iter()
        .filter(|t| t.status == "in_progress")
        .filter(|t| match t.started_at.as_deref() {
            Some(s) => days_since(today, s)
                .map(|d| d > i64::from(max_age_days))
                .unwrap_or(false),
            None => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Scores, Task, TaskId, Tasks};
    use std::collections::BTreeMap;

    fn make_task(id: u32, status: &str, started_at: Option<&str>) -> Task {
        Task {
            id: TaskId::Number(id),
            phase: 1,
            bundle: "test".to_string(),
            status: status.to_string(),
            title: format!("Task {id}"),
            scores: Scores { d: 2, b: 4, u: 4 },
            markers: vec![],
            depends_on: vec![],
            linear_id: None,
            assignee: None,
            module: None,
            branch: None,
            model: None,
            acceptance_criteria: vec![],
            out_of_scope: vec![],
            files_to_modify: vec![],
            shipped_in: None,
            body: None,
            created_at: None,
            started_at: started_at.map(String::from),
            done_at: None,
            scored_at: None,
            blocked_reason: None,
            cross_repo: vec![],
        }
    }

    fn make_tasks(task: Vec<Task>) -> Tasks {
        Tasks {
            schema_version: 1,
            project: "test".to_string(),
            default_branch: "main".to_string(),
            vision: None,
            focus: None,
            linear: None,
            phases: BTreeMap::new(),
            bundles: BTreeMap::new(),
            task,
        }
    }

    // parse_duration: positive cases
    #[test]
    fn parse_duration_days() {
        assert_eq!(parse_duration("7d").unwrap(), 7);
    }

    #[test]
    fn parse_duration_weeks() {
        assert_eq!(parse_duration("2w").unwrap(), 14);
    }

    #[test]
    fn parse_duration_months() {
        assert_eq!(parse_duration("6m").unwrap(), 180);
    }

    #[test]
    fn parse_duration_years() {
        assert_eq!(parse_duration("1y").unwrap(), 365);
    }

    #[test]
    fn parse_duration_large() {
        assert_eq!(parse_duration("30d").unwrap(), 30);
    }

    // parse_duration: negative cases
    #[test]
    fn parse_duration_missing_unit() {
        assert!(parse_duration("7").is_err());
    }

    #[test]
    fn parse_duration_empty() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_duration_bad_unit() {
        assert!(parse_duration("7x").is_err());
    }

    #[test]
    fn parse_duration_non_numeric() {
        assert!(parse_duration("abcd").is_err());
    }

    #[test]
    fn parse_duration_single_char() {
        assert!(parse_duration("d").is_err());
    }

    // find_stale tests
    #[test]
    fn find_stale_returns_only_old_in_progress() {
        // today = 2026-05-11
        // Task 1: in_progress, started 2026-03-01 (71 days ago) — stale at 30d
        // Task 2: in_progress, started 2026-05-05 (6 days ago) — NOT stale
        // Task 3: done, started 2026-01-01 — NOT in stale (wrong status)
        // Task 4: pending, no started_at — NOT stale
        let tasks = make_tasks(vec![
            make_task(1, "in_progress", Some("2026-03-01")),
            make_task(2, "in_progress", Some("2026-05-05")),
            make_task(3, "done", Some("2026-01-01")),
            make_task(4, "pending", None),
        ]);

        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, 1u32);
    }

    #[test]
    fn find_stale_empty_when_all_fresh() {
        let tasks = make_tasks(vec![
            make_task(1, "in_progress", Some("2026-05-10")),
            make_task(2, "in_progress", Some("2026-05-09")),
        ]);
        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert!(stale.is_empty());
    }

    #[test]
    fn find_stale_skips_missing_started_at() {
        let tasks = make_tasks(vec![make_task(1, "in_progress", None)]);
        let stale = find_stale(&tasks, 7, "2026-05-11");
        assert!(stale.is_empty());
    }

    #[test]
    fn find_stale_boundary_not_stale_at_exact_threshold() {
        // exactly 30 days: not stale (> not >=)
        let tasks = make_tasks(vec![make_task(1, "in_progress", Some("2026-04-11"))]);
        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert!(stale.is_empty());
    }

    #[test]
    fn find_stale_boundary_stale_one_over_threshold() {
        // 31 days: stale
        let tasks = make_tasks(vec![make_task(1, "in_progress", Some("2026-04-10"))]);
        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert_eq!(stale.len(), 1);
    }
}
