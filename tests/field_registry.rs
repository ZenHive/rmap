use std::collections::BTreeSet;

use rmap::creation_input::{CreationBuffers, STDIN_TASK_FIELDS, StdinTask};
use rmap::export::{EXPORTED_TASK_FIELDS, export_task_json_str};
use rmap::mutate::{add_task_str, canonical_task_key_index};
use rmap::schema::{Marker, Relation, Status, TASK_FIELDS, Task, Tasks};

#[test]
fn every_task_field_has_export_and_unique_canonical_position() {
    // Inspect the actual serde/schema shape, rather than a populated fixture
    // which can silently miss newly added optional fields.
    let schema = serde_json::to_value(schemars::schema_for!(Task)).unwrap();
    let properties = schema["properties"].as_object().unwrap();
    let persisted: BTreeSet<_> = properties.keys().map(String::as_str).collect();
    let registered: BTreeSet<_> = TASK_FIELDS.iter().map(|field| field.name).collect();
    assert_eq!(persisted, registered);
    let mut exported: BTreeSet<_> = EXPORTED_TASK_FIELDS.iter().copied().collect();
    for computed in ["eff", "dep_layer", "unlocks"] {
        assert!(exported.remove(computed));
    }
    assert_eq!(persisted, exported);
    assert_eq!(EXPORTED_TASK_FIELDS.len(), persisted.len() + 3);
    let positions: BTreeSet<_> = persisted
        .iter()
        .map(|name| canonical_task_key_index(name))
        .collect();
    assert!(!positions.contains(&u32::MAX));
    assert_eq!(
        positions.len(),
        persisted.len(),
        "canonical ranks must be unique"
    );
    assert_eq!(canonical_task_key_index("not_a_task_field"), u32::MAX);
}

#[test]
fn registry_creation_fields_flow_through_input_writer_export_and_diff() {
    let input = include_str!("skills_fixture/roadmap/tasks.toml");
    let task: StdinTask = toml::from_str(
        r#"
phase = 1
bundle = "alpha"
title = "registry flow"
scores = { d = 2, b = 3, u = 4 }
target_repo = "other"
body = "details"
touches = ["src/lib.rs"]
domains = ["rust"]
context_refs = ["DESIGN.md", "https://example.com/adr"]
checks = ["cargo test --test field_registry", "mix test test/order_test.exs"]
"#,
    )
    .unwrap();
    let buffers = CreationBuffers {
        id: None,
        depends_on: vec![],
        markers: vec![],
        acceptance_criteria: vec![],
        out_of_scope: vec![],
    };
    let fields = task.new_fields(&buffers, "2026-09-29");
    let (output, id) = add_task_str("test.toml", input, &fields).unwrap();
    let tasks: Tasks = toml::from_str(&output).unwrap();
    let added = tasks.task.iter().find(|task| task.id == id).unwrap();
    assert_eq!(added.target_repo.as_deref(), Some("other"));
    assert_eq!(added.body.as_deref(), Some("details"));
    assert_eq!(added.touches, ["src/lib.rs"]);
    assert_eq!(added.domains, ["rust"]);
    assert_eq!(added.context_refs, ["DESIGN.md", "https://example.com/adr"]);
    assert_eq!(
        added.checks,
        [
            "cargo test --test field_registry",
            "mix test test/order_test.exs"
        ]
    );
    assert_eq!(added.created_at.as_deref(), Some("2026-09-29"));
    let json: serde_json::Value =
        serde_json::from_str(&export_task_json_str(&tasks, Some(added)).unwrap()).unwrap();
    assert_eq!(json["target_repo"], "other");
    assert_eq!(json["touches"], serde_json::json!(["src/lib.rs"]));
    assert_eq!(
        json["context_refs"],
        serde_json::json!(["DESIGN.md", "https://example.com/adr"])
    );
    assert_eq!(
        json["checks"],
        serde_json::json!([
            "cargo test --test field_registry",
            "mix test test/order_test.exs"
        ])
    );

    let changed = output
        .replace("target_repo = \"other\"", "target_repo = \"elsewhere\"")
        .replace("body = \"details\"", "body = \"new details\"");
    let changed: Tasks = toml::from_str(&changed).unwrap();
    let diff = rmap::diff::diff_tasks(&tasks, &changed, true);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].changed_fields, ["target_repo", "body"]);
    let values = diff[0].values.as_ref().unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].field, "target_repo");
    assert_eq!(values[0].before, "other");
    assert_eq!(values[0].after, "elsewhere");
}

#[test]
fn transition_fields_are_excluded_from_creation() {
    for field in [
        "started_at",
        "done_at",
        "blocked_reason",
        "shipped_in",
        "landing_ref",
        "implemented",
        "delivered_by",
        "verified",
        "verified_by",
        "verification_ref",
        "attempts",
    ] {
        assert!(!STDIN_TASK_FIELDS.contains(&field), "{field}");
        let value = if field == "verified" {
            "true"
        } else if field == "attempts" {
            "[]"
        } else {
            "\"value\""
        };
        let input = format!(
            "phase = 1\nbundle = \"core\"\ntitle = \"x\"\nscores = {{ d = 1, b = 2, u = 3 }}\n{field} = {value}\n"
        );
        let error = toml::from_str::<StdinTask>(&input).unwrap_err().to_string();
        assert!(error.contains("unknown field"), "{error}");
    }
}

#[test]
fn vocabularies_roundtrip_known_and_invalid_wire_strings() {
    macro_rules! check {
        ($ty:ty) => {
            for wire in <$ty>::VALUES.iter().copied().chain(["invalid-value"]) {
                let value: $ty = serde_json::from_value(serde_json::json!(wire)).unwrap();
                assert_eq!(value.is_known(), wire != "invalid-value");
                assert_eq!(value.as_str(), wire);
                assert_eq!(serde_json::to_value(value).unwrap(), wire);
            }
        };
    }
    check!(Status);
    check!(Marker);
    check!(Relation);
    assert_eq!(Status::from("in_progress"), Status::InProgress);
    assert_eq!(Marker::from("parallel"), Marker::Parallel);
    assert_eq!(Relation::from("blocked_by"), Relation::BlockedBy);
}
