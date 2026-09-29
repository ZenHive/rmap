use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rmap::diff::{DiffStatus, diff_toml};
use rmap::export::export_json_str;
use rmap::validate::validate_tasks_str;
use tempfile::TempDir;

const CLEAN: &str = r#"
schema_version = 2
project = "spec_coverage"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "Core work"

[bundles.secondary]
phase = 1
order = 2
description = "Secondary work"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "pending"
title = "Clean task A"
scores = { d = 2, b = 4, u = 4 }
scored_at = "2026-04-15"

[[task]]
id = 2
phase = 1
bundle = "secondary"
status = "done"
implemented = "fixture"
verified = true
verified_by = "fixture-reviewer"
title = "Clean task B"
scores = { d = 1, b = 3, u = 3 }
scored_at = "2026-04-20"
"#;

fn write(dir: &TempDir, rel: &str, body: &str) {
    let path = dir.path().join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn doctor(dir: &TempDir, json: bool) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rmap"));
    command
        .env("RMAP_TODAY", "2026-05-11")
        .current_dir(dir.path())
        .arg("doctor")
        .arg("--tasks-path")
        .arg(dir.path().join("roadmap/tasks.toml"));
    if json {
        command.arg("--json");
    }
    let output = command.output().expect("run rmap doctor");
    (
        output.status.success(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn kinds(json: &serde_json::Value) -> Vec<&str> {
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["kind"].as_str().unwrap())
        .collect()
}

#[test]
fn doctor_reports_untested_active_rules_and_unknown_tags_in_three_comment_syntaxes() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "roadmap/tasks.toml",
        &format!(
            "{CLEAN}\n[specs.access]\npath = \"docs/access.md\"\nstatus = \"active\"\n\
             [specs.drafted]\npath = \"docs/draft.md\"\nstatus = \"draft\"\n\
             [specs.retired]\npath = \"docs/old.md\"\nstatus = \"retired\"\n\
             [spec_tests]\nglobs = [\"tests/**/*.rs\", \"test/**/*.exs\", \"db/**/*.sql\"]\n"
        ),
    );
    write(
        &dir,
        "docs/access.md",
        "ACCESS-1: Authenticate.\nACCESS-2: Authorize.\nACCESS-3: Audit.\nACCESS-4: Retain.\n",
    );
    write(
        &dir,
        "docs/draft.md",
        "DRAFT-1: Not yet a coverage target.\n",
    );
    write(
        &dir,
        "docs/old.md",
        "RET-1: Retired rule stays out of coverage.\n",
    );
    write(
        &dir,
        "tests/covered.rs",
        "// spec-tags: ACCESS-1, DRAFT-1\n",
    );
    write(&dir, "test/covered.exs", "# spec-tags: ACCESS-2\n");
    write(&dir, "db/covered.sql", "-- spec-tags: ACCESS-3\n");
    write(&dir, "tests/unknown.rs", "// spec-tags: GONE-1\n");
    write(&dir, "test/unknown.exs", "# spec-tags: GONE-2\n");
    write(&dir, "db/unknown.sql", "-- spec-tags: GONE-3\n");
    // Outside the globs: must not count as coverage for ACCESS-4.
    write(&dir, "notes/skip.txt", "spec-tags: ACCESS-4\n");

    let (ok, human) = doctor(&dir, false);
    assert!(ok, "doctor must exit 0:\n{human}");
    assert!(
        human.contains("rule ACCESS-4 in docs/access.md"),
        "untested rule must cite id and spec file:\n{human}"
    );
    assert!(human.contains("rule GONE-1 in tests/unknown.rs"), "{human}");
    assert!(human.contains("rule GONE-2 in test/unknown.exs"), "{human}");
    assert!(human.contains("rule GONE-3 in db/unknown.sql"), "{human}");
    assert!(
        !human.contains("DRAFT-1"),
        "draft rules are not untested:\n{human}"
    );
    assert!(
        !human.contains("RET-1"),
        "retired rules are not untested:\n{human}"
    );
    assert!(!human.contains("ACCESS-1"), "tagged active rule:\n{human}");
    assert!(!human.contains("ACCESS-2"), "{human}");
    assert!(!human.contains("ACCESS-3"), "{human}");

    let (ok, json_text) = doctor(&dir, true);
    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    assert_eq!(json["ok"], false);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|finding| {
            finding["kind"] == "untested_rule"
                && finding["rule"] == "ACCESS-4"
                && finding["file"] == "docs/access.md"
        }),
        "{json_text}"
    );
    for (rule, file) in [
        ("GONE-1", "tests/unknown.rs"),
        ("GONE-2", "test/unknown.exs"),
        ("GONE-3", "db/unknown.sql"),
    ] {
        assert!(
            findings.iter().any(|finding| {
                finding["kind"] == "unknown_rule_tag"
                    && finding["rule"] == rule
                    && finding["file"] == file
            }),
            "missing {rule} in {file}:\n{json_text}"
        );
    }
    assert_eq!(
        kinds(&json)
            .into_iter()
            .filter(|kind| *kind == "untested_rule" || *kind == "unknown_rule_tag")
            .count(),
        4,
        "{json_text}"
    );
}

#[test]
fn custom_marker_replaces_the_default() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "roadmap/tasks.toml",
        &format!(
            "{CLEAN}\n[specs.access]\npath = \"docs/access.md\"\nstatus = \"active\"\n\
             [spec_tests]\nglobs = [\"tests/**/*.rs\"]\nmarker = \"pins:\"\n"
        ),
    );
    write(&dir, "docs/access.md", "ACCESS-1: One.\nACCESS-2: Two.\n");
    write(
        &dir,
        "tests/pins.rs",
        "// pins: ACCESS-1\n// spec-tags: ACCESS-2\n",
    );

    let (ok, human) = doctor(&dir, false);
    assert!(ok, "{human}");
    assert!(human.contains("rule ACCESS-2 in docs/access.md"), "{human}");
    assert!(!human.contains("ACCESS-1"), "{human}");
}

#[test]
fn without_spec_test_config_or_without_specs_doctor_output_is_unchanged() {
    let bare = TempDir::new().unwrap();
    write(&bare, "roadmap/tasks.toml", CLEAN);
    let (ok, bare_json) = doctor(&bare, true);
    assert!(ok, "{bare_json}");
    let bare_value: serde_json::Value = serde_json::from_str(&bare_json).unwrap();
    assert_eq!(bare_value["ok"], true);
    assert!(bare_value["findings"].as_array().unwrap().is_empty());

    let specs_only = TempDir::new().unwrap();
    write(
        &specs_only,
        "roadmap/tasks.toml",
        &format!("{CLEAN}\n[specs.access]\npath = \"docs/access.md\"\nstatus = \"active\"\n"),
    );
    write(
        &specs_only,
        "docs/access.md",
        "ACCESS-1: Untagged but not configured.\n",
    );
    write(&specs_only, "tests/covered.rs", "// spec-tags: GONE-9\n");
    let (ok, specs_json) = doctor(&specs_only, true);
    assert!(ok, "{specs_json}");
    let specs_value: serde_json::Value = serde_json::from_str(&specs_json).unwrap();
    assert_eq!(specs_value["findings"], bare_value["findings"]);

    let config_only = TempDir::new().unwrap();
    write(
        &config_only,
        "roadmap/tasks.toml",
        &format!("{CLEAN}\n[spec_tests]\nglobs = [\"tests/**/*.rs\"]\n"),
    );
    write(&config_only, "tests/covered.rs", "// spec-tags: GONE-9\n");
    let (ok, config_json) = doctor(&config_only, true);
    assert!(ok, "{config_json}");
    let config_value: serde_json::Value = serde_json::from_str(&config_json).unwrap();
    assert_eq!(config_value["findings"], bare_value["findings"]);
    assert_eq!(
        doctor(&config_only, false).1.trim(),
        "rmap doctor — all checks passed"
    );
}

#[test]
fn blank_marker_and_empty_globs_fail_validate() {
    for (label, extra) in [
        (
            "marker",
            "[spec_tests]\nglobs = [\"tests/**/*.rs\"]\nmarker = \"\"\n",
        ),
        ("globs", "[spec_tests]\nglobs = []\n"),
        ("parent", "[spec_tests]\nglobs = [\"../secret/**\"]\n"),
    ] {
        let err = validate_tasks_str("memory.toml", &format!("{CLEAN}\n{extra}")).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("spec_tests"), "{label}: {message}");
    }
}

#[test]
fn spec_tests_round_trips_through_export_and_diff_and_is_omitted_when_absent() {
    let bare = validate_tasks_str("memory.toml", CLEAN).unwrap();
    let bare_json = export_json_str(&bare).unwrap();
    assert!(!bare_json.contains("spec_tests"));

    let configured = format!("{CLEAN}\n[spec_tests]\nglobs = [\"tests/**/*.rs\"]\n");
    let tasks = validate_tasks_str("memory.toml", &configured).unwrap();
    let json = export_json_str(&tasks).unwrap();
    let exported: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(exported["spec_tests"]["globs"][0], "tests/**/*.rs");
    assert!(exported["spec_tests"].get("marker").is_none());

    let marked = format!("{configured}marker = \"pins:\"\n");
    let marked_tasks = validate_tasks_str("memory.toml", &marked).unwrap();
    let added = diff_toml(&bare, &tasks, false);
    assert!(added.metadata.iter().any(|entry| {
        entry.key == "spec_tests" && entry.status == DiffStatus::Added && entry.values.is_none()
    }));
    let changed = diff_toml(&tasks, &marked_tasks, true);
    let entry = changed
        .metadata
        .iter()
        .find(|entry| entry.key == "spec_tests")
        .expect("marker change");
    assert_eq!(entry.status, DiffStatus::Changed);
    let values = entry.values.as_ref().expect("verbose values");
    assert_eq!(values[0].field, "spec_tests");
    assert!(values[0].after.to_string().contains("pins:"));

    let quiet = diff_toml(&tasks, &marked_tasks, false);
    let quiet_entry = quiet
        .metadata
        .iter()
        .find(|entry| entry.key == "spec_tests")
        .unwrap();
    assert!(quiet_entry.values.is_none());
}

/// The agent-contract registration. The test accepts the repository roadmap
/// either without it (appends it to a scratch copy) or with exactly this block.
const AGENT_CONTRACT_REGISTRATION: &str = r#"
[specs.agent_json]
path = "specs/agent-json.md"
status = "active"

[specs.agent_dispatch]
path = "specs/agent-dispatch.md"
status = "active"

[specs.agent_delegate]
path = "specs/agent-delegate.md"
status = "active"

[spec_tests]
globs = ["tests/**/*.rs"]
marker = "rmap-spec-tags:"
"#;

#[test]
fn agent_contract_specs_validate_and_flag_only_untagged_rules() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest.join("roadmap/tasks.toml")).unwrap();
    let registered = source
        .lines()
        .any(|line| line.starts_with("[specs.") || line == "[spec_tests]");
    let roadmap = if registered {
        // The live roadmap must register exactly this block, not a drifted copy.
        let live: serde_json::Value = serde_json::from_str(
            &export_json_str(&validate_tasks_str("roadmap/tasks.toml", &source).unwrap()).unwrap(),
        )
        .unwrap();
        let block: toml::Value = toml::from_str(AGENT_CONTRACT_REGISTRATION).unwrap();
        let block = serde_json::to_value(block).unwrap();
        assert_eq!(
            live["specs"], block["specs"],
            "live [specs] drifted from the contract block"
        );
        assert_eq!(
            live["spec_tests"], block["spec_tests"],
            "live [spec_tests] drifted"
        );
        source
    } else {
        format!("{source}\n{AGENT_CONTRACT_REGISTRATION}")
    };

    let dir = TempDir::new().unwrap();
    write(&dir, "roadmap/tasks.toml", &roadmap);
    copy_files_with_extension(&manifest.join("specs"), &dir.path().join("specs"), "md");
    copy_files_with_extension(&manifest.join("tests"), &dir.path().join("tests"), "rs");

    let tasks_path = dir.path().join("roadmap/tasks.toml");
    let validate = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .unwrap();
    assert!(
        validate.status.success(),
        "validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let specs_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("specs")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .unwrap();
    assert!(
        specs_out.status.success(),
        "specs:\n{}",
        String::from_utf8_lossy(&specs_out.stderr)
    );
    let catalog: serde_json::Value = serde_json::from_slice(&specs_out.stdout).unwrap();
    let mut parsed: Vec<String> = catalog
        .as_object()
        .unwrap()
        .values()
        .flat_map(|spec| spec["rules"].as_array().unwrap())
        .map(|rule| {
            assert!(rule["text"].is_string(), "current rule has text: {rule}");
            rule["id"].as_str().unwrap().to_string()
        })
        .collect();
    parsed.sort();
    parsed.dedup();
    let mut expected = [
        "DELEGATE-1",
        "DELEGATE-2",
        "DELEGATE-3",
        "DELEGATE-4",
        "DELEGATE-5",
        "DISPATCH-1",
        "DISPATCH-2",
        "DISPATCH-3",
        "DISPATCH-4",
        "DISPATCH-5",
        "DISPATCH-6",
        "JSON-1",
        "JSON-2",
        "JSON-3",
    ]
    .map(String::from)
    .to_vec();
    expected.sort();
    assert_eq!(parsed, expected, "parsed agent-contract rules");

    let (ok, json_text) = doctor(&dir, true);
    assert!(ok, "doctor must exit 0:\n{json_text}");
    let json: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let untested: Vec<&str> = findings
        .iter()
        .filter(|finding| finding["kind"] == "untested_rule")
        .map(|finding| finding["rule"].as_str().unwrap())
        .collect();
    let unknown: Vec<&str> = findings
        .iter()
        .filter(|finding| finding["kind"] == "unknown_rule_tag")
        .map(|finding| finding["rule"].as_str().unwrap())
        .collect();
    assert!(unknown.is_empty(), "unknown tags: {unknown:?}");
    assert_eq!(
        untested,
        vec!["JSON-1"],
        "only the deliberate additive-only gap is untagged:\n{json_text}"
    );
    assert!(findings.iter().any(|finding| {
        finding["kind"] == "untested_rule" && finding["file"] == "specs/agent-json.md"
    }));
}

fn copy_files_with_extension(from: &Path, to: &Path, extension: &str) {
    fn walk(from: &Path, to: &Path, extension: &str) {
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let dest = to.join(entry.file_name());
                fs::create_dir_all(&dest).unwrap();
                walk(&path, &dest, extension);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                fs::create_dir_all(to).unwrap();
                fs::copy(&path, to.join(entry.file_name())).unwrap();
            }
        }
    }
    walk(from, to, extension);
}
