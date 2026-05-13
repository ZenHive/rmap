use std::fs;
use std::path::Path;

use rmap::render::render_roadmap_str_with_today;
use rmap::validate::validate_tasks_str;

const TASKS: &str = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[phases.12]
name = "Per-Exchange Normalization"
order = 12
status = "in_progress"

[bundles.ticker_normalization]
phase = 12
order = 1
description = "Unified ticker fields across all exchanges"

[[task]]
id = 74
phase = 12
bundle = "ticker_normalization"
status = "done"
title = "parseTicker field map + coercion + enums"
scores = { d = 5, b = 8, u = 8 }
markers = ["parallel"]
shipped_in = "PR #21"

[[task]]
id = 75
phase = 12
bundle = "ticker_normalization"
status = "pending"
title = "parseOrder field map"
scores = { d = 6, b = 8, u = 8 }
depends_on = [74]
"#;

#[test]
fn golden_render_fixtures_are_byte_equal() {
    for case in golden_cases() {
        let tasks_path = case.join("tasks.toml");
        let input_path = case.join("ROADMAP.input.md");
        let expected_path = case.join("ROADMAP.md");
        let today_path = case.join("today.txt");

        let tasks_input = fs::read_to_string(&tasks_path).expect("read golden tasks.toml");
        let roadmap = fs::read_to_string(&input_path).expect("read golden input roadmap");
        let expected = fs::read_to_string(&expected_path).expect("read golden expected roadmap");
        let tasks = validate_tasks_str(tasks_path.display().to_string(), &tasks_input)
            .expect("golden tasks validate");

        let today = if today_path.exists() {
            fs::read_to_string(&today_path)
                .expect("read today.txt")
                .trim()
                .to_string()
        } else {
            rmap::today_iso()
        };

        let rendered =
            render_roadmap_str_with_today(&roadmap, &tasks, &today).expect("render golden roadmap");

        assert_eq!(rendered, expected, "golden case {}", case.display());
    }
}

#[test]
fn render_rejects_unclosed_marker() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = "<!-- TASKS:BEGIN phase=12 -->\nstale generated content\n";

    let err = render_roadmap_str_with_today(roadmap, &tasks, "2026-05-11")
        .expect_err("unclosed marker is rejected");

    assert!(err.to_string().contains("missing <!-- TASKS:END -->"));
}

#[test]
fn render_rejects_unclosed_focus_marker() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = "<!-- FOCUS:BEGIN -->\nstale generated content\n";

    let err = render_roadmap_str_with_today(roadmap, &tasks, "2026-05-11")
        .expect_err("unclosed FOCUS marker is rejected");

    assert!(err.to_string().contains("missing <!-- FOCUS:END -->"));
}

#[test]
fn render_rejects_unclosed_mermaid_marker() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = "<!-- MERMAID:BEGIN -->\nstale generated content\n";

    let err = render_roadmap_str_with_today(roadmap, &tasks, "2026-05-11")
        .expect_err("unclosed MERMAID marker is rejected");

    assert!(err.to_string().contains("missing <!-- MERMAID:END -->"));
}

fn golden_cases() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut cases = fs::read_dir(root)
        .expect("read golden fixtures")
        .map(|entry| entry.expect("read golden fixture entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    cases.sort();
    cases
}
