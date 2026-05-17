use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::schema::{Task, TaskId, Tasks};

const SUPPORTED_SCHEMA_VERSION: u32 = 2;
const VALID_STATUSES: &[&str] = &["pending", "in_progress", "blocked", "done", "superseded"];
const VALID_MILESTONE_STATUSES: &[&str] = &["pending", "active", "done"];
pub const VALID_MARKERS: &[&str] = &["parallel", "cx", "csr", "bug", "security", "docs"];
const VALID_ASSIGNEES: &[&str] = &["human", "claude", "codex", "cursor"];
const VALID_CROSS_REPO_RELATIONS: &[&str] = &["blocks", "blocked_by", "related"];
const MIN_SCORE: u32 = 1;
const MAX_SCORE: u32 = 10;
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

/// Runs every semantic check against an already-parsed `Tasks` and collects ALL findings
/// instead of short-circuiting on the first error.
///
/// The schema-parse step is excluded — by definition `tasks` is already parsed.
/// All other checks run in order; their errors accumulate into the returned Vec.
pub fn collect_findings(tasks: &Tasks, path: &str, input: &str) -> Vec<ValidateError> {
    let mut findings: Vec<ValidateError> = Vec::new();

    if let Err(e) = validate_schema_version(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_statuses(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_markers(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_scores(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_assignees(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_linear_ids(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_timestamps(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_blocked_reasons(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_implemented(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_dependencies(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_dependency_cycles(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_cross_repo_relations(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_phase_and_bundle_references(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_milestone_statuses(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_milestone_references(path, input, tasks) {
        findings.push(e);
    }
    if let Err(e) = validate_focus_phase(path, input, tasks) {
        findings.push(e);
    }

    findings
}

pub fn validate_tasks_str(path: impl Into<String>, input: &str) -> Result<Tasks, ValidateError> {
    let path = path.into();
    let tasks: Tasks = toml::from_str(input).map_err(|err| parse_error(&path, input, err))?;

    validate_schema_version(&path, input, &tasks)?;
    validate_statuses(&path, input, &tasks)?;
    validate_markers(&path, input, &tasks)?;
    validate_scores(&path, input, &tasks)?;
    validate_assignees(&path, input, &tasks)?;
    validate_linear_ids(&path, input, &tasks)?;
    validate_timestamps(&path, input, &tasks)?;
    validate_blocked_reasons(&path, input, &tasks)?;
    validate_implemented(&path, input, &tasks)?;
    validate_dependencies(&path, input, &tasks)?;
    validate_dependency_cycles(&path, input, &tasks)?;
    validate_cross_repo_relations(&path, input, &tasks)?;
    validate_phase_and_bundle_references(&path, input, &tasks)?;
    validate_milestone_statuses(&path, input, &tasks)?;
    validate_milestone_references(&path, input, &tasks)?;
    validate_focus_phase(&path, input, &tasks)?;

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
            "unsupported schema_version {} (expected {}); see CHANGELOG.md for v{} migration",
            tasks.schema_version, SUPPORTED_SCHEMA_VERSION, SUPPORTED_SCHEMA_VERSION
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

fn validate_scores(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        ensure_score_in_range(path, input, task, "d", task.scores.d)?;
        ensure_score_in_range(path, input, task, "b", task.scores.b)?;
        ensure_score_in_range(path, input, task, "u", task.scores.u)?;
    }

    Ok(())
}

fn ensure_score_in_range(
    path: &str,
    input: &str,
    task: &Task,
    field: &str,
    value: u32,
) -> Result<(), ValidateError> {
    if (MIN_SCORE..=MAX_SCORE).contains(&value) {
        return Ok(());
    }

    let locator = format!(
        "scores = {{ d = {}, b = {}, u = {} }}",
        task.scores.d, task.scores.b, task.scores.u
    );

    Err(semantic_error(
        path,
        line_containing(input, &locator).unwrap_or(FIRST_LINE_NUMBER),
        format!(
            "task {} scores.{field} = {value} must be in {MIN_SCORE}..={MAX_SCORE}",
            task.id
        ),
    ))
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

fn validate_dependency_cycles(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    let graph: HashMap<TaskId, &[TaskId]> = tasks
        .task
        .iter()
        .map(|task| (task.id.clone(), task.depends_on.as_slice()))
        .collect();

    let mut state: HashMap<TaskId, VisitState> = HashMap::new();
    let mut stack: Vec<TaskId> = Vec::new();

    for task in &tasks.task {
        if state.contains_key(&task.id) {
            continue;
        }

        if let Some(cycle) = detect_cycle(&task.id, &graph, &mut state, &mut stack) {
            let members = cycle
                .iter()
                .map(TaskId::to_string)
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(semantic_error(
                path,
                line_containing(input, "depends_on").unwrap_or(FIRST_LINE_NUMBER),
                format!("dependency cycle detected: {members}"),
            ));
        }
    }

    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum VisitState {
    InProgress,
    Done,
}

fn detect_cycle(
    node: &TaskId,
    graph: &HashMap<TaskId, &[TaskId]>,
    state: &mut HashMap<TaskId, VisitState>,
    stack: &mut Vec<TaskId>,
) -> Option<Vec<TaskId>> {
    state.insert(node.clone(), VisitState::InProgress);
    stack.push(node.clone());

    if let Some(dependencies) = graph.get(node) {
        for dependency in *dependencies {
            match state.get(dependency) {
                Some(VisitState::Done) => continue,
                Some(VisitState::InProgress) => {
                    let start = stack.iter().position(|id| id == dependency).unwrap_or(0);
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(dependency.clone());
                    return Some(cycle);
                }
                None => {
                    if let Some(cycle) = detect_cycle(dependency, graph, state, stack) {
                        return Some(cycle);
                    }
                }
            }
        }
    }

    stack.pop();
    state.insert(node.clone(), VisitState::Done);
    None
}

fn validate_timestamps(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        ensure_iso_date(path, input, "created_at", task.created_at.as_deref())?;
        ensure_iso_date(path, input, "started_at", task.started_at.as_deref())?;
        ensure_iso_date(path, input, "done_at", task.done_at.as_deref())?;
        ensure_iso_date(path, input, "scored_at", task.scored_at.as_deref())?;
    }

    Ok(())
}

fn ensure_iso_date(
    path: &str,
    input: &str,
    field: &str,
    value: Option<&str>,
) -> Result<(), ValidateError> {
    let Some(value) = value else {
        return Ok(());
    };

    if is_iso_8601_date(value) {
        return Ok(());
    }

    Err(semantic_error(
        path,
        line_containing(input, &format!("{field} = \"{value}\"")).unwrap_or(FIRST_LINE_NUMBER),
        format!(
            "{field} \"{value}\" must match YYYY-MM-DD format (4-digit year, 2-digit month, 2-digit day)"
        ),
    ))
}

fn is_iso_8601_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 {
        return false;
    }

    let digits =
        |range: std::ops::Range<usize>| range.into_iter().all(|i| bytes[i].is_ascii_digit());

    digits(0..4) && bytes[4] == b'-' && digits(5..7) && bytes[7] == b'-' && digits(8..10)
}

fn validate_blocked_reasons(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        if task.status != "blocked" {
            continue;
        }

        if task.blocked_reason.is_some() {
            continue;
        }

        return Err(semantic_error(
            path,
            line_containing(input, "status = \"blocked\"").unwrap_or(FIRST_LINE_NUMBER),
            format!("task {} is blocked but missing blocked_reason", task.id),
        ));
    }

    Ok(())
}

fn validate_implemented(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    for task in &tasks.task {
        if task.status != "done" {
            continue;
        }

        if task.implemented.as_deref().is_some_and(|s| !s.is_empty()) {
            continue;
        }

        return Err(semantic_error(
            path,
            line_containing(input, "status = \"done\"").unwrap_or(FIRST_LINE_NUMBER),
            format!("task {} is done but missing implemented", task.id),
        ));
    }

    Ok(())
}

fn validate_focus_phase(path: &str, input: &str, tasks: &Tasks) -> Result<(), ValidateError> {
    let Some(focus) = &tasks.focus else {
        return Ok(());
    };

    if tasks.phases.contains_key(&focus.phase.to_string()) {
        return Ok(());
    }

    Err(semantic_error(
        path,
        line_containing(input, &format!("phase = {}", focus.phase)).unwrap_or(FIRST_LINE_NUMBER),
        format!(
            "[focus].phase {} does not match any [phases.N] entry",
            focus.phase
        ),
    ))
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

fn validate_milestone_statuses(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
    for (key, milestone) in &tasks.milestones {
        if VALID_MILESTONE_STATUSES.contains(&milestone.status.as_str()) {
            continue;
        }
        return Err(semantic_error(
            path,
            line_containing(input, &format!("[milestones.{key}]")).unwrap_or(FIRST_LINE_NUMBER),
            format!(
                "milestone \"{key}\" has invalid status \"{}\" (expected one of pending, active, done)",
                milestone.status
            ),
        ));
    }
    Ok(())
}

fn validate_milestone_references(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
    for task in &tasks.task {
        let Some(name) = &task.milestone else {
            continue;
        };
        if tasks.milestones.contains_key(name) {
            continue;
        }
        return Err(semantic_error(
            path,
            line_containing(input, &format!("milestone = \"{name}\"")).unwrap_or(FIRST_LINE_NUMBER),
            format!("task {} references unknown milestone \"{name}\"", task.id),
        ));
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
