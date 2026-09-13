use std::collections::HashMap;

use serde::Serialize;

use crate::next_bundle::BundlePick;
use crate::schema::{
    Attempt, Bundle, CrossRepo, Focus, Linear, Milestone, Phase, Scores, Task, TaskId, Tasks,
};
use crate::scoring::rounded_efficiency;
use crate::topo::{compute_layers, compute_unlocks};

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
    target_repo: Option<&'a String>,
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
    /// Count of tasks that transitively depend on this one — computed unlock
    /// leverage ("what finishing this frees up"), over the whole in-repo graph.
    /// Computed, never persisted (like `eff` / `dep_layer`); `0` for a leaf.
    unlocks: usize,
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
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    domains: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    shipped_in: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    landing_ref: Option<&'a String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    verified_by: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_ref: Option<&'a String>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    attempts: &'a [Attempt],
    cross_repo: &'a [CrossRepo],
}

/// Every JSON key an `ExportedTask` can carry — the validation set for the
/// `--fields` projection (`rmap list` / `rmap ready`). Includes the computed
/// `eff` / `dep_layer` / `unlocks`. MIRROR SURFACE: adding an `ExportedTask` field means
/// adding it here; `exported_task_fields_cover_serialized_keys` guards drift.
pub const EXPORTED_TASK_FIELDS: &[&str] = &[
    "id",
    "phase",
    "bundle",
    "target_repo",
    "milestone",
    "status",
    "title",
    "scores",
    "eff",
    "dep_layer",
    "unlocks",
    "markers",
    "depends_on",
    "linear_id",
    "assignee",
    "module",
    "branch",
    "model",
    "acceptance_criteria",
    "out_of_scope",
    "files_to_modify",
    "touches",
    "domains",
    "shipped_in",
    "landing_ref",
    "body",
    "created_at",
    "started_at",
    "done_at",
    "scored_at",
    "blocked_reason",
    "implemented",
    "delivered_by",
    "verified",
    "verified_by",
    "verification_ref",
    "attempts",
    "cross_repo",
];

pub fn export_json_str(tasks: &Tasks) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&exported_tasks(tasks))
}

/// Token-cheap projection backing `--fields` on `rmap list` / `rmap ready`:
/// emits a JSON **array** of objects, each carrying only the requested
/// `fields` (the bulky `phases` / `bundles` / `milestones` envelope is dropped
/// on purpose). Keys absent on a given task (skipped optionals) simply don't
/// appear for that task. Every requested name is validated against
/// [`EXPORTED_TASK_FIELDS`]; the first unknown name is returned as `Err(name)`
/// so the caller can exit non-zero with the offending field. `all` supplies
/// the full graph for the computed `dep_layer`.
pub fn project_fields_json_str(
    all: &Tasks,
    task: &[&Task],
    fields: &[String],
) -> Result<String, String> {
    for field in fields {
        if !EXPORTED_TASK_FIELDS.contains(&field.as_str()) {
            return Err(field.clone());
        }
    }
    let requested: std::collections::HashSet<&str> = fields.iter().map(String::as_str).collect();
    let metrics = graph_metrics(all);
    let projected: Vec<serde_json::Value> = task
        .iter()
        .copied()
        .map(|t| {
            let value =
                serde_json::to_value(exported_task(t, &metrics)).unwrap_or(serde_json::Value::Null);
            match value {
                serde_json::Value::Object(map) => serde_json::Value::Object(
                    map.into_iter()
                        .filter(|(key, _)| requested.contains(key.as_str()))
                        .collect(),
                ),
                other => other,
            }
        })
        .collect();
    // Serializing an in-memory Value of finite numbers + strings cannot fail;
    // fall back to "[]" rather than panicking if that ever changes.
    Ok(
        serde_json::to_string_pretty(&serde_json::Value::Array(projected))
            .unwrap_or_else(|_| "[]".to_string()),
    )
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
    let metrics = graph_metrics(all);
    serde_json::to_string_pretty(&task.map(|t| exported_task(t, &metrics)))
}

pub fn export_tasks_array_json_str(all: &Tasks, tasks: &[&Task]) -> serde_json::Result<String> {
    let metrics = graph_metrics(all);
    let exported: Vec<ExportedTask<'_>> = tasks
        .iter()
        .copied()
        .map(|t| exported_task(t, &metrics))
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
    let metrics = graph_metrics(tasks);
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
                .map(|t| exported_task(t, &metrics))
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
    let metrics = graph_metrics(tasks);
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
            .map(|t| exported_task(t, &metrics))
            .collect(),
    }
}

/// Computed per-task graph metrics keyed by canonical `TaskId` string. Always
/// built from `tasks.task` (the whole graph), never from a filtered slice —
/// `dep_layer` / `unlocks` must reflect global depth and leverage even when only
/// a subset of tasks is being exported. Computed once per export call, then
/// shared across every `exported_task`.
struct GraphMetrics {
    layers: HashMap<String, usize>,
    unlocks: HashMap<String, usize>,
}

fn graph_metrics(tasks: &Tasks) -> GraphMetrics {
    let all: Vec<&Task> = tasks.task.iter().collect();
    GraphMetrics {
        layers: compute_layers(&all),
        unlocks: compute_unlocks(&all),
    }
}

fn exported_task<'a>(task: &'a Task, metrics: &GraphMetrics) -> ExportedTask<'a> {
    let key = task.id.to_string();
    ExportedTask {
        id: &task.id,
        phase: task.phase,
        bundle: &task.bundle,
        target_repo: task.target_repo.as_ref(),
        milestone: task.milestone.as_ref(),
        status: &task.status,
        title: &task.title,
        scores: &task.scores,
        eff: rounded_efficiency(task),
        dep_layer: metrics.layers.get(&key).copied().unwrap_or(0),
        unlocks: metrics.unlocks.get(&key).copied().unwrap_or(0),
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
        domains: &task.domains,
        shipped_in: task.shipped_in.as_ref(),
        landing_ref: task.landing_ref.as_ref(),
        body: task.body.as_ref(),
        created_at: task.created_at.as_ref(),
        started_at: task.started_at.as_ref(),
        done_at: task.done_at.as_ref(),
        scored_at: task.scored_at.as_ref(),
        blocked_reason: task.blocked_reason.as_ref(),
        implemented: task.implemented.as_ref(),
        delivered_by: task.delivered_by.as_ref(),
        verified: task.verified,
        verified_by: task.verified_by.as_ref(),
        verification_ref: task.verification_ref.as_ref(),
        attempts: &task.attempts,
        cross_repo: &task.cross_repo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A task with every ExportedTask-bearing field populated, so serialization
    // emits all keys (the `skip_serializing_if` optionals are present). Used to
    // guard `EXPORTED_TASK_FIELDS` against drift in either direction.
    const FULLY_POPULATED: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "Phase One"
order = 1
status = "in_progress"

[bundles.alpha]
phase = 1
order = 1
description = "alpha"

[milestones.v0_1]
name = "v0.1"
order = 1
status = "active"
target_version = "0.1.0"

[[task]]
id = 7
phase = 1
bundle = "alpha"
target_repo = "public-demo"
milestone = "v0_1"
status = "done"
title = "fully populated"
scores = { d = 2, b = 8, u = 8 }
markers = ["parallel"]
depends_on = [1]
linear_id = "ENG-7"
assignee = "claude"
module = "core"
branch = "feat/x"
model = "opus"
acceptance_criteria = ["ac"]
out_of_scope = ["oos"]
files_to_modify = ["src/a.rs"]
touches = ["src/b.rs"]
domains = ["rust", "otp"]
shipped_in = "abc123"
landing_ref = "https://github.com/org/repo/pull/42"
body = "intent"
created_at = "2026-01-01"
started_at = "2026-01-02"
done_at = "2026-01-03"
scored_at = "2026-01-01"
blocked_reason = "was blocked"
implemented = "what shipped"
delivered_by = "codex"
verified = true
verified_by = "grok/grok-4.5"
verification_ref = "harness-run:run-123"
attempts = [{ at = "2026-01-02", by = "claude", report = "reviewer rejected: tests red" }]
cross_repo = [{ repo = "other", task_id = 5, relation = "blocks" }]
"#;

    #[test]
    fn exported_task_fields_cover_serialized_keys() {
        let tasks: Tasks = toml::from_str(FULLY_POPULATED).expect("valid toml");
        let metrics = graph_metrics(&tasks);
        let value =
            serde_json::to_value(exported_task(&tasks.task[0], &metrics)).expect("serialize task");
        let serialized: std::collections::BTreeSet<String> =
            value.as_object().expect("object").keys().cloned().collect();
        let declared: std::collections::BTreeSet<String> =
            EXPORTED_TASK_FIELDS.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            serialized, declared,
            "EXPORTED_TASK_FIELDS must exactly match the keys a fully-populated ExportedTask serializes"
        );
    }
}
