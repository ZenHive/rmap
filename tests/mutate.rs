use rmap::mutate::{MarkerOp, TransitionFields, update_markers_str, update_status_str};
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"# Roadmap source.
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
    let updated = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            ..TransitionFields::default()
        },
    )
    .expect("update status");

    assert!(updated.contains("# Roadmap source."));
    assert!(updated.contains("id = 74\nphase = 12\nbundle = \"simple\"\nstatus = \"done\""));
    assert!(updated.contains("implemented = \"shipped\""));
    assert!(
        updated.contains("id = \"78b\"\nphase = 12\nbundle = \"simple\"\nstatus = \"pending\"")
    );
    validate_tasks_str("roadmap/tasks.toml", &updated).expect("updated tasks validate");
}

#[test]
fn update_status_supports_string_task_ids() {
    let updated = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "78b",
        "in_progress",
        TransitionFields::default(),
    )
    .expect("update status");

    assert!(
        updated.contains("id = \"78b\"\nphase = 12\nbundle = \"simple\"\nstatus = \"in_progress\"")
    );
}

#[test]
fn update_status_rejects_unknown_task_id() {
    let err = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "999",
        "done",
        TransitionFields {
            implemented: Some("x"),
            ..TransitionFields::default()
        },
    )
    .expect_err("unknown id is rejected");

    assert!(err.to_string().contains("unknown task id 999"), "{err}");
}

#[test]
fn update_status_rejects_invalid_status() {
    let err = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "shipped",
        TransitionFields::default(),
    )
    .expect_err("invalid status is rejected");

    assert!(
        err.to_string().contains("invalid status \"shipped\""),
        "{err}"
    );
}

#[test]
fn update_status_verified_requires_evaluator_provenance() {
    let err = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            verified: Some(true),
            ..TransitionFields::default()
        },
    )
    .expect_err("provenance-free verification is rejected");

    assert!(err.to_string().contains("--verified-by"), "{err}");
}

#[test]
fn update_status_writes_verification_provenance() {
    let updated = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            verified: Some(true),
            verified_by: Some("grok/grok-4.5"),
            verification_ref: Some("harness-run:run-123"),
            ..TransitionFields::default()
        },
    )
    .expect("verified transition with provenance");

    assert!(updated.contains("verified = true"), "{updated}");
    assert!(
        updated.contains("verified_by = \"grok/grok-4.5\""),
        "{updated}"
    );
    assert!(
        updated.contains("verification_ref = \"harness-run:run-123\""),
        "{updated}"
    );
}

#[test]
fn update_status_rejects_provenance_without_verification() {
    let err = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            verified_by: Some("reviewer"),
            ..TransitionFields::default()
        },
    )
    .expect_err("provenance without --verified is rejected");

    assert!(err.to_string().contains("require --verified"), "{err}");
}

#[test]
fn update_status_blocked_writes_then_auto_clears_reason() {
    // Blocking writes the reason...
    let blocked = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "blocked",
        TransitionFields {
            blocked_reason: Some("waiting on vendor"),
            ..TransitionFields::default()
        },
    )
    .expect("block task");
    assert!(
        blocked.contains("blocked_reason = \"waiting on vendor\""),
        "blocked_reason not written:\n{blocked}"
    );
    validate_tasks_str("roadmap/tasks.toml", &blocked).expect("blocked tasks validate");

    // ...and leaving blocked drops it again.
    let unblocked = update_status_str(
        "roadmap/tasks.toml",
        &blocked,
        "74",
        "in_progress",
        TransitionFields::default(),
    )
    .expect("unblock task");
    assert!(
        !unblocked.contains("blocked_reason"),
        "stale blocked_reason not cleared when leaving blocked:\n{unblocked}"
    );
}

#[test]
fn update_status_ignores_reason_on_non_blocked_transition() {
    let updated = update_status_str(
        "roadmap/tasks.toml",
        TASKS,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            blocked_reason: Some("ignored"),
            ..TransitionFields::default()
        },
    )
    .expect("update status");

    assert!(
        !updated.contains("blocked_reason"),
        "blocked_reason must not be written on a non-blocked transition:\n{updated}"
    );
}

const TASKS_WITH_TRAILING_FIELDS: &str = r#"schema_version = 2
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
model = "claude-opus-4-8"
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

const TASKS_WITH_EXISTING_MARKERS: &str = r#"schema_version = 2
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
model = "claude-opus-4-8"
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

fn landing_in_progress(input: &str, landing_ref: Option<&str>) -> String {
    update_status_str(
        "roadmap/tasks.toml",
        input,
        "74",
        "in_progress",
        TransitionFields {
            landing_ref,
            ..TransitionFields::default()
        },
    )
    .expect("in_progress with landing_ref")
}

#[test]
fn update_status_landing_ref_persists_on_in_progress() {
    let updated = landing_in_progress(TASKS, Some("https://example.com/pr/1"));
    assert!(
        updated.contains("landing_ref = \"https://example.com/pr/1\""),
        "landing_ref not written:\n{updated}"
    );
    assert!(updated.contains("status = \"in_progress\""));
    validate_tasks_str("roadmap/tasks.toml", &updated).expect("updated tasks validate");
}

#[test]
fn update_status_landing_ref_on_already_in_progress_leaves_started_at() {
    let first = landing_in_progress(TASKS, Some("https://example.com/pr/1"));
    let started = first
        .lines()
        .find(|line| line.starts_with("started_at = "))
        .expect("started_at written on first in_progress")
        .to_string();

    let second = landing_in_progress(&first, Some("https://example.com/pr/2"));
    let started_again = second
        .lines()
        .find(|line| line.starts_with("started_at = "))
        .expect("started_at still present");
    assert_eq!(
        started, started_again,
        "started_at must not churn:\n{second}"
    );
    assert!(
        second.contains("landing_ref = \"https://example.com/pr/2\""),
        "landing_ref not overwritten:\n{second}"
    );
    assert_eq!(
        second.matches("started_at = ").count(),
        1,
        "exactly one started_at:\n{second}"
    );
}

#[test]
fn update_status_empty_landing_ref_clears_field() {
    let set = landing_in_progress(TASKS, Some("https://example.com/pr/1"));
    let cleared = landing_in_progress(&set, Some(""));
    let task_74 = cleared
        .split("[[task]]")
        .find(|chunk| chunk.contains("id = 74\n"))
        .expect("task 74");
    assert!(
        !task_74.contains("landing_ref"),
        "empty --landing-ref must drop the field:\n{task_74}"
    );
    assert!(task_74.contains("status = \"in_progress\""));
}

#[test]
fn update_status_rejects_landing_ref_on_non_in_progress() {
    for status in ["pending", "done", "blocked", "superseded"] {
        let err = update_status_str(
            "roadmap/tasks.toml",
            TASKS,
            "74",
            status,
            TransitionFields {
                landing_ref: Some("https://example.com/pr/1"),
                implemented: if status == "done" {
                    Some("shipped")
                } else {
                    None
                },
                blocked_reason: if status == "blocked" {
                    Some("waiting")
                } else {
                    None
                },
                ..TransitionFields::default()
            },
        )
        .expect_err("landing_ref on other statuses is rejected");
        let message = err.to_string();
        assert!(
            message.contains("--landing-ref") && message.contains("in_progress"),
            "error must name the rule for status {status}: {message}"
        );
        assert!(
            message.contains(status),
            "error must name the rejected status {status}: {message}"
        );
    }
}

#[test]
fn update_status_preserves_landing_ref_on_done_and_blocked() {
    let in_progress = landing_in_progress(TASKS, Some("https://example.com/pr/1"));

    let done = update_status_str(
        "roadmap/tasks.toml",
        &in_progress,
        "74",
        "done",
        TransitionFields {
            implemented: Some("shipped"),
            ..TransitionFields::default()
        },
    )
    .expect("done");
    let done_task = done
        .split("[[task]]")
        .find(|chunk| chunk.contains("id = 74\n"))
        .expect("task 74");
    assert!(
        done_task.contains("landing_ref = \"https://example.com/pr/1\""),
        "done must keep landing_ref:\n{done_task}"
    );

    let blocked = update_status_str(
        "roadmap/tasks.toml",
        &in_progress,
        "74",
        "blocked",
        TransitionFields {
            blocked_reason: Some("pr closed unmerged"),
            ..TransitionFields::default()
        },
    )
    .expect("blocked");
    let blocked_task = blocked
        .split("[[task]]")
        .find(|chunk| chunk.contains("id = 74\n"))
        .expect("task 74");
    assert!(
        blocked_task.contains("landing_ref = \"https://example.com/pr/1\""),
        "blocked must keep landing_ref:\n{blocked_task}"
    );
}

#[test]
fn update_status_clears_landing_ref_on_pending() {
    let in_progress = landing_in_progress(TASKS, Some("https://example.com/pr/1"));
    let pending = update_status_str(
        "roadmap/tasks.toml",
        &in_progress,
        "74",
        "pending",
        TransitionFields::default(),
    )
    .expect("pending");
    let pending_task = pending
        .split("[[task]]")
        .find(|chunk| chunk.contains("id = 74\n"))
        .expect("task 74");
    assert!(
        !pending_task.contains("landing_ref"),
        "pending must clear landing_ref:\n{pending_task}"
    );
}
