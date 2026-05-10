use rmap::render::render_roadmap_str;
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
fn render_replaces_only_marked_phase_block() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = r#"# Project Roadmap

Hand-written intro stays exactly here.

<!-- TASKS:BEGIN phase=12 -->
stale generated content
<!-- TASKS:END -->

Hand-written outro stays too.
"#;
    let expected = r#"# Project Roadmap

Hand-written intro stays exactly here.

<!-- TASKS:BEGIN phase=12 -->
| Task | Status | Eff | Markers | Title |
|------|--------|-----|---------|-------|
| 74 | done | 1.60 | parallel | parseTicker field map + coercion + enums |
| 75 | pending | 1.33 |  | parseOrder field map |
<!-- TASKS:END -->

Hand-written outro stays too.
"#;

    let rendered = render_roadmap_str(roadmap, &tasks).expect("render roadmap");

    assert_eq!(rendered, expected);
}

#[test]
fn render_leaves_unmatched_phase_block_empty_table() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = r#"Before
<!-- TASKS:BEGIN phase=99 -->
stale generated content
<!-- TASKS:END -->
After
"#;
    let expected = r#"Before
<!-- TASKS:BEGIN phase=99 -->
| Task | Status | Eff | Markers | Title |
|------|--------|-----|---------|-------|
<!-- TASKS:END -->
After
"#;

    let rendered = render_roadmap_str(roadmap, &tasks).expect("render roadmap");

    assert_eq!(rendered, expected);
}

#[test]
fn render_rejects_unclosed_marker() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");
    let roadmap = "<!-- TASKS:BEGIN phase=12 -->\nstale generated content\n";

    let err = render_roadmap_str(roadmap, &tasks).expect_err("unclosed marker is rejected");

    assert!(err.to_string().contains("missing <!-- TASKS:END -->"));
}
