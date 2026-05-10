use crate::schema::{Task, Tasks};

pub fn next_task<'a>(tasks: &'a Tasks, marker: Option<&str>) -> Option<&'a Task> {
    tasks
        .task
        .iter()
        .filter(|task| task.status == "pending")
        .filter(|task| marker_matches(task, marker))
        .filter(|task| is_unblocked(task, tasks))
        .fold(None, |best, task| match best {
            Some(best_task) if efficiency(best_task) >= efficiency(task) => Some(best_task),
            _ => Some(task),
        })
}

pub fn format_next_task(task: &Task) -> String {
    format!(
        "Task {} [Eff:{}] {}",
        task.id,
        format_efficiency(efficiency(task)),
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

fn efficiency(task: &Task) -> f64 {
    f64::from(task.scores.b + task.scores.u) / (2.0 * f64::from(task.scores.d))
}

fn format_efficiency(value: f64) -> String {
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
