use rmap::diff::{DiffStatus, diff_metadata, diff_tasks};
use rmap::validate::validate_tasks_str;

const BASE_TASKS: &str = r#"
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
id = 74
phase = 12
bundle = "simple"
status = "pending"
title = "parseTicker field map"
scores = { d = 4, b = 8, u = 8 }

[[task]]
id = 75
phase = 12
bundle = "simple"
status = "pending"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }
"#;

const CURRENT_TASKS: &str = r#"
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
status = "done"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 6, b = 8, u = 8 }
"#;

#[test]
fn classifies_added_removed_and_changed_tasks() {
    let base = validate_tasks_str("base", BASE_TASKS).expect("valid base tasks");
    let current = validate_tasks_str("current", CURRENT_TASKS).expect("valid current tasks");

    let diff = diff_tasks(&base, &current);

    assert_eq!(diff.len(), 3);
    assert_eq!(diff[0].id.to_string(), "74");
    assert_eq!(diff[0].status, DiffStatus::Removed);
    assert_eq!(diff[0].changed_fields, Vec::<String>::new());
    assert_eq!(diff[1].id.to_string(), "75");
    assert_eq!(diff[1].status, DiffStatus::Changed);
    assert_eq!(diff[1].changed_fields, ["status"]);
    assert_eq!(diff[2].id.to_string(), "78b");
    assert_eq!(diff[2].status, DiffStatus::Added);
}

const METADATA_BASE: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "main"

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
"#;

const METADATA_CURRENT: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"

[phases.12]
name = "Response parsing contract (v2)"
order = 12
status = "in_progress"

[phases.13]
name = "Per-exchange normalization"
order = 13
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple normalization tasks"
"#;

#[test]
fn classifies_metadata_scalar_optional_and_map_changes() {
    let base = validate_tasks_str("base", METADATA_BASE).expect("valid base tasks");
    let current = validate_tasks_str("current", METADATA_CURRENT).expect("valid current tasks");

    let diff = diff_metadata(&base, &current);
    let entries: Vec<(&str, &DiffStatus)> =
        diff.iter().map(|d| (d.key.as_str(), &d.status)).collect();

    assert_eq!(
        entries,
        vec![
            ("default_branch", &DiffStatus::Changed),
            ("linear", &DiffStatus::Added),
            ("phases.12", &DiffStatus::Changed),
            ("phases.13", &DiffStatus::Added),
            ("bundles.orders", &DiffStatus::Removed),
        ]
    );
}

#[test]
fn unchanged_metadata_produces_no_entries() {
    let base = validate_tasks_str("base", METADATA_BASE).expect("valid base tasks");
    let same = validate_tasks_str("same", METADATA_BASE).expect("valid same tasks");

    assert!(diff_metadata(&base, &same).is_empty());
}

const LINEAR_BASE: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"
"#;

const LINEAR_TEAM_CHANGED: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "ABC"
workspace_url = "https://linear.app/efries"
"#;

#[test]
fn linear_subfield_changes_report_with_field_granularity() {
    let base = validate_tasks_str("base", LINEAR_BASE).expect("valid base tasks");
    let current = validate_tasks_str("current", LINEAR_TEAM_CHANGED).expect("valid current tasks");

    let diff = diff_metadata(&base, &current);
    let entries: Vec<(&str, &DiffStatus)> =
        diff.iter().map(|d| (d.key.as_str(), &d.status)).collect();

    assert_eq!(entries, vec![("linear.team_key", &DiffStatus::Changed)]);
}
