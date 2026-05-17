use std::str::FromStr;

use toml_edit::DocumentMut;

const TASKS_WITH_COMMENTS: &str = r#"# Roadmap source data.
schema_version = 2
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"
workspace_url = "https://linear.app/efries"

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
implemented = "fixture"
title = "parseTicker field map + coercion + enums"
scores = { d = 5, b = 8, u = 8 }
markers = ["parallel"]
linear_id = "INE-247"
shipped_in = "PR #21"
"#;

#[test]
fn toml_edit_round_trip_preserves_unmodified_tasks_file() {
    let document = DocumentMut::from_str(TASKS_WITH_COMMENTS).expect("parse tasks document");

    assert_eq!(document.to_string(), TASKS_WITH_COMMENTS);
}

#[test]
fn golden_tasks_round_trip_without_spurious_diff() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut cases = std::fs::read_dir(root)
        .expect("read golden fixtures")
        .map(|entry| entry.expect("read golden fixture entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    cases.sort();

    for case in cases {
        let path = case.join("tasks.toml");
        let source = std::fs::read_to_string(&path).expect("read golden tasks.toml");
        let document = DocumentMut::from_str(&source).expect("parse golden tasks.toml");

        assert_eq!(
            document.to_string(),
            source,
            "golden case {}",
            case.display()
        );
    }
}
