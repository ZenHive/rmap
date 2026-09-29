use rmap::delegate::{DelegateTarget, format_delegate_prompt};
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
schema_version = 2
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
implemented = "fixture"
title = "parseTicker field map"
scores = { d = 4, b = 8, u = 8 }

[[task]]
id = 75
phase = 12
bundle = "orders"
target_repo = "ccxt_client"
status = "pending"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }
depends_on = [74]
markers = ["parallel"]
linear_id = "INE-300"
assignee = "codex"
model = "claude-opus-4-7"
domains = ["rust", "agent-routing"]
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
context_refs = [
  "docs/order-parsing.md",
  "https://example.com/adr/0012-order-map",
]
checks = [
  "cargo test --test parse_order",
  "mix test test/order_map_test.exs",
]
cross_repo = [
  { repo = "ccxt_client", task_id = 42, linear_id = "INE-310", relation = "blocks" },
]
body = """
Normalize order payloads across exchanges.
"""
"#;

const MINIMAL_TASKS: &str = r#"
schema_version = 2
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
schema_version = 2
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
    assert!(prompt.contains("- Target repo: ccxt_client"), "{prompt}");
    assert!(prompt.contains("- Model: claude-opus-4-7"), "{prompt}");
    assert!(
        prompt.contains("- Domains: rust, agent-routing"),
        "{prompt}"
    );
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
    assert!(prompt.contains("## Read first"), "{prompt}");
    assert!(prompt.contains("- docs/order-parsing.md"), "{prompt}");
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
    assert!(prompt.contains("## Reviewer checks"), "{prompt}");
    assert!(
        prompt.contains("Hints for the reviewer, not an automated gate."),
        "{prompt}"
    );
    assert!(
        prompt.contains("- cargo test --test parse_order"),
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
fn delegate_defaults_target_repo_to_roadmap_project() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Codex).expect("task 75");

    assert!(prompt.contains("- Project: ccxt_extract"), "{prompt}");
    assert!(prompt.contains("- Target repo: ccxt_extract"), "{prompt}");
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
fn omits_domains_bullet_when_unset() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    assert!(prompt.contains("## Context"), "{prompt}");
    assert!(!prompt.contains("- Domains:"), "{prompt}");
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
    assert!(!prompt.contains("## Read first"), "{prompt}");
    assert!(!prompt.contains("## Reviewer checks"), "{prompt}");
    assert!(!prompt.contains("## Out of scope"), "{prompt}");
    assert!(!prompt.contains("## Files to modify"), "{prompt}");
}

#[test]
fn emits_canonical_section_order_with_distinguishing_line() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "75", DelegateTarget::Claude).expect("task 75");

    // (header, distinguishing line). Locked order when the optional sections
    // are present: Context → Read first → Task → Acceptance criteria →
    // Reviewer checks → Out of scope → Files to modify → Scoring →
    // Environment notes. `## Prior attempts`, `## What was actually
    // implemented`, and `## Instructions` stay outside this contract.
    let expected: &[(&str, &str)] = &[
        ("## Context", "- Target: claude"),
        ("## Read first", "docs/order-parsing.md"),
        ("## Task", "Normalize order payloads across exchanges."),
        (
            "## Acceptance criteria",
            "- [ ] parseOrder accepts spot and futures payloads",
        ),
        (
            "## Reviewer checks",
            "Hints for the reviewer, not an automated gate.",
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
fn done_tasks_check_acceptance_criteria_open_tasks_do_not() {
    let base = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "core"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "STATUS"
title = "checkbox"
scores = { d = 2, b = 4, u = 4 }
acceptance_criteria = ["the box tracks status"]
IMPLEMENTED
"#;

    for (status, extra, mark) in [
        ("pending", "", " "),
        ("in_progress", "", " "),
        ("blocked", "blocked_reason = \"waiting\"\n", " "),
        ("superseded", "", " "),
        ("done", "implemented = \"shipped\"\n", "x"),
    ] {
        let input = base.replace("STATUS", status).replace("IMPLEMENTED", extra);
        let tasks = validate_tasks_str("roadmap/tasks.toml", &input).unwrap_or_else(|err| {
            panic!("{status} fixture should validate: {err}");
        });
        let prompt = format_delegate_prompt(&tasks, "1", DelegateTarget::Claude)
            .unwrap_or_else(|| panic!("{status} prompt"));
        let expected = format!("- [{mark}] the box tracks status");
        assert!(
            prompt.contains(&expected),
            "{status} should render {expected:?}:\n{prompt}"
        );
        if mark == " " {
            assert!(
                !prompt.contains("- [x]"),
                "{status} must stay unchecked:\n{prompt}"
            );
        }
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
    assert!(
        prompt.contains("Project toolchains are not guaranteed"),
        "{prompt}"
    );
    assert!(!footer_names_toolchain(environment_notes(&prompt)));
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

const MILESTONE_TASKS: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.7]
name = "Release lines"
order = 7
status = "in_progress"

[bundles.macros]
phase = 7
order = 1
description = "Generated data plane"

[milestones.v0_1]
name = "v0.1 — first usable"
order = 1
status = "active"
target_version = "0.1.0"

[milestones.v1_0]
name = "v1.0 — production"
order = 2
status = "pending"

[[task]]
id = 1
phase = 7
bundle = "macros"
milestone = "v0_1"
status = "pending"
title = "macro: parseTicker"
scores = { d = 3, b = 6, u = 6 }

[[task]]
id = 2
phase = 7
bundle = "macros"
milestone = "v1_0"
status = "pending"
title = "macro: parseOrder"
scores = { d = 3, b = 6, u = 6 }

[[task]]
id = 3
phase = 7
bundle = "macros"
status = "pending"
title = "macro: shared (no milestone)"
scores = { d = 3, b = 6, u = 6 }
"#;

#[test]
fn delegate_emits_milestone_bullet_with_target_version() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MILESTONE_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "1", DelegateTarget::Claude).expect("task 1");

    assert!(
        prompt.contains("- Milestone: v0_1 (target=0.1.0)"),
        "expected milestone bullet with target version; got:\n{prompt}"
    );
}

#[test]
fn delegate_emits_milestone_bullet_without_target_version() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MILESTONE_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "2", DelegateTarget::Claude).expect("task 2");

    assert!(
        prompt.contains("- Milestone: v1_0"),
        "expected bare milestone bullet; got:\n{prompt}"
    );
    assert!(
        !prompt.contains("- Milestone: v1_0 (target="),
        "no target qualifier when unset; got:\n{prompt}"
    );
}

#[test]
fn environment_footer_names_no_language_toolchain_or_package_manager() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MINIMAL_TASKS).expect("valid tasks");
    let targets = [
        DelegateTarget::Claude,
        DelegateTarget::Codex,
        DelegateTarget::Cursor,
        DelegateTarget::Grok,
        DelegateTarget::Antigravity,
        DelegateTarget::Pi,
        DelegateTarget::Droid,
        DelegateTarget::Kimi,
    ];
    assert_eq!(targets.len(), 8, "one footer per --to target");

    for target in targets {
        let prompt = format_delegate_prompt(&tasks, "75", target).expect("task 75");
        let notes = environment_notes(&prompt);
        assert!(
            notes.contains("## Environment notes"),
            "{target} missing footer:\n{prompt}"
        );
        assert!(
            !footer_names_toolchain(notes),
            "{target} environment notes name a language, toolchain, or package manager:\n{notes}"
        );
    }
}

fn environment_notes(prompt: &str) -> &str {
    let start = prompt
        .find("## Environment notes")
        .expect("environment notes");
    let rest = &prompt[start..];
    let end = rest
        .find("\n## ")
        .map(|index| index + 1)
        .unwrap_or(rest.len());
    &rest[..end]
}

fn footer_names_toolchain(notes: &str) -> bool {
    const BANNED: &[&str] = &[
        "elixir",
        "erlang",
        "rust",
        "python",
        "ruby",
        "javascript",
        "typescript",
        "golang",
        "java",
        "kotlin",
        "swift",
        "php",
        "dart",
        "node",
        "mix",
        "cargo",
        "npm",
        "pip",
        "bundler",
        "asdf",
        "hex.pm",
        "crates.io",
        "npmjs",
        "pypi",
        "rubygems",
        "yarn",
        "pnpm",
        "maven",
        "gradle",
        "composer",
        "poetry",
    ];
    notes.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        BANNED.iter().any(|word| contains_word(&lower, word))
    })
}

fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let needle = word.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let abs = start + pos;
        let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric();
        let after = abs + needle.len();
        let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
        // Treat '.' as part of a registry host (`hex.pm`) rather than a boundary.
        let after_ok = after_ok && (after >= bytes.len() || bytes[after] != b'.');
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

#[test]
fn delegate_omits_milestone_bullet_when_unset() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", MILESTONE_TASKS).expect("valid tasks");
    let prompt = format_delegate_prompt(&tasks, "3", DelegateTarget::Claude).expect("task 3");

    assert!(
        !prompt.contains("- Milestone:"),
        "no bullet when milestone unset; got:\n{prompt}"
    );
}
