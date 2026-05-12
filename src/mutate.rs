use std::collections::HashSet;
use std::str::FromStr;

use thiserror::Error;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

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
pub fn update_status_many_str(
    path: impl Into<String>,
    input: &str,
    ids: &[&str],
    new_status: &str,
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

    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };

        if target.contains(id.as_str()) {
            task["status"] = Item::Value(Value::from(new_status));
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
) -> Result<String, MutateError> {
    update_status_many_str(path, input, &[task_id], new_status)
}

fn item_to_task_id(item: &Item) -> Option<String> {
    item.as_value().and_then(|value| {
        value
            .as_integer()
            .map(|id| id.to_string())
            .or_else(|| value.as_str().map(str::to_string))
    })
}

/// Atomically apply a series of `+marker` / `-marker` operations to a single task.
/// Add is a no-op if the marker is already present; remove is a no-op if absent.
/// Validation runs once at the end; if any op produces an invalid file the input
/// is returned unchanged. Preserves comments and formatting via `toml_edit`.
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

/// Atomically add an in-repo and/or cross-repo dependency to a single task.
/// Both targets are optional but at least one must be provided. Each is a no-op
/// when the dependency is already present (in-repo: matching task_id; cross-repo:
/// matching `(repo, task_id, relation)` triple). Re-validates after edit so cycles
/// and unknown task ids are rejected before the file is written.
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
                if let Ok(numeric) = other.parse::<i64>() {
                    array.push(numeric);
                } else {
                    array.push(other);
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

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_TOML: &str = r#"
schema_version = 1
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
        let result = update_status_many_str("test.toml", SIMPLE_TOML, &[], "done");
        assert!(
            matches!(result, Err(MutateError::EmptyIds)),
            "expected EmptyIds, got: {result:?}"
        );
    }
}
