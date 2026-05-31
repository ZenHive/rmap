use std::collections::HashMap;

use serde::Serialize;

use crate::next_bundle::BundlePick;
use crate::schema::{
    Bundle, CrossRepo, Focus, Linear, Milestone, Phase, Scores, Task, TaskId, Tasks,
};
use crate::scoring::rounded_efficiency;
use crate::topo::compute_layers;

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
    milestones: &'a std::collections::BTreeMap<String, Milestone>,
    task: Vec<ExportedTask<'a>>,
}

#[derive(Serialize)]
struct ExportedTask<'a> {
    id: &'a TaskId,
    phase: u32,
    bundle: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    milestone: Option<&'a String>,
    status: &'a str,
    title: &'a str,
    scores: &'a Scores,
    eff: f64,
    /// Longest-path dependency depth over the whole in-repo graph (computed,
    /// never persisted — like `eff`). `0` for tasks with no in-repo dep; within
    /// a result set the lowest `dep_layer` present is the current parallel wave.
    dep_layer: usize,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a String>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    acceptance_criteria: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    out_of_scope: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    files_to_modify: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    touches: &'a [String],
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
    #[serde(skip_serializing_if = "Option::is_none")]
    implemented: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delivered_by: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verified: Option<bool>,
    cross_repo: &'a [CrossRepo],
}

pub fn export_json_str(tasks: &Tasks) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks(tasks))
}

/// Compact (single-line) form of `export_json_str`, used for the `rmap-data`
/// island embedded in `rmap render --html`. Same envelope as `data.json`, no
/// pretty-printing — keeps the static HTML under its size budget.
pub fn export_compact_json_str(tasks: &Tasks) -> serde_json::Result<String> {
    serde_json::to_string(&exported_tasks(tasks))
}

pub fn export_filtered_json_str(tasks: &Tasks, task: &[&Task]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks_with(tasks, task.iter().copied()))
}

pub fn export_task_json_str(all: &Tasks, task: Option<&Task>) -> serde_json::Result<String> {
    let layers = graph_layers(all);
    serde_json::to_string_pretty(&task.map(|t| exported_task(t, &layers)))
}

pub fn export_tasks_array_json_str(all: &Tasks, tasks: &[&Task]) -> serde_json::Result<String> {
    let layers = graph_layers(all);
    let exported: Vec<ExportedTask<'_>> = tasks
        .iter()
        .copied()
        .map(|t| exported_task(t, &layers))
        .collect();
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
    let layers = graph_layers(tasks);
    let (bundle, exported_tasks) = match pick {
        Some(p) => (
            Some(BundlePickInfo {
                name: p.name,
                phase: p.bundle.phase,
                description: p.bundle.description.as_str(),
            }),
            p.tasks
                .iter()
                .copied()
                .map(|t| exported_task(t, &layers))
                .collect(),
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
    let layers = graph_layers(tasks);
    ExportedTasks {
        schema_version: tasks.schema_version,
        project: &tasks.project,
        default_branch: &tasks.default_branch,
        vision: tasks.vision.as_ref(),
        focus: tasks.focus.as_ref(),
        linear: tasks.linear.as_ref(),
        phases: &tasks.phases,
        bundles: &tasks.bundles,
        milestones: &tasks.milestones,
        task: task
            .into_iter()
            .map(|t| exported_task(t, &layers))
            .collect(),
    }
}

/// Longest-path dependency layers over the full in-repo graph, keyed by
/// canonical `TaskId` string. Always built from `tasks.task` (the whole graph),
/// never from a filtered slice — `dep_layer` must reflect global depth even
/// when only a subset of tasks is being exported.
fn graph_layers(tasks: &Tasks) -> HashMap<String, usize> {
    let all: Vec<&Task> = tasks.task.iter().collect();
    compute_layers(&all)
}

fn exported_task<'a>(task: &'a Task, layers: &HashMap<String, usize>) -> ExportedTask<'a> {
    ExportedTask {
        id: &task.id,
        phase: task.phase,
        bundle: &task.bundle,
        milestone: task.milestone.as_ref(),
        status: &task.status,
        title: &task.title,
        scores: &task.scores,
        eff: rounded_efficiency(task),
        dep_layer: layers.get(&task.id.to_string()).copied().unwrap_or(0),
        markers: &task.markers,
        depends_on: &task.depends_on,
        linear_id: task.linear_id.as_ref(),
        assignee: task.assignee.as_ref(),
        module: task.module.as_ref(),
        branch: task.branch.as_ref(),
        model: task.model.as_ref(),
        acceptance_criteria: &task.acceptance_criteria,
        out_of_scope: &task.out_of_scope,
        files_to_modify: &task.files_to_modify,
        touches: &task.touches,
        shipped_in: task.shipped_in.as_ref(),
        body: task.body.as_ref(),
        created_at: task.created_at.as_ref(),
        started_at: task.started_at.as_ref(),
        done_at: task.done_at.as_ref(),
        scored_at: task.scored_at.as_ref(),
        blocked_reason: task.blocked_reason.as_ref(),
        implemented: task.implemented.as_ref(),
        delivered_by: task.delivered_by.as_ref(),
        verified: task.verified,
        cross_repo: &task.cross_repo,
    }
}
