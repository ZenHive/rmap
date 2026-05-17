use std::cmp::Ordering;

use crate::query::{TaskFilter, matches_bundle, matches_marker, matches_milestone};
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

/// Top `count` `pending` tasks whose `depends_on` are all `done`, Eff-ranked.
///
/// Honors `filter.marker`, `filter.bundle`, and `filter.milestone`. `filter.status` is ignored
/// (next is implicitly `"pending"`-only) and `filter.phase` is ignored (focus
/// preference is sourced from `[focus].phase`, not the user). When `[focus]`
/// is set, focus-phase candidates win over higher-Eff candidates in other
/// phases — bundle filtering happens BEFORE the focus partition, so
/// `--bundle X` restricts the candidate pool first and focus only ranks
/// within the bundle.
///
/// For `count > 1`, the result fills with focus-phase candidates first (Eff
/// desc, stable on ties) and falls through to non-focus candidates only if
/// the focus pool has fewer than `count` entries. Returns up to `count`
/// tasks; fewer is fine when the eligible pool is smaller.
pub fn next_tasks<'a>(tasks: &'a Tasks, filter: &TaskFilter, count: usize) -> Vec<&'a Task> {
    if count == 0 {
        return Vec::new();
    }

    let candidates = tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| matches_bundle(task, filter.bundle.as_deref()))
        .filter(|task| matches_milestone(task, filter.milestone.as_deref()))
        .filter(|task| matches_marker(task, filter.marker.as_deref()))
        .filter(|task| is_unblocked(task, tasks));

    let focus_phase = tasks.focus.as_ref().map(|focus| focus.phase);
    let (mut focus, mut other): (Vec<_>, Vec<_>) = candidates.partition(|task| match focus_phase {
        Some(phase) => task.phase == phase,
        None => true,
    });

    sort_by_eff_desc(&mut focus);
    sort_by_eff_desc(&mut other);

    let mut result: Vec<&Task> = focus.into_iter().take(count).collect();
    if result.len() < count {
        let needed = count - result.len();
        result.extend(other.into_iter().take(needed));
    }
    result
}

/// Highest-Eff pending unblocked task — thin wrapper over [`next_tasks`].
pub fn next_task<'a>(tasks: &'a Tasks, filter: &TaskFilter) -> Option<&'a Task> {
    next_tasks(tasks, filter, 1).into_iter().next()
}

fn sort_by_eff_desc(tasks: &mut [&Task]) {
    tasks.sort_by(|a, b| {
        efficiency(b)
            .partial_cmp(&efficiency(a))
            .unwrap_or(Ordering::Equal)
    });
}

pub fn format_next_task(task: &Task) -> String {
    let eff = efficiency(task);
    format!(
        "Task {} [Eff:{}] {} {}",
        task.id,
        format_efficiency(eff),
        tier_glyph(eff),
        task.title
    )
}

fn is_unblocked(task: &Task, tasks: &Tasks) -> bool {
    task.depends_on.iter().all(|dependency| {
        tasks
            .task
            .iter()
            .any(|candidate| candidate.id == *dependency && candidate.status == "done")
    })
}
