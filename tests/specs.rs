use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

const BASE: &str = r#"schema_version = 2
project = "fixture"
default_branch = "main"
[specs.access]
path = "docs/access.md"
status = "active"
[phases.1]
name = "Core"
order = 1
status = "pending"
[bundles.core]
phase = 1
order = 1
description = "Core"
[[task]]
id = 1
phase = 1
bundle = "core"
status = "pending"
title = "Update access"
scores = { d = 2, b = 3, u = 4 }
spec_changes = [{ rule = "ACCESS-1", op = "change" }, { rule = "ACCESS-3", op = "add" }]
[[task]]
id = 2
phase = 1
bundle = "core"
status = "done"
title = "Remove obsolete access"
implemented = "Removed"
scores = { d = 2, b = 3, u = 4 }
spec_changes = [{ rule = "ACCESS-2", op = "remove" }]
"#;

fn fixture(layout: &str, manifest: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    for path in ["roadmap", "docs", layout] {
        fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    fs::write(dir.path().join(manifest), "").unwrap();
    fs::write(dir.path().join("roadmap/tasks.toml"), BASE).unwrap();
    fs::write(
        dir.path().join("docs/access.md"),
        "# Access\nACCESS-1: Users must authenticate.\n prose ACCESS-9 ignored\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("ROADMAP.md"),
        "# Roadmap\n<!-- rmap:start -->\n<!-- rmap:end -->\n",
    )
    .unwrap();
    dir
}
fn run(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rmap"))
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap()
}
fn ok(dir: &TempDir, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn specs_work_in_rust_and_javascript_layouts() {
    for (layout, manifest) in [("tests", "Cargo.toml"), ("__tests__", "package.json")] {
        let dir = fixture(layout, manifest);
        ok(&dir, &["validate"]);
        let json: serde_json::Value =
            serde_json::from_str(&ok(&dir, &["specs", "--json"])).unwrap();
        assert_eq!(
            json["access"]["rules"][0]["text"],
            "ACCESS-1: Users must authenticate."
        );
        assert_eq!(json["access"]["rules"][0]["history"][0]["task"], 1);
        assert_eq!(json["access"]["rules"][2]["history"][0]["op"], "remove");
        assert!(json["access"]["rules"][2]["text"].is_null());
        assert!(ok(&dir, &["specs"]).contains("task 2 [done] remove"));
        let prompt = ok(&dir, &["delegate", "1", "--to", "codex"]);
        assert!(prompt.contains("ACCESS-1 (change)\n\n> ACCESS-1: Users must authenticate."));
        assert!(prompt.contains("ACCESS-3 (add)"));
        assert!(prompt.contains("No current rule text"));
        assert!(ok(&dir, &["show", "1"]).contains("spec_changes: change ACCESS-1"));
        let show: serde_json::Value =
            serde_json::from_str(&ok(&dir, &["show", "1", "--json"])).unwrap();
        assert_eq!(show["spec_changes"][0]["op"], "change");
        let list: serde_json::Value =
            serde_json::from_str(&ok(&dir, &["list", "--rule", "ACCESS-2", "--json"])).unwrap();
        assert_eq!(list["task"].as_array().unwrap().len(), 1);
        assert_eq!(list["task"][0]["id"], 2);
        assert!(ok(&dir, &["list", "--rule", "ACCESS-2", "--status", "pending"]).is_empty());
        ok(&dir, &["render"]);
        let data: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.path().join("roadmap/data.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(data["task"][0]["spec_changes"], show["spec_changes"]);
        // Resolve against the explicit roadmap path even from another cwd.
        let out = Command::new(env!("CARGO_BIN_EXE_rmap"))
            .current_dir("/")
            .args(["specs", "--tasks-path"])
            .arg(dir.path().join("roadmap/tasks.toml"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn invalid_specs_and_live_deltas_reject_without_writes() {
    for (toml, markdown, needle) in [
        (
            BASE.replace("ACCESS-1\", op = \"change", "ACCESS-9\", op = \"change"),
            "ACCESS-1 text\n",
            "ACCESS-9",
        ),
        (
            BASE.replace("ACCESS-1\", op = \"change", "ACCESS-9\", op = \"remove"),
            "ACCESS-1 text\n",
            "ACCESS-9",
        ),
        (
            BASE.replace("ACCESS-3", "UNKNOWN-3"),
            "ACCESS-1 text\n",
            "UNKNOWN-3",
        ),
        (
            BASE.to_string(),
            "ACCESS-1 text\nACCESS-1 duplicate\n",
            "ACCESS-1",
        ),
        (
            BASE.replace("docs/access.md", "missing.md"),
            "ACCESS-1 text\n",
            "missing.md",
        ),
        (
            BASE.replace(
                "[phases.1]",
                "[specs.other]\npath = \"docs/other.md\"\nstatus = \"draft\"\n[phases.1]",
            ),
            "ACCESS-1 text\n",
            "ACCESS-1",
        ),
    ] {
        let dir = fixture("spec", "Gemfile");
        let path = dir.path().join("roadmap/tasks.toml");
        fs::write(&path, &toml).unwrap();
        fs::write(dir.path().join("docs/access.md"), markdown).unwrap();
        fs::write(dir.path().join("docs/other.md"), "ACCESS-1 duplicate\n").unwrap();
        let out = run(&dir, &["validate"]);
        assert!(!out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(needle),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(fs::read_to_string(path).unwrap(), toml);
    }
}

#[test]
fn creation_roundtrips_and_rejects_invalid_delta_atomically() {
    let dir = fixture("tests", "Cargo.toml");
    for (rule, success) in [("ACCESS-1", true), ("UNKNOWN-1", false)] {
        let path = dir.path().join("roadmap/tasks.toml");
        let before = fs::read_to_string(&path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_rmap"))
            .current_dir(dir.path())
            .args(["new", "--from-stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        write!(child.stdin.take().unwrap(), "[[task]]\nphase = 1\nbundle = \"core\"\ntitle = \"Added\"\nscores = {{ d = 2, b = 3, u = 4 }}\nspec_changes = [{{ rule = \"{rule}\", op = \"add\" }}]\n").unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let after = fs::read_to_string(&path).unwrap();
        if success {
            let tasks = rmap::validate::validate_tasks_file(&path).unwrap();
            assert_eq!(tasks.task.last().unwrap().spec_changes[0].rule, rule);
            let old: rmap::schema::Tasks = toml::from_str(&before).unwrap();
            let changed = after.replace("op = \"add\"", "op = \"change\"");
            let changed: rmap::schema::Tasks = toml::from_str(&changed).unwrap();
            let diff = rmap::diff::diff_tasks(&old, &changed, false);
            assert!(diff.iter().any(|entry| {
                entry
                    .changed_fields
                    .iter()
                    .any(|field| field == "spec_changes")
            }));
        } else {
            assert_eq!(before, after);
        }
    }
}

#[test]
fn prefix_ownership_and_schema_are_enforced() {
    let dir = fixture("tests", "Cargo.toml");
    let path = dir.path().join("roadmap/tasks.toml");
    let shared = BASE.replace(
        "[phases.1]",
        "[specs.other]\npath = \"docs/other.md\"\nstatus = \"retired\"\n[phases.1]",
    );
    fs::write(dir.path().join("docs/other.md"), "ACCESS-8 another rule\n").unwrap();
    fs::write(&path, &shared).unwrap();
    let error = rmap::validate::validate_tasks_file(&path)
        .unwrap_err()
        .to_string();
    assert!(error.contains("prefix ACCESS belongs to both"), "{error}");
    for invalid in [
        BASE.replace("op = \"change\"", "op = \"implement\""),
        BASE.replace("status = \"active\"", "status = \"pending\""),
        BASE.replace(
            "path = \"docs/access.md\"",
            "path = \"docs/access.md\"\nrules = [\"ACCESS-1\"]",
        ),
    ] {
        fs::write(&path, &invalid).unwrap();
        assert!(rmap::validate::validate_tasks_file(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }
    let before: rmap::schema::Tasks = toml::from_str(BASE).unwrap();
    let after: rmap::schema::Tasks = toml::from_str(&BASE.replace("active", "retired")).unwrap();
    let diff = rmap::diff::diff_metadata(&before, &after, false);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].key, "specs.access");
}

#[test]
fn terminal_references_are_not_rechecked_and_empty_specs_are_valid() {
    let dir = fixture("tests", "Cargo.toml");
    let path = dir.path().join("roadmap/tasks.toml");
    let terminal = BASE.replace("status = \"pending\"", "status = \"superseded\"");
    fs::write(&path, terminal).unwrap();
    fs::write(dir.path().join("docs/access.md"), "# Empty spec\n").unwrap();
    ok(&dir, &["validate"]);
    assert!(!ok(&dir, &["list", "--rule", "ACCESS-1"]).is_empty());
    fs::write(&path, BASE).unwrap();
    assert!(!run(&dir, &["validate"]).status.success());
}

#[test]
fn git_snapshot_validation_does_not_require_spec_files() {
    // `rmap diff` validates `git show` output at `{ref}:roadmap/tasks.toml`.
    let tasks = rmap::validate::validate_tasks_str("main:roadmap/tasks.toml", BASE)
        .expect("snapshot label is not a tasks file");
    assert!(tasks.specs.contains_key("access"));
    assert_eq!(tasks.task[0].spec_changes.len(), 2);
}

#[test]
fn diff_against_committed_specs_reports_task_edits() {
    let dir = fixture("tests", "Cargo.toml");
    git(&dir, &["init", "-b", "main"]);
    git(
        &dir,
        &["add", "roadmap/tasks.toml", "docs/access.md", "ROADMAP.md"],
    );
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
    let path = dir.path().join("roadmap/tasks.toml");
    let updated = fs::read_to_string(&path).unwrap().replace(
        "title = \"Update access\"",
        "title = \"Update access rules\"",
    );
    fs::write(&path, updated).unwrap();
    let stdout = ok(&dir, &["diff", "--against", "main"]);
    assert!(stdout.contains("changed Task 1: title"), "{stdout}");
}

fn git(dir: &TempDir, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
