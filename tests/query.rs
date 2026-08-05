use rmap::query::{TaskFilter, find_task, list_tasks};
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
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

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 6, b = 8, u = 8 }
markers = ["cx"]
"#;

#[test]
fn finds_task_by_numeric_or_string_id() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    assert_eq!(
        find_task(&tasks, "75").expect("task 75").title,
        "parseOrder field map"
    );
    assert_eq!(
        find_task(&tasks, "78b").expect("task 78b").title,
        "parseOHLCV object-shape exchanges"
    );
    assert!(find_task(&tasks, "999").is_none());
}

#[test]
fn lists_tasks_matching_all_filters_in_toml_order() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let filter = TaskFilter {
        status: Some("pending".to_string()),
        marker: Some("parallel".to_string()),
        phase: Some(12),
        bundle: None,
        target_repo: None,
        milestone: None,
        delivered_by: None,
    };

    let listed = list_tasks(&tasks, &filter);

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id.to_string(), "75");
}

#[test]
fn target_repo_filter_uses_explicit_value_and_roadmap_project_default() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let external = list_tasks(
        &tasks,
        &TaskFilter {
            target_repo: Some("ccxt_client".to_string()),
            ..Default::default()
        },
    );
    assert_eq!(
        external
            .iter()
            .map(|task| task.id.to_string())
            .collect::<Vec<_>>(),
        ["75"]
    );

    let local = list_tasks(
        &tasks,
        &TaskFilter {
            target_repo: Some("ccxt_extract".to_string()),
            ..Default::default()
        },
    );
    assert_eq!(
        local
            .iter()
            .map(|task| task.id.to_string())
            .collect::<Vec<_>>(),
        ["74", "78b"]
    );
}

#[test]
fn empty_filter_lists_every_task_in_toml_order() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let listed = list_tasks(&tasks, &TaskFilter::default());

    assert_eq!(
        listed
            .iter()
            .map(|task| task.id.to_string())
            .collect::<Vec<_>>(),
        ["74", "75", "78b"]
    );
}
