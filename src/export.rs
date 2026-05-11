use serde::Serialize;

use crate::schema::{Bundle, CrossRepo, Linear, Phase, Scores, Task, TaskId, Tasks};
use crate::scoring::efficiency;

#[derive(Serialize)]
struct ExportedTasks<'a> {
    schema_version: u32,
    project: &'a str,
    default_branch: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    linear: Option<&'a Linear>,
    phases: &'a std::collections::BTreeMap<String, Phase>,
    bundles: &'a std::collections::BTreeMap<String, Bundle>,
    task: Vec<ExportedTask<'a>>,
}

#[derive(Serialize)]
struct ExportedTask<'a> {
    id: &'a TaskId,
    phase: u32,
    bundle: &'a str,
    status: &'a str,
    title: &'a str,
    scores: &'a Scores,
    eff: f64,
    markers: &'a [String],
    depends_on: &'a [TaskId],
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_id: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee: Option<&'a String>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    acceptance_criteria: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    shipped_in: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a String>,
    cross_repo: &'a [CrossRepo],
}

pub fn export_json_str(tasks: &Tasks) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks(tasks))
}

pub fn export_filtered_json_str(tasks: &Tasks, task: &[&Task]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks_with(tasks, task.iter().copied()))
}

pub fn export_task_json_str(task: Option<&Task>) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&task.map(exported_task))
}

fn exported_tasks(tasks: &Tasks) -> ExportedTasks<'_> {
    exported_tasks_with(tasks, tasks.task.iter())
}

fn exported_tasks_with<'a>(
    tasks: &'a Tasks,
    task: impl IntoIterator<Item = &'a Task>,
) -> ExportedTasks<'a> {
    ExportedTasks {
        schema_version: tasks.schema_version,
        project: &tasks.project,
        default_branch: &tasks.default_branch,
        linear: tasks.linear.as_ref(),
        phases: &tasks.phases,
        bundles: &tasks.bundles,
        task: task.into_iter().map(exported_task).collect(),
    }
}

fn exported_task(task: &Task) -> ExportedTask<'_> {
    ExportedTask {
        id: &task.id,
        phase: task.phase,
        bundle: &task.bundle,
        status: &task.status,
        title: &task.title,
        scores: &task.scores,
        eff: rounded_efficiency(task),
        markers: &task.markers,
        depends_on: &task.depends_on,
        linear_id: task.linear_id.as_ref(),
        assignee: task.assignee.as_ref(),
        acceptance_criteria: &task.acceptance_criteria,
        shipped_in: task.shipped_in.as_ref(),
        body: task.body.as_ref(),
        cross_repo: &task.cross_repo,
    }
}

fn rounded_efficiency(task: &Task) -> f64 {
    (efficiency(task) * 100.0).round() / 100.0
}
