//! `rmap milestones` selector listing.
//!
//! Mirrors `bundles.rs` end-to-end: pure helpers (`list_milestones`,
//! `format_milestones_human`, `milestones_json`) plus a `StatusCounts` /
//! `NextTaskSummary` payload reused from the bundles module so the JSON shape
//! and the five-branch glyph ladder stay in lock-step.
//!
//! Sort order: milestone.status (active first, pending next, done last) →
//! `milestone.order` ascending. The active-first surfacing is the load-bearing
//! affordance for the "what release am I cutting next?" daily-path query —
//! milestones intentionally cross phases, so focus-phase grouping does not
//! apply (unlike bundles).

use serde::Serialize;

use crate::bundles::{NextTaskSummary, StatusCounts};
use crate::next::next_task;
use crate::query::TaskFilter;
use crate::schema::{Task, Tasks};

#[derive(Debug, Default)]
pub struct MilestoneFilter {
    pub has_next: bool,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MilestoneSummary<'a> {
    pub key: &'a str,
    pub name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    pub order: u32,
    pub status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_version: Option<&'a str>,
    pub task_count: u32,
    pub status_counts: StatusCounts,
    pub next_task: Option<NextTaskSummary<'a>>,
}

#[derive(Debug, Serialize)]
pub struct MilestonesJson<'a> {
    pub schema_version: u32,
    pub milestones: Vec<MilestoneSummary<'a>>,
}

/// Build `MilestoneSummary` entries for every `[milestones.*]` in `tasks`.
///
/// Per-milestone `next_task` reuses `next::next_task` with a milestone-restricted
/// `TaskFilter` — single source of truth for dep-satisfaction. Sort order:
/// milestone status (active > pending > done) then `milestone.order` ascending.
pub fn list_milestones<'a>(
    tasks: &'a Tasks,
    filter: &MilestoneFilter,
) -> Vec<MilestoneSummary<'a>> {
    let mut entries: Vec<MilestoneSummary<'a>> = tasks
        .milestones
        .iter()
        .map(|(key, milestone)| {
            let mut counts = StatusCounts::default();
            for task in tasks
                .task
                .iter()
                .filter(|task| task.milestone.as_deref() == Some(key.as_str()))
            {
                bump_status(&mut counts, task);
            }
            let task_count = counts.pending + counts.in_progress + counts.done + counts.blocked;

            let next_filter = TaskFilter {
                milestone: Some(key.clone()),
                ..Default::default()
            };
            let next = next_task(tasks, &next_filter).map(next_task_summary);

            MilestoneSummary {
                key: key.as_str(),
                name: milestone.name.as_str(),
                description: milestone.description.as_deref(),
                order: milestone.order,
                status: milestone.status.as_str(),
                target_version: milestone.target_version.as_deref(),
                task_count,
                status_counts: counts,
                next_task: next,
            }
        })
        .filter(|summary| !filter.has_next || summary.next_task.is_some())
        .filter(|summary| {
            filter
                .status
                .as_deref()
                .is_none_or(|status| summary.status == status)
        })
        .collect();

    entries.sort_by_key(|summary| (status_rank(summary.status), summary.order));
    entries
}

/// Render the human-readable `rmap milestones` table.
///
/// Empty-state rules: when `tasks.milestones.is_empty()`, returns
/// `"(no milestones declared)\n"`. When milestones exist but `summaries` is
/// empty (post-filter), returns `"(no milestones match filter)\n"`. Otherwise
/// one row per milestone, sorted active-first.
pub fn format_milestones_human(tasks: &Tasks, summaries: &[MilestoneSummary<'_>]) -> String {
    if tasks.milestones.is_empty() {
        return "(no milestones declared)\n".to_string();
    }
    if summaries.is_empty() {
        return "(no milestones match filter)\n".to_string();
    }

    let mut out = String::new();
    for summary in summaries {
        out.push_str(&format_milestone_row(summary));
        out.push('\n');
    }
    out
}

/// JSON envelope for `rmap milestones --json`.
pub fn milestones_json<'a>(
    tasks: &'a Tasks,
    summaries: Vec<MilestoneSummary<'a>>,
) -> MilestonesJson<'a> {
    MilestonesJson {
        schema_version: tasks.schema_version,
        milestones: summaries,
    }
}

fn format_milestone_row(summary: &MilestoneSummary<'_>) -> String {
    let target = match summary.target_version {
        Some(v) => format!(" [target={v}]"),
        None => String::new(),
    };
    format!(
        "  {}    {}/{} {}  [{}]  — {}{}",
        summary.key,
        summary.status_counts.done,
        summary.task_count,
        status_glyph_or_next(summary),
        summary.status,
        summary.name,
        target,
    )
}

fn status_glyph_or_next(summary: &MilestoneSummary<'_>) -> String {
    let counts = &summary.status_counts;
    // Five-branch ladder mirrors `bundles::status_glyph_or_next` exactly:
    // (1) all-done → ✅
    if counts.done == summary.task_count {
        return "✅".to_string();
    }
    // (2) pending == 0 && in_progress > 0 → 🚧
    if counts.pending == 0 && counts.in_progress > 0 {
        return "🚧".to_string();
    }
    // (3) pending == 0 && in_progress == 0 && blocked > 0 → all-blocked ⛔
    if counts.pending == 0 && counts.in_progress == 0 && counts.blocked > 0 {
        return "all-blocked ⛔".to_string();
    }
    // (4) pending > 0 and next_task is None → pending:<n> (deps unmet) ⏸
    // (5) otherwise → next:<id> [Eff:<eff>] <tier_glyph>
    use crate::scoring::{format_efficiency, tier_glyph};
    match &summary.next_task {
        Some(next) => format!(
            "next:{} [Eff:{}] {}",
            next.id,
            format_efficiency(next.eff),
            tier_glyph(next.eff),
        ),
        None => format!("pending:{} (deps unmet) ⏸", counts.pending),
    }
}

fn bump_status(counts: &mut StatusCounts, task: &Task) {
    match task.status.as_str() {
        "pending" => counts.pending += 1,
        "in_progress" => counts.in_progress += 1,
        "done" => counts.done += 1,
        "blocked" => counts.blocked += 1,
        _ => {}
    }
}

fn next_task_summary(task: &Task) -> NextTaskSummary<'_> {
    NextTaskSummary {
        id: &task.id,
        title: task.title.as_str(),
        eff: crate::scoring::rounded_efficiency(task),
    }
}

fn status_rank(status: &str) -> u8 {
    match status {
        "active" => 0,
        "pending" => 1,
        "done" => 2,
        _ => 3,
    }
}
