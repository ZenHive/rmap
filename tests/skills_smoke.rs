use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// Parses `SKILLS.md`, extracts every fenced ```bash``` block whose first
/// non-comment line starts with `rmap `, and runs each in a fresh temp-dir
/// copy of `tests/skills_fixture/`. The expected exit code is the optional
/// `# exit: <N>` annotation inside the block (default 0).
///
/// `RMAP_TODAY` is pinned to `2026-05-12` so date-sensitive output is
/// reproducible across runs.
#[test]
fn skills_md_bash_blocks_match_declared_exit_codes() {
    let skills_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SKILLS.md");
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/skills_fixture");

    let skills = fs::read_to_string(&skills_path).expect("read SKILLS.md");
    let commands = parse_bash_blocks(&skills);
    assert!(
        !commands.is_empty(),
        "SKILLS.md contained no rmap commands — did the parser miss them?"
    );

    for cmd in &commands {
        let tmp = TempDir::with_prefix("rmap-skills-").expect("create temp test dir");
        copy_dir_recursive(&fixture_root, tmp.path());
        prepare_fixture(tmp.path());

        let argv: Vec<&str> = cmd.line.split_whitespace().skip(1).collect();
        let mut command = Command::new(env!("CARGO_BIN_EXE_rmap"));
        command
            .args(&argv)
            .current_dir(tmp.path())
            .env("RMAP_TODAY", "2026-05-12");

        let output = if let Some(stdin_payload) = &cmd.stdin {
            command.stdin(Stdio::piped());
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            let mut child = command.spawn().expect("spawn rmap with stdin");
            {
                let mut child_stdin = child.stdin.take().expect("piped stdin");
                child_stdin
                    .write_all(stdin_payload.as_bytes())
                    .expect("write stdin payload");
            }
            child.wait_with_output().expect("collect rmap output")
        } else {
            command.output().expect("run rmap")
        };

        let actual = output.status.code();
        assert_eq!(
            actual,
            Some(cmd.expected_exit),
            "SKILLS.md line {}: `{}` expected exit {}, got {:?}\nstdout:\n{}\nstderr:\n{}",
            cmd.source_line,
            cmd.line,
            cmd.expected_exit,
            actual,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

struct SmokeCommand {
    line: String,
    expected_exit: i32,
    source_line: usize,
    /// Optional stdin payload, captured from a `# stdin: <<EOF` … `# EOF` block.
    stdin: Option<String>,
}

fn parse_bash_blocks(skills: &str) -> Vec<SmokeCommand> {
    let mut commands = Vec::new();
    let mut in_bash = false;
    let mut block_start_line = 0usize;
    let mut block_lines: Vec<&str> = Vec::new();

    for (idx, line) in skills.lines().enumerate() {
        let trimmed = line.trim();
        if !in_bash && trimmed == "```bash" {
            in_bash = true;
            block_start_line = idx + 2; // first line inside the fence is idx+1; 1-indexed for human reporting
            block_lines.clear();
        } else if in_bash && trimmed == "```" {
            in_bash = false;
            if let Some(cmd) = extract_command(&block_lines, block_start_line) {
                commands.push(cmd);
            }
        } else if in_bash {
            block_lines.push(line);
        }
    }
    commands
}

fn extract_command(lines: &[&str], start_line: usize) -> Option<SmokeCommand> {
    let mut expected_exit = 0;
    let mut found: Option<(String, usize)> = None;
    let mut stdin: Option<String> = None;
    let mut collecting_stdin: Option<Vec<String>> = None;

    for (offset, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(buffer) = collecting_stdin.as_mut() {
            if trimmed == "# EOF" {
                stdin = Some(buffer.join("\n"));
                collecting_stdin = None;
            } else {
                let payload_line = line.strip_prefix("# ").unwrap_or(line);
                buffer.push(payload_line.to_string());
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# exit:") {
            let raw = rest.trim();
            expected_exit = raw.parse().unwrap_or_else(|_| {
                panic!(
                    "SKILLS.md line {}: malformed `# exit:` annotation `{}` — expected integer",
                    start_line + offset,
                    raw
                )
            });
        } else if trimmed.starts_with("# stdin:") {
            collecting_stdin = Some(Vec::new());
        } else if trimmed.starts_with("rmap ") || trimmed == "rmap" {
            if let Some((prev_line, prev_source)) = &found {
                panic!(
                    "SKILLS.md lines {}-{}: bash block contains more than one rmap invocation \
                     (`{}` then `{}`). Split the documented commands into separate ```bash``` blocks \
                     so each is exercised by the smoke test.",
                    prev_source,
                    start_line + offset,
                    prev_line,
                    trimmed
                );
            }
            found = Some((trimmed.to_string(), start_line + offset));
        }
    }

    found.map(|(line, source_line)| SmokeCommand {
        line,
        expected_exit,
        source_line,
        stdin,
    })
}

/// Pre-rendering + git init makes every documented command's setup uniform.
/// `rmap render` is idempotent against a fresh fixture, so re-running across
/// commands costs nothing; `git init -b main` + commit gives `rmap diff` a base
/// ref (`default_branch = "main"` in the fixture).
fn prepare_fixture(dir: &Path) {
    let render = Command::new(env!("CARGO_BIN_EXE_rmap"))
        .arg("render")
        .current_dir(dir)
        .env("RMAP_TODAY", "2026-05-12")
        .output()
        .expect("run rmap render for fixture prep");
    assert!(
        render.status.success(),
        "rmap render in fixture prep failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&render.stdout),
        String::from_utf8_lossy(&render.stderr),
    );

    run_git(dir, &["init", "-b", "main"]);
    run_git(
        dir,
        &[
            "-c",
            "user.email=rmap-smoke@example.test",
            "-c",
            "user.name=rmap-smoke",
            "add",
            "ROADMAP.md",
            "roadmap/tasks.toml",
            "roadmap/data.json",
        ],
    );
    run_git(
        dir,
        &[
            "-c",
            "user.email=rmap-smoke@example.test",
            "-c",
            "user.name=rmap-smoke",
            "commit",
            "-m",
            "skills_fixture base",
        ],
    );
}

fn run_git(dir: &Path, args: &[&str]) {
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

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create dst");
    for entry in fs::read_dir(src).expect("read fixture dir") {
        let entry = entry.expect("read fixture entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy fixture file");
        }
    }
}

