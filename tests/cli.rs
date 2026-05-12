use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

const VALID_TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.foundation]
phase = 1
order = 1
description = "Foundation tasks"

[[task]]
id = 1
phase = 1
bundle = "foundation"
status = "pending"
title = "Add validation"
scores = { d = 2, b = 8, u = 8 }
"#;

const ROADMAP: &str = r#"# Roadmap

Keep me.

<!-- TASKS:BEGIN phase=1 -->
stale
<!-- TASKS:END -->
"#;

const PHASE4_TASKS: &str = r#"
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

[bundles.orders]
phase = 12
order = 2
description = "Order normalization tasks"

[[task]]
id = 74
phase = 12
bundle = "simple"
status = "done"
title = "parseTicker field map"
scores = { d = 4, b = 8, u = 8 }

[[task]]
id = 75
phase = 12
bundle = "orders"
status = "pending"
title = "parseOrder field map"
scores = { d = 5, b = 9, u = 9 }
depends_on = [74]
markers = ["parallel"]
assignee = "codex"
acceptance_criteria = ["parseOrder accepts spot payloads"]
"#;

#[test]
fn validate_command_accepts_valid_tasks_file() {
    let path = write_temp_tasks("valid_tasks.toml", VALID_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap validate");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));
}

#[test]
fn validate_check_render_accepts_current_roadmap() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .current_dir(&dir)
        .output()
        .expect("run rmap render");
    assert!(
        render.status.success(),
        "expected render success, stderr: {}",
        String::from_utf8_lossy(&render.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--check-render")
        .current_dir(&dir)
        .output()
        .expect("run rmap validate --check-render");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));
}

#[test]
fn validate_check_render_tasks_path_derives_conventional_roadmap_path() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap render");
    assert!(
        render.status.success(),
        "expected render success, stderr: {}",
        String::from_utf8_lossy(&render.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--check-render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap validate --check-render");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));
}

#[test]
fn validate_check_render_exits_two_when_roadmap_is_stale() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--check-render")
        .current_dir(&dir)
        .output()
        .expect("run rmap validate --check-render");

    assert_eq!(output.status.code(), Some(2), "expected render drift");
    assert!(String::from_utf8_lossy(&output.stderr).contains("run rmap render"));
}

#[test]
fn validate_check_render_keeps_validation_errors_as_exit_one() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(
        &dir.join("roadmap"),
        "tasks.toml",
        &VALID_TASKS.replace("pending", "shipped"),
    );
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--check-render")
        .current_dir(&dir)
        .output()
        .expect("run rmap validate --check-render");

    assert_eq!(output.status.code(), Some(1), "expected validation error");
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid status \"shipped\""));
}

#[test]
fn validate_command_rejects_invalid_tasks_file() {
    let path = write_temp_tasks(
        "invalid_tasks.toml",
        &VALID_TASKS.replace("pending", "shipped"),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap validate");

    assert!(!output.status.success(), "expected validation failure");
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid status \"shipped\""));
}

#[test]
fn render_command_updates_roadmap_file() {
    let dir = temp_dir();
    let tasks_path = write_file(&dir, "tasks.toml", VALID_TASKS);
    let roadmap_path = write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(dir.join("data.json"))
        .output()
        .expect("run rmap render");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rendered = fs::read_to_string(&roadmap_path).expect("read rendered roadmap");
    assert!(rendered.contains("Keep me."));
    assert!(
        rendered.contains(
            "| Task 1 | ⬜ | 🎁 **foundation** · Add validation [D:2/B:8/U:8 → Eff:4.0?] 🎯 |"
        ),
        "{rendered}"
    );
    assert!(!rendered.contains("stale"));

    let data = fs::read_to_string(dir.join("data.json")).expect("read rendered data json");
    let value: serde_json::Value = serde_json::from_str(&data).expect("valid data json");
    assert_eq!(value["project"], "ccxt_extract");
    assert_eq!(value["task"][0]["eff"], 4.0);
}

#[test]
fn render_command_uses_conventional_paths_by_default() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .current_dir(&dir)
        .output()
        .expect("run rmap render");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rendered = fs::read_to_string(dir.join("ROADMAP.md")).expect("read rendered roadmap");
    assert!(rendered.contains("| Task 1 | ⬜ |"));
    assert!(dir.join("roadmap/data.json").exists());
}

#[test]
fn export_json_command_prints_json_to_stdout() {
    let path = write_temp_tasks("valid_tasks.toml", VALID_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("export")
        .arg("json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap export json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(value["project"], "ccxt_extract");
    assert_eq!(value["task"][0]["eff"], 4.0);
}

#[test]
fn render_stdout_prints_roadmap_without_writing_files() {
    let dir = temp_dir();
    let tasks_path = write_file(&dir, "tasks.toml", VALID_TASKS);
    let roadmap_path = write_file(&dir, "ROADMAP.md", ROADMAP);
    let data_path = dir.join("data.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--stdout")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(&data_path)
        .output()
        .expect("run rmap render --stdout");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(String::from_utf8_lossy(&output.stdout).contains("| Task 1 | ⬜ |"));
    assert_eq!(
        fs::read_to_string(&roadmap_path).expect("read roadmap"),
        ROADMAP
    );
    assert!(!data_path.exists());
}

#[test]
fn render_dry_reports_without_writing_files() {
    let dir = temp_dir();
    let tasks_path = write_file(&dir, "tasks.toml", VALID_TASKS);
    let roadmap_path = write_file(&dir, "ROADMAP.md", ROADMAP);
    let data_path = dir.join("data.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--dry")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(&data_path)
        .output()
        .expect("run rmap render --dry");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(String::from_utf8_lossy(&output.stdout).contains("would render"));
    assert_eq!(
        fs::read_to_string(&roadmap_path).expect("read roadmap"),
        ROADMAP
    );
    assert!(!data_path.exists());
}

#[test]
fn status_command_updates_tasks_and_rerenders_outputs() {
    let dir = temp_dir();
    let tasks_path = write_file(&dir, "tasks.toml", PHASE4_TASKS);
    let roadmap_path = write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );
    let data_path = dir.join("data.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("75")
        .arg("done")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(&data_path)
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(tasks.contains("id = 75\nphase = 12\nbundle = \"orders\"\nstatus = \"done\""));

    let roadmap = fs::read_to_string(&roadmap_path).expect("read updated roadmap");
    assert!(roadmap.contains("| Task 75 `[P]` | ✅ |"));

    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&data_path).expect("read data json"))
            .expect("valid data json");
    assert_eq!(data["task"][1]["status"], "done");
}

#[test]
fn next_command_prints_highest_eff_pending_unblocked_task() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--marker")
        .arg("parallel")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");
    assert!(stdout.contains("Eff:1.8"), "{stdout}");
}

#[test]
fn next_command_prints_nothing_and_exits_zero_when_no_task_matches() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--marker")
        .arg("csr")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}

#[test]
fn next_json_prints_task_object_or_null() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--json")
        .arg("--marker")
        .arg("parallel")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(value["id"], 75);
    assert_eq!(value["eff"], 1.8);

    let no_match = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--json")
        .arg("--marker")
        .arg("csr")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --json with no match");

    assert!(
        no_match.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&no_match.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&no_match.stdout).expect("stdout is valid json");
    assert_eq!(value, serde_json::Value::Null);
}

#[test]
fn show_command_prints_human_and_json_views() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("75")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show");

    assert!(
        human.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&human.stderr)
    );
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");
    assert!(stdout.contains("parseOrder field map"), "{stdout}");
    assert!(stdout.contains("depends_on: 74"), "{stdout}");

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("75")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show --json");

    assert!(
        json.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&json.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    assert_eq!(value["id"], 75);
    assert_eq!(value["title"], "parseOrder field map");
    assert_eq!(value["eff"], 1.8);
    assert_eq!(value["assignee"], "codex");
    assert_eq!(
        value["acceptance_criteria"],
        serde_json::json!(["parseOrder accepts spot payloads"])
    );
}

#[test]
fn show_unknown_task_exits_one() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("999")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show unknown task");

    assert_eq!(output.status.code(), Some(1), "expected unknown task error");
    assert!(String::from_utf8_lossy(&output.stderr).contains("task 999 not found"));
}

#[test]
fn delegate_command_prints_agent_prompt() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("75")
        .arg("--to")
        .arg("claude")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap delegate");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("# Task 75: parseOrder field map"),
        "{stdout}"
    );
    assert!(stdout.contains("Target agent: claude"), "{stdout}");
    assert!(
        stdout.contains("Stored assignee: codex (overridden)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("- Task 74 [done] parseTicker field map"),
        "{stdout}"
    );
    assert!(
        stdout.contains("- [ ] parseOrder accepts spot payloads"),
        "{stdout}"
    );
}

#[test]
fn delegate_unknown_task_exits_one() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("999")
        .arg("--to")
        .arg("codex")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap delegate unknown task");

    assert_eq!(output.status.code(), Some(1), "expected unknown task error");
    assert!(String::from_utf8_lossy(&output.stderr).contains("task 999 not found"));
}

#[test]
fn delegate_rejects_unknown_target() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("75")
        .arg("--to")
        .arg("bad-agent")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap delegate with invalid target");

    assert!(!output.status.success(), "expected clap validation error");
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid value"));
}

#[test]
fn list_command_filters_tasks_and_prints_json_envelope() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("list")
        .arg("--status")
        .arg("pending")
        .arg("--marker")
        .arg("parallel")
        .arg("--phase")
        .arg("12")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap list");

    assert!(
        human.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&human.stderr)
    );
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");
    assert!(!stdout.contains("Task 74"), "{stdout}");

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("list")
        .arg("--status")
        .arg("pending")
        .arg("--marker")
        .arg("parallel")
        .arg("--phase")
        .arg("12")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap list --json");

    assert!(
        json.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&json.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    assert_eq!(value["project"], "ccxt_extract");
    assert_eq!(value["task"].as_array().expect("task array").len(), 1);
    assert_eq!(value["task"][0]["id"], 75);
}

#[test]
fn schema_json_command_emits_parseable_schema_for_tasks_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("schema")
        .output()
        .expect("run rmap schema");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let schema: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(schema["title"], "Tasks");
    assert!(schema["properties"]["task"].is_object());

    let tasks =
        rmap::validate::validate_tasks_str("roadmap/tasks.toml", VALID_TASKS).expect("valid tasks");
    let tasks_json = serde_json::to_value(tasks).expect("tasks serialize to json");
    let compiled = jsonschema::JSONSchema::compile(&schema).expect("schema compiles");

    assert!(
        compiled.is_valid(&tasks_json),
        "emitted schema should validate serialized tasks"
    );
}

#[test]
fn schema_command_rejects_removed_json_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("schema")
        .arg("--json")
        .output()
        .expect("run rmap schema --json");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --json flag is passed to schema"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--json") || stderr.contains("unexpected"),
        "expected clap to mention --json or unexpected argument in stderr, got: {stderr}"
    );
}

#[test]
fn diff_command_reports_added_removed_and_changed_tasks_against_git_ref() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["add", "roadmap/tasks.toml", "ROADMAP.md"]);
    git(
        &dir,
        &[
            "-c",
            "user.email=rmap@example.test",
            "-c",
            "user.name=rmap",
            "commit",
            "-m",
            "initial",
        ],
    );

    let current = PHASE4_TASKS
        .replace(
            "status = \"pending\"\ntitle = \"parseOrder field map\"",
            "status = \"done\"\ntitle = \"parseOrder field map\"",
        )
        .replace("depends_on = [74]\n", "")
        .replace(
            "status = \"pending\"\n\n[bundles.simple]",
            "status = \"in_progress\"\n\n[bundles.simple]",
        )
        .replace(
            r#"
[[task]]
id = 74
phase = 12
bundle = "simple"
status = "done"
title = "parseTicker field map"
scores = { d = 4, b = 8, u = 8 }
"#,
            "",
        )
        + r#"

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 6, b = 8, u = 8 }
"#;
    fs::write(dir.join("roadmap/tasks.toml"), current).expect("write current tasks");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("diff")
        .arg("--against")
        .arg("main")
        .current_dir(&dir)
        .output()
        .expect("run rmap diff");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changed phases.12"), "{stdout}");
    assert!(stdout.contains("removed Task 74"), "{stdout}");
    assert!(stdout.contains("changed Task 75: status"), "{stdout}");
    assert!(stdout.contains("added Task 78b"), "{stdout}");

    let json_output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("diff")
        .arg("--against")
        .arg("main")
        .arg("--json")
        .current_dir(&dir)
        .output()
        .expect("run rmap diff --json");

    assert!(
        json_output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("stdout is valid json");
    let metadata = value["metadata"].as_array().expect("metadata array");
    assert!(
        metadata
            .iter()
            .any(|entry| entry["key"] == "phases.12" && entry["status"] == "changed"),
        "{value}"
    );
    let tasks = value["tasks"].as_array().expect("tasks array");
    assert!(
        tasks
            .iter()
            .any(|entry| entry["id"] == 74 && entry["status"] == "removed"),
        "{value}"
    );
    assert!(
        tasks.iter().any(|entry| entry["id"] == 75
            && entry["status"] == "changed"
            && entry["changed_fields"] == serde_json::json!(["status", "depends_on"])),
        "{value}"
    );
    assert!(
        tasks
            .iter()
            .any(|entry| entry["id"] == "78b" && entry["status"] == "added"),
        "{value}"
    );
}

const MULTI_STATUS_TASKS: &str = r#"
schema_version = 1
project = "multi_test"
default_branch = "main"

[phases.1]
name = "Alpha"
order = 1
status = "pending"

[bundles.alpha]
phase = 1
order = 1
description = "Alpha bundle"

[[task]]
id = 1
phase = 1
bundle = "alpha"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = 2
phase = 1
bundle = "alpha"
status = "pending"
title = "Task Two"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = 3
phase = 1
bundle = "alpha"
status = "pending"
title = "Task Three"
scores = { d = 1, b = 5, u = 5 }
"#;

#[test]
fn status_command_multi_id_flips_all_tasks() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    // Render first so ROADMAP.md is current (avoids re-render error).
    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("1,2,3")
        .arg("done")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap status 1,2,3 done");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    // All three tasks must now be done.
    let done_count = tasks.matches("status = \"done\"").count();
    assert_eq!(done_count, 3, "expected 3 done tasks, got:\n{tasks}");
}

#[test]
fn status_command_unknown_id_aborts_entire_write() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let original = fs::read_to_string(&tasks_path).expect("read original tasks");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("1,99")
        .arg("done")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap status 1,99 done");

    assert!(
        !output.status.success(),
        "expected failure for unknown id 99"
    );

    // File must be byte-equal to the original — no partial write.
    let after = fs::read_to_string(&tasks_path).expect("read tasks after failed run");
    assert_eq!(
        original, after,
        "file must not be modified when any id is unknown"
    );
}

#[test]
fn status_command_single_id_still_works() {
    // Confirm the existing single-ID path is unaffected by the comma-split change.
    // This mirrors the existing status_command_updates_tasks_and_rerenders_outputs test
    // but uses MULTI_STATUS_TASKS for isolation.
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("2")
        .arg("done")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap status 2 done");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    let done_count = tasks.matches("status = \"done\"").count();
    assert_eq!(done_count, 1, "expected exactly 1 done task:\n{tasks}");
    assert!(
        tasks.contains("id = 2\nphase = 1\nbundle = \"alpha\"\nstatus = \"done\""),
        "{tasks}"
    );
}

#[test]
fn mark_command_adds_and_removes_markers_atomically() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    // Task 75 starts with markers = ["parallel"]. Add `cx`, drop `parallel`.
    // (Flags precede the positional `+x`/`-x` ops because clap's
    // `allow_hyphen_values` would otherwise capture `--tasks-path` as a value.)
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("75")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("+cx")
        .arg("-parallel")
        .current_dir(&dir)
        .output()
        .expect("run rmap mark 75 +cx -parallel");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    // toml_edit preserves whitespace from the original `["parallel"]` decor when
    // mutating the array in place, so allow a trailing space inside the brackets.
    let cx_count = tasks.matches("\"cx\"").count();
    let parallel_count = tasks.matches("\"parallel\"").count();
    assert_eq!(cx_count, 1, "expected exactly one cx marker:\n{tasks}");
    assert_eq!(parallel_count, 0, "parallel should be removed:\n{tasks}");
}

#[test]
fn mark_command_is_idempotent_on_repeated_add_or_remove() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    // Adding a marker that already exists, then removing one that's absent.
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("75")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("+parallel")
        .arg("-cx")
        .current_dir(&dir)
        .output()
        .expect("run rmap mark 75 +parallel -cx");

    assert!(
        output.status.success(),
        "idempotent ops should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read tasks");
    let parallel_count = tasks.matches("\"parallel\"").count();
    assert_eq!(parallel_count, 1, "expected one parallel marker:\n{tasks}");
    assert!(!tasks.contains("\"cx\""), "cx must not appear:\n{tasks}");
}

#[test]
fn mark_command_rejects_invalid_marker_name_with_no_partial_write() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let original = fs::read_to_string(&tasks_path).expect("read original");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("75")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("+bogus")
        .current_dir(&dir)
        .output()
        .expect("run rmap mark 75 +bogus");

    assert!(
        !output.status.success(),
        "expected validation failure for invalid marker name"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid marker"),
        "stderr should mention invalid marker, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(original, after, "file must be unchanged after failed mark");
}

#[test]
fn mark_command_rejects_invalid_op_prefix() {
    let path = write_temp_tasks("phase4_mark.toml", PHASE4_TASKS);
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("75")
        .arg("--tasks-path")
        .arg(&path)
        .arg("parallel") // missing + or -
        .output()
        .expect("run rmap mark with bad op prefix");

    assert!(
        !output.status.success(),
        "expected failure for missing +/- prefix"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid marker op"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mark_command_unknown_task_id_aborts_write() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let original = fs::read_to_string(&tasks_path).expect("read original");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("999")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("+cx")
        .current_dir(&dir)
        .output()
        .expect("run rmap mark 999 +cx");

    assert!(!output.status.success(), "unknown id should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown task id 999"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(original, after, "file must be unchanged");
}

#[test]
fn depend_command_adds_in_repo_dependency() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("2")
        .arg("on")
        .arg("1")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 2 on 1");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(
        tasks.contains("depends_on = [1]"),
        "expected depends_on = [1] on task 2:\n{tasks}"
    );

    // Idempotent — second invocation must not duplicate.
    let again = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("2")
        .arg("on")
        .arg("1")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 2 on 1 (repeat)");
    assert!(again.status.success(), "repeat must succeed (idempotent)");
    let tasks2 = fs::read_to_string(&tasks_path).expect("read after repeat");
    assert_eq!(
        tasks2.matches("depends_on").count(),
        1,
        "depends_on should appear once, got:\n{tasks2}"
    );
}

#[test]
fn depend_command_adds_cross_repo_dependency_with_default_relation() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    // Explicit relation.
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("2")
        .arg("--cross-repo")
        .arg("other_repo:42:blocked_by")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 2 --cross-repo other_repo:42:blocked_by");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(tasks.contains("repo = \"other_repo\""), "{tasks}");
    assert!(tasks.contains("relation = \"blocked_by\""), "{tasks}");

    // Default-relation form on a different task.
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("3")
        .arg("--cross-repo")
        .arg("other_repo:99")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 3 --cross-repo other_repo:99");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tasks = fs::read_to_string(&tasks_path).expect("read tasks");
    assert!(
        tasks.contains("relation = \"blocks\""),
        "expected default relation 'blocks':\n{tasks}"
    );
}

#[test]
fn depend_command_rejects_cycle() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    // First: 2 depends on 1. OK.
    let first = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("2")
        .arg("on")
        .arg("1")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 2 on 1");
    assert!(first.status.success());

    let before = fs::read_to_string(&tasks_path).expect("read before cycle attempt");

    // Now: 1 depends on 2 — closes a cycle.
    let cycle = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("1")
        .arg("on")
        .arg("2")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 1 on 2");

    assert!(!cycle.status.success(), "cycle attempt must fail");
    assert!(
        String::from_utf8_lossy(&cycle.stderr).contains("cycle"),
        "stderr should mention cycle, got: {}",
        String::from_utf8_lossy(&cycle.stderr)
    );

    let after = fs::read_to_string(&tasks_path).expect("read after cycle attempt");
    assert_eq!(
        before, after,
        "tasks.toml must be unchanged after failed cycle write"
    );
}

#[test]
fn depend_command_unknown_task_id_aborts_write() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MULTI_STATUS_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("render");

    let original = fs::read_to_string(&tasks_path).expect("read original");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("999")
        .arg("on")
        .arg("1")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap depend 999 on 1");

    assert!(!output.status.success(), "unknown id should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown task id 999"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(original, after, "file must be unchanged");
}

#[test]
fn depend_command_requires_at_least_one_dependency() {
    let path = write_temp_tasks("phase_dep_args.toml", MULTI_STATUS_TASKS);
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("1")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap depend 1 (no targets)");

    assert!(!output.status.success(), "depend with no target must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no dependency specified"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn depend_command_rejects_malformed_cross_repo_spec() {
    let path = write_temp_tasks("phase_dep_bad_cross.toml", MULTI_STATUS_TASKS);
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("1")
        .arg("--cross-repo")
        .arg("missing_colon")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap depend with malformed --cross-repo");

    assert!(!output.status.success(), "malformed --cross-repo must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid cross-repo spec"),
        "stderr should mention invalid cross-repo spec, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn depend_command_rejects_on_without_target() {
    let path = write_temp_tasks("phase_dep_dangling_on.toml", MULTI_STATUS_TASKS);
    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("depend")
        .arg("1")
        .arg("on")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap depend 1 on (no target)");

    assert!(!output.status.success(), "missing target must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing target task id after `on`"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_temp_tasks(file_name: &str, contents: &str) -> PathBuf {
    let dir = temp_dir();
    write_file(&dir, file_name, contents)
}

fn temp_dir() -> PathBuf {
    let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rmap-test-{}-{}-{}",
        std::process::id(),
        unique,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos()
    ));

    fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}

fn write_file(dir: &std::path::Path, file_name: &str, contents: &str) -> PathBuf {
    let path = dir.join(file_name);
    fs::write(&path, contents).expect("write test tasks file");
    path
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// stale command fixtures
// ---------------------------------------------------------------------------

const STALE_TASKS: &str = r#"
schema_version = 1
project = "stale_test"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "Core work"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "in_progress"
title = "Old in-progress task"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-03-01"

[[task]]
id = 2
phase = 1
bundle = "core"
status = "in_progress"
title = "Fresh in-progress task"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-05-05"

[[task]]
id = 3
phase = 1
bundle = "core"
status = "done"
title = "Done task with old date"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-01-01"

[[task]]
id = 4
phase = 1
bundle = "core"
status = "pending"
title = "Pending task no started_at"
scores = { d = 2, b = 4, u = 4 }
"#;

const FRESH_TASKS: &str = r#"
schema_version = 1
project = "fresh_test"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "Core work"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "in_progress"
title = "Fresh task A"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-05-10"

[[task]]
id = 2
phase = 1
bundle = "core"
status = "in_progress"
title = "Fresh task B"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-05-09"
"#;

// ---------------------------------------------------------------------------
// stale command tests
// ---------------------------------------------------------------------------

#[test]
fn stale_command_lists_in_progress_tasks_older_than_threshold() {
    let path = write_temp_tasks("stale_tasks.toml", STALE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("stale")
        .arg("--over")
        .arg("30d")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap stale");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Task 1 is stale (71 days old)
    assert!(
        stdout.contains("Task 1"),
        "expected Task 1 in output:\n{stdout}"
    );
    // Task 2 is fresh (6 days)
    assert!(
        !stdout.contains("Task 2"),
        "Task 2 should not appear:\n{stdout}"
    );
    // Task 3 is done, not in_progress
    assert!(
        !stdout.contains("Task 3"),
        "Task 3 should not appear:\n{stdout}"
    );
    // Task 4 is pending
    assert!(
        !stdout.contains("Task 4"),
        "Task 4 should not appear:\n{stdout}"
    );
}

#[test]
fn stale_command_empty_result_exits_zero() {
    let path = write_temp_tasks("fresh_tasks.toml", FRESH_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("stale")
        .arg("--over")
        .arg("30d")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap stale");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "expected empty stdout, got:\n{stdout}"
    );
}

#[test]
fn stale_command_emits_filtered_json() {
    let path = write_temp_tasks("stale_tasks_json.toml", STALE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("stale")
        .arg("--over")
        .arg("30d")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap stale --json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let task_arr = json["task"].as_array().expect("'task' should be an array");
    assert_eq!(
        task_arr.len(),
        1,
        "expected exactly 1 stale task, got: {task_arr:?}"
    );
    assert_eq!(task_arr[0]["id"], 1, "expected Task 1 in JSON output");
}

#[test]
fn stale_command_rejects_malformed_duration() {
    let path = write_temp_tasks("stale_bad_dur.toml", STALE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("stale")
        .arg("--over")
        .arg("7")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap stale --over 7");

    assert!(
        !output.status.success(),
        "expected non-zero exit for malformed duration"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duration") || stderr.contains("invalid"),
        "expected duration error in stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// doctor command fixtures
// ---------------------------------------------------------------------------

/// All tasks scored within 30d of 2026-05-11 (scored_at = 2026-04-15, 26 days ago).
/// Two bundles in the same phase so neither covers the full phase (avoids the
/// degenerate-bundle doctor lint).
const DOCTOR_CLEAN_TASKS: &str = r#"
schema_version = 1
project = "doctor_test"
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
id = 10
phase = 1
bundle = "core"
status = "pending"
title = "Clean task A"
scores = { d = 2, b = 4, u = 4 }
scored_at = "2026-04-15"

[[task]]
id = 11
phase = 1
bundle = "secondary"
status = "done"
title = "Clean task B"
scores = { d = 1, b = 3, u = 3 }
scored_at = "2026-04-20"
"#;

/// One stale in-progress task (started 2026-01-01, 130d idle at 2026-05-11) and one task
/// with missing scored_at. Two bundles in the same phase so neither covers the full
/// phase, and scores are all D<5 and B<8 so the missing-AC lint doesn't fire either —
/// keeps this fixture targeting just stale + score-decay.
const DOCTOR_DIRTY_TASKS: &str = r#"
schema_version = 1
project = "doctor_dirty"
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
id = 20
phase = 1
bundle = "core"
status = "in_progress"
title = "Stale task"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-01-01"
scored_at = "2026-04-20"

[[task]]
id = 21
phase = 1
bundle = "secondary"
status = "pending"
title = "No score date"
scores = { d = 1, b = 3, u = 3 }
"#;

const DOCTOR_ROADMAP: &str = r#"# Roadmap

<!-- TASKS:BEGIN phase=1 -->
stale
<!-- TASKS:END -->
"#;

// ---------------------------------------------------------------------------
// doctor command tests
// ---------------------------------------------------------------------------

#[test]
fn doctor_command_clean_fixture_reports_no_findings() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", DOCTOR_CLEAN_TASKS);
    write_file(&dir, "ROADMAP.md", DOCTOR_ROADMAP);

    // Sync ROADMAP.md so there is no drift finding.
    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap render");
    assert!(
        render.status.success(),
        "render failed: {}",
        String::from_utf8_lossy(&render.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap doctor");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("all checks passed"),
        "expected clean message, got:\n{stdout}"
    );
}

#[test]
fn doctor_command_surfaces_stale_and_decay() {
    let path = write_temp_tasks("doctor_dirty.toml", DOCTOR_DIRTY_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor dirty");

    assert!(
        output.status.success(),
        "expected exit 0 (informational), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Stale finding for task 20
    assert!(
        stdout.contains("Stale"),
        "expected Stale section:\n{stdout}"
    );
    assert!(
        stdout.contains("task 20"),
        "expected task 20 in stale section:\n{stdout}"
    );
    // Score decay for task 21 (missing scored_at)
    assert!(
        stdout.contains("Score decay"),
        "expected Score decay section:\n{stdout}"
    );
    assert!(
        stdout.contains("task 21"),
        "expected task 21 in decay section:\n{stdout}"
    );
}

#[test]
fn doctor_command_detects_drift() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", DOCTOR_CLEAN_TASKS);
    // Write a stale ROADMAP.md — not rendered yet, so it's out of sync.
    write_file(&dir, "ROADMAP.md", DOCTOR_ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap doctor drift");

    assert!(
        output.status.success(),
        "expected exit 0 even when drift detected, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Drift") || stdout.contains("out of sync"),
        "expected drift finding:\n{stdout}"
    );
}

#[test]
fn doctor_command_emits_json() {
    let path = write_temp_tasks("doctor_dirty_json.toml", DOCTOR_DIRTY_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --json");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");

    assert_eq!(json["ok"], false, "expected ok: false");

    let findings = json["findings"].as_array().expect("findings array");
    let kinds: Vec<&str> = findings.iter().filter_map(|f| f["kind"].as_str()).collect();
    assert!(
        kinds.contains(&"stale"),
        "expected stale finding in JSON:\n{kinds:?}"
    );
    assert!(
        kinds.contains(&"score_decay"),
        "expected score_decay finding in JSON:\n{kinds:?}"
    );
}

#[test]
fn doctor_command_clean_fixture_json_ok() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", DOCTOR_CLEAN_TASKS);
    write_file(&dir, "ROADMAP.md", DOCTOR_ROADMAP);

    // Sync ROADMAP.md.
    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap render");
    assert!(render.status.success(), "render failed");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap doctor --json clean");

    assert!(output.status.success(), "expected exit 0");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], true, "expected ok: true");
    assert_eq!(
        json["findings"].as_array().expect("findings array").len(),
        0,
        "expected empty findings"
    );
}

// ---------------------------------------------------------------------------
// doctor lint fixtures (degenerate_bundle, missing_acceptance_criteria)
// ---------------------------------------------------------------------------

/// Bundle "everything" covers all 3 tasks of phase 1 → degenerate. Task 30 is
/// substantive (d=5) and pending without acceptance_criteria → missing-AC.
/// Task 31 is high-B (b=8) and in_progress without AC → missing-AC. Task 32 is
/// done so it's skipped by the AC lint even though it would otherwise trigger.
const DOCTOR_LINT_TASKS: &str = r#"
schema_version = 1
project = "doctor_lints"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.everything]
phase = 1
order = 1
description = "Covers everything in phase 1"

[[task]]
id = 30
phase = 1
bundle = "everything"
status = "pending"
title = "Substantive D=5 without AC"
scores = { d = 5, b = 4, u = 4 }
scored_at = "2026-04-20"

[[task]]
id = 31
phase = 1
bundle = "everything"
status = "in_progress"
title = "High-B without AC"
scores = { d = 3, b = 8, u = 5 }
started_at = "2026-05-05"
scored_at = "2026-04-20"

[[task]]
id = 32
phase = 1
bundle = "everything"
status = "done"
title = "Substantive but done — should be skipped"
scores = { d = 6, b = 9, u = 7 }
scored_at = "2026-04-20"
done_at = "2026-05-01"
"#;

#[test]
fn doctor_command_surfaces_degenerate_bundle() {
    let path = write_temp_tasks("doctor_lint_degen.toml", DOCTOR_LINT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor lint");

    assert!(
        output.status.success(),
        "expected exit 0 (informational), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Degenerate bundles"),
        "expected Degenerate bundles section:\n{stdout}"
    );
    assert!(
        stdout.contains("\"everything\""),
        "expected bundle name in finding:\n{stdout}"
    );
    assert!(
        stdout.contains("phase 1"),
        "expected phase reference in finding:\n{stdout}"
    );
}

#[test]
fn doctor_command_surfaces_missing_acceptance_criteria() {
    let path = write_temp_tasks("doctor_lint_ac.toml", DOCTOR_LINT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor lint ac");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Missing acceptance_criteria"),
        "expected Missing acceptance_criteria section:\n{stdout}"
    );
    assert!(
        stdout.contains("task 30"),
        "expected task 30 (d=5) in finding:\n{stdout}"
    );
    assert!(
        stdout.contains("task 31"),
        "expected task 31 (b=8) in finding:\n{stdout}"
    );
    // Task 32 is done — should be skipped.
    assert!(
        !stdout.contains("task 32"),
        "expected task 32 (done) to be skipped:\n{stdout}"
    );
}

#[test]
fn doctor_command_lint_findings_in_json() {
    let path = write_temp_tasks("doctor_lint_json.toml", DOCTOR_LINT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --json lint");

    assert!(
        output.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(json["ok"], false, "expected ok: false");

    let findings = json["findings"].as_array().expect("findings array");
    let kinds: Vec<&str> = findings.iter().filter_map(|f| f["kind"].as_str()).collect();

    assert!(
        kinds.contains(&"degenerate_bundle"),
        "expected degenerate_bundle in JSON kinds:\n{kinds:?}"
    );
    assert!(
        kinds.contains(&"missing_acceptance_criteria"),
        "expected missing_acceptance_criteria in JSON kinds:\n{kinds:?}"
    );

    // Verify the missing_acceptance_criteria finding carries the embedded Scores.
    let ac_finding = findings
        .iter()
        .find(|f| f["kind"].as_str() == Some("missing_acceptance_criteria"))
        .expect("missing_acceptance_criteria finding present");
    assert!(
        ac_finding["scores"]["d"].is_number(),
        "expected scores.d field on finding:\n{ac_finding}"
    );
}
