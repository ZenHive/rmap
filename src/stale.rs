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

/// True when the task carries a non-blank `landing_ref` (an open PR or other
/// landing pointer). Empty/whitespace values do not count — they are not a
/// recorded landing.
pub fn has_landing_ref(task: &Task) -> bool {
    task.landing_ref
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
}

/// Returns in-progress tasks whose `started_at` is older than `max_age_days` from `today`.
/// Tasks without `started_at`, or with malformed dates, are skipped (validation rejects malformed
/// dates upstream — defensive skip rather than panic). In-progress tasks that
/// carry a `landing_ref` are excluded — they are waiting on a human merge, not
/// on an implementer; see [`find_awaiting_landing`].
pub fn find_stale<'a>(tasks: &'a Tasks, max_age_days: u32, today: &str) -> Vec<&'a Task> {
    tasks
        .task
        .iter()
        .filter(|t| t.status == "in_progress")
        .filter(|t| !has_landing_ref(t))
        .filter(|t| match t.started_at.as_deref() {
            Some(s) => days_since(today, s)
                .map(|d| d > i64::from(max_age_days))
                .unwrap_or(false),
            None => false,
        })
        .collect()
}

/// In-progress tasks that carry a `landing_ref` — waiting on a human merge,
/// not on an implementer. Independent of `started_at` age.
pub fn find_awaiting_landing(tasks: &Tasks) -> Vec<&Task> {
    tasks
        .task
        .iter()
        .filter(|t| t.status == "in_progress" && has_landing_ref(t))
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
            target_repo: None,
            milestone: None,
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
            touches: vec![],
            domains: vec![],
            shipped_in: None,
            landing_ref: None,
            body: None,
            created_at: None,
            started_at: started_at.map(String::from),
            done_at: None,
            scored_at: None,
            blocked_reason: None,
            implemented: None,
            delivered_by: None,
            verified: None,
            verified_by: None,
            verification_ref: None,
            attempts: vec![],
            cross_repo: vec![],
        }
    }

    fn make_tasks(task: Vec<Task>) -> Tasks {
        Tasks {
            schema_version: 1,
            project: "test".to_string(),
            default_branch: "main".to_string(),
            changelog_path: None,
            vision: None,
            focus: None,
            linear: None,
            phases: BTreeMap::new(),
            bundles: BTreeMap::new(),
            milestones: BTreeMap::new(),
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

    fn with_landing_ref(mut task: Task, landing_ref: &str) -> Task {
        task.landing_ref = Some(landing_ref.to_string());
        task
    }

    #[test]
    fn find_stale_excludes_in_progress_with_landing_ref() {
        let tasks = make_tasks(vec![
            with_landing_ref(
                make_task(1, "in_progress", Some("2026-03-01")),
                "https://example.com/pr/1",
            ),
            make_task(2, "in_progress", Some("2026-03-01")),
        ]);
        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, 2u32);

        let awaiting = find_awaiting_landing(&tasks);
        assert_eq!(awaiting.len(), 1);
        assert_eq!(awaiting[0].id, 1u32);
    }

    #[test]
    fn find_awaiting_landing_ignores_blank_and_non_in_progress() {
        let tasks = make_tasks(vec![
            with_landing_ref(make_task(1, "in_progress", Some("2026-03-01")), "  "),
            with_landing_ref(
                make_task(2, "done", Some("2026-03-01")),
                "https://example.com/pr/2",
            ),
            with_landing_ref(
                make_task(3, "in_progress", Some("2026-05-10")),
                "https://example.com/pr/3",
            ),
        ]);
        let awaiting = find_awaiting_landing(&tasks);
        assert_eq!(awaiting.len(), 1);
        assert_eq!(awaiting[0].id, 3u32);
        // blank landing_ref does not exclude from stale
        let stale = find_stale(&tasks, 30, "2026-05-11");
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, 1u32);
    }
}
