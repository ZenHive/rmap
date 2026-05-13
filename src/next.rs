use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

pub fn next_task<'a>(tasks: &'a Tasks, marker: Option<&str>) -> Option<&'a Task> {
    let candidates = tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| marker_matches(task, marker))
        .filter(|task| is_unblocked(task, tasks));

    // When `[focus].phase` is set, only fall back to other phases if no
    // focus-phase candidate exists. Highest Eff still wins inside each tier;
    // ties preserve TOML order via the `>=` short-circuit.
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

fn marker_matches(task: &Task, marker: Option<&str>) -> bool {
    marker.is_none_or(|marker| task.markers.iter().any(|task_marker| task_marker == marker))
}

fn is_unblocked(task: &Task, tasks: &Tasks) -> bool {
    task.depends_on.iter().all(|dependency| {
        tasks
            .task
            .iter()
            .any(|candidate| candidate.id == *dependency && candidate.status == "done")
    })
}
