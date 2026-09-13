use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Serialize;
use serde_json::Value;

use crate::schema::{Bundle, Linear, Milestone, Phase, Task, TaskId, Tasks};

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ChangedValue {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct TaskDiff {
    pub id: TaskId,
    pub status: DiffStatus,
    pub changed_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<ChangedValue>>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct MetadataDiff {
    pub key: String,
    pub status: DiffStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<ChangedValue>>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct TomlDiff {
    pub metadata: Vec<MetadataDiff>,
    pub tasks: Vec<TaskDiff>,
}

impl TomlDiff {
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty() && self.tasks.is_empty()
    }
}

// Verbose-mode whitelists. Members are surfaced via `values: [{field, before, after}]`
// on Changed entries when `--verbose` is on. Renaming or removing a member is a
// breaking change to the agent contract — bump `schema_version` first.
const TASK_VERBOSE_WHITELIST: &[&str] = &[
    "phase",
    "bundle",
    "target_repo",
    "milestone",
    "status",
    "title",
    "scores",
    "markers",
    "depends_on",
    "linear_id",
    "assignee",
    "module",
    "branch",
    "model",
    "shipped_in",
    "landing_ref",
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
];
const METADATA_VERBOSE_WHITELIST: &[&str] = &[
    "schema_version",
    "project",
    "default_branch",
    "changelog_path",
    "vision",
    "focus",
    "linear.team_key",
    "linear.workspace_url",
];

pub fn diff_toml(base: &Tasks, current: &Tasks, verbose: bool) -> TomlDiff {
    TomlDiff {
        metadata: diff_metadata(base, current, verbose),
        tasks: diff_tasks(base, current, verbose),
    }
}

pub fn diff_tasks(base: &Tasks, current: &Tasks, verbose: bool) -> Vec<TaskDiff> {
    let current_by_id = task_map(current);
    let mut seen = HashSet::new();
    let mut diff = Vec::new();

    for task in &base.task {
        let key = task.id.to_string();
        seen.insert(key.clone());

        match current_by_id.get(&key) {
            Some(current_task) => {
                let (changed_fields, values) = task_changes(task, current_task, verbose);
                if !changed_fields.is_empty() {
                    diff.push(TaskDiff {
                        id: task.id.clone(),
                        status: DiffStatus::Changed,
                        changed_fields,
                        values,
                    });
                }
            }
            None => diff.push(TaskDiff {
                id: task.id.clone(),
                status: DiffStatus::Removed,
                changed_fields: Vec::new(),
                values: None,
            }),
        }
    }

    for task in &current.task {
        if seen.contains(&task.id.to_string()) {
            continue;
        }

        diff.push(TaskDiff {
            id: task.id.clone(),
            status: DiffStatus::Added,
            changed_fields: Vec::new(),
            values: None,
        });
    }

    diff
}

pub fn diff_metadata(base: &Tasks, current: &Tasks, verbose: bool) -> Vec<MetadataDiff> {
    let mut diff = Vec::new();

    if base.schema_version != current.schema_version {
        diff.push(scalar_change(
            "schema_version",
            verbose,
            &base.schema_version,
            &current.schema_version,
        ));
    }
    if base.project != current.project {
        diff.push(scalar_change(
            "project",
            verbose,
            &base.project,
            &current.project,
        ));
    }
    if base.default_branch != current.default_branch {
        diff.push(scalar_change(
            "default_branch",
            verbose,
            &base.default_branch,
            &current.default_branch,
        ));
    }
    if let Some(entry) = diff_optional(
        "changelog_path",
        base.changelog_path.as_ref(),
        current.changelog_path.as_ref(),
        verbose,
    ) {
        diff.push(entry);
    }
    if let Some(entry) = diff_optional(
        "vision",
        base.vision.as_ref(),
        current.vision.as_ref(),
        verbose,
    ) {
        diff.push(entry);
    }
    if let Some(entry) = diff_optional(
        "focus",
        base.focus.as_ref(),
        current.focus.as_ref(),
        verbose,
    ) {
        diff.push(entry);
    }
    match (&base.linear, &current.linear) {
        (None, None) => {}
        (None, Some(_)) => diff.push(MetadataDiff {
            key: "linear".to_string(),
            status: DiffStatus::Added,
            values: None,
        }),
        (Some(_), None) => diff.push(MetadataDiff {
            key: "linear".to_string(),
            status: DiffStatus::Removed,
            values: None,
        }),
        (Some(base_linear), Some(current_linear)) => {
            diff.extend(diff_linear(base_linear, current_linear, verbose));
        }
    }

    diff.extend(map_diff("phases", &base.phases, &current.phases));
    diff.extend(map_diff("bundles", &base.bundles, &current.bundles));
    diff.extend(map_diff(
        "milestones",
        &base.milestones,
        &current.milestones,
    ));

    diff
}

pub fn format_diff(diff: &TomlDiff, verbose: bool) -> String {
    if diff.is_empty() {
        return "no changes".to_string();
    }

    let mut lines = Vec::new();
    for entry in &diff.metadata {
        lines.push(match entry.status {
            DiffStatus::Added => format!("added {}", entry.key),
            DiffStatus::Removed => format!("removed {}", entry.key),
            DiffStatus::Changed => format!("changed {}", entry.key),
        });
        if verbose && let Some(values) = &entry.values {
            append_value_lines(&mut lines, values);
        }
    }
    for entry in &diff.tasks {
        lines.push(match entry.status {
            DiffStatus::Added => format!("added Task {}", entry.id),
            DiffStatus::Removed => format!("removed Task {}", entry.id),
            DiffStatus::Changed => {
                format!(
                    "changed Task {}: {}",
                    entry.id,
                    entry.changed_fields.join(", ")
                )
            }
        });
        if verbose && let Some(values) = &entry.values {
            append_value_lines(&mut lines, values);
        }
    }

    lines.join("\n")
}

fn append_value_lines(lines: &mut Vec<String>, values: &[ChangedValue]) {
    for value in values {
        lines.push(format!(
            "  {}: {} → {}",
            value.field, value.before, value.after
        ));
    }
}

fn scalar_change<T: Serialize>(key: &str, verbose: bool, before: &T, after: &T) -> MetadataDiff {
    MetadataDiff {
        key: key.to_string(),
        status: DiffStatus::Changed,
        values: metadata_values(key, verbose, before, after),
    }
}

fn metadata_values<T: Serialize>(
    key: &str,
    verbose: bool,
    before: &T,
    after: &T,
) -> Option<Vec<ChangedValue>> {
    if !verbose || !METADATA_VERBOSE_WHITELIST.contains(&key) {
        return None;
    }
    Some(vec![ChangedValue {
        field: key.to_string(),
        before: serde_json::to_value(before).expect("schema types always serialize"),
        after: serde_json::to_value(after).expect("schema types always serialize"),
    }])
}

fn diff_optional<T: PartialEq + Serialize>(
    key: &str,
    base: Option<&T>,
    current: Option<&T>,
    verbose: bool,
) -> Option<MetadataDiff> {
    match (base, current) {
        (None, None) => None,
        (None, Some(_)) => Some(MetadataDiff {
            key: key.to_string(),
            status: DiffStatus::Added,
            values: None,
        }),
        (Some(_), None) => Some(MetadataDiff {
            key: key.to_string(),
            status: DiffStatus::Removed,
            values: None,
        }),
        (Some(base_value), Some(current_value)) => (base_value != current_value)
            .then(|| scalar_change(key, verbose, base_value, current_value)),
    }
}

fn diff_linear(base: &Linear, current: &Linear, verbose: bool) -> Vec<MetadataDiff> {
    let mut diff = Vec::new();
    if base.team_key != current.team_key {
        diff.push(scalar_change(
            "linear.team_key",
            verbose,
            &base.team_key,
            &current.team_key,
        ));
    }
    if base.workspace_url != current.workspace_url {
        diff.push(scalar_change(
            "linear.workspace_url",
            verbose,
            &base.workspace_url,
            &current.workspace_url,
        ));
    }
    diff
}

trait MapEntry: PartialEq {}
impl MapEntry for Phase {}
impl MapEntry for Bundle {}
impl MapEntry for Milestone {}

fn map_diff<V: MapEntry>(
    namespace: &str,
    base: &BTreeMap<String, V>,
    current: &BTreeMap<String, V>,
) -> Vec<MetadataDiff> {
    let keys: BTreeSet<&String> = base.keys().chain(current.keys()).collect();

    keys.into_iter()
        .filter_map(|key| match (base.get(key), current.get(key)) {
            (Some(base_value), Some(current_value)) => {
                (base_value != current_value).then(|| MetadataDiff {
                    key: format!("{namespace}.{key}"),
                    status: DiffStatus::Changed,
                    values: None,
                })
            }
            (None, Some(_)) => Some(MetadataDiff {
                key: format!("{namespace}.{key}"),
                status: DiffStatus::Added,
                values: None,
            }),
            (Some(_), None) => Some(MetadataDiff {
                key: format!("{namespace}.{key}"),
                status: DiffStatus::Removed,
                values: None,
            }),
            (None, None) => None,
        })
        .collect()
}

fn task_map(tasks: &Tasks) -> BTreeMap<String, &Task> {
    // Keyed by Display so TaskId::Number(74) and TaskId::Text("74") match
    // across base/current. Authors don't mix forms for one task.
    tasks
        .task
        .iter()
        .map(|task| (task.id.to_string(), task))
        .collect()
}

// Single-point-of-edit for per-field diff granularity. When `schema::Task` gains
// a field, add it here AND to `export::ExportedTask`. Also decide whether the
// field belongs on `TASK_VERBOSE_WHITELIST` above — it's part of the agent
// contract. `id` is intentionally absent (it's the map key).
fn task_changes(
    base: &Task,
    current: &Task,
    verbose: bool,
) -> (Vec<String>, Option<Vec<ChangedValue>>) {
    let mut fields = Vec::new();
    let mut values: Vec<ChangedValue> = Vec::new();

    macro_rules! diff_fields {
        ($($field:ident),* $(,)?) => {
            $(if base.$field != current.$field {
                let name = stringify!($field);
                fields.push(name.to_string());
                if verbose && TASK_VERBOSE_WHITELIST.contains(&name) {
                    values.push(ChangedValue {
                        field: name.to_string(),
                        before: serde_json::to_value(&base.$field).expect("schema types always serialize"),
                        after: serde_json::to_value(&current.$field).expect("schema types always serialize"),
                    });
                }
            })*
        };
    }

    diff_fields!(
        phase,
        bundle,
        target_repo,
        milestone,
        status,
        title,
        scores,
        markers,
        depends_on,
        linear_id,
        assignee,
        module,
        branch,
        model,
        acceptance_criteria,
        out_of_scope,
        files_to_modify,
        touches,
        domains,
        shipped_in,
        landing_ref,
        body,
        created_at,
        started_at,
        done_at,
        scored_at,
        blocked_reason,
        implemented,
        delivered_by,
        verified,
        verified_by,
        verification_ref,
        attempts,
        cross_repo,
    );

    let values = if verbose && !values.is_empty() {
        Some(values)
    } else {
        None
    };
    (fields, values)
}
