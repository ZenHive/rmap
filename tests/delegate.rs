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
model = "claude-opus-4-7"
acceptance_criteria = [
  "parseOrder accepts spot and futures payloads",
  "Empty venues field maps to null",
]
out_of_scope = [
  "Do not refactor parseTicker",
  "Do not touch INE-200",
]
files_to_modify = [
  "src/parse_order.rs",
  "tests/parse_order.rs",
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

const MODULE_FALLBACK_TASKS: &str = r#"
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
module = "src/parse_order.rs"
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
    assert!(prompt.contains("## Context"), "{prompt}");
    assert!(prompt.contains("- Target: claude"), "{prompt}");
    assert!(
        prompt.contains("- Stored assignee: codex (overridden)"),
        "{prompt}"
    );
    assert!(prompt.contains("- Project: ccxt_extract"), "{prompt}");
    assert!(prompt.contains("- Model: claude-opus-4-7"), "{prompt}");
    assert!(prompt.contains("- Markers: parallel"), "{prompt}");
    assert!(prompt.contains("- Linear: INE-300"), "{prompt}");
    assert!(prompt.contains("### Dependencies"), "{prompt}");
    assert!(
        prompt.contains("- Task 74 [done] parseTicker field map"),
        "{prompt}"
    );
    assert!(prompt.contains("### Cross-repo dependencies"), "{prompt}");
    assert!(
        prompt.contains("- blocks ccxt_client task 42 (INE-310)"),
        "{prompt}"
    );
    assert!(prompt.contains("## Task"), "{prompt}");
    assert!(
        prompt.contains("Normalize order payloads across exchanges."),
        "{prompt}"
    );
    assert!(prompt.contains("## Acceptance criteria"), "{prompt}");
    assert!(
        prompt.contains("- [ ] parseOrder accepts spot and futures payloads"),
        "{prompt}"
    );
    assert!(prompt.contains("## Out of scope"), "{prompt}");
    assert!(prompt.contains("- Do not refactor parseTicker"), "{prompt}");
    // Plain bullets, never checkbox form.
    assert!(
        !prompt.contains("- [ ] Do not refactor parseTicker"),
        "{prompt}"
    );
    assert!(prompt.contains("## Files to modify"), "{prompt}");
    assert!(prompt.contains("- src/parse_order.rs"), "{prompt}");
    // Plain bullets, never checkbox form.
    assert!(!prompt.contains("- [ ] src/parse_order.rs"), "{prompt}");
    assert!(prompt.contains("## Scoring"), "{prompt}");
    assert!(prompt.contains("[D:5/B:9/U:9 → Eff:1.8"), "{prompt}");
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

    assert!(prompt.contains("- Target: codex"), "{prompt}");
    assert!(!prompt.contains("Stored assignee:"), "{prompt}");
}

#[test]
fn omits_model_bullet_when_unset() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(prompt.contains("## Context"), "{prompt}");
    assert!(!prompt.contains("- Model:"), "{prompt}");
}

#[test]
fn formats_minimal_delegate_prompt_without_optional_sections() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(
        prompt.contains("# Task 75: parseOrder field map"),
        "{prompt}"
    );
    assert!(prompt.contains("## Context"), "{prompt}");
    assert!(prompt.contains("- Target: claude"), "{prompt}");
    assert!(prompt.contains("## Scoring"), "{prompt}");
    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(!prompt.contains("Stored assignee:"), "{prompt}");
    assert!(!prompt.contains("### Dependencies"), "{prompt}");
    assert!(!prompt.contains("### Cross-repo dependencies"), "{prompt}");
    assert!(!prompt.contains("## Task\n"), "{prompt}");
    assert!(!prompt.contains("## Acceptance criteria"), "{prompt}");
    assert!(!prompt.contains("## Out of scope"), "{prompt}");
    assert!(!prompt.contains("## Files to modify"), "{prompt}");
}

#[test]
fn emits_canonical_section_order_with_distinguishing_line() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    // (header, distinguishing line). The seven headers are the AC4 contract:
    // Context → Task → Acceptance criteria → Out of scope → Files to modify
    // → Scoring → Environment notes. `## Instructions` is the footer; not part
    // of the section contract but does follow Environment notes.
    let expected: &[(&str, &str)] = &[
        ("## Context", "- Target: claude"),
        ("## Task", "Normalize order payloads across exchanges."),
        (
            "## Acceptance criteria",
            "- [ ] parseOrder accepts spot and futures payloads",
        ),
        ("## Out of scope", "- Do not refactor parseTicker"),
        ("## Files to modify", "- src/parse_order.rs"),
        ("## Scoring", "[D:5/B:9/U:9 → Eff:1.8"),
        ("## Environment notes", "Local execution"),
    ];

    let mut last_index = 0usize;
    for (header, distinguishing) in expected {
        let header_index = prompt
            .find(header)
            .unwrap_or_else(|| panic!("missing header {header:?} in:\n{prompt}"));
        assert!(
            prompt.contains(distinguishing),
            "section {header:?} missing distinguishing line {distinguishing:?} in:\n{prompt}"
        );
        assert!(
            header_index >= last_index,
            "section {header:?} appeared out of canonical order in:\n{prompt}"
        );
        last_index = header_index;
    }
}

#[test]
fn files_to_modify_falls_back_to_module_when_empty() {
    let tasks =
        validate_tasks_str("roadmap/tasks.toml", MODULE_FALLBACK_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(prompt.contains("## Files to modify"), "{prompt}");
    assert!(prompt.contains("- src/parse_order.rs"), "{prompt}");
}

#[test]
fn files_to_modify_section_omitted_when_both_empty() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(!prompt.contains("## Files to modify"), "{prompt}");
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
    assert!(prompt.contains("Network access varies"), "{prompt}");
    assert!(prompt.contains("hex.pm"), "{prompt}");
    // Cursor-specific phrasing must NOT bleed into the Codex footer.
    assert!(!prompt.contains("Run the full project harness"), "{prompt}");
}

#[test]
fn cursor_target_emits_cursor_environment_footer() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Cursor).expect("task 75");

    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(prompt.contains("Run the full project harness"), "{prompt}");
    assert!(!prompt.contains("Network access varies"), "{prompt}");
}

#[test]
fn claude_target_emits_local_environment_footer() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(prompt.contains("## Environment notes"), "{prompt}");
    assert!(prompt.contains("Local execution"), "{prompt}");
    assert!(!prompt.contains("Network access varies"), "{prompt}");
    assert!(!prompt.contains("Run the full project harness"), "{prompt}");
}
