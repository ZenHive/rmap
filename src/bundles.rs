use serde::Serialize;

use crate::next::next_task;
use crate::query::TaskFilter;
use crate::schema::{Task, TaskId, Tasks};
use crate::scoring::{format_efficiency, rounded_efficiency, tier_glyph};

#[derive(Debug, Default)]
pub struct BundleFilter {
    pub phase: Option<u32>,
    pub has_next: bool,
    pub in_focus: bool,
}

#[derive(Debug, Serialize)]
pub struct BundleSummary<'a> {
    pub name: &'a str,
    pub phase: u32,
    pub phase_name: &'a str,
    pub order: u32,
    pub description: &'a str,
    pub task_count: u32,
    pub status_counts: StatusCounts,
    pub next_task: Option<NextTaskSummary<'a>>,
    pub in_focus: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct StatusCounts {
    pub pending: u32,
    pub in_progress: u32,
    pub done: u32,
    pub blocked: u32,
}

#[derive(Debug, Serialize)]
pub struct NextTaskSummary<'a> {
    pub id: &'a TaskId,
    pub title: &'a str,
    pub eff: f64,
}

#[derive(Debug, Serialize)]
pub struct BundlesJson<'a> {
    pub schema_version: u32,
    pub focus_phase: Option<u32>,
    pub bundles: Vec<BundleSummary<'a>>,
}

/// Build `BundleSummary` entries for every `[bundles.*]` in `tasks`.
///
/// Per-bundle `next_task` reuses `next::next_task` with a bundle-restricted
/// `TaskFilter` — single source of truth for dep-satisfaction. Sort order:
/// focus-phase bundles first, then `phase.order` asc, then `bundle.order` asc.
pub fn list_bundles<'a>(tasks: &'a Tasks, filter: &BundleFilter) -> Vec<BundleSummary<'a>> {
    let focus_phase = tasks.focus.as_ref().map(|focus| focus.phase);

    let mut entries: Vec<(u32, BundleSummary<'a>)> = tasks
        .bundles
        .iter()
        .filter_map(|(name, bundle)| {
            let phase = tasks.phases.get(&bundle.phase.to_string())?;
            let mut counts = StatusCounts::default();
            for task in tasks.task.iter().filter(|task| task.bundle == *name) {
                bump_status(&mut counts, task);
            }
            let task_count = counts.pending + counts.in_progress + counts.done + counts.blocked;

            let next_filter = TaskFilter {
                bundle: Some(name.clone()),
                ..Default::default()
            };
            let next = next_task(tasks, &next_filter).map(next_task_summary);

            let in_focus = focus_phase == Some(bundle.phase);

            Some((
                phase.order,
                BundleSummary {
                    name: name.as_str(),
                    phase: bundle.phase,
                    phase_name: phase.name.as_str(),
                    order: bundle.order,
                    description: bundle.description.as_str(),
                    task_count,
                    status_counts: counts,
                    next_task: next,
                    in_focus,
                },
            ))
        })
        .filter(|(_, summary)| filter.phase.is_none_or(|phase| summary.phase == phase))
        .filter(|(_, summary)| !filter.has_next || summary.next_task.is_some())
        .filter(|(_, summary)| !filter.in_focus || summary.in_focus)
        .collect();

    entries.sort_by_key(|(phase_order, summary)| {
        (u8::from(!summary.in_focus), *phase_order, summary.order)
    });

    entries.into_iter().map(|(_, summary)| summary).collect()
}

/// Render the human-readable `rmap bundles` table.
///
/// Empty-state rules: when `tasks.bundles.is_empty()`, returns
/// `"(no bundles declared)\n"`. When bundles exist but `summaries` is empty
/// (post-filter), returns `"(no bundles match filter)\n"`. Otherwise emits
/// per-phase headers followed by per-bundle rows.
pub fn format_bundles_human(tasks: &Tasks, summaries: &[BundleSummary<'_>]) -> String {
    if tasks.bundles.is_empty() {
        return "(no bundles declared)\n".to_string();
    }
    if summaries.is_empty() {
        return "(no bundles match filter)\n".to_string();
    }

    let mut out = String::new();
    let mut current_phase: Option<u32> = None;
    for summary in summaries {
        if current_phase != Some(summary.phase) {
            if current_phase.is_some() {
                out.push('\n');
            }
            let phase_status = tasks
                .phases
                .get(&summary.phase.to_string())
                .map(|phase| phase.status.as_str())
                .unwrap_or("unknown");
            let focus_tag = if summary.in_focus { ", focus" } else { "" };
            out.push_str(&format!(
                "phase {} — {}  [{}{}]\n",
                summary.phase, summary.phase_name, phase_status, focus_tag
            ));
            current_phase = Some(summary.phase);
        }
        out.push_str(&format_bundle_row(summary));
        out.push('\n');
    }
    out
}

/// JSON envelope for `rmap bundles --json`. `focus_phase` serializes as `null`
/// when unset (deliberately not skipped).
pub fn bundles_json<'a>(tasks: &'a Tasks, summaries: Vec<BundleSummary<'a>>) -> BundlesJson<'a> {
    BundlesJson {
        schema_version: tasks.schema_version,
        focus_phase: tasks.focus.as_ref().map(|focus| focus.phase),
        bundles: summaries,
    }
}

fn format_bundle_row(summary: &BundleSummary<'_>) -> String {
    format!(
        "  {}    {}/{} {}  — {}",
        summary.name,
        summary.status_counts.done,
        summary.task_count,
        status_glyph_or_next(summary),
        summary.description,
    )
}

fn status_glyph_or_next(summary: &BundleSummary<'_>) -> String {
    let counts = &summary.status_counts;
    // Rules evaluated in AC order:
    // (1) all-done → ✅  (vacuously true when task_count == 0)
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
        _ => {} // schema validation rejects unknown statuses
    }
}

fn next_task_summary(task: &Task) -> NextTaskSummary<'_> {
    NextTaskSummary {
        id: &task.id,
        title: task.title.as_str(),
        eff: rounded_efficiency(task),
    }
}
