use super::*;

pub(super) fn validate_dependencies(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
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

pub(super) fn validate_dependency_cycles(
    path: &str,
    input: &str,
    tasks: &Tasks,
) -> Result<(), ValidateError> {
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
