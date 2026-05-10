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
            "| Task 1 | ⬜ | 🎁 **foundation** · Add validation [D:2/B:8/U:8 → Eff:4.0] 🎯 |"
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
