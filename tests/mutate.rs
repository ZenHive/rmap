use rmap::mutate::update_status_str;
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"# Roadmap source.
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
markers = ["parallel"]

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 4, b = 8, u = 8 }
"#;

#[test]
fn update_status_changes_only_target_status_and_preserves_comments() {
    let updated =
        update_status_str("roadmap/tasks.toml", TASKS, "74", "done").expect("update status");

    assert!(updated.contains("# Roadmap source."));
    assert!(updated.contains("id = 74\nphase = 12\nbundle = \"simple\"\nstatus = \"done\""));
    assert!(
        updated.contains("id = \"78b\"\nphase = 12\nbundle = \"simple\"\nstatus = \"pending\"")
    );
    validate_tasks_str("roadmap/tasks.toml", &updated).expect("updated tasks validate");
}

#[test]
fn update_status_supports_string_task_ids() {
    let updated = update_status_str("roadmap/tasks.toml", TASKS, "78b", "in_progress")
        .expect("update status");

    assert!(
        updated.contains("id = \"78b\"\nphase = 12\nbundle = \"simple\"\nstatus = \"in_progress\"")
    );
}

#[test]
fn update_status_rejects_unknown_task_id() {
    let err = update_status_str("roadmap/tasks.toml", TASKS, "999", "done")
        .expect_err("unknown id is rejected");

    assert!(err.to_string().contains("unknown task id 999"), "{err}");
}

#[test]
fn update_status_rejects_invalid_status() {
    let err = update_status_str("roadmap/tasks.toml", TASKS, "74", "shipped")
        .expect_err("invalid status is rejected");

    assert!(
        err.to_string().contains("invalid status \"shipped\""),
        "{err}"
    );
}
