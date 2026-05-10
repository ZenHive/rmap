use rmap::next::next_task;
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
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

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 6, b = 8, u = 8 }
markers = ["parallel"]

[[task]]
id = 83
phase = 12
bundle = "simple"
status = "pending"
title = "Response envelope paths"
scores = { d = 4, b = 9, u = 9 }
depends_on = [75]
markers = ["parallel"]
"#;

#[test]
fn selects_highest_eff_pending_unblocked_task() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, None).expect("next task");

    assert_eq!(task.id.to_string(), "75");
}

#[test]
fn excludes_tasks_blocked_by_incomplete_dependencies() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, Some("parallel")).expect("next task");

    assert_eq!(task.id.to_string(), "75");
    assert_ne!(task.id.to_string(), "83");
}

#[test]
fn marker_filter_excludes_non_matching_tasks() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, Some("parallel")).expect("next task");

    assert!(task.markers.contains(&"parallel".to_string()));
}

#[test]
fn no_matching_next_task_returns_none() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    assert!(next_task(&tasks, Some("csr")).is_none());
}

#[test]
fn ties_preserve_toml_order() {
    let input = TASKS.replace(
        r#"scores = { d = 6, b = 8, u = 8 }
markers = ["parallel"]"#,
        r#"scores = { d = 5, b = 9, u = 9 }
markers = ["parallel"]"#,
    );
    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("valid tasks");

    let task = next_task(&tasks, Some("parallel")).expect("next task");

    assert_eq!(task.id.to_string(), "75");
}
