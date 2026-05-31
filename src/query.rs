use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<String>,
    pub marker: Option<String>,
    pub phase: Option<u32>,
    pub bundle: Option<String>,
    pub milestone: Option<String>,
    pub delivered_by: Option<String>,
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
        .filter(|task| matches_milestone(task, filter.milestone.as_deref()))
        .filter(|task| matches_delivered_by(task, filter.delivered_by.as_deref()))
        .collect()
}

pub(crate) fn matches_marker(task: &Task, marker: Option<&str>) -> bool {
    marker.is_none_or(|marker| task.markers.iter().any(|task_marker| task_marker == marker))
}

pub(crate) fn matches_bundle(task: &Task, bundle: Option<&str>) -> bool {
    bundle.is_none_or(|bundle| task.bundle == bundle)
}

pub(crate) fn matches_milestone(task: &Task, milestone: Option<&str>) -> bool {
    milestone.is_none_or(|name| task.milestone.as_deref() == Some(name))
}

pub(crate) fn matches_delivered_by(task: &Task, agent: Option<&str>) -> bool {
    agent.is_none_or(|name| task.delivered_by.as_deref() == Some(name))
}

pub fn format_task(task: &Task) -> String {
    let eff = efficiency(task);
    let mut lines = vec![
        format!("Task {}", task.id),
        format!("title: {}", task.title),
        format!("status: {}", task.status),
        format!("phase: {}", task.phase),
        format!("bundle: {}", task.bundle),
    ];

    if let Some(milestone) = &task.milestone {
        lines.push(format!("milestone: {milestone}"));
    }

    lines.push(format!(
        "scores: D:{}/B:{}/U:{} -> Eff:{} {}",
        task.scores.d,
        task.scores.b,
        task.scores.u,
        format_efficiency(eff),
        tier_glyph(eff)
    ));

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

    if let Some(model) = &task.model {
        lines.push(format!("model: {model}"));
    }

    if !task.acceptance_criteria.is_empty() {
        lines.push("acceptance_criteria:".to_string());
        lines.extend(
            task.acceptance_criteria
                .iter()
                .map(|criterion| format!("- {criterion}")),
        );
    }

    if !task.out_of_scope.is_empty() {
        lines.push("out_of_scope:".to_string());
        lines.extend(task.out_of_scope.iter().map(|item| format!("- {item}")));
    }

    if let Some(shipped_in) = &task.shipped_in {
        lines.push(format!("shipped_in: {shipped_in}"));
    }

    if !task.cross_repo.is_empty() {
        lines.push(format!("cross_repo: {}", task.cross_repo.len()));
    }

    // When both body and implemented are present, disambiguate the headers so
    // a reader can tell "original intent" from "what shipped" at a glance.
    // Bare `body:` / `implemented:` headers when only one is present preserve
    // the established shape for pending / in_progress / blocked tasks.
    let body_header = if task.body.is_some() && task.implemented.is_some() {
        "body (original intent):"
    } else {
        "body:"
    };
    let implemented_header = if task.body.is_some() && task.implemented.is_some() {
        "implemented (what shipped):"
    } else {
        "implemented:"
    };

    if let Some(body) = &task.body {
        lines.push(body_header.to_string());
        lines.push(body.trim().to_string());
    }

    if let Some(implemented) = &task.implemented {
        lines.push(implemented_header.to_string());
        lines.push(implemented.trim().to_string());
    }

    if let Some(delivered_by) = &task.delivered_by {
        lines.push(format!("delivered_by: {delivered_by}"));
    }

    if let Some(verified) = task.verified {
        lines.push(format!("verified: {}", if verified { "yes" } else { "no" }));
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

pub(crate) fn matches_phase(task: &Task, phase: Option<u32>) -> bool {
    phase.is_none_or(|phase| task.phase == phase)
}
