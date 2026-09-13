use rmap::diff::{DiffStatus, diff_metadata};
use rmap::export::export_json_str;
use rmap::render::render_roadmap_str_with_today;
use rmap::validate::validate_tasks_str;
use serde_json::json;
use std::process::Command;

fn input(project: &str, phase: &str) -> String {
    format!(
        r#"schema_version = 2
project = "demo"
default_branch = "main"
{project}
[phases.3]
name = "Foo"
order = 3
status = "done"
{phase}
"#
    )
}

#[test]
fn archive_link_precedence_and_omission() {
    for (project, phase, expected) in [
        (
            "",
            "",
            "> 0 tasks. See [CHANGELOG.md](CHANGELOG.md#phase-3-foo).",
        ),
        ("changelog_path = false", "", "> 0 tasks."),
        ("", "changelog = false", "> 0 tasks."),
        (
            "changelog_path = \"packages/x/CHANGELOG.md\"",
            "",
            "> 0 tasks. See [CHANGELOG.md](packages/x/CHANGELOG.md#phase-3-foo).",
        ),
        (
            "changelog_path = \"root.md\"",
            "changelog = false",
            "> 0 tasks.",
        ),
        (
            "changelog_path = false",
            "changelog = \"docs/History.md\"",
            "> 0 tasks. See [History.md](docs/History.md#phase-3-foo).",
        ),
        (
            "changelog_path = \"root.md\"",
            "changelog = \"docs/History.md\"",
            "> 0 tasks. See [History.md](docs/History.md#phase-3-foo).",
        ),
    ] {
        let tasks = validate_tasks_str("tasks.toml", &input(project, phase)).unwrap();
        let rendered = render_roadmap_str_with_today(
            "<!-- TASKS:BEGIN phase=3 -->\nstale\n<!-- TASKS:END -->\n",
            &tasks,
            "2026-09-13",
        )
        .unwrap();
        assert_eq!(
            rendered,
            format!("<!-- TASKS:BEGIN phase=3 -->\n{expected}\n<!-- TASKS:END -->\n")
        );
    }
}

#[test]
fn invalid_changelog_values_fail_cli_validation_and_render_before_writes() {
    let dir = tempfile::tempdir().unwrap();
    let tasks_path = dir.path().join("tasks.toml");
    let roadmap_path = dir.path().join("ROADMAP.md");
    let data_path = dir.path().join("data.json");
    for value in ["true", "\"\"", "\"   \"", "\"\\t\\n\"", "123", "[]", "{}"] {
        for (project, phase) in [
            (format!("changelog_path = {value}"), String::new()),
            (String::new(), format!("changelog = {value}")),
        ] {
            let source = input(&project, &phase);
            std::fs::write(&tasks_path, &source).unwrap();
            std::fs::write(&roadmap_path, "unchanged roadmap").unwrap();
            std::fs::write(&data_path, "unchanged data").unwrap();
            for command in ["validate", "render"] {
                let mut cli = Command::new(env!("CARGO_BIN_EXE_rmap"));
                cli.args([command, "--tasks-path"]).arg(&tasks_path);
                if command == "render" {
                    cli.arg("--roadmap-path")
                        .arg(&roadmap_path)
                        .arg("--data-path")
                        .arg(&data_path);
                }
                let output = cli.output().unwrap();
                assert_eq!(output.status.code(), Some(4), "{source}");
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("changelog"),
                    "{output:?}"
                );
            }
            assert_eq!(std::fs::read_to_string(&tasks_path).unwrap(), source);
            assert_eq!(
                std::fs::read_to_string(&roadmap_path).unwrap(),
                "unchanged roadmap"
            );
            assert_eq!(
                std::fs::read_to_string(&data_path).unwrap(),
                "unchanged data"
            );
        }
    }
}

#[test]
fn changelog_metadata_exports_and_diffs() {
    let absent = validate_tasks_str("tasks.toml", &input("", "")).unwrap();
    let exported: serde_json::Value =
        serde_json::from_str(&export_json_str(&absent).unwrap()).unwrap();
    assert!(exported.get("changelog_path").is_none());
    assert!(exported["phases"]["3"].get("changelog").is_none());

    for (project, phase) in [
        ("\"packages/x/CHANGELOG.md\"", "false"),
        ("false", "\"docs/History.md\""),
    ] {
        let configured = validate_tasks_str(
            "tasks.toml",
            &input(
                &format!("changelog_path = {project}"),
                &format!("changelog = {phase}"),
            ),
        )
        .unwrap();
        let exported: serde_json::Value =
            serde_json::from_str(&export_json_str(&configured).unwrap()).unwrap();
        assert_eq!(
            exported["changelog_path"],
            serde_json::from_str::<serde_json::Value>(project).unwrap()
        );
        assert_eq!(
            exported["phases"]["3"]["changelog"],
            serde_json::from_str::<serde_json::Value>(phase).unwrap()
        );
        for (base, current, status) in [
            (&absent, &configured, DiffStatus::Added),
            (&configured, &absent, DiffStatus::Removed),
        ] {
            let diff = diff_metadata(base, current, true);
            assert_eq!(diff.len(), 2);
            assert_eq!(diff[0].key, "changelog_path");
            assert_eq!(diff[0].status, status);
            assert_eq!(diff[1].key, "phases.3");
            assert_eq!(diff[1].status, DiffStatus::Changed);
        }
        let changed = validate_tasks_str(
            "tasks.toml",
            &input("changelog_path = \"other.md\"", "changelog = \"other.md\""),
        )
        .unwrap();
        let verbose = diff_metadata(&configured, &changed, true);
        assert!(verbose.iter().all(|d| d.status == DiffStatus::Changed));
        let path_diff = verbose
            .iter()
            .find(|entry| entry.key == "changelog_path")
            .expect("changelog_path changed");
        let values = path_diff
            .values
            .as_ref()
            .expect("verbose changelog_path values");
        assert_eq!(values[0].field, "changelog_path");
    }
}

#[test]
fn schema_describes_changelog_path_or_false_at_both_levels() {
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("schema")
        .output()
        .unwrap();
    assert!(output.status.success());
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let compiled = jsonschema::JSONSchema::compile(&schema).unwrap();
    let tasks = validate_tasks_str("tasks.toml", &input("", "")).unwrap();
    for (value, valid) in [
        (json!(false), true),
        (json!("packages/x/CHANGELOG.md"), true),
        (json!(true), false),
        (json!(""), false),
        (json!(" \t\n"), false),
        (json!(1), false),
        (json!([]), false),
        (json!({}), false),
    ] {
        for phase in [false, true] {
            let mut document = serde_json::to_value(&tasks).unwrap();
            if phase {
                document["phases"]["3"]["changelog"] = value.clone();
            } else {
                document["changelog_path"] = value.clone();
            }
            assert_eq!(compiled.is_valid(&document), valid, "{document}");
        }
    }
}
