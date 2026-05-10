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
    assert!(message.contains("roadmap/tasks.toml:35"));
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
fn rejects_linear_id_that_does_not_match_team_key() {
    let input = VALID_TASKS.replace("linear_id = \"INE-300\"", "linear_id = \"OPS-300\"");

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("linear id is rejected");

    let message = err.to_string();
    assert!(message.contains("roadmap/tasks.toml:39"));
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
    assert!(message.contains("roadmap/tasks.toml:38"));
    assert!(message.contains("task 75 depends on unknown task 999"));
}
