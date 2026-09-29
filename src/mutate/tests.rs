use super::*;

const SIMPLE_TOML: &str = r#"
schema_version = 2
project = "test"
default_branch = "main"

[phases.1]
name = "Phase One"
order = 1
status = "pending"

[bundles.b]
phase = 1
order = 1
description = "Bundle B"

[[task]]
id = 1
phase = 1
bundle = "b"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }
acceptance_criteria = ["Observable result"]
"#;

#[test]
fn update_status_many_str_empty_ids_returns_empty_ids_error() {
    let result = update_status_many_str(
        "test.toml",
        SIMPLE_TOML,
        &[],
        "done",
        TransitionFields::default(),
    );
    assert!(
        matches!(result, Err(MutateError::EmptyIds)),
        "expected EmptyIds, got: {result:?}"
    );
}

/// A roadmap whose task ids are TOML strings (`id = "1"`) — harness's
/// convention, and the shape that exposed the `next_task_id` collision bug.
const STRING_ID_TOML: &str = r#"
schema_version = 2
project = "test"
default_branch = "main"

[phases.1]
name = "Phase One"
order = 1
status = "pending"

[bundles.b]
phase = 1
order = 1
description = "Bundle B"

[[task]]
id = "1"
phase = 1
bundle = "b"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = "2"
phase = 1
bundle = "b"
status = "pending"
title = "Task Two"
scores = { d = 1, b = 5, u = 5 }
"#;

fn new_task_fields(title: &str) -> NewTaskFields<'_> {
    NewTaskFields {
        phase: 1,
        bundle: "b",
        title,
        scores: (1, 5, 5),
        status: "pending",
        ..Default::default()
    }
}

#[test]
fn add_task_to_string_id_file_continues_the_numeric_sequence() {
    // Before the fix `next_task_id` skipped string ids, so a string-id
    // roadmap restarted at 1 and the new task collided with `id = "1"`.
    let fields = new_task_fields("Task Three");
    let (_, allocated) = add_task_str("test.toml", STRING_ID_TOML, &fields).expect("add_task_str");

    assert_eq!(allocated, 3, "next id after \"1\" / \"2\" must be 3, not 1");
}

#[test]
fn add_task_to_string_id_file_writes_the_id_as_a_string() {
    // The new id must mirror the file's form, or the document ends up with
    // `id = "1"` and `id = 3` mixed — and duplicate detection, keyed on
    // `TaskId`, cannot see `Number(3)` and `Text("3")` as the same id.
    let fields = new_task_fields("Task Three");
    let (output, _) = add_task_str("test.toml", STRING_ID_TOML, &fields).expect("add_task_str");

    assert!(
        output.contains("id = \"3\""),
        "new id must be string-typed to match the file:\n{output}"
    );
    assert!(
        !output.contains("\nid = 3\n"),
        "new id must not be written as a bare integer:\n{output}"
    );
}

#[test]
fn add_task_to_integer_id_file_still_writes_the_id_as_an_integer() {
    // rmap's own roadmap uses integer ids — the fix must not regress it.
    let fields = new_task_fields("Task Two");
    let (output, allocated) =
        add_task_str("test.toml", SIMPLE_TOML, &fields).expect("add_task_str");

    assert_eq!(allocated, 2);
    assert!(
        output.contains("\nid = 2\n"),
        "integer-id roadmaps must keep integer ids:\n{output}"
    );
    assert!(
        !output.contains("id = \"2\""),
        "must not switch an integer-id roadmap to string ids:\n{output}"
    );
}

#[test]
fn update_assignee_str_sets_agent_and_model() {
    let updated = update_assignee_str(
        "test.toml",
        SIMPLE_TOML,
        "1",
        Some("cursor"),
        Some("composer-2.5-fast"),
    )
    .expect("assign agent + model");

    assert!(
        updated.contains("assignee = \"cursor\""),
        "assignee set:\n{updated}"
    );
    assert!(
        updated.contains("model = \"composer-2.5-fast\""),
        "model set:\n{updated}"
    );
}

#[test]
fn update_assignee_str_rejects_agent_without_model_on_live_task() {
    let err = update_assignee_str("test.toml", SIMPLE_TOML, "1", Some("claude"), None)
        .expect_err("missing model rejected");

    let message = err.to_string();
    assert!(
        message.contains("missing model"),
        "expected missing model in error: {message}"
    );
    assert!(
        message.contains("a dispatchable task must pin the LLM it runs on"),
        "expected pin explanation: {message}"
    );
}

#[test]
fn update_assignee_str_clears_assignee_and_model() {
    let with_assignee = update_assignee_str(
        "test.toml",
        SIMPLE_TOML,
        "1",
        Some("cursor"),
        Some("composer-2.5-fast"),
    )
    .expect("assign first");

    let cleared =
        update_assignee_str("test.toml", &with_assignee, "1", None, None).expect("clear assignee");

    assert!(
        !cleared.contains("assignee ="),
        "assignee removed:\n{cleared}"
    );
    assert!(!cleared.contains("model ="), "model removed:\n{cleared}");
}

#[test]
fn update_assignee_str_unknown_id_errors() {
    let err = update_assignee_str(
        "test.toml",
        SIMPLE_TOML,
        "999",
        Some("claude"),
        Some("gpt-5"),
    )
    .expect_err("unknown id");

    assert!(
        matches!(err, MutateError::UnknownTaskId(ref id) if id == "999"),
        "expected UnknownTaskId, got: {err:?}"
    );
}
