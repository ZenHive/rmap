use serde::Serialize;

use crate::schema::{Bundle, CrossRepo, Linear, Phase, Scores, Task, Tasks};

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
    id: u32,
    phase: u32,
    bundle: &'a str,
    status: &'a str,
    title: &'a str,
    scores: &'a Scores,
    eff: f64,
    markers: &'a [String],
    depends_on: &'a [u32],
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_id: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipped_in: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a String>,
    cross_repo: &'a [CrossRepo],
}

pub fn export_json_str(tasks: &Tasks) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks(tasks))
}

fn exported_tasks(tasks: &Tasks) -> ExportedTasks<'_> {
    ExportedTasks {
        schema_version: tasks.schema_version,
        project: &tasks.project,
        default_branch: &tasks.default_branch,
        linear: tasks.linear.as_ref(),
        phases: &tasks.phases,
        bundles: &tasks.bundles,
        task: tasks.task.iter().map(exported_task).collect(),
    }
}

fn exported_task(task: &Task) -> ExportedTask<'_> {
    ExportedTask {
        id: task.id,
        phase: task.phase,
        bundle: &task.bundle,
        status: &task.status,
        title: &task.title,
        scores: &task.scores,
        eff: efficiency(task),
        markers: &task.markers,
        depends_on: &task.depends_on,
        linear_id: task.linear_id.as_ref(),
        shipped_in: task.shipped_in.as_ref(),
        body: task.body.as_ref(),
        cross_repo: &task.cross_repo,
    }
}

fn efficiency(task: &Task) -> f64 {
    f64::from(task.scores.b + task.scores.u) / (2.0 * f64::from(task.scores.d))
}
