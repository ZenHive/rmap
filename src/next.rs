use std::cmp::Ordering;
use std::collections::HashSet;

use crate::query::{TaskFilter, matches_bundle, matches_marker, matches_milestone, matches_phase};
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

/// Top `count` `pending` tasks whose `depends_on` are all `done`, ranked by
/// a lexicographic key: **(focus-phase × active-milestone) → Eff desc**.
///
/// Honors `filter.marker`, `filter.bundle`, and `filter.milestone`.
/// `filter.status` and `filter.phase` are ignored (next is implicitly
/// `"pending"`-only; focus comes from `[focus].phase`, not the caller).
/// Bundle / milestone filters narrow the candidate pool BEFORE tiering, so
/// `--bundle X` / `--milestone Y` restrict the pool first and ranking only
/// orders within that pool.
///
/// Ranking tiers (lower wins; **focus dominant over milestone**):
///   0 — in `[focus].phase` AND pinned to any `active` milestone
///   1 — in `[focus].phase` only
///   2 — pinned to an `active` milestone only
///   3 — neither
/// Within a tier, Eff descending; ties preserve TOML order (stable sort).
///
/// Degenerate cases preserve prior behavior: when `[focus]` is unset every
/// task is treated as "in focus" for tiering purposes, so tiers collapse to
/// 0/1 (milestone-only bias) or, when no milestone is `active`, to all
/// tier 1 (pure Eff descending). When `[focus]` is set but no milestone is
/// `active`, only tiers 1 and 3 are occupied — equivalent to the original
/// focus-phase partition.
///
/// Returns up to `count` tasks; fewer is fine when the eligible pool is
/// smaller.
pub fn next_tasks<'a>(tasks: &'a Tasks, filter: &TaskFilter, count: usize) -> Vec<&'a Task> {
    if count == 0 {
        return Vec::new();
    }

    let mut candidates: Vec<&Task> = tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| matches_bundle(task, filter.bundle.as_deref()))
        .filter(|task| matches_milestone(task, filter.milestone.as_deref()))
        .filter(|task| matches_marker(task, filter.marker.as_deref()))
        .filter(|task| is_unblocked(task, tasks))
        .collect();

    rank_tasks(&mut candidates, tasks);
    candidates.into_iter().take(count).collect()
}

/// All `pending` tasks whose every `depends_on` is `done` — the parallel-safe
/// dispatch set — ranked by the same 4-tier key as [`next_tasks`]. Unlike
/// `next`, this honors `filter.phase` and returns the entire set when `count`
/// is `None` (default-all). The result is mutually independent by
/// construction: a pending task whose deps are all `done` cannot depend on
/// another pending task (that dep would have to be `done`), so there is no
/// `--independent` flag — it would always be a no-op.
pub fn ready_tasks<'a>(
    tasks: &'a Tasks,
    filter: &TaskFilter,
    count: Option<usize>,
) -> Vec<&'a Task> {
    let mut candidates: Vec<&Task> = tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| matches_marker(task, filter.marker.as_deref()))
        .filter(|task| matches_phase(task, filter.phase))
        .filter(|task| matches_bundle(task, filter.bundle.as_deref()))
        .filter(|task| matches_milestone(task, filter.milestone.as_deref()))
        .filter(|task| is_unblocked(task, tasks))
        .collect();

    rank_tasks(&mut candidates, tasks);
    match count {
        Some(n) => candidates.into_iter().take(n).collect(),
        None => candidates,
    }
}

/// Highest-Eff pending unblocked task — thin wrapper over [`next_tasks`].
pub fn next_task<'a>(tasks: &'a Tasks, filter: &TaskFilter) -> Option<&'a Task> {
    next_tasks(tasks, filter, 1).into_iter().next()
}

/// Sort tasks in place by the 4-tier lexicographic key (focus-phase ×
/// active-milestone, focus dominant) then Eff descending. Shared by
/// [`next_tasks`] and [`ready_tasks`] so the ranking stays a single source of
/// truth. Ties preserve TOML order (stable sort).
fn rank_tasks(candidates: &mut [&Task], tasks: &Tasks) {
    let focus_phase = tasks.focus.as_ref().map(|focus| focus.phase);
    let active_milestones: HashSet<&str> = tasks
        .milestones
        .iter()
        .filter(|(_, milestone)| milestone.status == "active")
        .map(|(name, _)| name.as_str())
        .collect();

    candidates.sort_by(|a, b| {
        tier(a, focus_phase, &active_milestones)
            .cmp(&tier(b, focus_phase, &active_milestones))
            .then_with(|| {
                efficiency(b)
                    .partial_cmp(&efficiency(a))
                    .unwrap_or(Ordering::Equal)
            })
    });
}

fn tier(task: &Task, focus_phase: Option<u32>, active_milestones: &HashSet<&str>) -> u8 {
    let in_focus = focus_phase.is_none_or(|phase| task.phase == phase);
    let in_active = task
        .milestone
        .as_deref()
        .is_some_and(|name| active_milestones.contains(name));
    match (in_focus, in_active) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
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

pub(crate) fn is_unblocked(task: &Task, tasks: &Tasks) -> bool {
    task.depends_on.iter().all(|dependency| {
        tasks
            .task
            .iter()
            .any(|candidate| candidate.id == *dependency && candidate.status == "done")
    })
}
