use rmap::next::{next_task, next_tasks};
use rmap::query::TaskFilter;
use rmap::validate::validate_tasks_str;

fn marker_filter(marker: Option<&str>) -> TaskFilter {
    TaskFilter {
        marker: marker.map(String::from),
        ..Default::default()
    }
}

const TASKS: &str = r#"
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

[[task]]
id = "78b"
phase = 12
bundle = "simple"
status = "pending"
title = "parseOHLCV object-shape exchanges"
scores = { d = 6, b = 8, u = 8 }
markers = ["parallel"]

[[task]]
id = 83
phase = 12
bundle = "simple"
status = "pending"
title = "Response envelope paths"
scores = { d = 4, b = 9, u = 9 }
depends_on = [75]
markers = ["parallel"]
"#;

#[test]
fn selects_highest_eff_pending_unblocked_task() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, &marker_filter(None)).expect("next task");

    assert_eq!(task.id.to_string(), "75");
}

#[test]
fn excludes_tasks_blocked_by_incomplete_dependencies() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, &marker_filter(Some("parallel"))).expect("next task");

    assert_eq!(task.id.to_string(), "75");
    assert_ne!(task.id.to_string(), "83");
}

#[test]
fn marker_filter_excludes_non_matching_tasks() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let task = next_task(&tasks, &marker_filter(Some("parallel"))).expect("next task");

    assert!(task.markers.contains(&"parallel".to_string()));
}

#[test]
fn no_matching_next_task_returns_none() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    assert!(next_task(&tasks, &marker_filter(Some("csr"))).is_none());
}

#[test]
fn focus_phase_wins_over_higher_eff_in_other_phases() {
    let input = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[focus]
phase = 13

[phases.12]
name = "Phase 12"
order = 12
status = "in_progress"

[phases.13]
name = "Phase 13"
order = 13
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple"

[bundles.thirteen]
phase = 13
order = 1
description = "Thirteen"

[[task]]
id = 50
phase = 12
bundle = "simple"
status = "pending"
title = "Higher-Eff task in phase 12"
scores = { d = 2, b = 10, u = 10 }

[[task]]
id = 60
phase = 13
bundle = "thirteen"
status = "pending"
title = "Lower-Eff task in focus phase"
scores = { d = 6, b = 8, u = 8 }
"#;

    let tasks = rmap::validate::validate_tasks_str("tasks.toml", input).expect("valid");

    let task = next_task(&tasks, &marker_filter(None)).expect("next task");

    assert_eq!(task.id.to_string(), "60");
}

#[test]
fn focus_phase_falls_back_when_no_candidate_in_focus() {
    let input = r#"
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[focus]
phase = 13

[phases.12]
name = "Phase 12"
order = 12
status = "in_progress"

[phases.13]
name = "Phase 13"
order = 13
status = "pending"

[bundles.simple]
phase = 12
order = 1
description = "Simple"

[bundles.thirteen]
phase = 13
order = 1
description = "Thirteen"

[[task]]
id = 50
phase = 12
bundle = "simple"
status = "pending"
title = "Phase 12 candidate"
scores = { d = 4, b = 8, u = 8 }

[[task]]
id = 60
phase = 13
bundle = "thirteen"
status = "done"
title = "Done in focus phase"
scores = { d = 6, b = 8, u = 8 }
"#;

    let tasks = rmap::validate::validate_tasks_str("tasks.toml", input).expect("valid");

    let task = next_task(&tasks, &marker_filter(None)).expect("next task");

    assert_eq!(task.id.to_string(), "50");
}

#[test]
fn next_tasks_count_one_returns_singleton_matching_next_task() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let selected = next_tasks(&tasks, &marker_filter(None), 1);
    let single = next_task(&tasks, &marker_filter(None)).expect("next task");

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, single.id);
}

#[test]
fn next_tasks_count_three_returns_eff_ranked_array() {
    let input = r#"
schema_version = 1
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

    let tasks = validate_tasks_str("tasks.toml", input).expect("valid");

    let selected = next_tasks(&tasks, &TaskFilter::default(), 3);

    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].id.to_string(), "2");
    assert_eq!(selected[1].id.to_string(), "3");
    assert_eq!(selected[2].id.to_string(), "1");
}

#[test]
fn next_tasks_count_exceeds_eligible_returns_min() {
    let tasks = validate_tasks_str("roadmap/tasks.toml", TASKS).expect("valid tasks");

    let selected = next_tasks(&tasks, &TaskFilter::default(), 100);

    // Fixture has two pending unblocked tasks (75 and 78b); 83 depends on 75 (pending).
    assert_eq!(selected.len(), 2);
}

#[test]
fn next_tasks_focus_phase_fills_before_other_phases() {
    let input = r#"
schema_version = 1
project = "demo"
default_branch = "main"

[focus]
phase = 13

[phases.12]
name = "Phase 12"
order = 12
status = "in_progress"

[phases.13]
name = "Phase 13"
order = 13
status = "in_progress"

[bundles.twelve]
phase = 12
order = 1
description = "Twelve"

[bundles.thirteen]
phase = 13
order = 1
description = "Thirteen"

[[task]]
id = 10
phase = 12
bundle = "twelve"
status = "pending"
title = "Higher Eff non-focus A"
scores = { d = 2, b = 10, u = 10 }

[[task]]
id = 11
phase = 12
bundle = "twelve"
status = "pending"
title = "Higher Eff non-focus B"
scores = { d = 2, b = 10, u = 9 }

[[task]]
id = 20
phase = 13
bundle = "thirteen"
status = "pending"
title = "Lower Eff focus A"
scores = { d = 5, b = 6, u = 6 }

[[task]]
id = 21
phase = 13
bundle = "thirteen"
status = "pending"
title = "Lower Eff focus B"
scores = { d = 6, b = 6, u = 6 }
"#;

    let tasks = validate_tasks_str("tasks.toml", input).expect("valid");

    let selected = next_tasks(&tasks, &TaskFilter::default(), 3);

    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].id.to_string(), "20");
    assert_eq!(selected[1].id.to_string(), "21");
    assert_eq!(selected[2].id.to_string(), "10");
}

#[test]
fn ties_preserve_toml_order() {
    let input = TASKS.replace(
        r#"scores = { d = 6, b = 8, u = 8 }
markers = ["parallel"]"#,
        r#"scores = { d = 5, b = 9, u = 9 }
markers = ["parallel"]"#,
    );
    let tasks = validate_tasks_str("roadmap/tasks.toml", &input).expect("valid tasks");

    let task = next_task(&tasks, &marker_filter(Some("parallel"))).expect("next task");

    assert_eq!(task.id.to_string(), "75");
}
