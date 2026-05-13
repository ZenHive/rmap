use std::fmt::Write;

use clap::ValueEnum;

use crate::query::find_task;
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum DelegateTarget {
    Claude,
    Codex,
    Cursor,
}

impl DelegateTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }
}

impl std::fmt::Display for DelegateTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

pub fn format_delegate_prompt(tasks: &Tasks, id: &str, target: DelegateTarget) -> Option<String> {
    let task = find_task(tasks, id)?;
    Some(format_prompt(tasks, task, target))
}

fn format_prompt(tasks: &Tasks, task: &Task, target: DelegateTarget) -> String {
    let mut prompt = String::new();
    line!(prompt, "# Task {}: {}", task.id, task.title);
    line!(prompt);
    line!(prompt, "Target agent: {target}");
    if let Some(assignee) = &task.assignee
        && assignee != target.as_str()
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
    let eff = efficiency(task);
    line!(
        prompt,
        "Scores: D:{}/B:{}/U:{} -> Eff:{} {}",
        task.scores.d,
        task.scores.b,
        task.scores.u,
        format_efficiency(eff),
        tier_glyph(eff)
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
    append_files_to_modify(&mut prompt, task);
    append_out_of_scope(&mut prompt, task);
    append_agent_notes(&mut prompt, target);
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

fn append_files_to_modify(prompt: &mut String, task: &Task) {
    if task.files_to_modify.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Files to modify");
    for path in &task.files_to_modify {
        line!(prompt, "- {path}");
    }
}

fn append_out_of_scope(prompt: &mut String, task: &Task) {
    if task.out_of_scope.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Out of scope");
    for item in &task.out_of_scope {
        line!(prompt, "- {item}");
    }
}

// Per-agent reachability notes. Source of truth is
// `~/.claude/includes/cloud-agent-environments.md`; when that changes
// (e.g. Codex sandbox gains/loses a capability), sync these literals by hand.
fn append_agent_notes(prompt: &mut String, target: DelegateTarget) {
    line!(prompt);
    line!(prompt, "## Environment notes");
    match target {
        DelegateTarget::Codex => {
            line!(
                prompt,
                "- Network access varies by environment config: default is offline. The \"Common dependencies\" preset reaches crates.io / npmjs / PyPI and ~70 dev domains; hex.pm and many vendor docs are NOT in that preset. Try before trusting; fall back to context already in this prompt when blocked."
            );
            line!(
                prompt,
                "- Project toolchains are not guaranteed (e.g. Elixir/Erlang/mix are absent on the default image). Don't claim harness runs you couldn't actually execute."
            );
            line!(
                prompt,
                "- Verify against shipped code and the context already in this prompt; flag uncertainty explicitly rather than guessing from training-data recall."
            );
            line!(
                prompt,
                "- The local reviewer runs the harness — list addressed acceptance criteria in the PR description."
            );
        }
        DelegateTarget::Cursor => {
            line!(
                prompt,
                "- Full network: hex.pm / crates.io / npm and general HTTP are reachable."
            );
            line!(
                prompt,
                "- Run the full project harness green before opening the PR (format, compile, lint, tests, type-check as applicable)."
            );
            line!(
                prompt,
                "- A red harness at PR-open is a push-back finding regardless of severity; the reviewer expects mechanical drift caught upstream of review."
            );
            line!(
                prompt,
                "- asdf shims may intercept toolchain binaries — set explicit PATHs if a runtime appears \"missing\" mid-session."
            );
        }
        DelegateTarget::Claude => {
            line!(
                prompt,
                "- Local execution: full toolchain, internet, and project state available."
            );
            line!(
                prompt,
                "- Run verification commands and report the actual output, not a summary."
            );
        }
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
