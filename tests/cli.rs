use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

const VALID_TASKS: &str = r#"
schema_version = 2
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

[bundles.orders]
phase = 12
order = 2
description = "Order normalization tasks"

[[task]]
id = 74
phase = 12
bundle = "simple"
status = "done"
implemented = "fixture"
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
fn validate_command_rejects_duplicate_task_ids() {
    let input = r#"
schema_version = 2
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
title = "First"
scores = { d = 1, b = 1, u = 1 }

[[task]]
id = 1
phase = 1
bundle = "foundation"
status = "pending"
title = "Second — same id"
scores = { d = 1, b = 1, u = 1 }
"#;
    let path = write_temp_tasks("duplicate_ids.toml", input);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap validate");

    assert!(
        !output.status.success(),
        "expected non-zero exit on duplicate ids, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate task id"),
        "expected agent-grep substring `duplicate task id`, stderr: {stderr}"
    );
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
fn render_html_writes_self_contained_index() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--html")
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap render --html");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html_path = dir.join("roadmap/dist/index.html");
    let html = fs::read_to_string(&html_path).expect("read rendered html");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("id=\"rmap-data\""));
    assert!(html.contains("data-id="));
    assert!(html.contains("<style"));
    assert!(html.contains("@media print"));
    assert!(html.contains("<svg"));
    // `--html` is additive — ROADMAP.md + data.json are still written.
    assert!(dir.join("ROADMAP.md").exists());
    assert!(dir.join("roadmap/data.json").exists());
}

#[test]
fn render_without_html_flag_does_not_write_index() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .current_dir(&dir)
        .output()
        .expect("run rmap render");

    assert!(output.status.success());
    assert!(!dir.join("roadmap/dist/index.html").exists());
}

#[test]
fn render_html_dry_writes_nothing() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--html")
        .arg("--dry")
        .current_dir(&dir)
        .output()
        .expect("run rmap render --html --dry");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("would write"));
    assert!(stdout.contains("index.html"));
    assert!(!dir.join("roadmap/dist/index.html").exists());
}

#[test]
fn render_html_stdout_prints_html_without_writing() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", VALID_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--html")
        .arg("--stdout")
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap render --html --stdout");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("<!DOCTYPE html>"));
    assert!(!dir.join("roadmap/dist/index.html").exists());
}

#[test]
fn render_html_carries_data_attrs_and_is_idempotent() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);
    let html_path = dir.join("roadmap/dist/index.html");

    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
            .arg("render")
            .arg("--html")
            .current_dir(&dir)
            .env("RMAP_TODAY", "2026-05-14")
            .output()
            .expect("run rmap render --html");
        assert!(
            output.status.success(),
            "expected success, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read_to_string(&html_path).expect("read rendered html")
    };

    let first = run();
    for attr in [
        "data-id=",
        "data-status=",
        "data-eff=",
        "data-markers=",
        "data-depends-on=",
        "data-phase=",
    ] {
        assert!(first.contains(attr), "missing {attr}");
    }
    assert!(first.contains("data-id=\"74\""));
    assert!(first.contains("data-id=\"75\""));

    let second = run();
    assert_eq!(first, second, "render --html should be idempotent");
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
        .arg("--implemented")
        .arg("test-shipped")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(&data_path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(tasks.contains("id = 75\nphase = 12\nbundle = \"orders\"\nstatus = \"done\""));
    assert!(tasks.contains("implemented = \"test-shipped\""));
    assert!(
        tasks.contains("done_at = \"2026-05-14\""),
        "expected auto-filled done_at, got:\n{tasks}"
    );

    let roadmap = fs::read_to_string(&roadmap_path).expect("read updated roadmap");
    assert!(roadmap.contains("| Task 75 `[P]` | ✅ |"));

    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&data_path).expect("read data json"))
            .expect("valid data json");
    assert_eq!(data["task"][1]["status"], "done");
    assert_eq!(data["task"][1]["done_at"], "2026-05-14");
}

#[test]
fn status_in_progress_auto_fills_started_at() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let path = write_file(&dir.join("roadmap"), "tasks.toml", PHASE4_TASKS);
    write_file(
        &dir,
        "ROADMAP.md",
        ROADMAP.replace("phase=1", "phase=12").as_str(),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("75")
        .arg("in_progress")
        .arg("--tasks-path")
        .arg(&path)
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&path).expect("read updated tasks");
    assert!(tasks.contains("status = \"in_progress\""));
    assert!(
        tasks.contains("started_at = \"2026-05-14\""),
        "expected auto-filled started_at, got:\n{tasks}"
    );
    // No done_at written for an in_progress transition.
    assert!(
        !tasks.contains("done_at"),
        "in_progress must not write done_at, got:\n{tasks}"
    );
}

const STATUS_PRESERVE_TASKS: &str = r#"
schema_version = 2
project = "preserve"
default_branch = "main"

[phases.1]
name = "Preserve phase"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "Preservation fixture"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "done"
implemented = "fixture"
title = "already done"
scores = { d = 2, b = 4, u = 5 }
started_at = "2026-05-01"
done_at = "2026-05-02"
"#;

#[test]
fn status_preserves_existing_done_at_on_reflip() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let path = write_file(&dir.join("roadmap"), "tasks.toml", STATUS_PRESERVE_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("1")
        .arg("done")
        .arg("--tasks-path")
        .arg(&path)
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&path).expect("read updated tasks");
    assert!(
        tasks.contains("done_at = \"2026-05-02\""),
        "existing done_at must be preserved, got:\n{tasks}"
    );
    assert!(
        !tasks.contains("2026-05-14"),
        "today's date must not appear when timestamps were already present, got:\n{tasks}"
    );
    assert!(
        tasks.contains("started_at = \"2026-05-01\""),
        "existing started_at must be preserved, got:\n{tasks}"
    );
}

#[test]
fn status_pending_does_not_set_timestamps() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let path = write_file(&dir.join("roadmap"), "tasks.toml", STATUS_PRESERVE_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("1")
        .arg("pending")
        .arg("--tasks-path")
        .arg(&path)
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&path).expect("read updated tasks");
    assert!(tasks.contains("status = \"pending\""));
    // Existing timestamps preserved — re-opening a task keeps audit trail.
    assert!(tasks.contains("done_at = \"2026-05-02\""));
    assert!(tasks.contains("started_at = \"2026-05-01\""));
    assert!(
        !tasks.contains("2026-05-14"),
        "pending transition must not write any new timestamp, got:\n{tasks}"
    );
}

const STATUS_BULK_TASKS: &str = r#"
schema_version = 2
project = "bulk"
default_branch = "main"

[phases.1]
name = "Bulk phase"
order = 1
status = "pending"

[bundles.core]
phase = 1
order = 1
description = "Bulk fixture"

[[task]]
id = 1
phase = 1
bundle = "core"
status = "pending"
title = "first"
scores = { d = 2, b = 4, u = 5 }

[[task]]
id = 2
phase = 1
bundle = "core"
status = "pending"
title = "second"
scores = { d = 2, b = 4, u = 5 }
"#;

#[test]
fn status_bulk_auto_fills_each_task_independently() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let path = write_file(&dir.join("roadmap"), "tasks.toml", STATUS_BULK_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("status")
        .arg("1,2")
        .arg("done")
        .arg("--implemented")
        .arg("bulk-shipped")
        .arg("--tasks-path")
        .arg(&path)
        .current_dir(&dir)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap status");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&path).expect("read updated tasks");
    // Each task gets its own done_at line — count occurrences directly.
    let count = tasks.matches("done_at = \"2026-05-14\"").count();
    assert_eq!(
        count, 2,
        "expected done_at on both bulk-flipped tasks, got:\n{tasks}"
    );
    assert_eq!(tasks.matches("status = \"done\"").count(), 2);
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
    assert!(stdout.contains("- Target: claude"), "{stdout}");
    assert!(
        stdout.contains("- Stored assignee: codex (overridden)"),
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
fn list_command_filters_by_bundle() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("list")
        .arg("--bundle")
        .arg("orders")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap list --bundle");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");
    assert!(!stdout.contains("Task 74"), "{stdout}");
}

#[test]
fn next_command_filters_by_bundle() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--bundle")
        .arg("orders")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --bundle");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");
}

#[test]
fn next_command_composes_bundle_and_marker() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let matched = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--bundle")
        .arg("orders")
        .arg("--marker")
        .arg("parallel")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --bundle orders --marker parallel");

    assert!(
        matched.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&matched.stderr)
    );
    let stdout = String::from_utf8_lossy(&matched.stdout);
    assert!(stdout.contains("Task 75"), "{stdout}");

    // simple bundle has no parallel-marked pending task — composition should yield nothing.
    let no_match = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--bundle")
        .arg("simple")
        .arg("--marker")
        .arg("parallel")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --bundle simple --marker parallel");

    assert!(
        no_match.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&no_match.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&no_match.stdout), "");
}

#[test]
fn next_command_unknown_bundle_returns_null_json() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--bundle")
        .arg("nonexistent")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --bundle nonexistent --json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(value, serde_json::Value::Null);
}

const NEXT_COUNT_TASKS: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "Phase 1"
order = 1
status = "in_progress"

[bundles.alpha]
phase = 1
order = 1
description = "Alpha"

[[task]]
id = 1
phase = 1
bundle = "alpha"
status = "pending"
title = "Low Eff"
scores = { d = 5, b = 5, u = 5 }

[[task]]
id = 2
phase = 1
bundle = "alpha"
status = "pending"
title = "High Eff"
scores = { d = 2, b = 10, u = 10 }

[[task]]
id = 3
phase = 1
bundle = "alpha"
status = "pending"
title = "Mid Eff"
scores = { d = 3, b = 8, u = 8 }
"#;

#[test]
fn next_command_count_one_default_emits_bare_object_json() {
    let path = write_temp_tasks("next_count.toml", NEXT_COUNT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --json");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim_start();
    assert!(
        trimmed.starts_with('{'),
        "expected bare object, got: {stdout}"
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value["id"], 2);
}

#[test]
fn next_command_count_three_json_emits_eff_ranked_array() {
    let path = write_temp_tasks("next_count.toml", NEXT_COUNT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--count")
        .arg("3")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --count 3 --json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let array = value.as_array().expect("expected array");
    assert_eq!(array.len(), 3);
    assert_eq!(array[0]["id"], 2);
    assert_eq!(array[1]["id"], 3);
    assert_eq!(array[2]["id"], 1);
}

#[test]
fn next_command_count_exceeds_eligible_returns_min_without_error() {
    let path = write_temp_tasks("next_count.toml", NEXT_COUNT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--count")
        .arg("100")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --count 100 --json");

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(value.as_array().expect("array").len(), 3);
}

#[test]
fn next_command_count_three_human_prints_one_line_per_task() {
    let path = write_temp_tasks("next_count.toml", NEXT_COUNT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--count")
        .arg("3")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --count 3");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim_end().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("Task 2 ["), "{}", lines[0]);
    assert!(lines[1].starts_with("Task 3 ["), "{}", lines[1]);
    assert!(lines[2].starts_with("Task 1 ["), "{}", lines[2]);
}

#[test]
fn next_command_count_zero_is_rejected_at_parse_time() {
    let path = write_temp_tasks("next_count.toml", NEXT_COUNT_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--count")
        .arg("0")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --count 0");

    assert!(!output.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("count") || stderr.to_lowercase().contains("usage"),
        "expected clap error mentioning count or usage, got: {stderr}"
    );
}

#[test]
fn list_command_unknown_bundle_returns_empty_envelope() {
    let path = write_temp_tasks("phase4_tasks.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("list")
        .arg("--bundle")
        .arg("nonexistent")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap list --bundle nonexistent --json");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(value["project"], "ccxt_extract");
    assert_eq!(value["task"].as_array().expect("task array").len(), 0);
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
            "status = \"done\"\nimplemented = \"diff-test-shipped\"\ntitle = \"parseOrder field map\"",
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
implemented = "fixture"
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
            && entry["changed_fields"]
                == serde_json::json!(["status", "depends_on", "implemented"])),
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
schema_version = 2
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
        .arg("--implemented")
        .arg("bulk-multi-shipped")
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
        .arg("--implemented")
        .arg("single-shipped")
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
fn mark_command_places_new_markers_field_in_canonical_position() {
    // Task 74 in PHASE4_TASKS lacks `markers`. Adding one should land between
    // `scores` and any later fields, mirroring `add_task_str`'s canonical
    // write order rather than appending at the end of the table.
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

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("mark")
        .arg("74")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("+parallel")
        .current_dir(&dir)
        .output()
        .expect("run rmap mark 74 +parallel");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    // Task 74's block should now have markers right after scores. The next
    // task (75) starts with `id = 75`, so finding the substring works.
    let task_74_block_start = tasks.find("id = 74").expect("task 74 id present");
    let task_75_block_start = tasks.find("id = 75").expect("task 75 id present");
    let task_74_block = &tasks[task_74_block_start..task_75_block_start];

    let scores_idx = task_74_block
        .find("scores = {")
        .expect("scores present in task 74");
    let markers_idx = task_74_block
        .find("markers = ")
        .expect("markers present in task 74");
    assert!(
        scores_idx < markers_idx,
        "markers should land after scores in task 74's block:\n{task_74_block}"
    );
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

// Regression: `rmap depend <src> on <numeric-string-id>` used to write the
// target as an integer, which the validator can't reconcile against the
// string-keyed task and rejects with "unknown task". See Task 23.
#[test]
fn depend_command_handles_numeric_only_string_target_id() {
    const STRING_ID_TASKS: &str = r#"
schema_version = 2
project = "string_id_test"
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
id = "1"
phase = 1
bundle = "alpha"
status = "pending"
title = "Task One"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = "2"
phase = 1
bundle = "alpha"
status = "pending"
title = "Task Two"
scores = { d = 1, b = 5, u = 5 }

[[task]]
id = "2b"
phase = 1
bundle = "alpha"
status = "pending"
title = "Task Two B"
scores = { d = 1, b = 5, u = 5 }
"#;

    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", STRING_ID_TASKS);
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
        tasks.contains(r#"depends_on = ["1"]"#),
        "expected depends_on = [\"1\"] (string form matching target's id shape):\n{tasks}"
    );

    let validate = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap validate");
    assert!(
        validate.status.success(),
        "validate must pass after edit, stderr: {}",
        String::from_utf8_lossy(&validate.stderr)
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

// ---------------------------------------------------------------------------
// new --from-stdin tests
// ---------------------------------------------------------------------------

/// Materialize a project directory shaped like `paths::resolve` expects:
/// `<root>/roadmap/tasks.toml` + `<root>/ROADMAP.md`. Returns
/// `(root, tasks_path, roadmap_path, data_path)`.
fn write_new_stdin_fixture(tasks: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", tasks);
    let roadmap_path = write_file(&dir, "ROADMAP.md", ROADMAP);
    let data_path = dir.join("roadmap/data.json");

    // Pre-render so ROADMAP.md + data.json reflect the input tasks; create_task
    // re-renders, but it expects a roadmap with TASKS markers in place.
    let prep = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .env("RMAP_TODAY", "2026-05-12")
        .current_dir(&dir)
        .output()
        .expect("run rmap render for new fixture prep");
    assert!(
        prep.status.success(),
        "fixture prep render failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&prep.stdout),
        String::from_utf8_lossy(&prep.stderr)
    );

    (dir, tasks_path, roadmap_path, data_path)
}

/// Drive `rmap new --from-stdin` with the given TOML payload piped on stdin.
fn run_new_from_stdin(
    tasks_path: &std::path::Path,
    roadmap_path: &std::path::Path,
    data_path: &std::path::Path,
    payload: &str,
    today: &str,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rmap"));
    command
        .arg("new")
        .arg("--from-stdin")
        .arg("--tasks-path")
        .arg(tasks_path)
        .arg("--roadmap-path")
        .arg(roadmap_path)
        .arg("--data-path")
        .arg(data_path)
        .env("RMAP_TODAY", today)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("spawn rmap new --from-stdin");
    {
        let mut child_stdin = child.stdin.take().expect("piped stdin");
        child_stdin
            .write_all(payload.as_bytes())
            .expect("write stdin payload");
    }
    child.wait_with_output().expect("collect rmap new output")
}

const NEW_STDIN_TASKS: &str = r#"
schema_version = 2
project = "new_stdin_test"
default_branch = "main"

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
title = "Existing task"
scores = { d = 2, b = 6, u = 6 }
created_at = "2026-05-12"
scored_at = "2026-05-12"
"#;

#[test]
fn new_from_stdin_appends_task_and_rerenders() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Stdin-authored task"
scores = { d = 3, b = 7, u = 7 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("created task 2"), "stdout: {stdout}");

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(tasks.contains("Stdin-authored task"), "tasks: {tasks}");
    assert!(
        tasks.contains("id = 2"),
        "expected auto-allocated id = 2; tasks: {tasks}"
    );

    let roadmap = fs::read_to_string(&roadmap_path).expect("read updated roadmap");
    assert!(
        roadmap.contains("Stdin-authored task"),
        "roadmap: {roadmap}"
    );

    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&data_path).expect("read data json"))
            .expect("valid data json");
    let new_row = data["task"]
        .as_array()
        .expect("task array")
        .iter()
        .find(|row| row["id"] == 2)
        .expect("new row in data.json");
    assert_eq!(new_row["title"], "Stdin-authored task");
    assert!(new_row["eff"].is_number(), "eff computed: {new_row}");
}

#[test]
fn new_from_stdin_round_trips_model_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Model-pinned task"
scores = { d = 2, b = 6, u = 6 }
module = "src/foo.rs"
model = "claude-opus-4-7"
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Round-trips into tasks.toml, written immediately after `module` (canonical
    // key order — `add_task_str` write order mirrors `canonical_task_key_index`).
    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    let module_idx = tasks.find("module = ").expect("module present");
    let model_idx = tasks
        .find(r#"model = "claude-opus-4-7""#)
        .expect("model present");
    assert!(
        module_idx < model_idx,
        "model should follow module in canonical order; tasks:\n{tasks}"
    );

    // Surfaces in data.json.
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&data_path).expect("read data json"))
            .expect("valid data json");
    let new_row = data["task"]
        .as_array()
        .expect("task array")
        .iter()
        .find(|row| row["id"] == 2)
        .expect("new row in data.json");
    assert_eq!(new_row["model"], "claude-opus-4-7");

    // Surfaces in `rmap show` (human + --json).
    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show");
    assert!(human.status.success());
    assert!(
        String::from_utf8_lossy(&human.stdout).contains("model: claude-opus-4-7"),
        "human show stdout: {}",
        String::from_utf8_lossy(&human.stdout)
    );

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    assert!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    assert_eq!(value["model"], "claude-opus-4-7");
}

#[test]
fn new_from_stdin_round_trips_out_of_scope_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Scoped task"
scores = { d = 2, b = 6, u = 6 }
acceptance_criteria = ["Does the thing"]
out_of_scope = ["Does not touch the unrelated thing", "No DB migration"]
body = "Some body prose."
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 1. Round-trips into tasks.toml after acceptance_criteria (canonical order).
    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    let ac_idx = tasks
        .find("acceptance_criteria = ")
        .expect("acceptance_criteria present");
    let oos_idx = tasks.find("out_of_scope = ").expect("out_of_scope present");
    assert!(
        ac_idx < oos_idx,
        "out_of_scope should follow acceptance_criteria in canonical order; tasks:\n{tasks}"
    );

    // 2. Re-render + re-read: an explicit second render leaves the field intact
    // (create_task already re-rendered once; this round-trips again to assert
    // the field survives idempotent render passes).
    let rerun = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .arg("--roadmap-path")
        .arg(&roadmap_path)
        .arg("--data-path")
        .arg(&data_path)
        .env("RMAP_TODAY", "2026-05-12")
        .output()
        .expect("run rmap render");
    assert!(
        rerun.status.success(),
        "second render failed; stderr: {}",
        String::from_utf8_lossy(&rerun.stderr)
    );
    let tasks_after = fs::read_to_string(&tasks_path).expect("read tasks after re-render");
    assert!(
        tasks_after.contains("out_of_scope = "),
        "out_of_scope dropped on re-render; tasks_after: {tasks_after}"
    );

    // 3. `rmap show <id>` renders an `out_of_scope:` section.
    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show");
    assert!(human.status.success());
    let human_stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        human_stdout.contains("out_of_scope:"),
        "human show missing out_of_scope section; stdout: {human_stdout}"
    );
    assert!(
        human_stdout.contains("- Does not touch the unrelated thing"),
        "human show missing oos bullet; stdout: {human_stdout}"
    );

    // 4. `rmap show <id> --json` emits `out_of_scope` as an array.
    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    assert!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    let oos = value["out_of_scope"]
        .as_array()
        .expect("out_of_scope is an array");
    assert_eq!(oos.len(), 2);
    assert_eq!(oos[0], "Does not touch the unrelated thing");
    assert_eq!(oos[1], "No DB migration");

    // 5. `rmap delegate <id>` renders an `## Out of scope` section.
    let delegate = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("2")
        .arg("--to")
        .arg("claude")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap delegate");
    assert!(
        delegate.status.success(),
        "delegate failed; stderr: {}",
        String::from_utf8_lossy(&delegate.stderr)
    );
    let delegate_stdout = String::from_utf8_lossy(&delegate.stdout);
    assert!(
        delegate_stdout.contains("## Out of scope"),
        "delegate missing `## Out of scope`; stdout: {delegate_stdout}"
    );
    assert!(
        delegate_stdout.contains("- Does not touch the unrelated thing"),
        "delegate missing oos bullet; stdout: {delegate_stdout}"
    );
}

#[test]
fn new_from_stdin_omits_out_of_scope_section_when_unset() {
    // Mirror of `new_from_stdin_appends_task_and_rerenders` for the empty case:
    // `out_of_scope` must not appear in tasks.toml, `rmap show` human output,
    // delegate prompt, or `rmap show --json` (skip_serializing_if mirrors
    // `acceptance_criteria`).
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "No-oos task"
scores = { d = 2, b = 5, u = 5 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(output.status.success());

    let tasks = fs::read_to_string(&tasks_path).expect("read tasks");
    assert!(
        !tasks.contains("out_of_scope"),
        "out_of_scope must not be emitted when unset; tasks: {tasks}"
    );

    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show");
    assert!(!String::from_utf8_lossy(&human.stdout).contains("out_of_scope:"));

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("valid json");
    assert!(
        value.get("out_of_scope").is_none(),
        "out_of_scope must be skip-serialized when empty; json: {value}"
    );

    let delegate = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("2")
        .arg("--to")
        .arg("claude")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap delegate");
    assert!(!String::from_utf8_lossy(&delegate.stdout).contains("## Out of scope"));

    // Silence unused-variable warnings for the helper return tuple.
    let _ = roadmap_path;
    let _ = data_path;
}

#[test]
fn new_from_stdin_auto_allocates_id_when_omitted() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "First"
scores = { d = 1, b = 4, u = 4 }

[[task]]
phase = 1
bundle = "foundation"
title = "Second"
scores = { d = 1, b = 4, u = 4 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("created task 2, 3"),
        "expected sequential ids 2, 3; stdout: {stdout}"
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(tasks.contains("id = 2"));
    assert!(tasks.contains("id = 3"));
}

#[test]
fn new_from_stdin_multi_task_atomic_on_failure() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);
    let before = fs::read_to_string(&tasks_path).expect("read before");

    // Second task references an unknown phase — entire batch must reject.
    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "First (valid)"
scores = { d = 1, b = 4, u = 4 }

[[task]]
phase = 99
bundle = "foundation"
title = "Second (invalid phase)"
scores = { d = 1, b = 4, u = 4 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        !output.status.success(),
        "expected failure on unknown phase"
    );

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(
        before, after,
        "tasks.toml must be byte-equal after rejected batch"
    );
    assert!(
        !after.contains("First (valid)"),
        "valid first task must not leak through"
    );
}

#[test]
fn new_from_stdin_rejects_status_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);
    let before = fs::read_to_string(&tasks_path).expect("read before");

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Tries to create a done task"
status = "done"
implemented = "fixture"
scores = { d = 2, b = 5, u = 5 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        !output.status.success(),
        "stdin status field must be rejected (creation produces pending tasks only)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown field") && stderr.contains("status"),
        "stderr should mention unknown `status` field; got: {stderr}"
    );

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(
        before, after,
        "tasks.toml must be byte-equal on rejected stdin payload"
    );
}

#[test]
fn new_from_stdin_rejects_duplicate_id() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);
    let before = fs::read_to_string(&tasks_path).expect("read before");

    let payload = r#"
[[task]]
id = 1
phase = 1
bundle = "foundation"
title = "Collides with existing id 1"
scores = { d = 2, b = 5, u = 5 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(!output.status.success(), "duplicate id must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate task id"),
        "stderr should mention duplicate; got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(before, after, "tasks.toml unchanged on duplicate-id reject");
}

#[test]
fn new_from_stdin_rejects_unknown_bundle() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);
    let before = fs::read_to_string(&tasks_path).expect("read before");

    let payload = r#"
[[task]]
phase = 1
bundle = "nonexistent"
title = "References unknown bundle"
scores = { d = 2, b = 5, u = 5 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(!output.status.success(), "unknown bundle must fail");

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(
        before, after,
        "tasks.toml unchanged on unknown-bundle reject"
    );
}

#[test]
fn new_from_stdin_inherits_validation_for_cycles() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);
    let before = fs::read_to_string(&tasks_path).expect("read before");

    // Self-dependency forms a 1-cycle the validator must detect.
    let payload = r#"
[[task]]
id = 99
phase = 1
bundle = "foundation"
title = "Self-cycle task"
scores = { d = 2, b = 5, u = 5 }
depends_on = [99]
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(!output.status.success(), "cycle must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cycle"),
        "stderr should mention cycle; got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = fs::read_to_string(&tasks_path).expect("read after");
    assert_eq!(before, after, "tasks.toml unchanged on cycle reject");
}

#[test]
fn new_from_stdin_pinned_today() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Today-stamped task"
scores = { d = 2, b = 6, u = 6 }
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(
        tasks.contains("created_at = \"2026-05-12\""),
        "created_at should default to RMAP_TODAY; tasks: {tasks}"
    );
    assert!(
        tasks.contains("scored_at = \"2026-05-12\""),
        "scored_at should default to RMAP_TODAY; tasks: {tasks}"
    );
}

#[test]
fn new_from_stdin_round_trips_branch_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Branch-tagged task"
scores = { d = 2, b = 6, u = 6 }
branch = "feat/task-25-smoke"
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(
        tasks.contains("branch = \"feat/task-25-smoke\""),
        "branch should round-trip into tasks.toml; tasks: {tasks}"
    );

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    assert!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    assert_eq!(
        value["branch"].as_str(),
        Some("feat/task-25-smoke"),
        "show --json should surface branch; value: {value}"
    );
}

#[test]
fn new_from_stdin_round_trips_files_to_modify_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "File-scoped task"
scores = { d = 2, b = 6, u = 6 }
files_to_modify = ["src/foo.rs", "tests/foo.rs"]
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(
        tasks.contains("files_to_modify = [\"src/foo.rs\", \"tests/foo.rs\"]"),
        "files_to_modify should round-trip into tasks.toml; tasks: {tasks}"
    );

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    assert!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    let files = value["files_to_modify"]
        .as_array()
        .expect("files_to_modify is an array");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0], "src/foo.rs");
    assert_eq!(files[1], "tests/foo.rs");

    // `rmap delegate` already renders `## Files to modify` from this field
    // (see src/delegate.rs::append_files_to_modify) — exercise the read path
    // end-to-end so a regression in either creation or rendering surfaces.
    let delegate = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("delegate")
        .arg("2")
        .arg("--to")
        .arg("claude")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap delegate");
    assert!(
        delegate.status.success(),
        "delegate failed; stderr: {}",
        String::from_utf8_lossy(&delegate.stderr)
    );
    let delegate_stdout = String::from_utf8_lossy(&delegate.stdout);
    assert!(
        delegate_stdout.contains("## Files to modify"),
        "delegate missing `## Files to modify`; stdout: {delegate_stdout}"
    );
    assert!(
        delegate_stdout.contains("src/foo.rs"),
        "delegate missing file bullet; stdout: {delegate_stdout}"
    );
}

#[test]
fn new_from_stdin_round_trips_cross_repo_field() {
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Cross-repo task"
scores = { d = 2, b = 6, u = 6 }
cross_repo = [
  { repo = "other-repo", task_id = 42, relation = "blocks" },
  { repo = "third-repo", task_id = "alpha", linear_id = "ABC-1", relation = "blocked_by" },
]
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    assert!(
        tasks.contains("cross_repo = ["),
        "cross_repo array should round-trip into tasks.toml; tasks: {tasks}"
    );
    // Integer task_id stays integer, text task_id stays string (mirrors
    // add_dependency_str's shape).
    assert!(
        tasks.contains("task_id = 42"),
        "integer task_id should serialize as integer; tasks: {tasks}"
    );
    assert!(
        tasks.contains("task_id = \"alpha\""),
        "text task_id should serialize as string; tasks: {tasks}"
    );
    assert!(
        tasks.contains("linear_id = \"ABC-1\""),
        "optional linear_id should be preserved; tasks: {tasks}"
    );

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .output()
        .expect("run rmap show --json");
    assert!(json.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    let entries = value["cross_repo"]
        .as_array()
        .expect("cross_repo is an array");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["repo"], "other-repo");
    assert_eq!(entries[0]["task_id"], 42);
    assert_eq!(entries[0]["relation"], "blocks");
    assert_eq!(entries[1]["repo"], "third-repo");
    assert_eq!(entries[1]["task_id"], "alpha");
    assert_eq!(entries[1]["linear_id"], "ABC-1");
    assert_eq!(entries[1]["relation"], "blocked_by");
}

#[test]
fn new_from_stdin_emits_canonical_order_for_creation_fields() {
    // Asserts the writer in add_task_str + canonical_task_key_index produce
    // the expected key ordering when all three Task-25 fields are present:
    //   out_of_scope (10) < files_to_modify (11) < cross_repo (12) < model (16) < branch (17)
    let (_dir, tasks_path, roadmap_path, data_path) = write_new_stdin_fixture(NEW_STDIN_TASKS);

    let payload = r#"
[[task]]
phase = 1
bundle = "foundation"
title = "Full-shape task"
scores = { d = 2, b = 6, u = 6 }
out_of_scope = ["No DB migration"]
files_to_modify = ["src/foo.rs"]
cross_repo = [{ repo = "other-repo", task_id = 1, relation = "blocks" }]
model = "claude-opus-4-7"
branch = "feat/task-25"
"#;

    let output = run_new_from_stdin(
        &tasks_path,
        &roadmap_path,
        &data_path,
        payload,
        "2026-05-12",
    );
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tasks = fs::read_to_string(&tasks_path).expect("read updated tasks");
    // Anchor on `\n` to avoid matching `default_branch = ` at the document head.
    let oos_idx = tasks
        .find("\nout_of_scope = ")
        .expect("out_of_scope present");
    let ftm_idx = tasks
        .find("\nfiles_to_modify = ")
        .expect("files_to_modify present");
    let xrepo_idx = tasks.find("\ncross_repo = ").expect("cross_repo present");
    let model_idx = tasks.find("\nmodel = ").expect("model present");
    let branch_idx = tasks.find("\nbranch = ").expect("branch present");

    assert!(
        oos_idx < ftm_idx,
        "files_to_modify should follow out_of_scope; tasks:\n{tasks}"
    );
    assert!(
        ftm_idx < xrepo_idx,
        "cross_repo should follow files_to_modify; tasks:\n{tasks}"
    );
    assert!(
        xrepo_idx < model_idx,
        "model should follow cross_repo; tasks:\n{tasks}"
    );
    assert!(
        model_idx < branch_idx,
        "branch should follow model; tasks:\n{tasks}"
    );
}

const NEXT_BUNDLE_FOCUS_VS_OTHER: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[focus]
phase = 1

[phases.1]
name = "Focus"
order = 1
status = "in_progress"

[phases.2]
name = "Other"
order = 2
status = "pending"

[bundles.focus_b]
phase = 1
order = 1
description = "focus bundle"

[bundles.other_b]
phase = 2
order = 1
description = "other bundle"

[[task]]
id = 1
phase = 1
bundle = "focus_b"
status = "pending"
title = "low-eff focus"
scores = { d = 5, b = 5, u = 5 }

[[task]]
id = 2
phase = 2
bundle = "other_b"
status = "pending"
title = "high-eff other"
scores = { d = 2, b = 10, u = 10 }
"#;

#[test]
fn next_bundle_focus_phase_wins_over_higher_sum_eff_other_phase() {
    let path = write_temp_tasks("nb_focus.toml", NEXT_BUNDLE_FOCUS_VS_OTHER);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("bundle focus_b  phase 1 — Focus"),
        "focus_b should win over other_b; stdout: {stdout}"
    );
    assert!(
        stdout.contains("Task 1"),
        "task 1 should appear; stdout: {stdout}"
    );
    assert!(
        !stdout.contains("Task 2"),
        "task 2 (other phase) should not appear; stdout: {stdout}"
    );
}

const NEXT_BUNDLE_TIE_BY_ORDER: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.alpha]
phase = 1
order = 2
description = "second"

[bundles.beta]
phase = 1
order = 1
description = "first"

[[task]]
id = 1
phase = 1
bundle = "alpha"
status = "pending"
title = "a"
scores = { d = 3, b = 6, u = 6 }

[[task]]
id = 2
phase = 1
bundle = "beta"
status = "pending"
title = "b"
scores = { d = 3, b = 6, u = 6 }
"#;

#[test]
fn next_bundle_tie_broken_by_bundle_order() {
    let path = write_temp_tasks("nb_tie.toml", NEXT_BUNDLE_TIE_BY_ORDER);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("bundle beta  phase 1 — P"),
        "beta (order=1) wins over alpha (order=2); stdout: {stdout}"
    );
}

const NEXT_BUNDLE_ALL_BLOCKED: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.blocked_b]
phase = 1
order = 1
description = "all blocked"

[bundles.healthy]
phase = 1
order = 2
description = "has work"

[[task]]
id = 1
phase = 1
bundle = "blocked_b"
status = "blocked"
title = "wait"
scores = { d = 3, b = 3, u = 3 }
blocked_reason = "external"

[[task]]
id = 2
phase = 1
bundle = "healthy"
status = "pending"
title = "go"
scores = { d = 3, b = 3, u = 3 }
"#;

#[test]
fn next_bundle_all_blocked_bundle_is_skipped() {
    let path = write_temp_tasks("nb_blocked.toml", NEXT_BUNDLE_ALL_BLOCKED);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("bundle healthy"),
        "healthy bundle picked over all-blocked; stdout: {stdout}"
    );
}

const NEXT_BUNDLE_UNMET_DEP: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.gated]
phase = 1
order = 1
description = "external dep"

[bundles.open]
phase = 1
order = 2
description = "no deps"

[[task]]
id = 1
phase = 1
bundle = "open"
status = "pending"
title = "external pending"
scores = { d = 3, b = 3, u = 3 }

[[task]]
id = 2
phase = 1
bundle = "gated"
status = "pending"
title = "gated by 1"
scores = { d = 3, b = 9, u = 9 }
depends_on = [1]
"#;

#[test]
fn next_bundle_unmet_external_dep_skips_bundle() {
    let path = write_temp_tasks("nb_unmet.toml", NEXT_BUNDLE_UNMET_DEP);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("bundle open"),
        "open bundle picked (gated has unmet external dep); stdout: {stdout}"
    );
}

const NEXT_BUNDLE_FORCE_PICK: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.high]
phase = 1
order = 1
description = "higher Eff"

[bundles.low]
phase = 1
order = 2
description = "lower Eff"

[[task]]
id = 1
phase = 1
bundle = "high"
status = "pending"
title = "h"
scores = { d = 2, b = 10, u = 10 }

[[task]]
id = 2
phase = 1
bundle = "low"
status = "pending"
title = "l"
scores = { d = 5, b = 5, u = 5 }
"#;

#[test]
fn next_bundle_force_pick_bypasses_ranking() {
    let path = write_temp_tasks("nb_force.toml", NEXT_BUNDLE_FORCE_PICK);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--bundle")
        .arg("low")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --bundle low");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("bundle low"),
        "--bundle low force-picks lower-eff bundle; stdout: {stdout}"
    );
    assert!(
        stdout.contains("Task 2"),
        "task 2 in body; stdout: {stdout}"
    );
}

#[test]
fn next_bundle_json_envelope_shape_is_stable() {
    let path = write_temp_tasks("nb_json.toml", NEXT_BUNDLE_FOCUS_VS_OTHER);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --json");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");

    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["focus_phase"], 1);
    assert_eq!(value["bundle"]["name"], "focus_b");
    assert_eq!(value["bundle"]["phase"], 1);
    assert_eq!(value["bundle"]["description"], "focus bundle");

    let tasks = value["tasks"].as_array().expect("tasks is array");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["id"], 1);
    assert!(
        tasks[0]["eff"].is_number(),
        "tasks[0].eff must be numeric; got {:?}",
        tasks[0]["eff"]
    );
    assert_eq!(tasks[0]["status"], "pending");
}

const NEXT_BUNDLE_TOPO_CHAIN: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.chain]
phase = 1
order = 1
description = "internal chain"

[[task]]
id = 1
phase = 1
bundle = "chain"
status = "pending"
title = "first"
scores = { d = 5, b = 5, u = 5 }

[[task]]
id = 2
phase = 1
bundle = "chain"
status = "pending"
title = "second"
scores = { d = 2, b = 10, u = 10 }
depends_on = [1]
"#;

#[test]
fn next_bundle_emits_topological_order_within_bundle() {
    let path = write_temp_tasks("nb_topo.toml", NEXT_BUNDLE_TOPO_CHAIN);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --json");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    let tasks = value["tasks"].as_array().expect("tasks is array");
    assert_eq!(tasks.len(), 2);
    // Task 2 has higher Eff but depends on 1, so 1 emits first.
    assert_eq!(tasks[0]["id"], 1);
    assert_eq!(tasks[1]["id"], 2);
}

#[test]
fn next_bundle_phase_override_changes_effective_focus() {
    let path = write_temp_tasks("nb_phase.toml", NEXT_BUNDLE_FOCUS_VS_OTHER);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--phase")
        .arg("2")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --phase 2 --json");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(
        value["focus_phase"], 2,
        "--phase 2 overrides stored focus.phase=1 in envelope"
    );
    assert_eq!(value["bundle"]["name"], "other_b");
}

const NEXT_BUNDLE_NO_ACTIONABLE: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "P"
order = 1
status = "in_progress"

[bundles.idle]
phase = 1
order = 1
description = "no work"

[[task]]
id = 1
phase = 1
bundle = "idle"
status = "done"
implemented = "fixture"
title = "done one"
scores = { d = 3, b = 3, u = 3 }
started_at = "2026-05-01"
done_at = "2026-05-02"
"#;

#[test]
fn next_bundle_empty_pick_goes_to_stderr_with_exit_zero() {
    let path = write_temp_tasks("nb_empty.toml", NEXT_BUNDLE_NO_ACTIONABLE);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle");

    assert!(output.status.success(), "empty pick still exits 0");
    assert!(
        output.stdout.is_empty(),
        "stdout must be empty; got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("none — no actionable bundle in any phase"),
        "stderr message; got: {stderr}"
    );
}

#[test]
fn next_bundle_force_pick_zero_actionable_uses_bundle_specific_stderr() {
    let path = write_temp_tasks("nb_force_empty.toml", NEXT_BUNDLE_NO_ACTIONABLE);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--bundle")
        .arg("idle")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --bundle idle");

    assert!(output.status.success(), "zero-actionable still exits 0");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("none — bundle 'idle' has no actionable pending tasks"),
        "bundle-specific empty message; got: {stderr}"
    );
}

#[test]
fn next_bundle_missing_bundle_errors_with_exit_one() {
    let path = write_temp_tasks("nb_missing.toml", NEXT_BUNDLE_FORCE_PICK);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next-bundle")
        .arg("--bundle")
        .arg("ghost")
        .arg("--tasks-path")
        .arg(&path)
        .env("RMAP_TODAY", "2026-05-14")
        .output()
        .expect("run rmap next-bundle --bundle ghost");

    assert!(!output.status.success(), "missing bundle is a hard error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("bundle 'ghost' is not declared in tasks.toml"),
        "missing-bundle message; got: {stderr}"
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
schema_version = 2
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
implemented = "fixture"
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
schema_version = 2
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
schema_version = 2
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
implemented = "fixture"
title = "Clean task B"
scores = { d = 1, b = 3, u = 3 }
scored_at = "2026-04-20"
"#;

/// One stale in-progress task (started 2026-01-01, 130d idle at 2026-05-11) and one task
/// with missing scored_at. Two bundles in the same phase so neither covers the full
/// phase, and scores are all D<5 and B<8 so the missing-AC lint doesn't fire either —
/// keeps this fixture targeting just stale + score-decay.
const DOCTOR_DIRTY_TASKS: &str = r#"
schema_version = 2
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
schema_version = 2
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
implemented = "fixture"
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

// ---------------------------------------------------------------------------
// doctor threshold-override fixtures + tests (Tasks 12 & 13)
// ---------------------------------------------------------------------------

/// One in-progress task 40d idle (started 2026-04-01, at RMAP_TODAY=2026-05-11)
/// and one pending task scored 40d ago. At the default 30d cutoff both fire
/// (stale + score-decay); at `--threshold-days 60` neither does. Two bundles in
/// one phase so no degenerate-bundle finding; scores are D<5/B<8 so missing-AC
/// stays quiet.
const DOCTOR_THRESHOLD_TASKS: &str = r#"
schema_version = 2
project = "doctor_threshold"
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
id = 40
phase = 1
bundle = "core"
status = "in_progress"
title = "Idle 40 days"
scores = { d = 2, b = 4, u = 4 }
started_at = "2026-04-01"
scored_at = "2026-05-10"

[[task]]
id = 41
phase = 1
bundle = "secondary"
status = "pending"
title = "Scored 40 days ago"
scores = { d = 1, b = 3, u = 3 }
scored_at = "2026-04-01"
"#;

/// Task 50 sits at D=4/B=4 — below the default 5/8 missing-AC bar but caught by
/// `--ac-threshold 4`. Task 51 at D=2/B=3 stays below even the lowered bar, so
/// it doubles as an over-flagging control. Both pending with recent `scored_at`
/// (no decay) and no `started_at` (not stale). Two bundles → no degenerate.
const DOCTOR_AC_THRESHOLD_TASKS: &str = r#"
schema_version = 2
project = "doctor_ac_threshold"
default_branch = "main"

[phases.1]
name = "Foundation"
order = 1
status = "pending"

[bundles.alpha]
phase = 1
order = 1
description = "Alpha work"

[bundles.beta]
phase = 1
order = 2
description = "Beta work"

[[task]]
id = 50
phase = 1
bundle = "alpha"
status = "pending"
title = "Straddles the AC bar"
scores = { d = 4, b = 4, u = 5 }
scored_at = "2026-05-10"

[[task]]
id = 51
phase = 1
bundle = "beta"
status = "pending"
title = "Below even the lowered bar"
scores = { d = 2, b = 3, u = 3 }
scored_at = "2026-05-10"
"#;

#[test]
fn doctor_command_threshold_days_suppresses_stale_and_decay() {
    let path = write_temp_tasks("doctor_threshold.toml", DOCTOR_THRESHOLD_TASKS);

    // Default 30d cutoff: both findings fire.
    let default_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor default");
    assert!(default_out.status.success(), "expected exit 0");
    let default_stdout = String::from_utf8_lossy(&default_out.stdout);
    assert!(
        default_stdout.contains("Stale") && default_stdout.contains("task 40"),
        "expected stale finding at default cutoff:\n{default_stdout}"
    );
    assert!(
        default_stdout.contains("Score decay") && default_stdout.contains("task 41"),
        "expected score-decay finding at default cutoff:\n{default_stdout}"
    );

    // --threshold-days 60: both ages fall under the cutoff.
    let override_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--threshold-days")
        .arg("60")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --threshold-days 60");
    assert!(
        override_out.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&override_out.stderr)
    );
    let override_stdout = String::from_utf8_lossy(&override_out.stdout);
    assert!(
        override_stdout.contains("all checks passed"),
        "expected --threshold-days 60 to suppress stale + decay:\n{override_stdout}"
    );
}

#[test]
fn doctor_command_ac_threshold_changes_missing_ac_set() {
    let path = write_temp_tasks("doctor_ac_threshold.toml", DOCTOR_AC_THRESHOLD_TASKS);

    // Default 5/8 bar: neither D=4/B=4 task qualifies.
    let default_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor default");
    assert!(default_out.status.success(), "expected exit 0");
    let default_stdout = String::from_utf8_lossy(&default_out.stdout);
    assert!(
        default_stdout.contains("all checks passed"),
        "expected no missing-AC finding at default bar:\n{default_stdout}"
    );

    // --ac-threshold 4: task 50 (D=4) now qualifies; task 51 (D=2/B=3) still does not.
    let override_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--ac-threshold")
        .arg("4")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --ac-threshold 4");
    assert!(
        override_out.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&override_out.stderr)
    );
    let override_stdout = String::from_utf8_lossy(&override_out.stdout);
    assert!(
        override_stdout.contains("Missing acceptance_criteria")
            && override_stdout.contains("task 50"),
        "expected task 50 flagged at --ac-threshold 4:\n{override_stdout}"
    );
    assert!(
        !override_stdout.contains("task 51"),
        "expected task 51 (D=2/B=3) to stay below the lowered bar:\n{override_stdout}"
    );
}

#[test]
fn doctor_command_thresholds_in_json() {
    let path = write_temp_tasks("doctor_thresholds_json.toml", DOCTOR_DIRTY_TASKS);

    // Default invocation: thresholds object carries the constant defaults.
    let default_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --json default");
    assert!(default_out.status.success(), "expected exit 0");
    let default_json: serde_json::Value =
        serde_json::from_slice(&default_out.stdout).expect("stdout should be valid JSON");
    assert_eq!(default_json["thresholds"]["days"], 30);
    assert_eq!(default_json["thresholds"]["ac_difficulty"], 5);
    assert_eq!(default_json["thresholds"]["ac_benefit"], 8);

    // Overrides flow into the JSON envelope; --ac-threshold collapses both AC fields.
    let override_out = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .env("RMAP_TODAY", "2026-05-11")
        .arg("doctor")
        .arg("--json")
        .arg("--threshold-days")
        .arg("45")
        .arg("--ac-threshold")
        .arg("6")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap doctor --json with overrides");
    assert!(
        override_out.status.success(),
        "expected exit 0, stderr: {}",
        String::from_utf8_lossy(&override_out.stderr)
    );
    let override_json: serde_json::Value =
        serde_json::from_slice(&override_out.stdout).expect("stdout should be valid JSON");
    assert_eq!(override_json["thresholds"]["days"], 45);
    assert_eq!(override_json["thresholds"]["ac_difficulty"], 6);
    assert_eq!(override_json["thresholds"]["ac_benefit"], 6);
}

// ---------------------------------------------------------------------------
// `rmap bundles` — Task 16 / batch_selection
// ---------------------------------------------------------------------------

const BUNDLES_MULTI_PHASE_TASKS: &str = r#"
schema_version = 2
project = "bundles_demo"
default_branch = "main"

[focus]
phase = 1

[phases.1]
name = "Alpha"
order = 1
status = "in_progress"

[phases.2]
name = "Beta"
order = 2
status = "pending"

[bundles.alpha]
phase = 1
order = 1
description = "Alpha work in focus"

[bundles.stalled]
phase = 1
order = 2
description = "Alpha follow-up blocked by in_progress dep"

[bundles.beta]
phase = 2
order = 1
description = "Beta work"

[[task]]
id = 1
phase = 1
bundle = "alpha"
status = "done"
implemented = "fixture"
title = "alpha setup"
scores = { d = 2, b = 4, u = 4 }

[[task]]
id = 2
phase = 1
bundle = "alpha"
status = "pending"
title = "alpha next"
scores = { d = 3, b = 9, u = 9 }
depends_on = [1]

[[task]]
id = 3
phase = 1
bundle = "stalled"
status = "in_progress"
title = "stalled work in flight"
scores = { d = 5, b = 5, u = 5 }
started_at = "2026-05-01"

[[task]]
id = 4
phase = 1
bundle = "stalled"
status = "pending"
title = "stalled follow-up"
scores = { d = 5, b = 5, u = 5 }
depends_on = [3]

[[task]]
id = 5
phase = 2
bundle = "beta"
status = "pending"
title = "beta task"
scores = { d = 3, b = 6, u = 6 }
"#;

const BUNDLES_EMPTY_TASKS: &str = r#"
schema_version = 2
project = "empty"
default_branch = "main"
"#;

const BUNDLES_ALL_DONE_TASKS: &str = r#"
schema_version = 2
project = "all_done"
default_branch = "main"

[phases.1]
name = "Done"
order = 1
status = "done"

[bundles.finished]
phase = 1
order = 1
description = "All work complete"

[[task]]
id = 1
phase = 1
bundle = "finished"
status = "done"
implemented = "fixture"
title = "complete"
scores = { d = 2, b = 4, u = 4 }
done_at = "2026-05-01"
"#;

const BUNDLES_ALL_BLOCKED_TASKS: &str = r#"
schema_version = 2
project = "all_blocked"
default_branch = "main"

[phases.1]
name = "Blocked"
order = 1
status = "in_progress"

[bundles.stuck]
phase = 1
order = 1
description = "Everything blocked"

[[task]]
id = 1
phase = 1
bundle = "stuck"
status = "blocked"
title = "waiting on vendor"
scores = { d = 3, b = 6, u = 6 }
blocked_reason = "waiting on vendor approval"
"#;

const BUNDLES_IN_FLIGHT_TASKS: &str = r#"
schema_version = 2
project = "in_flight"
default_branch = "main"

[phases.1]
name = "Active"
order = 1
status = "in_progress"

[bundles.flying]
phase = 1
order = 1
description = "All work in flight"

[[task]]
id = 1
phase = 1
bundle = "flying"
status = "in_progress"
title = "midair work"
scores = { d = 3, b = 6, u = 6 }
started_at = "2026-05-01"
"#;

fn run_bundles(path: &PathBuf, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rmap"));
    cmd.arg("bundles");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--tasks-path")
        .arg(path)
        .output()
        .expect("run rmap bundles")
}

#[test]
fn bundles_command_groups_by_phase_with_focus_first() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);
    let output = run_bundles(&path, &[]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let phase1 = stdout
        .find("phase 1 — Alpha")
        .unwrap_or_else(|| panic!("phase 1 header missing:\n{stdout}"));
    let phase2 = stdout
        .find("phase 2 — Beta")
        .unwrap_or_else(|| panic!("phase 2 header missing:\n{stdout}"));
    assert!(
        phase1 < phase2,
        "focus phase 1 must precede phase 2:\n{stdout}"
    );
    assert!(
        stdout.contains("[in_progress, focus]"),
        "expected focus tag on phase 1 header:\n{stdout}"
    );
    let alpha = stdout.find("alpha ").unwrap();
    let stalled = stdout.find("stalled ").unwrap();
    let beta = stdout.find("beta ").unwrap();
    assert!(alpha < stalled, "alpha must precede stalled:\n{stdout}");
    assert!(stalled < beta, "stalled must precede beta:\n{stdout}");
}

#[test]
fn bundles_command_emits_json_envelope() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);
    let output = run_bundles(&path, &["--json"]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["focus_phase"], 1);
    let bundles = value["bundles"].as_array().expect("bundles is array");
    assert_eq!(bundles.len(), 3, "expected 3 bundles:\n{stdout}");

    // First entry sorts alpha first (focus, phase_order=1, bundle_order=1).
    assert_eq!(bundles[0]["name"], "alpha");
    assert_eq!(bundles[0]["phase"], 1);
    assert_eq!(bundles[0]["in_focus"], true);
    assert!(bundles[0]["next_task"]["id"].is_number());
    assert_eq!(bundles[0]["next_task"]["title"], "alpha next");
    assert!(bundles[0]["next_task"]["eff"].is_number());

    // status_counts always emits all four keys, zero-filled.
    let counts = &bundles[0]["status_counts"];
    for key in ["pending", "in_progress", "done", "blocked"] {
        assert!(
            counts.get(key).is_some(),
            "expected status_counts.{key} on:\n{stdout}"
        );
    }
}

#[test]
fn bundles_command_filter_phase() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);
    let output = run_bundles(&path, &["--phase", "2"]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("beta "), "{stdout}");
    assert!(!stdout.contains("alpha "), "{stdout}");
    assert!(!stdout.contains("stalled "), "{stdout}");
}

#[test]
fn bundles_command_filter_has_next() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);
    let output = run_bundles(&path, &["--has-next"]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha "), "{stdout}");
    assert!(stdout.contains("beta "), "{stdout}");
    assert!(
        !stdout.contains("stalled "),
        "stalled has no next, should be filtered out:\n{stdout}"
    );
}

#[test]
fn bundles_command_filter_in_focus() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);
    let output = run_bundles(&path, &["--in-focus"]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha "), "{stdout}");
    assert!(stdout.contains("stalled "), "{stdout}");
    assert!(!stdout.contains("beta "), "{stdout}");
}

#[test]
fn bundles_command_empty_bundles() {
    let path = write_temp_tasks("bundles_empty.toml", BUNDLES_EMPTY_TASKS);

    let human = run_bundles(&path, &[]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        stdout.contains("(no bundles declared)"),
        "expected empty-state message:\n{stdout}"
    );

    let json_out = run_bundles(&path, &["--json"]);
    assert!(json_out.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out.stdout)).expect("valid JSON");
    let bundles = value["bundles"].as_array().expect("bundles is array");
    assert!(bundles.is_empty(), "expected empty array:\n{value}");
    assert!(value["focus_phase"].is_null());
}

#[test]
fn bundles_command_all_done_renders_check_glyph() {
    let path = write_temp_tasks("bundles_all_done.toml", BUNDLES_ALL_DONE_TASKS);
    let output = run_bundles(&path, &[]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✅"), "expected ✅ glyph:\n{stdout}");
    assert!(
        !stdout.contains("next:"),
        "expected no next: hint on all-done:\n{stdout}"
    );
}

#[test]
fn bundles_command_all_blocked_renders_blocked_glyph() {
    let path = write_temp_tasks("bundles_all_blocked.toml", BUNDLES_ALL_BLOCKED_TASKS);
    let output = run_bundles(&path, &[]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("all-blocked ⛔"),
        "expected all-blocked ⛔ glyph:\n{stdout}"
    );
}

#[test]
fn bundles_command_in_flight_renders_construction_glyph() {
    let path = write_temp_tasks("bundles_in_flight.toml", BUNDLES_IN_FLIGHT_TASKS);
    let output = run_bundles(&path, &[]);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🚧"), "expected 🚧 glyph:\n{stdout}");
    assert!(
        !stdout.contains("next:"),
        "expected no next: hint when all tasks in_progress:\n{stdout}"
    );

    let json_out = run_bundles(&path, &["--json"]);
    assert!(json_out.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out.stdout)).expect("valid JSON");
    let flying = value["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "flying")
        .expect("flying bundle in JSON");
    assert_eq!(flying["status_counts"]["in_progress"], 1);
    assert_eq!(flying["status_counts"]["pending"], 0);
    assert!(flying["next_task"].is_null());
}

#[test]
fn bundles_command_pending_with_unmet_deps_renders_pause_glyph() {
    let path = write_temp_tasks("bundles_multi.toml", BUNDLES_MULTI_PHASE_TASKS);

    let human = run_bundles(&path, &[]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        stdout.contains("pending:1 (deps unmet) ⏸"),
        "expected stalled bundle to render pause-glyph row:\n{stdout}"
    );

    let json_out = run_bundles(&path, &["--json"]);
    assert!(json_out.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out.stdout)).expect("valid JSON");
    let stalled = value["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "stalled")
        .expect("stalled bundle in JSON");
    assert!(
        stalled["next_task"].is_null(),
        "expected next_task null on stalled:\n{stalled}"
    );
    assert_eq!(stalled["status_counts"]["pending"], 1);
}

const MILESTONE_TASKS: &str = r#"
schema_version = 2
project = "demo"
default_branch = "main"

[phases.1]
name = "Phase 1"
order = 1
status = "in_progress"

[bundles.alpha]
phase = 1
order = 1
description = "Alpha"

[bundles.beta]
phase = 1
order = 2
description = "Beta"

[milestones.v0_1]
name = "v0.1 — first cut"
order = 1
status = "active"
target_version = "0.1.0"

[milestones.v1_0]
name = "v1.0 — production"
order = 2
status = "pending"

[[task]]
id = 1
phase = 1
bundle = "alpha"
milestone = "v0_1"
status = "pending"
title = "Task one"
scores = { d = 2, b = 8, u = 8 }

[[task]]
id = 2
phase = 1
bundle = "beta"
milestone = "v1_0"
status = "pending"
title = "Task two"
scores = { d = 2, b = 8, u = 8 }

[[task]]
id = 3
phase = 1
bundle = "alpha"
status = "pending"
title = "Task three (no milestone)"
scores = { d = 2, b = 6, u = 6 }
"#;

#[test]
fn list_command_filters_by_milestone() {
    let path = write_temp_tasks("milestone_list.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("list")
        .arg("--milestone")
        .arg("v0_1")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap list --milestone v0_1");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 1"), "{stdout}");
    assert!(!stdout.contains("Task 2"), "{stdout}");
    assert!(!stdout.contains("Task 3"), "{stdout}");
}

#[test]
fn next_command_filters_by_milestone() {
    let path = write_temp_tasks("milestone_next.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--milestone")
        .arg("v0_1")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --milestone v0_1");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 1"), "{stdout}");
}

#[test]
fn next_command_composes_milestone_and_bundle() {
    let path = write_temp_tasks("milestone_compose.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--milestone")
        .arg("v1_0")
        .arg("--bundle")
        .arg("beta")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --milestone v1_0 --bundle beta");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task 2"), "{stdout}");

    // milestone v1_0 + bundle alpha = empty (no task has both).
    let empty = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("next")
        .arg("--milestone")
        .arg("v1_0")
        .arg("--bundle")
        .arg("alpha")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap next --milestone v1_0 --bundle alpha");

    assert!(empty.status.success());
    assert_eq!(String::from_utf8_lossy(&empty.stdout), "");
}

#[test]
fn milestone_command_sets_then_unsets() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MILESTONE_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);

    // initial render so check-render gates pass
    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("initial render");
    assert!(
        render.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&render.stderr)
    );

    // pin task 3 to v0_1
    let pin = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestone")
        .arg("3")
        .arg("v0_1")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap milestone 3 v0_1");
    assert!(
        pin.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&pin.stderr)
    );

    let after_pin = fs::read_to_string(&tasks_path).expect("read tasks.toml");
    assert!(
        after_pin.matches("milestone = \"v0_1\"").count() >= 2,
        "task 3 gains milestone:\n{after_pin}"
    );

    // unpin task 3
    let unpin = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestone")
        .arg("3")
        .arg("none")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap milestone 3 none");
    assert!(
        unpin.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&unpin.stderr)
    );

    let after_unpin = fs::read_to_string(&tasks_path).expect("read tasks.toml");
    // back to 2 occurrences (tasks 1 and 2 still have milestone)
    assert_eq!(
        after_unpin.matches("milestone = ").count(),
        2,
        "task 3 milestone removed:\n{after_unpin}"
    );
}

#[test]
fn milestone_command_rejects_unknown_target() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MILESTONE_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);
    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("initial render");

    let before = fs::read_to_string(&tasks_path).expect("read pre-state");

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestone")
        .arg("3")
        .arg("v_nope")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("run rmap milestone 3 v_nope");

    assert!(
        !output.status.success(),
        "unknown milestone is a hard error"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown milestone") || stderr.contains("v_nope"),
        "stderr should mention unknown milestone: {stderr}"
    );

    // file untouched
    let after = fs::read_to_string(&tasks_path).expect("read post-state");
    assert_eq!(before, after, "rejected mutation leaves file byte-equal");
}

#[test]
fn new_command_accepts_milestone_via_stdin() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("roadmap")).expect("create roadmap dir");
    let tasks_path = write_file(&dir.join("roadmap"), "tasks.toml", MILESTONE_TASKS);
    write_file(&dir, "ROADMAP.md", ROADMAP);
    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .output()
        .expect("initial render");

    let stdin_payload = r#"
[[task]]
phase = 1
bundle = "alpha"
milestone = "v0_1"
title = "New milestone task"
scores = { d = 2, b = 6, u = 6 }
"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("new")
        .arg("--from-stdin")
        .arg("--tasks-path")
        .arg(&tasks_path)
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rmap new");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_payload.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait rmap new");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = fs::read_to_string(&tasks_path).expect("read tasks.toml");
    assert!(
        after.contains("title = \"New milestone task\""),
        "new task appended:\n{after}"
    );
    assert!(
        after.matches("milestone = \"v0_1\"").count() >= 2,
        "new task carries milestone = v0_1:\n{after}"
    );
}

#[test]
fn milestones_command_lists_with_next_glyphs() {
    let path = write_temp_tasks("milestones_list.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestones")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap milestones");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // active milestone surfaces first
    let v0_1_pos = stdout.find("v0_1").expect("v0_1 in output");
    let v1_0_pos = stdout.find("v1_0").expect("v1_0 in output");
    assert!(
        v0_1_pos < v1_0_pos,
        "active v0_1 sorts before pending v1_0:\n{stdout}"
    );
    // glyph ladder: v0_1 has a pending task 1 with deps satisfied → next: + Eff
    assert!(
        stdout.contains("next:1") || stdout.contains("next:2"),
        "next-task hint:\n{stdout}"
    );
    // status badges visible
    assert!(stdout.contains("[active]"), "{stdout}");
    assert!(stdout.contains("[pending]"), "{stdout}");
    // target_version surfaces when set
    assert!(stdout.contains("target=0.1.0"), "{stdout}");
}

#[test]
fn milestones_command_emits_json_envelope() {
    let path = write_temp_tasks("milestones_json.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestones")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap milestones --json");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    assert_eq!(value["schema_version"], 2);
    let milestones = value["milestones"].as_array().expect("milestones array");
    assert_eq!(milestones.len(), 2);
    // active sorts first
    assert_eq!(milestones[0]["key"], "v0_1");
    assert_eq!(milestones[0]["status"], "active");
    assert_eq!(milestones[0]["target_version"], "0.1.0");
    assert_eq!(milestones[0]["task_count"], 1);
    assert_eq!(milestones[0]["status_counts"]["pending"], 1);
    assert!(
        milestones[0]["next_task"].is_object(),
        "next_task populated"
    );
    assert_eq!(milestones[1]["key"], "v1_0");
    assert!(milestones[1]["target_version"].is_null());
}

#[test]
fn milestones_command_filter_has_next() {
    // remove milestones' only candidate by marking task 2 as blocked
    let tasks = MILESTONE_TASKS.replace(
        "[[task]]\nid = 2\nphase = 1\nbundle = \"beta\"\nmilestone = \"v1_0\"\nstatus = \"pending\"",
        "[[task]]\nid = 2\nphase = 1\nbundle = \"beta\"\nmilestone = \"v1_0\"\nstatus = \"blocked\"\nblocked_reason = \"upstream\"",
    );
    let path = write_temp_tasks("milestones_hasnext.toml", &tasks);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestones")
        .arg("--has-next")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap milestones --has-next --json");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    let milestones = value["milestones"].as_array().expect("array");
    // v0_1 still has next:1; v1_0's only task is blocked → no next → filtered out.
    assert_eq!(milestones.len(), 1);
    assert_eq!(milestones[0]["key"], "v0_1");
}

#[test]
fn milestones_command_filter_status() {
    let path = write_temp_tasks("milestones_filter_status.toml", MILESTONE_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestones")
        .arg("--status")
        .arg("pending")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap milestones --status pending");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is valid json");
    let milestones = value["milestones"].as_array().expect("array");
    assert_eq!(milestones.len(), 1);
    assert_eq!(milestones[0]["key"], "v1_0");
}

#[test]
fn milestones_command_empty_milestones() {
    let path = write_temp_tasks("milestones_empty.toml", PHASE4_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("milestones")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap milestones on empty");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(no milestones declared)"),
        "empty-state line:\n{stdout}"
    );
}

#[test]
fn show_command_surfaces_milestone_line() {
    let path = write_temp_tasks("show_milestone.toml", MILESTONE_TASKS);

    let human = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("1")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show 1");
    assert!(
        human.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&human.stderr)
    );
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("milestone: v0_1"), "{text}");

    let json = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("1")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show 1 --json");
    let value: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("stdout is valid json");
    assert_eq!(value["milestone"], "v0_1");

    // task 3 has no milestone → no milestone field in JSON, no line in human view.
    let no_milestone = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("show")
        .arg("3")
        .arg("--json")
        .arg("--tasks-path")
        .arg(&path)
        .output()
        .expect("run rmap show 3 --json");
    let value: serde_json::Value =
        serde_json::from_slice(&no_milestone.stdout).expect("stdout is valid json");
    assert!(
        value.get("milestone").is_none() || value["milestone"].is_null(),
        "task 3 has no milestone surface in JSON: {value}"
    );
}
