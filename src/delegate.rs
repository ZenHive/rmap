use std::fmt::Write;

use crate::query::find_task;
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency};

// `Write for String` is infallible — the macro lets call sites read like
// plain text without `.expect(...)` noise on every line.
macro_rules! line {
    ($buf:expr) => {
        $buf.push('\n')
    };
    ($buf:expr, $($arg:tt)*) => {
        writeln!($buf, $($arg)*).expect("writeln to String is infallible")
    };
}

pub fn format_delegate_prompt(tasks: &Tasks, id: &str, target: &str) -> Option<String> {
    let task = find_task(tasks, id)?;
    Some(format_prompt(tasks, task, target))
}

fn format_prompt(tasks: &Tasks, task: &Task, target: &str) -> String {
    let mut prompt = String::new();
    line!(prompt, "# Task {}: {}", task.id, task.title);
    line!(prompt);
    line!(prompt, "Target agent: {target}");
    if let Some(assignee) = &task.assignee
        && assignee != target
    {
        // Surface the override so the receiving agent knows the stored
        // routing intent differed from the explicit `--to`. When they match,
        // the line is redundant with `Target agent:` above.
        line!(prompt, "Stored assignee: {assignee} (overridden)");
    }
    line!(prompt, "Project: {}", tasks.project);
    line!(prompt, "Status: {}", task.status);
    line!(prompt, "Phase: {}", task.phase);
    line!(prompt, "Bundle: {}", task.bundle);
    line!(
        prompt,
        "Scores: D:{}/B:{}/U:{} -> Eff:{}",
        task.scores.d,
        task.scores.b,
        task.scores.u,
        format_efficiency(efficiency(task))
    );

    if !task.markers.is_empty() {
        line!(prompt, "Markers: {}", task.markers.join(", "));
    }
    if let Some(linear_id) = &task.linear_id {
        line!(prompt, "Linear: {linear_id}");
    }
    if let Some(shipped_in) = &task.shipped_in {
        line!(prompt, "Shipped in: {shipped_in}");
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

    line!(prompt);
    line!(prompt, "## Dependencies");
    for dependency in &task.depends_on {
        match find_task(tasks, &dependency.to_string()) {
            Some(dependency_task) => line!(
                prompt,
                "- Task {} [{}] {}",
                dependency_task.id,
                dependency_task.status,
                dependency_task.title
            ),
            None => line!(prompt, "- Task {dependency}"),
        }
    }
}

fn append_cross_repo(prompt: &mut String, task: &Task) {
    if task.cross_repo.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Cross-repo dependencies");
    for dependency in &task.cross_repo {
        let linear = dependency
            .linear_id
            .as_ref()
            .map(|linear_id| format!(" ({linear_id})"))
            .unwrap_or_default();
        line!(
            prompt,
            "- {} {} task {}{}",
            dependency.relation,
            dependency.repo,
            dependency.task_id,
            linear
        );
    }
}

fn append_body(prompt: &mut String, task: &Task) {
    let Some(body) = task.body.as_ref().map(|body| body.trim()) else {
        return;
    };
    if body.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Task body");
    line!(prompt, "{body}");
}

fn append_acceptance_criteria(prompt: &mut String, task: &Task) {
    if task.acceptance_criteria.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Acceptance criteria");
    for criterion in &task.acceptance_criteria {
        line!(prompt, "- [ ] {criterion}");
    }
}

fn append_instructions(prompt: &mut String) {
    line!(prompt);
    line!(prompt, "## Instructions");
    line!(prompt, "- Inspect the repo before editing.");
    line!(prompt, "- Implement only this task's scope.");
    line!(prompt, "- Add or update tests for the changed behavior.");
    line!(prompt, "- Report verification commands and results.");
}
