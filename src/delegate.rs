use std::fmt::Write;

use crate::query::find_task;
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency};

pub fn format_delegate_prompt(tasks: &Tasks, id: &str, target: &str) -> Option<String> {
    let task = find_task(tasks, id)?;
    Some(format_prompt(tasks, task, target))
}

fn format_prompt(tasks: &Tasks, task: &Task, target: &str) -> String {
    let mut prompt = String::new();
    writeln!(prompt, "# Task {}: {}", task.id, task.title).expect("write to string");
    writeln!(prompt).expect("write to string");
    writeln!(prompt, "Target agent: {target}").expect("write to string");
    if let Some(assignee) = &task.assignee {
        writeln!(prompt, "Stored assignee: {assignee}").expect("write to string");
    }
    writeln!(prompt, "Project: {}", tasks.project).expect("write to string");
    writeln!(prompt, "Status: {}", task.status).expect("write to string");
    writeln!(prompt, "Phase: {}", task.phase).expect("write to string");
    writeln!(prompt, "Bundle: {}", task.bundle).expect("write to string");
    writeln!(
        prompt,
        "Scores: D:{}/B:{}/U:{} -> Eff:{}",
        task.scores.d,
        task.scores.b,
        task.scores.u,
        format_efficiency(efficiency(task))
    )
    .expect("write to string");

    if !task.markers.is_empty() {
        writeln!(prompt, "Markers: {}", task.markers.join(", ")).expect("write to string");
    }
    if let Some(linear_id) = &task.linear_id {
        writeln!(prompt, "Linear: {linear_id}").expect("write to string");
    }
    if let Some(shipped_in) = &task.shipped_in {
        writeln!(prompt, "Shipped in: {shipped_in}").expect("write to string");
    }

    append_dependencies(&mut prompt, tasks, task);
    append_cross_repo(&mut prompt, task);
    append_body(&mut prompt, task);
    append_acceptance_criteria(&mut prompt, task);
    append_instructions(&mut prompt);

    prompt
}

fn append_dependencies(prompt: &mut String, tasks: &Tasks, task: &Task) {
    if task.depends_on.is_empty() {
        return;
    }

    writeln!(prompt).expect("write to string");
    writeln!(prompt, "## Dependencies").expect("write to string");
    for dependency in &task.depends_on {
        match find_task(tasks, &dependency.to_string()) {
            Some(dependency_task) => writeln!(
                prompt,
                "- Task {} [{}] {}",
                dependency_task.id, dependency_task.status, dependency_task.title
            )
            .expect("write to string"),
            None => writeln!(prompt, "- Task {dependency}").expect("write to string"),
        }
    }
}

fn append_cross_repo(prompt: &mut String, task: &Task) {
    if task.cross_repo.is_empty() {
        return;
    }

    writeln!(prompt).expect("write to string");
    writeln!(prompt, "## Cross-repo dependencies").expect("write to string");
    for dependency in &task.cross_repo {
        let linear = dependency
            .linear_id
            .as_ref()
            .map(|linear_id| format!(" ({linear_id})"))
            .unwrap_or_default();
        writeln!(
            prompt,
            "- {} {} task {}{}",
            dependency.relation, dependency.repo, dependency.task_id, linear
        )
        .expect("write to string");
    }
}

fn append_body(prompt: &mut String, task: &Task) {
    let Some(body) = task.body.as_ref().map(|body| body.trim()) else {
        return;
    };
    if body.is_empty() {
        return;
    }

    writeln!(prompt).expect("write to string");
    writeln!(prompt, "## Task body").expect("write to string");
    writeln!(prompt, "{body}").expect("write to string");
}

fn append_acceptance_criteria(prompt: &mut String, task: &Task) {
    if task.acceptance_criteria.is_empty() {
        return;
    }

    writeln!(prompt).expect("write to string");
    writeln!(prompt, "## Acceptance criteria").expect("write to string");
    for criterion in &task.acceptance_criteria {
        writeln!(prompt, "- [ ] {criterion}").expect("write to string");
    }
}

fn append_instructions(prompt: &mut String) {
    writeln!(prompt).expect("write to string");
    writeln!(prompt, "## Instructions").expect("write to string");
    writeln!(prompt, "- Inspect the repo before editing.").expect("write to string");
    writeln!(prompt, "- Implement only this task's scope.").expect("write to string");
    writeln!(prompt, "- Add or update tests for the changed behavior.").expect("write to string");
    writeln!(prompt, "- Report verification commands and results.").expect("write to string");
}
