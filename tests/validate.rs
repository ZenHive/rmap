use rmap::validate::validate_tasks_str;

const VALID_TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"

[phases.12]
name = "Per-Exchange Normalization"
order = 12
status = "in_progress"

[bundles.ticker_normalization]
phase = 12
order = 1
description = "Unified ticker fields across all exchanges"

[[task]]
id = 74
phase = 12
bundle = "ticker_normalization"
status = "done"
title = "parseTicker field map + coercion + enums"
scores = { d = 5, b = 8, u = 8 }
markers = ["parallel"]
linear_id = "INE-247"
assignee = "claude"
acceptance_criteria = ["Ticker fields are normalized"]
shipped_in = "PR #21"

[[task]]
id = 75
phase = 12
bundle = "ticker_normalization"
status = "pending"
title = "parseOrder field map"
scores = { d = 6, b = 8, u = 8 }
depends_on = [74]
linear_id = "INE-300"
"#;

#[test]
fn valid_tasks_toml_deserializes_and_validates() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", VALID_TASKS).expect("valid tasks");

    assert_eq!(tasks.schema_version, 1);
    assert_eq!(tasks.project, "ccxt_extract");
    assert_eq!(tasks.task.len(), 2);
    assert_eq!(tasks.task[0].id, 74);
    assert_eq!(tasks.task[0].assignee.as_deref(), Some("claude"));
    assert_eq!(
        tasks.task[0].acceptance_criteria,
        vec!["Ticker fields are normalized"]
    );
    assert_eq!(tasks.task[1].depends_on, vec![74]);
}

#[test]
fn rejects_unknown_schema_version_with_migration_hint() {
    let input = VALID_TASKS.replace("schema_version = 1", "schema_version = 2");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("schema 2 is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:2"));
    assert!(message.contains("unsupported schema_version 2"));
    assert!(message.contains("rmap migrate"));
}

#[test]
fn rejects_invalid_status() {
    let input = VALID_TASKS.replace("status = \"pending\"", "status = \"shipped\"");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("status is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:37"));
    assert!(message.contains("invalid status \"shipped\""));
}

#[test]
fn rejects_invalid_marker() {
    let input = VALID_TASKS.replace("markers = [\"parallel\"]", "markers = [\"fast\"]");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("marker is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:27"));
    assert!(message.contains("invalid marker \"fast\""));
}

#[test]
fn rejects_invalid_assignee() {
    let input = VALID_TASKS.replace("assignee = \"claude\"", "assignee = \"bot\"");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("assignee is rejected");

    let message = err.to_string();
    assert!(message.contains("invalid assignee \"bot\""));
}

#[test]
fn rejects_linear_id_that_does_not_match_team_key() {
    let input = VALID_TASKS.replace("linear_id = \"INE-300\"", "linear_id = \"OPS-300\"");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("linear id is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:41"));
    assert!(message.contains("linear_id \"OPS-300\" must match INE-<integer>"));
}

#[test]
fn skips_linear_id_format_check_when_linear_table_is_absent() {
    let input = VALID_TASKS
        .replace(
            r#"
[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"
"#,
            "",
        )
        .replace("linear_id = \"INE-247\"", "linear_id = \"not-linear\"");

    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("linear is opt-in");

    assert_eq!(tasks.task[0].linear_id.as_deref(), Some("not-linear"));
}

#[test]
fn rejects_orphan_dependencies() {
    let input = VALID_TASKS.replace("depends_on = [74]", "depends_on = [999]");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("dependency is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:40"));
    assert!(message.contains("task 75 depends on unknown task 999"));
}

#[test]
fn rejects_invalid_cross_repo_relation() {
    let input = VALID_TASKS.replace(
        "linear_id = \"INE-300\"",
        r#"linear_id = "INE-300"
cross_repo = [
  { repo = "ccxt_client", task_id = 42, relation = "duplicates" },
]"#,
    );

    let err = validate_tasks_str("roadmap/tasks.toml", &input)
        .expect_err("cross-repo relation is rejected");

    let message = err.to_string();
    assert!(message.contains("invalid cross_repo relation \"duplicates\""));
}

#[test]
fn rejects_task_that_references_unknown_phase() {
    let input = VALID_TASKS.replace("phase = 12", "phase = 99");

    let err =
        validate_tasks_str("roadmap/tasks.toml", &input).expect_err("unknown phase is rejected");

    let message = err.to_string();
    assert!(message.contains("task 74 references unknown phase 99"));
}

#[test]
fn rejects_task_that_references_unknown_bundle() {
    let input = VALID_TASKS.replace(
        "bundle = \"ticker_normalization\"",
        "bundle = \"order_normalization\"",
    );

    let err =
        validate_tasks_str("roadmap/tasks.toml", &input).expect_err("unknown bundle is rejected");

    let message = err.to_string();
    assert!(message.contains("task 74 references unknown bundle \"order_normalization\""));
}

#[test]
fn rejects_dependency_cycle_between_tasks() {
    let input = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.12]
name = "Cycle phase"
order = 12
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple"

[[task]]
id = 1
phase = 12
bundle = "simple"
status = "pending"
title = "A"
scores = { d = 1, b = 1, u = 1 }
depends_on = [2]

[[task]]
id = 2
phase = 12
bundle = "simple"
status = "pending"
title = "B"
scores = { d = 1, b = 1, u = 1 }
depends_on = [1]
"#;

    let err = validate_tasks_str("roadmap/tasks.toml", input).expect_err("cycle is rejected");

    let message = err.to_string();
    assert!(message.contains("dependency cycle"));
    assert!(message.contains("1"));
    assert!(message.contains("2"));
}

#[test]
fn rejects_self_dependency_cycle() {
    let input = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.12]
name = "Self-cycle"
order = 12
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple"

[[task]]
id = 1
phase = 12
bundle = "simple"
status = "pending"
title = "Self"
scores = { d = 1, b = 1, u = 1 }
depends_on = [1]
"#;

    let err = validate_tasks_str("roadmap/tasks.toml", input).expect_err("self-cycle is rejected");

    assert!(err.to_string().contains("dependency cycle"));
}

#[test]
fn rejects_invalid_timestamp_format() {
    let input = VALID_TASKS.replace(
        "scores = { d = 5, b = 8, u = 8 }",
        "scores = { d = 5, b = 8, u = 8 }\ncreated_at = \"2026-1-1\"",
    );

    let err =
        validate_tasks_str("roadmap/tasks.toml", &input).expect_err("invalid timestamp rejected");

    let message = err.to_string();
    assert!(message.contains("created_at"));
    assert!(message.contains("YYYY-MM-DD format"));
}

#[test]
fn accepts_well_formed_timestamps() {
    let input = VALID_TASKS.replace(
        "scores = { d = 5, b = 8, u = 8 }",
        r#"scores = { d = 5, b = 8, u = 8 }
created_at = "2026-04-01"
started_at = "2026-04-12"
done_at = "2026-04-30"
scored_at = "2026-04-15""#,
    );

    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("valid timestamps");

    assert_eq!(tasks.task[0].created_at.as_deref(), Some("2026-04-01"));
    assert_eq!(tasks.task[0].started_at.as_deref(), Some("2026-04-12"));
    assert_eq!(tasks.task[0].done_at.as_deref(), Some("2026-04-30"));
    assert_eq!(tasks.task[0].scored_at.as_deref(), Some("2026-04-15"));
}

#[test]
fn rejects_blocked_status_without_blocked_reason() {
    let input = VALID_TASKS.replace("status = \"pending\"", "status = \"blocked\"");

    let err = validate_tasks_str("roadmap/tasks.toml", &input)
        .expect_err("blocked without reason rejected");

    let message = err.to_string();
    assert!(message.contains("blocked but missing blocked_reason"));
}

#[test]
fn accepts_blocked_status_with_reason() {
    let input = VALID_TASKS.replace(
        "status = \"pending\"\ntitle = \"parseOrder field map\"",
        "status = \"blocked\"\nblocked_reason = \"waiting on legal\"\ntitle = \"parseOrder field map\"",
    );

    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("blocked with reason");

    assert_eq!(tasks.task[1].status, "blocked");
    assert_eq!(
        tasks.task[1].blocked_reason.as_deref(),
        Some("waiting on legal")
    );
}

#[test]
fn rejects_focus_phase_referencing_unknown_phase() {
    let input = VALID_TASKS.replace(
        "[linear]",
        r#"[focus]
phase = 99

[linear]"#,
    );

    let err =
        validate_tasks_str("roadmap/tasks.toml", &input).expect_err("unknown focus phase rejected");

    let message = err.to_string();
    assert!(message.contains("[focus].phase 99"));
}

#[test]
fn accepts_focus_phase_matching_declared_phase() {
    let input = VALID_TASKS.replace(
        "[linear]",
        r#"[focus]
phase = 12

[linear]"#,
    );

    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("valid focus");

    assert_eq!(tasks.focus.expect("focus").phase, 12);
}
