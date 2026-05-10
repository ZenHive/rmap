use std::str::FromStr;

use toml_edit::DocumentMut;

const TASKS_WITH_COMMENTS: &str = r#"# Roadmap source data.
schema_version = 1
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
