use rmap::export::export_json_str;
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
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
assignee = "codex"
acceptance_criteria = ["Ticker fields are normalized"]

[[task]]
id = 75
phase = 12
bundle = "ticker_normalization"
status = "pending"
title = "parseOrder field map"
scores = { d = 6, b = 8, u = 8 }
depends_on = [74]
"#;

#[test]
fn exports_validated_tasks_with_computed_efficiency() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let json = export_json_str(&tasks).expect("export json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["project"], "ccxt_extract");
    assert_eq!(value["default_branch"], "development");
    assert_eq!(value["linear"]["team_key"], "INE");
    assert_eq!(value["task"][0]["id"], 74);
    assert_eq!(value["task"][0]["eff"], 1.6);
    assert_eq!(value["task"][0]["assignee"], "codex");
    assert_eq!(
        value["task"][0]["acceptance_criteria"],
        serde_json::json!(["Ticker fields are normalized"])
    );
    assert_eq!(value["task"][1]["id"], 75);
    assert_eq!(value["task"][1]["eff"], 1.33);
}

#[test]
fn exports_top_level_focus_when_set() {
    let input = TASKS.replace("[linear]", "[focus]\nphase = 12\n\n[linear]");
    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("valid tasks");

    let json = export_json_str(&tasks).expect("export json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    assert_eq!(value["focus"]["phase"], 12);
}

#[test]
fn omits_focus_key_when_absent() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let json = export_json_str(&tasks).expect("export json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    assert!(
        value.get("focus").is_none(),
        "focus should be omitted, got {value}"
    );
}

#[test]
fn rejects_eff_in_source_toml() {
    let input = TASKS.replace(
        "scores = { d = 5, b = 8, u = 8 }",
        "scores = { d = 5, b = 8, u = 8 }\neff = 1.6",
    );

    let err = validate_tasks_str("roadmap/tasks.toml", &input).expect_err("eff is rejected");

    assert!(err.to_string().contains("unknown field `eff`"), "{err}");
}
