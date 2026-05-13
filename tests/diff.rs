use rmap::diff::{DiffStatus, diff_metadata, diff_tasks, diff_toml};
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

    let diff = diff_tasks(&base, &current, false);

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

    let diff = diff_metadata(&base, &current, false);
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

    assert!(diff_metadata(&base, &same, false).is_empty());
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
fn focus_add_remove_and_change_show_up_in_metadata_diff() {
    let no_focus = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.12]
name = "P"
order = 12
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "S"
"#;
    let focus_12 = no_focus.replace("[phases.12]", "[focus]\nphase = 12\n\n[phases.12]");
    let focus_13 = no_focus
        .replace(
            "[phases.12]\nname = \"P\"\norder = 12\nstatus = \"pending\"",
            "[focus]\nphase = 13\n\n[phases.12]\nname = \"P\"\norder = 12\nstatus = \"pending\"\n\n[phases.13]\nname = \"Q\"\norder = 13\nstatus = \"pending\"",
        );

    let none = validate_tasks_str("none", no_focus).expect("valid");
    let p12 = validate_tasks_str("p12", &focus_12).expect("valid");
    let p13 = validate_tasks_str("p13", &focus_13).expect("valid");

    let added = diff_metadata(&none, &p12, false);
    let removed = diff_metadata(&p12, &none, false);
    let changed = diff_metadata(&p12, &p13, false);

    assert!(
        added
            .iter()
            .any(|d| d.key == "focus" && d.status == DiffStatus::Added)
    );
    assert!(
        removed
            .iter()
            .any(|d| d.key == "focus" && d.status == DiffStatus::Removed)
    );
    assert!(
        changed
            .iter()
            .any(|d| d.key == "focus" && d.status == DiffStatus::Changed)
    );
}

#[test]
fn linear_subfield_changes_report_with_field_granularity() {
    let base = validate_tasks_str("base", LINEAR_BASE).expect("valid base tasks");
    let current = validate_tasks_str("current", LINEAR_TEAM_CHANGED).expect("valid current tasks");

    let diff = diff_metadata(&base, &current, false);
    let entries: Vec<(&str, &DiffStatus)> =
        diff.iter().map(|d| (d.key.as_str(), &d.status)).collect();

    assert_eq!(entries, vec![("linear.team_key", &DiffStatus::Changed)]);
}

const VERBOSE_BASE: &str = r#"
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
status = "in_progress"
title = "parseOrder field map"
scores = { d = 5, b = 8, u = 8 }
body = "old body"
"#;

const VERBOSE_CURRENT: &str = r#"
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
out_of_scope = ["Do not refactor parseTicker"]
files_to_modify = ["src/parse_order.rs"]
body = "new body"
"#;

#[test]
fn verbose_emits_before_after_for_whitelisted_changed_fields() {
    let base = validate_tasks_str("base", VERBOSE_BASE).expect("valid base");
    let current = validate_tasks_str("current", VERBOSE_CURRENT).expect("valid current");

    let diff = diff_tasks(&base, &current, true);

    assert_eq!(diff.len(), 1);
    let entry = &diff[0];
    assert_eq!(entry.status, DiffStatus::Changed);
    assert_eq!(
        entry.changed_fields,
        [
            "status",
            "scores",
            "out_of_scope",
            "files_to_modify",
            "body"
        ]
    );

    let values = entry
        .values
        .as_ref()
        .expect("verbose mode populates values");
    let fields: Vec<&str> = values.iter().map(|v| v.field.as_str()).collect();
    // Whitelist excludes `body`, `out_of_scope`, and `files_to_modify`;
    // declaration order from `diff_fields!` is preserved for the remaining
    // whitelisted entries.
    assert_eq!(fields, ["status", "scores"]);

    assert_eq!(values[0].before, serde_json::json!("in_progress"));
    assert_eq!(values[0].after, serde_json::json!("done"));
    assert_eq!(
        values[1].before,
        serde_json::json!({"d": 5, "b": 8, "u": 8})
    );
    assert_eq!(values[1].after, serde_json::json!({"d": 5, "b": 9, "u": 9}));
}

#[test]
fn verbose_off_produces_no_values_field() {
    let base = validate_tasks_str("base", VERBOSE_BASE).expect("valid base");
    let current = validate_tasks_str("current", VERBOSE_CURRENT).expect("valid current");

    let diff = diff_toml(&base, &current, false);

    // Top-level metadata is unchanged in this fixture — only the task entry should appear.
    assert!(diff.metadata.is_empty());
    assert_eq!(diff.tasks.len(), 1);
    assert!(diff.tasks[0].values.is_none());

    // JSON envelope must remain byte-identical to the pre-verbose shape: `values`
    // is skipped when None, so no `"values"` key appears anywhere.
    let json = serde_json::to_string(&diff).expect("serialize diff");
    assert!(!json.contains("\"values\""), "{json}");
}

#[test]
fn verbose_metadata_emits_before_after_for_whitelisted_keys() {
    let base = validate_tasks_str("base", METADATA_BASE).expect("valid base");
    let current = validate_tasks_str("current", METADATA_CURRENT).expect("valid current");

    let diff = diff_metadata(&base, &current, true);
    let branch = diff
        .iter()
        .find(|entry| entry.key == "default_branch")
        .expect("default_branch changed");
    let values = branch.values.as_ref().expect("verbose populates values");
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].before, serde_json::json!("main"));
    assert_eq!(values[0].after, serde_json::json!("development"));

    // `phases.*` / `bundles.*` map entries are deliberately not on the metadata
    // whitelist (already key-granular).
    let phases = diff
        .iter()
        .find(|entry| entry.key == "phases.12")
        .expect("phases.12 changed");
    assert!(phases.values.is_none());
}

#[test]
fn verbose_added_and_removed_task_entries_carry_no_values() {
    // BASE_TASKS / CURRENT_TASKS produce one Added, one Removed, and one Changed.
    // Verbose mode must leave Added/Removed `values` at None — there's no
    // before/after for a wholesale add or remove.
    let base = validate_tasks_str("base", BASE_TASKS).expect("valid base tasks");
    let current = validate_tasks_str("current", CURRENT_TASKS).expect("valid current tasks");

    let diff = diff_tasks(&base, &current, true);

    for entry in &diff {
        match entry.status {
            DiffStatus::Added | DiffStatus::Removed => {
                assert!(
                    entry.values.is_none(),
                    "{:?} entry {:?} should carry no values",
                    entry.status,
                    entry.id,
                );
            }
            DiffStatus::Changed => {}
        }
    }
}
