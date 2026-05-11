use std::collections::HashSet;
use std::str::FromStr;

use thiserror::Error;
use toml_edit::{DocumentMut, Item, Value};

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
