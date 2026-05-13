use rmap::mutate::{MarkerOp, update_markers_str, update_status_str};
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

const TASKS_WITH_TRAILING_FIELDS: &str = r#"schema_version = 1
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
id = 5
phase = 12
bundle = "simple"
status = "pending"
title = "parseTrades field map"
scores = { d = 3, b = 6, u = 6 }
acceptance_criteria = [
  "accepts spot payloads",
  "accepts futures payloads",
]
assignee = "claude"
"#;

#[test]
fn update_markers_new_field_lands_in_canonical_position() {
    let updated = update_markers_str(
        "roadmap/tasks.toml",
        TASKS_WITH_TRAILING_FIELDS,
        "5",
        &[MarkerOp::Add("parallel")],
    )
    .expect("update markers");

    let scores_idx = updated.find("scores = {").expect("scores present");
    let markers_idx = updated
        .find("markers = [\"parallel\"]")
        .expect("markers present");
    let ac_idx = updated
        .find("acceptance_criteria = [")
        .expect("acceptance_criteria present");
    let assignee_idx = updated.find("assignee = ").expect("assignee present");

    assert!(
        scores_idx < markers_idx,
        "markers should follow scores; got\n{updated}"
    );
    assert!(
        markers_idx < ac_idx,
        "markers should precede acceptance_criteria; got\n{updated}"
    );
    assert!(
        ac_idx < assignee_idx,
        "acceptance_criteria should still precede assignee; got\n{updated}"
    );
    validate_tasks_str("roadmap/tasks.toml", &updated).expect("validates");
}

const TASKS_WITH_EXISTING_MARKERS: &str = r#"schema_version = 1
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
id = 7
phase = 12
bundle = "simple"
status = "pending"
title = "task seven"
scores = { d = 3, b = 6, u = 6 }
acceptance_criteria = [
  "ac1",
]
markers = ["parallel"]
assignee = "claude"
"#;

#[test]
fn update_markers_existing_field_does_not_reorder() {
    // Add `cx` to a task that already has `markers = ["parallel"]`.
    // The author placed markers AFTER acceptance_criteria — that order must be preserved.
    let updated = update_markers_str(
        "roadmap/tasks.toml",
        TASKS_WITH_EXISTING_MARKERS,
        "7",
        &[MarkerOp::Add("cx")],
    )
    .expect("update markers");

    let ac_idx = updated
        .find("acceptance_criteria = [")
        .expect("acceptance_criteria present");
    let markers_idx = updated.find("markers = ").expect("markers present");

    assert!(
        ac_idx < markers_idx,
        "author-placed marker order must survive idempotent-style adds; got\n{updated}"
    );
    assert!(updated.contains(r#"markers = ["parallel", "cx"]"#));
}

#[test]
fn update_markers_idempotent_add_does_not_reorder() {
    // `+parallel` on a task whose markers already contains "parallel" must be byte-equal.
    let updated = update_markers_str(
        "roadmap/tasks.toml",
        TASKS_WITH_EXISTING_MARKERS,
        "7",
        &[MarkerOp::Add("parallel")],
    )
    .expect("update markers");

    assert_eq!(updated, TASKS_WITH_EXISTING_MARKERS);
}
