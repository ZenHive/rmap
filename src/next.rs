use crate::query::{TaskFilter, matches_bundle, matches_marker};
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

/// Highest-Eff `pending` task whose `depends_on` are all `done`.
///
/// Honors `filter.marker` and `filter.bundle` only. `filter.status` is ignored
/// (next is implicitly `"pending"`-only) and `filter.phase` is ignored (focus
/// preference is sourced from `[focus].phase`, not the user). When `[focus]`
/// is set, focus-phase candidates win over higher-Eff candidates in other
/// phases — bundle filtering happens BEFORE the focus partition, so
/// `--bundle X` restricts the candidate pool first and focus only ranks
/// within the bundle.
pub fn next_task<'a>(tasks: &'a Tasks, filter: &TaskFilter) -> Option<&'a Task> {
    let candidates = tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| matches_bundle(task, filter.bundle.as_deref()))
        .filter(|task| matches_marker(task, filter.marker.as_deref()))
        .filter(|task| is_unblocked(task, tasks));

    let focus_phase = tasks.focus.as_ref().map(|focus| focus.phase);
    let (focus, other): (Vec<_>, Vec<_>) = candidates.partition(|task| match focus_phase {
        Some(phase) => task.phase == phase,
        None => true,
    });

    highest_efficiency(focus).or_else(|| highest_efficiency(other))
}

fn highest_efficiency<'a, I>(tasks: I) -> Option<&'a Task>
where
    I: IntoIterator<Item = &'a Task>,
{
    tasks.into_iter().fold(None, |best, task| match best {
        Some(best_task) if efficiency(best_task) >= efficiency(task) => Some(best_task),
        _ => Some(task),
    })
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
