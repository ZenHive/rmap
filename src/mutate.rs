mod status;
pub use status::{TransitionFields, update_status_many_str, update_status_str};

use std::collections::HashSet;
use std::str::FromStr;

use thiserror::Error;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

use crate::schema::Relation;
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
    /// A new positive verification claim lacked independent-evaluator provenance.
    #[error(
        "--verified requires a non-empty --verified-by; provenance-free verification claims are not allowed"
    )]
    MissingVerifiedBy,
    /// Provenance was supplied without the positive verification it explains.
    #[error("--verified-by/--verification-ref require --verified")]
    ProvenanceWithoutVerification,
    /// `--landing-ref` was passed on a status other than `in_progress`.
    #[error(
        "--landing-ref is only settable on `in_progress` (got `{0}`); an open PR is recorded while the task stays in progress"
    )]
    LandingRefWrongStatus(String),
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
    pub relation: Relation,
}

impl CrossRepoSpec {
    pub fn parse(input: &str) -> Result<Self, MutateError> {
        let parts: Vec<&str> = input.splitn(3, ':').collect();
        match parts.as_slice() {
            [repo, task_id] if !repo.is_empty() && !task_id.is_empty() => Ok(Self {
                repo: (*repo).to_string(),
                task_id: (*task_id).to_string(),
                relation: Relation::Blocks,
            }),
            [repo, task_id, relation]
                if !repo.is_empty() && !task_id.is_empty() && !relation.is_empty() =>
            {
                Ok(Self {
                    repo: (*repo).to_string(),
                    task_id: (*task_id).to_string(),
                    relation: (*relation).into(),
                })
            }
            _ => Err(MutateError::InvalidCrossRepoSpec(input.to_string())),
        }
    }
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

/// Set or clear a task's `assignee` and `model` fields.
///
/// `assignee = None` removes both fields (the `none` / `human` unassign path).
/// `assignee = Some(agent)` sets the assignee; `model = Some(m)` also sets
/// `model`, while `model = None` leaves an existing model untouched. Re-validates
/// after edit so the dispatchable-pin gate (`validate_dispatch_model`) rejects
/// live agent-assigned tasks without a model before any write.
pub fn update_assignee_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    assignee: Option<&str>,
    model: Option<&str>,
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

        match assignee {
            None => {
                task.remove("assignee");
                task.remove("model");
            }
            Some(agent) => {
                let assignee_was_present = task.contains_key("assignee");
                task.insert("assignee", Item::Value(Value::from(agent)));
                if !assignee_was_present {
                    task.sort_values_by(|a, _, b, _| {
                        canonical_task_key_index(a.get()).cmp(&canonical_task_key_index(b.get()))
                    });
                }

                if let Some(model_id) = model {
                    let model_was_present = task.contains_key("model");
                    task.insert("model", Item::Value(Value::from(model_id)));
                    if !model_was_present {
                        task.sort_values_by(|a, _, b, _| {
                            canonical_task_key_index(a.get())
                                .cmp(&canonical_task_key_index(b.get()))
                        });
                    }
                }
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
pub fn canonical_task_key_index(key: &str) -> u32 {
    crate::schema::TASK_FIELDS
        .iter()
        .find(|field| field.name == key)
        .map_or(u32::MAX, |field| field.canonical_rank)
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

pub use crate::creation_input::NewTaskFields;

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
    fields.write_fields(&mut table);

    tasks.push(table);

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok((output, allocated_id))
}

#[cfg(test)]
mod tests;
