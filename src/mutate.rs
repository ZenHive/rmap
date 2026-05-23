use std::collections::HashSet;
use std::str::FromStr;

use thiserror::Error;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

use crate::schema::CrossRepo;
use crate::validate::{ValidateError, validate_tasks_str};

#[derive(Debug, Error)]
pub enum MutateError {
    #[error("{0}")]
    Toml(String),
    #[error("{0}")]
    Validate(#[from] ValidateError),
    #[error("missing [[task]] array")]
    MissingTasks,
    #[error("unknown task id {0}")]
    UnknownTaskId(String),
    /// `new --from-stdin` (or interactive `new`) tried to use an id already in the file.
    #[error("duplicate task id {0}")]
    DuplicateId(String),
    /// Caller passed an empty ID list — nothing to mutate.
    #[error("no task ids provided")]
    EmptyIds,
    /// `mark` invocation with zero `+`/`-` operations.
    #[error("no marker operations provided")]
    EmptyOps,
    /// A `mark` argument did not start with `+` or `-`.
    #[error("invalid marker op {0:?} — must start with '+' or '-' followed by a marker name")]
    InvalidMarkerOp(String),
    /// `depend` invocation with neither an in-repo target nor `--cross-repo`.
    #[error("no dependency specified — pass `on <task_id>` or `--cross-repo <spec>`")]
    NoDependency,
    /// `--cross-repo <repo>:<task_id>[:<relation>]` could not be parsed.
    #[error("invalid cross-repo spec {0:?} — expected '<repo>:<task_id>[:<relation>]'")]
    InvalidCrossRepoSpec(String),
    /// Numeric id space exhausted — the highest existing id is `u32::MAX`.
    #[error("cannot auto-allocate task id: u32 id space exhausted (highest existing id = {0})")]
    IdExhausted(u32),
}

/// A single marker operation parsed from `mark`'s positional arguments.
#[derive(Debug, Clone, Copy)]
pub enum MarkerOp<'a> {
    Add(&'a str),
    Remove(&'a str),
}

impl<'a> MarkerOp<'a> {
    /// Parse a `+marker` / `-marker` token. Returns `InvalidMarkerOp` otherwise.
    pub fn parse(token: &'a str) -> Result<Self, MutateError> {
        let mut chars = token.chars();
        match chars.next() {
            Some('+') => {
                let name = chars.as_str();
                if name.is_empty() {
                    return Err(MutateError::InvalidMarkerOp(token.to_string()));
                }
                Ok(Self::Add(name))
            }
            Some('-') => {
                let name = chars.as_str();
                if name.is_empty() {
                    return Err(MutateError::InvalidMarkerOp(token.to_string()));
                }
                Ok(Self::Remove(name))
            }
            _ => Err(MutateError::InvalidMarkerOp(token.to_string())),
        }
    }
}

/// Parsed `--cross-repo <repo>:<task_id>[:<relation>]` argument.
/// `relation` defaults to `"blocks"` when omitted, matching the most common usage.
#[derive(Debug, Clone)]
pub struct CrossRepoSpec {
    pub repo: String,
    pub task_id: String,
    pub relation: String,
}

impl CrossRepoSpec {
    pub fn parse(input: &str) -> Result<Self, MutateError> {
        let parts: Vec<&str> = input.splitn(3, ':').collect();
        match parts.as_slice() {
            [repo, task_id] if !repo.is_empty() && !task_id.is_empty() => Ok(Self {
                repo: (*repo).to_string(),
                task_id: (*task_id).to_string(),
                relation: "blocks".to_string(),
            }),
            [repo, task_id, relation]
                if !repo.is_empty() && !task_id.is_empty() && !relation.is_empty() =>
            {
                Ok(Self {
                    repo: (*repo).to_string(),
                    task_id: (*task_id).to_string(),
                    relation: (*relation).to_string(),
                })
            }
            _ => Err(MutateError::InvalidCrossRepoSpec(input.to_string())),
        }
    }
}

/// Atomically flip the `status` field on every task in `ids` to `new_status`.
/// All-or-nothing: if any ID is missing, returns `UnknownTaskId` and the input
/// string is not modified. Duplicates in `ids` are idempotent; an empty slice
/// errors with `EmptyIds`. Preserves TOML comments and whitespace via `toml_edit`.
///
/// Transitioning into `done` auto-fills `done_at = today_iso()` when absent;
/// transitioning into `in_progress` auto-fills `started_at = today_iso()` when
/// absent. Existing timestamps are never overwritten — re-runs are idempotent,
/// and re-opening a task (`done → pending → done`) preserves the original
/// `done_at`. Other transitions (`pending`, `blocked`, `superseded`) do not
/// touch lifecycle timestamps. When a timestamp is newly inserted, the task's
/// keys are re-sorted into canonical order so the field lands near the other
/// small scalars instead of being appended after multi-line entries.
///
/// `implemented`: when transitioning to `done` and a value is provided, write
/// it to every matched task's `implemented` field (overwriting any existing
/// value — the transition-time content is the most current). Ignored on
/// non-`done` transitions. When `None`, no write; the conditional-required
/// `validate_implemented` check will reject the transition unless the task
/// already carries an `implemented` field.
pub fn update_status_many_str(
    path: impl Into<String>,
    input: &str,
    ids: &[&str],
    new_status: &str,
    implemented: Option<&str>,
) -> Result<String, MutateError> {
    if ids.is_empty() {
        return Err(MutateError::EmptyIds);
    }

    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    let target: HashSet<&str> = ids.iter().copied().collect();
    let mut matched: HashSet<&str> = HashSet::new();
    let timestamp_field = match new_status {
        "done" => Some("done_at"),
        "in_progress" => Some("started_at"),
        _ => None,
    };
    let implemented_to_write = if new_status == "done" {
        implemented
    } else {
        None
    };
    // Snapshot once per call so a bulk transition that straddles midnight
    // stamps every matched task with the same date.
    let today = timestamp_field.map(|_| crate::today_iso());

    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };

        if target.contains(id.as_str()) {
            task["status"] = Item::Value(Value::from(new_status));

            let mut needs_sort = false;

            if let Some(field) = timestamp_field
                && !task.contains_key(field)
            {
                let today = today.as_deref().expect("today set when timestamp_field is");
                task.insert(field, Item::Value(Value::from(today)));
                needs_sort = true;
            }

            if let Some(value) = implemented_to_write {
                let was_present = task.contains_key("implemented");
                task.insert("implemented", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if needs_sort {
                task.sort_values_by(|a, _, b, _| {
                    canonical_task_key_index(a.get()).cmp(&canonical_task_key_index(b.get()))
                });
            }

            // Record which input ID this matched (preserving the original &str).
            if let Some(&orig) = ids.iter().find(|&&s| s == id.as_str()) {
                matched.insert(orig);
            }
        }
    }

    // Report the first unmatched ID in slice order for determinism.
    if let Some(&missing) = ids.iter().find(|&&s| !matched.contains(s)) {
        return Err(MutateError::UnknownTaskId(missing.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

pub fn update_status_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    new_status: &str,
    implemented: Option<&str>,
) -> Result<String, MutateError> {
    update_status_many_str(path, input, &[task_id], new_status, implemented)
}

fn item_to_task_id(item: &Item) -> Option<String> {
    item.as_value().and_then(|value| {
        value
            .as_integer()
            .map(|id| id.to_string())
            .or_else(|| value.as_str().map(str::to_string))
    })
}

/// How a task's `id` is stored in the document — integer or string. Needed by
/// `add_dependency_str` so a numeric-only string id (`id = "1"`) gets a string
/// dependency entry; pushing an integer `1` would not reconcile against
/// `TaskId::Text("1")` at validation time.
enum TaskIdShape {
    Integer(i64),
    Text,
}

fn target_id_shape(tasks: &ArrayOfTables, target: &str) -> Option<TaskIdShape> {
    for task in tasks {
        let Some(value) = task.get("id").and_then(Item::as_value) else {
            continue;
        };
        if let Some(int) = value.as_integer()
            && int.to_string() == target
        {
            return Some(TaskIdShape::Integer(int));
        } else if let Some(text) = value.as_str()
            && text == target
        {
            return Some(TaskIdShape::Text);
        }
    }
    None
}

/// Atomically apply a series of `+marker` / `-marker` operations to a single task.
/// Add is a no-op if the marker is already present; remove is a no-op if absent.
/// Validation runs once at the end; if any op produces an invalid file the input
/// is returned unchanged. Preserves comments and formatting via `toml_edit`.
///
/// When a new `markers` field is inserted into a task that didn't have one, the
/// task's keys are re-sorted into canonical order (mirroring `add_task_str`'s
/// write order) so the new field lands near the other small scalars rather than
/// appended at the end. Existing-field mutations (marker already present) do not
/// trigger the reorder.
pub fn update_markers_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    ops: &[MarkerOp<'_>],
) -> Result<String, MutateError> {
    if ops.is_empty() {
        return Err(MutateError::EmptyOps);
    }

    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    let mut matched = false;
    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };
        if id.as_str() != task_id {
            continue;
        }
        matched = true;

        let was_present = task.contains_key("markers");
        let became_empty;
        {
            let entry = task
                .entry("markers")
                .or_insert(Item::Value(Value::Array(Array::new())));
            let array = entry
                .as_array_mut()
                .ok_or_else(|| MutateError::Toml("markers is not an array".to_string()))?;

            for op in ops {
                match op {
                    MarkerOp::Add(marker) => {
                        let present = array.iter().any(|v| v.as_str() == Some(*marker));
                        if !present {
                            array.push(*marker);
                        }
                    }
                    MarkerOp::Remove(marker) => {
                        let position = array.iter().position(|v| v.as_str() == Some(*marker));
                        if let Some(index) = position {
                            array.remove(index);
                        }
                    }
                }
            }

            became_empty = array.is_empty();
        }

        // No-op ops on an absent `markers` field must not leave `markers = []` behind.
        if !was_present && became_empty {
            task.remove("markers");
        } else if !was_present {
            // Newly inserted markers: place near the other small scalars.
            task.sort_values_by(|a, _, b, _| {
                canonical_task_key_index(a.get()).cmp(&canonical_task_key_index(b.get()))
            });
        }

        break;
    }

    if !matched {
        return Err(MutateError::UnknownTaskId(task_id.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

/// Pin or unpin a task's `milestone` field. `Some(name)` sets it; `None`
/// removes it (idempotent if already absent). The validator rejects unknown
/// milestone references, so a misspelled name leaves the file byte-equal.
/// Newly-inserted `milestone` fields auto-sort to their canonical slot
/// (between `bundle` and `status`); idempotent overwrites do not.
pub fn update_milestone_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    milestone: Option<&str>,
) -> Result<String, MutateError> {
    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    let mut matched = false;
    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };
        if id.as_str() != task_id {
            continue;
        }
        matched = true;

        match milestone {
            Some(name) => {
                let was_present = task.contains_key("milestone");
                task.insert("milestone", Item::Value(Value::from(name)));
                if !was_present {
                    task.sort_values_by(|a, _, b, _| {
                        canonical_task_key_index(a.get()).cmp(&canonical_task_key_index(b.get()))
                    });
                }
            }
            None => {
                task.remove("milestone");
            }
        }

        break;
    }

    if !matched {
        return Err(MutateError::UnknownTaskId(task_id.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

/// Canonical position for keys inside a `[[task]]` table, used to re-place a
/// freshly-inserted field (e.g. `markers` from `rmap mark <id> +x`) near the
/// other small scalars. Returns `u32::MAX` for unknown keys so they sort to
/// the end without disturbing each other (stable sort preserves their
/// relative order).
fn canonical_task_key_index(key: &str) -> u32 {
    match key {
        "id" => 0,
        "phase" => 1,
        "bundle" => 2,
        "milestone" => 3,
        "status" => 4,
        "title" => 5,
        "scores" => 6,
        "markers" => 7,
        "depends_on" => 8,
        "acceptance_criteria" => 9,
        "out_of_scope" => 10,
        "files_to_modify" => 11,
        "cross_repo" => 12,
        "assignee" => 13,
        "linear_id" => 14,
        "module" => 15,
        "model" => 16,
        "branch" => 17,
        "blocked_reason" => 18,
        "body" => 19,
        "implemented" => 20,
        "created_at" => 21,
        "started_at" => 22,
        "scored_at" => 23,
        "done_at" => 24,
        "shipped_in" => 25,
        _ => u32::MAX,
    }
}

/// Atomically add an in-repo and/or cross-repo dependency to a single task.
/// Both targets are optional but at least one must be provided. Each is a no-op
/// when the dependency is already present (in-repo: matching task_id; cross-repo:
/// matching `(repo, task_id, relation)` triple). Re-validates after edit so cycles
/// and unknown task ids are rejected before the file is written.
///
/// In-repo target ids mirror the resolved target task's storage shape: a numeric-only
/// string id like `"100"` is appended as a TOML string when the target itself is
/// stored as a string, and as an integer when stored as an integer — required for
/// the validator's identity comparison (see `target_id_shape`).
pub fn add_dependency_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    in_repo: Option<&str>,
    cross_repo: Option<&CrossRepoSpec>,
) -> Result<String, MutateError> {
    if in_repo.is_none() && cross_repo.is_none() {
        return Err(MutateError::NoDependency);
    }

    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    // Look up the in-repo target's id storage shape before mutating, so a
    // numeric-string-keyed task (`id = "1"`) gets a matching string entry
    // rather than the integer the validator can't reconcile.
    let in_repo_target_shape = in_repo.map(|other| target_id_shape(tasks, other));

    let mut matched = false;
    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };
        if id.as_str() != task_id {
            continue;
        }
        matched = true;

        if let Some(other) = in_repo {
            let entry = task
                .entry("depends_on")
                .or_insert(Item::Value(Value::Array(Array::new())));
            let array = entry
                .as_array_mut()
                .ok_or_else(|| MutateError::Toml("depends_on is not an array".to_string()))?;

            let present = array.iter().any(|value| match value {
                Value::Integer(int) => int.value().to_string() == other,
                Value::String(string) => string.value() == other,
                _ => false,
            });

            if !present {
                match in_repo_target_shape.as_ref().and_then(Option::as_ref) {
                    Some(TaskIdShape::Integer(value)) => array.push(*value),
                    Some(TaskIdShape::Text) => array.push(other),
                    None => {
                        // Target not found in this document — let the validator
                        // produce the canonical "unknown task" error after write.
                        if let Ok(numeric) = other.parse::<i64>() {
                            array.push(numeric);
                        } else {
                            array.push(other);
                        }
                    }
                }
            }
        }

        if let Some(spec) = cross_repo {
            let entry = task
                .entry("cross_repo")
                .or_insert(Item::Value(Value::Array(Array::new())));
            let array = entry
                .as_array_mut()
                .ok_or_else(|| MutateError::Toml("cross_repo is not an array".to_string()))?;

            let present = array.iter().any(|value| {
                let Some(table) = value.as_inline_table() else {
                    return false;
                };
                let same_repo = table.get("repo").and_then(Value::as_str) == Some(&spec.repo);
                let same_task = match table.get("task_id") {
                    Some(Value::Integer(int)) => int.value().to_string() == spec.task_id,
                    Some(Value::String(string)) => string.value() == &spec.task_id,
                    _ => false,
                };
                let same_relation =
                    table.get("relation").and_then(Value::as_str) == Some(&spec.relation);
                same_repo && same_task && same_relation
            });

            if !present {
                let mut table = InlineTable::new();
                table.insert("repo", Value::from(spec.repo.as_str()));
                if let Ok(numeric) = spec.task_id.parse::<i64>() {
                    table.insert("task_id", Value::from(numeric));
                } else {
                    table.insert("task_id", Value::from(spec.task_id.as_str()));
                }
                table.insert("relation", Value::from(spec.relation.as_str()));
                array.push(Value::InlineTable(table));
            }
        }

        break;
    }

    if !matched {
        return Err(MutateError::UnknownTaskId(task_id.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

/// Typed task input for `add_task_str`. Mirrors the subset of `schema::Task`
/// fields that callers (interactive `new`, `new --from-stdin`) can populate at
/// creation time. Lifecycle timestamps (`started_at`, `done_at`,
/// `blocked_reason`, `shipped_in`) are never settable on creation — those
/// transitions are owned by `rmap status` (and equivalent lifecycle mutators).
#[derive(Debug, Default, Clone)]
pub struct NewTaskFields<'a> {
    /// Explicit numeric id. `None` requests auto-allocation (max existing id + 1).
    pub id: Option<u32>,
    pub phase: u32,
    pub bundle: &'a str,
    pub milestone: Option<&'a str>,
    pub title: &'a str,
    pub scores: (u32, u32, u32),
    /// Defaults to `"pending"` when empty.
    pub status: &'a str,
    pub markers: &'a [&'a str],
    pub depends_on: &'a [u32],
    pub acceptance_criteria: &'a [&'a str],
    pub out_of_scope: &'a [&'a str],
    /// Slice of file paths the task is scoped to. Empty slice = skip serialization.
    pub files_to_modify: &'a [String],
    /// Slice of cross-repo dependencies. Empty slice = skip serialization. Each
    /// entry's `task_id` is written as an integer when parseable, otherwise a
    /// string — mirroring `add_dependency_str`'s shape.
    pub cross_repo: &'a [CrossRepo],
    pub assignee: Option<&'a str>,
    pub linear_id: Option<&'a str>,
    pub module: Option<&'a str>,
    pub model: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub body: Option<&'a str>,
    pub created_at: Option<&'a str>,
    pub scored_at: Option<&'a str>,
}

/// Walk a `[[task]]` array and return the next free numeric id (`max + 1`).
/// Ids are counted whether the file stores them as TOML integers (`id = 28`,
/// rmap's own convention) or as numeric strings (`id = "28"`, harness's
/// convention) — `TaskId` is dual-form, so auto-allocation has to read both;
/// reading only integers restarts the sequence at `1` on a string-id roadmap
/// and silently allocates a colliding id. A non-numeric text id (`id = "MW-7"`)
/// has no place in the numeric sequence and is skipped. Returns `1` when the
/// array holds no numeric id. Returns `Err(MutateError::IdExhausted)` if the
/// highest existing id is `u32::MAX` (a real project would need ~4B tasks).
fn next_task_id(tasks: &toml_edit::ArrayOfTables) -> Result<u32, MutateError> {
    let mut max: u32 = 0;
    for task in tasks.iter() {
        if let Some(value) = task.get("id").and_then(Item::as_value)
            && let Some(n) = task_id_value_as_u32(value)
            && n > max
        {
            max = n;
        }
    }
    max.checked_add(1).ok_or(MutateError::IdExhausted(max))
}

/// A task `id` TOML value as a `u32`, whether it is integer-typed (`28`) or a
/// numeric string (`"28"`). `None` for a non-numeric text id (`"MW-7"`) or an
/// integer outside `u32` range.
fn task_id_value_as_u32(value: &Value) -> Option<u32> {
    value
        .as_integer()
        .and_then(|n| u32::try_from(n).ok())
        .or_else(|| value.as_str().and_then(|s| s.parse::<u32>().ok()))
}

/// `true` when the roadmap stores task ids as TOML strings (`id = "1"`, as
/// harness's roadmap does) rather than integers (`id = 1`, rmap's own
/// convention). An auto-allocated id is serialized to match, so `rmap new`
/// never mixes the two forms within one file — a mix defeats duplicate
/// detection, which keys on `TaskId` whose `Number(1)` and `Text("1")` are
/// distinct values. A file carrying any string-typed id counts as
/// string-typed (a mixed file recovers toward strings); an empty array
/// defaults to integer.
fn ids_are_string_typed(tasks: &toml_edit::ArrayOfTables) -> bool {
    tasks.iter().any(|task| {
        task.get("id")
            .and_then(Item::as_value)
            .is_some_and(Value::is_str)
    })
}

/// Append a `[[task]]` to the document, optionally auto-allocating the id.
/// Returns `(updated_document, allocated_id)`. Re-validates the full document
/// after insertion via `validate_tasks_str`; on validation failure the caller's
/// input is left untouched (mutation lives only in the returned `String`).
///
/// Atomicity: a duplicate explicit id, unknown phase/bundle, cycle, or any
/// other schema violation produces `Err(_)` before the caller writes — the
/// on-disk file is byte-equal to its pre-call state. For multi-task ingestion
/// (e.g. `new --from-stdin` with multiple `[[task]]` blocks), callers chain
/// calls and re-thread the returned string; the first failure aborts the
/// batch.
pub fn add_task_str(
    path: impl Into<String>,
    input: &str,
    fields: &NewTaskFields<'_>,
) -> Result<(String, u32), MutateError> {
    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    let allocated_id = match fields.id {
        Some(explicit) => {
            let explicit_str = explicit.to_string();
            let clash = tasks.iter().any(|task| {
                task.get("id").and_then(item_to_task_id).as_deref() == Some(explicit_str.as_str())
            });
            if clash {
                return Err(MutateError::DuplicateId(explicit_str));
            }
            explicit
        }
        None => next_task_id(tasks)?,
    };

    let mut table = Table::new();
    // Suppress the `[[task]]` header from being implicit — we want the standard
    // header to be emitted on serialisation so the row is well-formed in the file.
    table.set_implicit(false);
    table["id"] = if ids_are_string_typed(tasks) {
        Item::Value(Value::from(allocated_id.to_string()))
    } else {
        Item::Value(Value::from(allocated_id as i64))
    };
    table["phase"] = Item::Value(Value::from(fields.phase as i64));
    table["bundle"] = Item::Value(Value::from(fields.bundle));
    if let Some(milestone) = fields.milestone {
        table["milestone"] = Item::Value(Value::from(milestone));
    }
    let status_value = if fields.status.is_empty() {
        "pending"
    } else {
        fields.status
    };
    table["status"] = Item::Value(Value::from(status_value));
    table["title"] = Item::Value(Value::from(fields.title));

    let mut scores = InlineTable::new();
    scores.insert("d", Value::from(fields.scores.0 as i64));
    scores.insert("b", Value::from(fields.scores.1 as i64));
    scores.insert("u", Value::from(fields.scores.2 as i64));
    table["scores"] = Item::Value(Value::InlineTable(scores));

    if !fields.markers.is_empty() {
        let mut array = Array::new();
        for marker in fields.markers {
            array.push(*marker);
        }
        table["markers"] = Item::Value(Value::Array(array));
    }

    if !fields.depends_on.is_empty() {
        let mut array = Array::new();
        for dep in fields.depends_on {
            array.push(*dep as i64);
        }
        table["depends_on"] = Item::Value(Value::Array(array));
    }

    if !fields.acceptance_criteria.is_empty() {
        let mut array = Array::new();
        for ac in fields.acceptance_criteria {
            array.push(*ac);
        }
        table["acceptance_criteria"] = Item::Value(Value::Array(array));
    }

    if !fields.out_of_scope.is_empty() {
        let mut array = Array::new();
        for item in fields.out_of_scope {
            array.push(*item);
        }
        table["out_of_scope"] = Item::Value(Value::Array(array));
    }

    if !fields.files_to_modify.is_empty() {
        let mut array = Array::new();
        for path in fields.files_to_modify {
            array.push(path.as_str());
        }
        table["files_to_modify"] = Item::Value(Value::Array(array));
    }

    if !fields.cross_repo.is_empty() {
        let mut array = Array::new();
        for entry in fields.cross_repo {
            let mut inline = InlineTable::new();
            inline.insert("repo", Value::from(entry.repo.as_str()));
            let task_id_str = entry.task_id.to_string();
            if let Ok(numeric) = task_id_str.parse::<i64>() {
                inline.insert("task_id", Value::from(numeric));
            } else {
                inline.insert("task_id", Value::from(task_id_str.as_str()));
            }
            if let Some(linear_id) = &entry.linear_id {
                inline.insert("linear_id", Value::from(linear_id.as_str()));
            }
            inline.insert("relation", Value::from(entry.relation.as_str()));
            array.push(Value::InlineTable(inline));
        }
        table["cross_repo"] = Item::Value(Value::Array(array));
    }

    if let Some(assignee) = fields.assignee {
        table["assignee"] = Item::Value(Value::from(assignee));
    }
    if let Some(linear_id) = fields.linear_id {
        table["linear_id"] = Item::Value(Value::from(linear_id));
    }
    if let Some(module) = fields.module {
        table["module"] = Item::Value(Value::from(module));
    }
    if let Some(model) = fields.model {
        table["model"] = Item::Value(Value::from(model));
    }
    if let Some(branch) = fields.branch {
        table["branch"] = Item::Value(Value::from(branch));
    }
    if let Some(body) = fields.body {
        table["body"] = Item::Value(Value::from(body));
    }
    if let Some(created_at) = fields.created_at {
        table["created_at"] = Item::Value(Value::from(created_at));
    }
    if let Some(scored_at) = fields.scored_at {
        table["scored_at"] = Item::Value(Value::from(scored_at));
    }

    tasks.push(table);

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok((output, allocated_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_TOML: &str = r#"
schema_version = 2
project = "test"
default_branch = "main"

[phases.1]
name = "Phase One"
order = 1
status = "pending"

[bundles.b]
phase = 1
order = 1
description = "Bundle B"

[[task]]
id = 1
phase = 1
bundle = "b"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }
"#;

    #[test]
    fn update_status_many_str_empty_ids_returns_empty_ids_error() {
        let result = update_status_many_str("test.toml", SIMPLE_TOML, &[], "done", None);
        assert!(
            matches!(result, Err(MutateError::EmptyIds)),
            "expected EmptyIds, got: {result:?}"
        );
    }

    /// A roadmap whose task ids are TOML strings (`id = "1"`) — harness's
    /// convention, and the shape that exposed the `next_task_id` collision bug.
    const STRING_ID_TOML: &str = r#"
schema_version = 2
project = "test"
default_branch = "main"

[phases.1]
name = "Phase One"
order = 1
status = "pending"

[bundles.b]
phase = 1
order = 1
description = "Bundle B"

[[task]]
id = "1"
phase = 1
bundle = "b"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = "2"
phase = 1
bundle = "b"
status = "pending"
title = "Task Two"
scores = { d = 1, b = 5, u = 5 }
"#;

    fn new_task_fields(title: &str) -> NewTaskFields<'_> {
        NewTaskFields {
            phase: 1,
            bundle: "b",
            title,
            scores: (1, 5, 5),
            status: "pending",
            ..Default::default()
        }
    }

    #[test]
    fn add_task_to_string_id_file_continues_the_numeric_sequence() {
        // Before the fix `next_task_id` skipped string ids, so a string-id
        // roadmap restarted at 1 and the new task collided with `id = "1"`.
        let fields = new_task_fields("Task Three");
        let (_, allocated) =
            add_task_str("test.toml", STRING_ID_TOML, &fields).expect("add_task_str");

        assert_eq!(allocated, 3, "next id after \"1\" / \"2\" must be 3, not 1");
    }

    #[test]
    fn add_task_to_string_id_file_writes_the_id_as_a_string() {
        // The new id must mirror the file's form, or the document ends up with
        // `id = "1"` and `id = 3` mixed — and duplicate detection, keyed on
        // `TaskId`, cannot see `Number(3)` and `Text("3")` as the same id.
        let fields = new_task_fields("Task Three");
        let (output, _) =
            add_task_str("test.toml", STRING_ID_TOML, &fields).expect("add_task_str");

        assert!(
            output.contains("id = \"3\""),
            "new id must be string-typed to match the file:\n{output}"
        );
        assert!(
            !output.contains("\nid = 3\n"),
            "new id must not be written as a bare integer:\n{output}"
        );
    }

    #[test]
    fn add_task_to_integer_id_file_still_writes_the_id_as_an_integer() {
        // rmap's own roadmap uses integer ids — the fix must not regress it.
        let fields = new_task_fields("Task Two");
        let (output, allocated) =
            add_task_str("test.toml", SIMPLE_TOML, &fields).expect("add_task_str");

        assert_eq!(allocated, 2);
        assert!(
            output.contains("\nid = 2\n"),
            "integer-id roadmaps must keep integer ids:\n{output}"
        );
        assert!(
            !output.contains("id = \"2\""),
            "must not switch an integer-id roadmap to string ids:\n{output}"
        );
    }
}
