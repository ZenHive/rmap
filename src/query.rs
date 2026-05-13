use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<String>,
    pub marker: Option<String>,
    pub phase: Option<u32>,
    pub bundle: Option<String>,
}

pub fn find_task<'a>(tasks: &'a Tasks, id: &str) -> Option<&'a Task> {
    tasks.task.iter().find(|task| task.id.to_string() == id)
}

pub fn list_tasks<'a>(tasks: &'a Tasks, filter: &TaskFilter) -> Vec<&'a Task> {
    tasks
        .task
        .iter()
        .filter(|task| matches_status(task, filter.status.as_deref()))
        .filter(|task| matches_marker(task, filter.marker.as_deref()))
        .filter(|task| matches_phase(task, filter.phase))
        .filter(|task| matches_bundle(task, filter.bundle.as_deref()))
        .collect()
}

pub(crate) fn matches_marker(task: &Task, marker: Option<&str>) -> bool {
    marker.is_none_or(|marker| task.markers.iter().any(|task_marker| task_marker == marker))
}

pub(crate) fn matches_bundle(task: &Task, bundle: Option<&str>) -> bool {
    bundle.is_none_or(|bundle| task.bundle == bundle)
}

pub fn format_task(task: &Task) -> String {
    let eff = efficiency(task);
    let mut lines = vec![
        format!("Task {}", task.id),
        format!("title: {}", task.title),
        format!("status: {}", task.status),
        format!("phase: {}", task.phase),
        format!("bundle: {}", task.bundle),
        format!(
            "scores: D:{}/B:{}/U:{} -> Eff:{} {}",
            task.scores.d,
            task.scores.b,
            task.scores.u,
            format_efficiency(eff),
            tier_glyph(eff)
        ),
    ];

    if !task.markers.is_empty() {
        lines.push(format!("markers: {}", task.markers.join(", ")));
    }

    if !task.depends_on.is_empty() {
        lines.push(format!(
            "depends_on: {}",
            task.depends_on
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if let Some(linear_id) = &task.linear_id {
        lines.push(format!("linear_id: {linear_id}"));
    }

    if let Some(assignee) = &task.assignee {
        lines.push(format!("assignee: {assignee}"));
    }

    if !task.acceptance_criteria.is_empty() {
        lines.push("acceptance_criteria:".to_string());
        lines.extend(
            task.acceptance_criteria
                .iter()
                .map(|criterion| format!("- {criterion}")),
        );
    }

    if let Some(shipped_in) = &task.shipped_in {
        lines.push(format!("shipped_in: {shipped_in}"));
    }

    if !task.cross_repo.is_empty() {
        lines.push(format!("cross_repo: {}", task.cross_repo.len()));
    }

    if let Some(body) = &task.body {
        lines.push("body:".to_string());
        lines.push(body.trim().to_string());
    }

    lines.join("\n")
}

pub fn format_task_row(task: &Task) -> String {
    let eff = efficiency(task);
    format!(
        "Task {} [{} Eff:{}] {} {}",
        task.id,
        task.status,
        format_efficiency(eff),
        tier_glyph(eff),
        task.title
    )
}

fn matches_status(task: &Task, status: Option<&str>) -> bool {
    status.is_none_or(|status| task.status == status)
}

fn matches_phase(task: &Task, phase: Option<u32>) -> bool {
    phase.is_none_or(|phase| task.phase == phase)
}
