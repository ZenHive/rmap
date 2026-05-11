use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Serialize;

use crate::schema::{Bundle, Linear, Phase, Task, TaskId, Tasks};

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TaskDiff {
    pub id: TaskId,
    pub status: DiffStatus,
    pub changed_fields: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MetadataDiff {
    pub key: String,
    pub status: DiffStatus,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TomlDiff {
    pub metadata: Vec<MetadataDiff>,
    pub tasks: Vec<TaskDiff>,
}

impl TomlDiff {
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty() && self.tasks.is_empty()
    }
}

pub fn diff_toml(base: &Tasks, current: &Tasks) -> TomlDiff {
    TomlDiff {
        metadata: diff_metadata(base, current),
        tasks: diff_tasks(base, current),
    }
}

pub fn diff_tasks(base: &Tasks, current: &Tasks) -> Vec<TaskDiff> {
    let current_by_id = task_map(current);
    let mut seen = HashSet::new();
    let mut diff = Vec::new();

    for task in &base.task {
        let key = task.id.to_string();
        seen.insert(key.clone());

        match current_by_id.get(&key) {
            Some(current_task) => {
                let changed_fields = changed_fields(task, current_task);
                if !changed_fields.is_empty() {
                    diff.push(TaskDiff {
                        id: task.id.clone(),
                        status: DiffStatus::Changed,
                        changed_fields,
                    });
                }
            }
            None => diff.push(TaskDiff {
                id: task.id.clone(),
                status: DiffStatus::Removed,
                changed_fields: Vec::new(),
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
        });
    }

    diff
}

pub fn diff_metadata(base: &Tasks, current: &Tasks) -> Vec<MetadataDiff> {
    let mut diff = Vec::new();

    if base.schema_version != current.schema_version {
        diff.push(scalar_change("schema_version"));
    }
    if base.project != current.project {
        diff.push(scalar_change("project"));
    }
    if base.default_branch != current.default_branch {
        diff.push(scalar_change("default_branch"));
    }
    match (&base.linear, &current.linear) {
        (None, None) => {}
        (None, Some(_)) => diff.push(MetadataDiff {
            key: "linear".to_string(),
            status: DiffStatus::Added,
        }),
        (Some(_), None) => diff.push(MetadataDiff {
            key: "linear".to_string(),
            status: DiffStatus::Removed,
        }),
        (Some(base_linear), Some(current_linear)) => {
            diff.extend(diff_linear(base_linear, current_linear));
        }
    }

    diff.extend(map_diff("phases", &base.phases, &current.phases));
    diff.extend(map_diff("bundles", &base.bundles, &current.bundles));

    diff
}

pub fn format_diff(diff: &TomlDiff) -> String {
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
    }

    lines.join("\n")
}

fn scalar_change(key: &str) -> MetadataDiff {
    MetadataDiff {
        key: key.to_string(),
        status: DiffStatus::Changed,
    }
}

fn diff_linear(base: &Linear, current: &Linear) -> Vec<MetadataDiff> {
    let mut diff = Vec::new();
    if base.team_key != current.team_key {
        diff.push(scalar_change("linear.team_key"));
    }
    if base.workspace_url != current.workspace_url {
        diff.push(scalar_change("linear.workspace_url"));
    }
    diff
}

trait MapEntry: PartialEq {}
impl MapEntry for Phase {}
impl MapEntry for Bundle {}

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
                })
            }
            (None, Some(_)) => Some(MetadataDiff {
                key: format!("{namespace}.{key}"),
                status: DiffStatus::Added,
            }),
            (Some(_), None) => Some(MetadataDiff {
                key: format!("{namespace}.{key}"),
                status: DiffStatus::Removed,
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
// a field, add it here (and to `export::ExportedTask`) — otherwise `rmap diff`
// silently misses the field. `id` is intentionally absent (it's the map key).
fn changed_fields(base: &Task, current: &Task) -> Vec<String> {
    let mut fields = Vec::new();

    macro_rules! diff_fields {
        ($($field:ident),* $(,)?) => {
            $(if base.$field != current.$field {
                fields.push(stringify!($field).to_string());
            })*
        };
    }

    diff_fields!(
        phase,
        bundle,
        status,
        title,
        scores,
        markers,
        depends_on,
        linear_id,
        assignee,
        acceptance_criteria,
        shipped_in,
        body,
        created_at,
        started_at,
        done_at,
        scored_at,
        blocked_reason,
        cross_repo,
    );

    fields
}
