use std::collections::HashSet;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::schema::{TaskId, Tasks};

const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const VALID_STATUSES: &[&str] = &["pending", "in_progress", "blocked", "done", "superseded"];
const VALID_MARKERS: &[&str] = &["parallel", "cx", "csr"];
const VALID_ASSIGNEES: &[&str] = &["human", "claude", "codex", "cursor"];
const VALID_CROSS_REPO_RELATIONS: &[&str] = &["blocks", "blocked_by", "related"];
const FIRST_LINE_NUMBER: usize = 1;

#[derive(Debug, Error)]
pub enum ValidateError {
    #[error("{path}:{line}: invalid TOML: {message}")]
    Parse {
        path: String,
        line: usize,
        message: String,
    },
    #[error("{path}:{line}: {message}")]
    Semantic {
        path: String,
        line: usize,
        message: String,
    },
}

pub fn validate_tasks_file(path: &Path) -> Result<Tasks, ValidateError> {
    let input = fs::read_to_string(path).map_err(|err| ValidateError::Parse {
        path: path.display().to_string(),
        line: FIRST_LINE_NUMBER,
        message: err.to_string(),
    })?;

    validate_tasks_str(path.display().to_string(), &input)
}

pub fn validate_tasks_str(path: impl Into<String>, input: &str) -> Result<Tasks, ValidateError> {
    let path = path.into();
    let tasks: Tasks = toml::from_str(input).map_err(|err| parse_error(&path, input, err))?;

    validate_schema_version(&path, input, &tasks)?;
    validate_statuses(&path, input, &tasks)?;
    validate_markers(&path, input, &tasks)?;
    validate_assignees(&path, input, &tasks)?;
    validate_linear_ids(&path, input, &tasks)?;
    validate_dependencies(&path, input, &tasks)?;
    validate_cross_repo_relations(&path, input, &tasks)?;
    validate_phase_and_bundle_references(&path, input, &tasks)?;

    Ok(tasks)
}

fn validate_schema_version(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    if tasks.schema_version == SUPPORTED_SCHEMA_VERSION {
        return Ok(());
    }

    Err(semantic_error(
        path,
        line_containing(input, "schema_version").unwrap_or(FIRST_LINE_NUMBER),
        format!(
            "unsupported schema_version {}; run rmap migrate before rendering",
            tasks.schema_version
        ),
    ))
}

fn validate_statuses(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for phase in tasks.phases.values() {
        ensure_status(path, input, &phase.status)?;
    }

    for task in &tasks.task {
        ensure_status(path, input, &task.status)?;
    }

    Ok(())
}

fn ensure_status(path: &str, input: &str, status: &str) -> Result<(), ValidateError> {
    if VALID_STATUSES.contains(&status) {
        return Ok(());
    }

    Err(semantic_error(
        path,
        line_containing(input, &format!("status = \"{status}\"")).unwrap_or(FIRST_LINE_NUMBER),
        format!("invalid status \"{status}\""),
    ))
}

fn validate_markers(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        for marker in &task.markers {
            if VALID_MARKERS.contains(&marker.as_str()) {
                continue;
            }

            return Err(semantic_error(
                path,
                line_containing(input, &format!("\"{marker}\"")).unwrap_or(FIRST_LINE_NUMBER),
                format!("invalid marker \"{marker}\""),
            ));
        }
    }

    Ok(())
}

fn validate_assignees(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        let Some(assignee) = &task.assignee else {
            continue;
        };

        if VALID_ASSIGNEES.contains(&assignee.as_str()) {
            continue;
        }

        return Err(semantic_error(
            path,
            line_containing(input, &format!("assignee = \"{assignee}\""))
                .unwrap_or(FIRST_LINE_NUMBER),
            format!("invalid assignee \"{assignee}\""),
        ));
    }

    Ok(())
}

fn validate_linear_ids(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    let Some(linear) = &tasks.linear else {
        return Ok(());
    };

    for task in &tasks.task {
        let Some(linear_id) = &task.linear_id else {
            continue;
        };

        if matches_linear_team(linear_id, &linear.team_key) {
            continue;
        }

        return Err(semantic_error(
            path,
            line_containing(input, &format!("linear_id = \"{linear_id}\""))
                .unwrap_or(FIRST_LINE_NUMBER),
            format!(
                "linear_id \"{linear_id}\" must match {}-<integer>",
                linear.team_key
            ),
        ));
    }

    Ok(())
}

fn validate_dependencies(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    let task_ids: HashSet<TaskId> = tasks.task.iter().map(|task| task.id.clone()).collect();

    for task in &tasks.task {
        for dependency in &task.depends_on {
            if task_ids.contains(dependency) {
                continue;
            }

            return Err(semantic_error(
                path,
                line_containing(input, "depends_on").unwrap_or(FIRST_LINE_NUMBER),
                format!("task {} depends on unknown task {dependency}", task.id),
            ));
        }
    }

    Ok(())
}

fn validate_cross_repo_relations(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
    for task in &tasks.task {
        for dependency in &task.cross_repo {
            if VALID_CROSS_REPO_RELATIONS.contains(&dependency.relation.as_str()) {
                continue;
            }

            return Err(semantic_error(
                path,
                line_containing(input, &format!("relation = \"{}\"", dependency.relation))
                    .unwrap_or(FIRST_LINE_NUMBER),
                format!("invalid cross_repo relation \"{}\"", dependency.relation),
            ));
        }
    }

    Ok(())
}

fn validate_phase_and_bundle_references(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
    for task in &tasks.task {
        if !tasks.phases.contains_key(&task.phase.to_string()) {
            return Err(semantic_error(
                path,
                line_containing(input, &format!("phase = {}", task.phase))
                    .unwrap_or(FIRST_LINE_NUMBER),
                format!("task {} references unknown phase {}", task.id, task.phase),
            ));
        }

        if !tasks.bundles.contains_key(&task.bundle) {
            return Err(semantic_error(
                path,
                line_containing(input, &format!("bundle = \"{}\"", task.bundle))
                    .unwrap_or(FIRST_LINE_NUMBER),
                format!(
                    "task {} references unknown bundle \"{}\"",
                    task.id, task.bundle
                ),
            ));
        }
    }

    Ok(())
}

fn matches_linear_team(linear_id: &str, team_key: &str) -> bool {
    let Some(number) = linear_id.strip_prefix(&format!("{team_key}-")) else {
        return false;
    };

    !number.is_empty() && number.chars().all(|character| character.is_ascii_digit())
}

fn parse_error(path: &str, input: &str, err: toml::de::Error) -> ValidateError {
    let line = err
        .span()
        .map(|span| line_for_offset(input, span.start))
        .unwrap_or(FIRST_LINE_NUMBER);

    ValidateError::Parse {
        path: path.to_string(),
        line,
        message: err.message().to_string(),
    }
}

fn semantic_error(path: &str, line: usize, message: String) -> ValidateError {
    ValidateError::Semantic {
        path: path.to_string(),
        line,
        message,
    }
}

fn line_containing(input: &str, needle: &str) -> Option<usize> {
    input
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + FIRST_LINE_NUMBER)
}

fn line_for_offset(input: &str, offset: usize) -> usize {
    input[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + FIRST_LINE_NUMBER
}
