use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const VALID_TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[[task]]
id = 1
phase = 1
bundle = "foundation"
status = "pending"
title = "Add validation"
scores = { d = 2, b = 8, u = 8 }
"#;

#[test]
fn validate_command_accepts_valid_tasks_file() {
    let path = write_temp_tasks("valid_tasks.toml", VALID_TASKS);

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
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
fn validate_command_rejects_invalid_tasks_file() {
    let path = write_temp_tasks(
        "invalid_tasks.toml",
        &VALID_TASKS.replace("pending", "shipped"),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("validate")
        .arg(&path)
        .output()
        .expect("run rmap validate");

    assert!(!output.status.success(), "expected validation failure");
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid status \"shipped\""));
}

fn write_temp_tasks(file_name: &str, contents: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rmap-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos()
    ));

    fs::create_dir_all(&dir).expect("create temp test dir");
    let path = dir.join(file_name);
    fs::write(&path, contents).expect("write test tasks file");
    path
}
