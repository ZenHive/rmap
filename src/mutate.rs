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
}

pub fn update_status_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    new_status: &str,
) -> Result<String, MutateError> {
    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;
    let mut found = false;

    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };

        if id != task_id {
            continue;
        }

        task["status"] = Item::Value(Value::from(new_status));
        found = true;
        break;
    }

    if !found {
        return Err(MutateError::UnknownTaskId(task_id.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

fn item_to_task_id(item: &Item) -> Option<String> {
    item.as_value().and_then(|value| {
        value
            .as_integer()
            .map(|id| id.to_string())
            .or_else(|| value.as_str().map(str::to_string))
    })
}
