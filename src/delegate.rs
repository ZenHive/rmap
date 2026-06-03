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
    Grok,
    Antigravity,
    Pi,
    Droid,
}

impl DelegateTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Grok => "grok",
            Self::Antigravity => "antigravity",
            Self::Pi => "pi",
            Self::Droid => "droid",
        }
    }
}

impl std::fmt::Display for DelegateTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Resolve which agent a delegate prompt targets.
///
/// Explicit `--to` always wins. Without it, the task's stored `assignee` is the
/// routing intent — `assignee` is THE agent-routing field; `--to` is the
/// render-time override. Errors guide the caller to pass `--to`: a task with no
/// assignee carries no routing intent to honor, and `human` is not a delegatable
/// target (delegating human-assigned work needs an explicit override).
pub fn resolve_target(task: &Task, to: Option<DelegateTarget>) -> Result<DelegateTarget, String> {
    if let Some(target) = to {
        return Ok(target);
    }
    match task.assignee.as_deref() {
        None => Err(format!(
            "task {} has no assignee; pass --to <agent>",
            task.id
        )),
        Some("human") => Err(format!(
            "task {} is assigned to human; pass --to <agent> to delegate anyway",
            task.id
        )),
        Some(assignee) => DelegateTarget::from_str(assignee, true).map_err(|_| {
            format!(
                "task {} assignee {assignee:?} is not a delegate target; pass --to <agent>",
                task.id
            )
        }),
    }
}

pub fn format_delegate_prompt(tasks: &Tasks, id: &str, target: DelegateTarget) -> Option<String> {
    let task = find_task(tasks, id)?;
    Some(format_prompt(tasks, task, target))
}

fn format_prompt(tasks: &Tasks, task: &Task, target: DelegateTarget) -> String {
    let mut prompt = String::new();
    line!(prompt, "# Task {}: {}", task.id, task.title);

    append_context(&mut prompt, tasks, task, target);
    append_task_body(&mut prompt, task);
    append_prior_attempts(&mut prompt, task);
    append_implemented(&mut prompt, task);
    append_acceptance_criteria(&mut prompt, task);
    append_out_of_scope(&mut prompt, task);
    append_files_to_modify(&mut prompt, task);
    append_scoring(&mut prompt, task);
    append_agent_notes(&mut prompt, target);
    append_instructions(&mut prompt);

    prompt
}

fn append_context(prompt: &mut String, tasks: &Tasks, task: &Task, target: DelegateTarget) {
    line!(prompt);
    line!(prompt, "## Context");
    line!(prompt, "- Target: {target}");
    line!(prompt, "- Project: {}", tasks.project);
    if let Some(assignee) = &task.assignee
        && assignee != target.as_str()
    {
        // Surface the override so the receiving agent knows the stored
        // routing intent differed from the explicit `--to`. When they match,
        // the bullet is redundant with `Target:` above.
        line!(prompt, "- Stored assignee: {assignee} (overridden)");
    }
    if let Some(model) = &task.model {
        line!(prompt, "- Model: {model}");
    }
    line!(prompt, "- Status: {}", task.status);
    line!(prompt, "- Phase: {}", task.phase);
    line!(prompt, "- Bundle: {}", task.bundle);
    if let Some(milestone_key) = &task.milestone {
        match tasks
            .milestones
            .get(milestone_key)
            .and_then(|m| m.target_version.as_deref())
        {
            Some(version) => line!(prompt, "- Milestone: {milestone_key} (target={version})"),
            None => line!(prompt, "- Milestone: {milestone_key}"),
        }
    }
    if !task.markers.is_empty() {
        line!(prompt, "- Markers: {}", task.markers.join(", "));
    }
    if let Some(linear_id) = &task.linear_id {
        line!(prompt, "- Linear: {linear_id}");
    }
    if let Some(shipped_in) = &task.shipped_in {
        line!(prompt, "- Shipped in: {shipped_in}");
    }

    append_dependencies(prompt, tasks, task);
    append_cross_repo(prompt, task);
}

fn append_dependencies(prompt: &mut String, tasks: &Tasks, task: &Task) {
    if task.depends_on.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "### Dependencies");
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
    line!(prompt, "### Cross-repo dependencies");
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

fn append_task_body(prompt: &mut String, task: &Task) {
    let Some(body) = task.body.as_ref().map(|body| body.trim()) else {
        return;
    };
    if body.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Task");
    line!(prompt, "{body}");
}

// Surface the failure evidence from prior dispatch attempts so the next
// implementer reads why earlier runs were rejected before starting — the
// implementer/reviewer "argument" carried across attempts, not re-litigated.
fn append_prior_attempts(prompt: &mut String, task: &Task) {
    if task.attempts.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## Prior attempts");
    line!(
        prompt,
        "This task returned to the queue {} time(s). Read the rejection evidence before starting — do not repeat these failures.",
        task.attempts.len()
    );
    for (index, attempt) in task.attempts.iter().enumerate() {
        let by = attempt.by.as_deref().unwrap_or("unknown");
        line!(prompt);
        line!(
            prompt,
            "### Attempt {} — {} by {}",
            index + 1,
            attempt.at,
            by
        );
        let report = attempt.report.trim();
        if !report.is_empty() {
            line!(prompt, "{report}");
        }
    }
}

fn append_implemented(prompt: &mut String, task: &Task) {
    let Some(implemented) = task.implemented.as_ref().map(|s| s.trim()) else {
        return;
    };
    if implemented.is_empty() {
        return;
    }

    line!(prompt);
    line!(prompt, "## What was actually implemented");
    line!(prompt, "{implemented}");
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

// When `files_to_modify` is empty but `module` is set, the module path is
// surfaced as a one-bullet hint so the delegated agent knows where to look
// even if the author didn't enumerate files. When both are empty the section
// is omitted — there is nothing to fall back to.
fn append_files_to_modify(prompt: &mut String, task: &Task) {
    let entries: Vec<&str> = if !task.files_to_modify.is_empty() {
        task.files_to_modify.iter().map(String::as_str).collect()
    } else if let Some(module) = &task.module {
        vec![module.as_str()]
    } else {
        return;
    };

    line!(prompt);
    line!(prompt, "## Files to modify");
    for entry in entries {
        line!(prompt, "- {entry}");
    }
}

fn append_scoring(prompt: &mut String, task: &Task) {
    let eff = efficiency(task);
    line!(prompt);
    line!(prompt, "## Scoring");
    line!(
        prompt,
        "[D:{}/B:{}/U:{} → Eff:{}] {}",
        task.scores.d,
        task.scores.b,
        task.scores.u,
        format_efficiency(eff),
        tier_glyph(eff)
    );
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
        DelegateTarget::Grok => {
            line!(
                prompt,
                "- Local execution: full toolchain, internet, and project state available. Reads AGENTS.md for project conventions."
            );
            line!(
                prompt,
                "- Run verification commands and report the actual output, not a summary."
            );
        }
        DelegateTarget::Antigravity => {
            line!(
                prompt,
                "- Local execution: full toolchain, internet, and project state available. Reads AGENTS.md (and GEMINI.md) for project conventions."
            );
            line!(
                prompt,
                "- `agy` resolves its workspace via git-common-dir and ignores the invocation cwd — it can edit the main checkout even when pointed at a worktree; don't rely on cwd-scoped isolation."
            );
            line!(
                prompt,
                "- Run verification commands and report the actual output, not a summary."
            );
        }
        DelegateTarget::Pi => {
            line!(
                prompt,
                "- Local execution: full toolchain, internet, and project state available. Reads AGENTS.md for project conventions."
            );
            line!(
                prompt,
                "- Runs a local LLM (free/unmetered) in autonomous permission mode."
            );
            line!(
                prompt,
                "- Run verification commands and report the actual output, not a summary."
            );
        }
        DelegateTarget::Droid => {
            line!(
                prompt,
                "- Local execution: full toolchain, internet, and project state available. Reads AGENTS.md for project conventions."
            );
            line!(
                prompt,
                "- Not yet a harness executor — this prompt is for the Factory Droid CLI or manual paste."
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
