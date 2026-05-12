use rmap::delegate::{DelegateTarget, format_delegate_prompt};
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"

[phases.12]
name = "Response parsing contract"
order = 12
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple normalization tasks"

[bundles.orders]
phase = 12
order = 2
description = "Order normalization tasks"

[[task]]
id = 74
phase = 12
bundle = "simple"
status = "done"
title = "parseTicker field map"
scores = { d = 4, b = 8, u = 8 }

[[task]]
id = 75
phase = 12
bundle = "orders"
status = "pending"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }
depends_on = [74]
markers = ["parallel"]
linear_id = "INE-300"
assignee = "codex"
acceptance_criteria = [
  "parseOrder accepts spot and futures payloads",
  "Empty venues field maps to null",
]
cross_repo = [
  { repo = "ccxt_client", task_id = 42, linear_id = "INE-310", relation = "blocks" },
]
body = """
Normalize order payloads across exchanges.
"""
"#;

const MINIMAL_TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.12]
name = "Response parsing contract"
order = 12
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple normalization tasks"

[[task]]
id = 75
phase = 12
bundle = "simple"
status = "pending"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }
"#;

#[test]
fn formats_full_delegate_prompt_for_agent_target() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    // Delegate to a target that differs from the stored assignee to exercise
    // the override-surfacing path.
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(
        prompt.contains("# Task 75: parseOrder field map"),
        "{prompt}"
    );
    assert!(prompt.contains("Target agent: claude"), "{prompt}");
    assert!(
        prompt.contains("Stored assignee: codex (overridden)"),
        "{prompt}"
    );
    assert!(prompt.contains("Project: ccxt_extract"), "{prompt}");
    assert!(
        prompt.contains("Scores: D:5/B:9/U:9 -> Eff:1.8"),
        "{prompt}"
    );
    assert!(prompt.contains("Markers: parallel"), "{prompt}");
    assert!(prompt.contains("Linear: INE-300"), "{prompt}");
    assert!(
        prompt.contains("- Task 74 [done] parseTicker field map"),
        "{prompt}"
    );
    assert!(
        prompt.contains("- blocks ccxt_client task 42 (INE-310)"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Normalize order payloads across exchanges."),
        "{prompt}"
    );
    assert!(
        prompt.contains("- [ ] parseOrder accepts spot and futures payloads"),
        "{prompt}"
    );
    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(
        prompt.contains("Inspect the repo before editing."),
        "{prompt}"
    );
}

#[test]
fn omits_stored_assignee_when_target_matches() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Codex).expect("task 75");

    assert!(prompt.contains("Target agent: codex"), "{prompt}");
    assert!(!prompt.contains("Stored assignee:"), "{prompt}");
}

#[test]
fn formats_minimal_delegate_prompt_without_optional_sections() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(
        prompt.contains("# Task 75: parseOrder field map"),
        "{prompt}"
    );
    assert!(prompt.contains("Target agent: claude"), "{prompt}");
    assert!(!prompt.contains("Stored assignee:"), "{prompt}");
    assert!(!prompt.contains("Dependencies\n"), "{prompt}");
    assert!(!prompt.contains("Acceptance criteria\n"), "{prompt}");
}

#[test]
fn missing_delegate_task_returns_none() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    assert!(format_delegate_prompt(&tasks, "999", DelegateTarget::Codex).is_none());
}

#[test]
fn codex_target_emits_codex_environment_footer() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Codex).expect("task 75");

    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(prompt.contains("No external HTTP"), "{prompt}");
    // Cursor-specific phrasing must NOT bleed into the Codex footer.
    assert!(!prompt.contains("Run the full project harness"), "{prompt}");
}

#[test]
fn cursor_target_emits_cursor_environment_footer() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Cursor).expect("task 75");

    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(prompt.contains("Run the full project harness"), "{prompt}");
    assert!(!prompt.contains("No external HTTP"), "{prompt}");
}

#[test]
fn claude_target_emits_local_environment_footer() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(prompt.contains("Local execution"), "{prompt}");
    assert!(!prompt.contains("No external HTTP"), "{prompt}");
    assert!(!prompt.contains("Run the full project harness"), "{prompt}");
}
