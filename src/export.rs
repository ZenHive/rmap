use serde::Serialize;

use crate::next_bundle::BundlePick;
use crate::schema::{Bundle, CrossRepo, Focus, Linear, Phase, Scores, Task, TaskId, Tasks};
use crate::scoring::efficiency;

#[derive(Serialize)]
struct ExportedTasks<'a> {
    schema_version: u32,
    project: &'a str,
    default_branch: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    vision: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<&'a Focus>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<&'a String>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    acceptance_criteria: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    out_of_scope: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    files_to_modify: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    shipped_in: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    done_at: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scored_at: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked_reason: Option<&'a String>,
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

pub fn export_tasks_array_json_str(tasks: &[&Task]) -> serde_json::Result<String> {
    let exported: Vec<ExportedTask<'_>> = tasks.iter().copied().map(exported_task).collect();
    serde_json::to_string_pretty(&exported)
}

/// JSON envelope for `rmap next-bundle --json`.
///
/// Shape: `{ schema_version, focus_phase, bundle: {name, phase, description} | null, tasks: [ExportedTask, ...] }`.
/// `focus_phase` is the **effective** focus phase the selector used (after any
/// `--phase` override), not the stored `[focus].phase`. `bundle` and `tasks`
/// are `null` / `[]` together when there is no pick.
pub fn export_bundle_pick_json_str(
    tasks: &Tasks,
    effective_focus: Option<u32>,
    pick: Option<&BundlePick<'_>>,
) -> serde_json::Result<String> {
    let (bundle, exported_tasks) = match pick {
        Some(p) => (
            Some(BundlePickInfo {
                name: p.name,
                phase: p.bundle.phase,
                description: p.bundle.description.as_str(),
            }),
            p.tasks.iter().copied().map(exported_task).collect(),
        ),
        None => (None, Vec::new()),
    };
    let envelope = BundlePickJson {
        schema_version: tasks.schema_version,
        focus_phase: effective_focus,
        bundle,
        tasks: exported_tasks,
    };
    serde_json::to_string_pretty(&envelope)
}

#[derive(Serialize)]
struct BundlePickJson<'a> {
    schema_version: u32,
    focus_phase: Option<u32>,
    bundle: Option<BundlePickInfo<'a>>,
    tasks: Vec<ExportedTask<'a>>,
}

#[derive(Serialize)]
struct BundlePickInfo<'a> {
    name: &'a str,
    phase: u32,
    description: &'a str,
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
        vision: tasks.vision.as_ref(),
        focus: tasks.focus.as_ref(),
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
        module: task.module.as_ref(),
        branch: task.branch.as_ref(),
        acceptance_criteria: &task.acceptance_criteria,
        out_of_scope: &task.out_of_scope,
        files_to_modify: &task.files_to_modify,
        shipped_in: task.shipped_in.as_ref(),
        body: task.body.as_ref(),
        created_at: task.created_at.as_ref(),
        started_at: task.started_at.as_ref(),
        done_at: task.done_at.as_ref(),
        scored_at: task.scored_at.as_ref(),
        blocked_reason: task.blocked_reason.as_ref(),
        cross_repo: &task.cross_repo,
    }
}

fn rounded_efficiency(task: &Task) -> f64 {
    (efficiency(task) * 100.0).round() / 100.0
}
