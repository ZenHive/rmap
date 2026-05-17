//! Pure selector: pick one session-sized bundle and emit every actionable
//! pending task in it, dep-topologically ordered. Backs `rmap next-bundle`.
//!
//! Ranking (default branch):
//! 1. Determine effective focus phase: `filter.phase.or(tasks.focus.phase)`.
//! 2. For every `[bundles.<name>]`, compute the broad-actionable set: pending
//!    tasks whose every dep is either `done` (anywhere) or an in-bundle
//!    pending task that is itself broadly actionable (recursive). Skip
//!    bundles with an empty actionable set.
//! 3. Sort candidates by `(in_focus_phase desc, sum_eff desc, bundle.order asc)`.
//! 4. Pick the first remaining bundle.
//!
//! Force-pick branch (`filter.bundle = Some(name)`):
//! - If the bundle is declared, return it with its (possibly empty) actionable
//!   set topologically ordered.
//! - If the bundle is not declared, returns `None` — the caller is expected
//!   to error with a clear "bundle not declared" message (the CLI does so).
//!
//! The `today` parameter mirrors `next::next_tasks` for future score-decay-
//! sensitive ranking; unused today.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::query::matches_bundle;
use crate::schema::{Bundle, Task, TaskId, Tasks};
use crate::scoring::efficiency;

#[derive(Debug, Default)]
pub struct NextBundleFilter {
    pub phase: Option<u32>,
    pub bundle: Option<String>,
}

#[derive(Debug)]
pub struct BundlePick<'a> {
    pub name: &'a str,
    pub bundle: &'a Bundle,
    pub tasks: Vec<&'a Task>,
}

pub fn pick<'a>(
    tasks: &'a Tasks,
    filter: &NextBundleFilter,
    _today: &str,
) -> Option<BundlePick<'a>> {
    if let Some(requested) = filter.bundle.as_deref() {
        let (name, bundle) = tasks.bundles.get_key_value(requested)?;
        let actionable = compute_actionable(tasks, name.as_str());
        let ordered = topo_order(&actionable);
        return Some(BundlePick {
            name: name.as_str(),
            bundle,
            tasks: ordered,
        });
    }

    let effective_focus = filter.phase.or(tasks.focus.as_ref().map(|f| f.phase));

    let mut candidates: Vec<Candidate<'a>> = tasks
        .bundles
        .iter()
        .filter_map(|(name, bundle)| {
            let actionable = compute_actionable(tasks, name);
            if actionable.is_empty() {
                return None;
            }
            let sum_eff: f64 = actionable.iter().map(|t| efficiency(t)).sum();
            let in_focus = effective_focus == Some(bundle.phase);
            Some(Candidate {
                name: name.as_str(),
                bundle,
                actionable,
                sum_eff,
                in_focus,
            })
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.in_focus
            .cmp(&a.in_focus)
            .then_with(|| b.sum_eff.partial_cmp(&a.sum_eff).unwrap_or(Ordering::Equal))
            .then_with(|| a.bundle.order.cmp(&b.bundle.order))
    });

    let winner = candidates.into_iter().next()?;
    let ordered = topo_order(&winner.actionable);
    Some(BundlePick {
        name: winner.name,
        bundle: winner.bundle,
        tasks: ordered,
    })
}

struct Candidate<'a> {
    name: &'a str,
    bundle: &'a Bundle,
    actionable: Vec<&'a Task>,
    sum_eff: f64,
    in_focus: bool,
}

fn compute_actionable<'a>(tasks: &'a Tasks, bundle_name: &str) -> Vec<&'a Task> {
    let mut memo: HashMap<&'a TaskId, bool> = HashMap::new();
    let mut result: Vec<&'a Task> = Vec::new();
    for task in tasks
        .task
        .iter()
        .filter(|t| matches_bundle(t, Some(bundle_name)))
    {
        if task.status != "pending" {
            continue;
        }
        if is_actionable(task, tasks, bundle_name, &mut memo, &mut HashSet::new()) {
            result.push(task);
        }
    }
    result
}

fn is_actionable<'a>(
    task: &'a Task,
    tasks: &'a Tasks,
    bundle_name: &str,
    memo: &mut HashMap<&'a TaskId, bool>,
    visiting: &mut HashSet<&'a TaskId>,
) -> bool {
    if let Some(&cached) = memo.get(&task.id) {
        return cached;
    }
    if !visiting.insert(&task.id) {
        // Cycle — `validate` should reject these, but be defensive.
        return false;
    }

    let mut ok = task.status == "pending";
    if ok {
        for dep_id in &task.depends_on {
            let Some(dep_task) = tasks.task.iter().find(|t| &t.id == dep_id) else {
                ok = false;
                break;
            };
            if dep_task.status == "done" {
                continue;
            }
            // Dep isn't done — must be in-bundle, pending, and itself actionable.
            if dep_task.bundle != bundle_name {
                ok = false;
                break;
            }
            if dep_task.status != "pending" {
                ok = false;
                break;
            }
            if !is_actionable(dep_task, tasks, bundle_name, memo, visiting) {
                ok = false;
                break;
            }
        }
    }

    visiting.remove(&task.id);
    memo.insert(&task.id, ok);
    ok
}

fn topo_order<'a>(actionable: &[&'a Task]) -> Vec<&'a Task> {
    if actionable.is_empty() {
        return Vec::new();
    }

    let actionable_ids: HashSet<&'a TaskId> = actionable.iter().map(|t| &t.id).collect();

    let mut in_degree: HashMap<&'a TaskId, usize> = HashMap::new();
    for t in actionable {
        let count = t
            .depends_on
            .iter()
            .filter(|d| actionable_ids.contains(d))
            .count();
        in_degree.insert(&t.id, count);
    }

    let mut order: Vec<&'a Task> = actionable.to_vec();
    order.sort_by(|a, b| {
        efficiency(b)
            .partial_cmp(&efficiency(a))
            .unwrap_or(Ordering::Equal)
    });

    let mut result: Vec<&'a Task> = Vec::with_capacity(actionable.len());
    let mut emitted: HashSet<&'a TaskId> = HashSet::new();

    while result.len() < actionable.len() {
        let mut emitted_one = false;
        for t in &order {
            if emitted.contains(&t.id) {
                continue;
            }
            if in_degree[&t.id] == 0 {
                result.push(*t);
                emitted.insert(&t.id);
                for other in &order {
                    if emitted.contains(&other.id) {
                        continue;
                    }
                    if other.depends_on.contains(&t.id) {
                        *in_degree
                            .get_mut(&other.id)
                            .expect("in_degree has every id") -= 1;
                    }
                }
                emitted_one = true;
                break;
            }
        }
        if !emitted_one {
            // Cycle — `validate` should reject; defensive break.
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_tasks_str;

    fn parse(toml: &str) -> Tasks {
        validate_tasks_str("test.toml".to_string(), toml).expect("valid fixture")
    }

    const FOCUS_VS_OTHER_PHASE: &str = r#"
schema_version = 2
project = "p"
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
    fn focus_phase_beats_higher_eff_other_phase() {
        let tasks = parse(FOCUS_VS_OTHER_PHASE);
        let pick = pick(&tasks, &NextBundleFilter::default(), "2026-05-14");
        let pick = pick.expect("a bundle is picked");
        assert_eq!(pick.name, "focus_b");
        assert_eq!(pick.tasks.len(), 1);
        assert_eq!(pick.tasks[0].id, 1u32);
    }

    #[test]
    fn topo_order_respects_internal_chain() {
        let toml = r#"
schema_version = 2
project = "p"
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
        let tasks = parse(toml);
        let pick = pick(&tasks, &NextBundleFilter::default(), "2026-05-14");
        let pick = pick.expect("a bundle is picked");
        assert_eq!(pick.name, "chain");
        // Task 2 has higher Eff but depends on 1, so 1 emits first.
        assert_eq!(pick.tasks.len(), 2);
        assert_eq!(pick.tasks[0].id, 1u32);
        assert_eq!(pick.tasks[1].id, 2u32);
    }

    #[test]
    fn all_blocked_bundle_skipped() {
        let toml = r#"
schema_version = 2
project = "p"
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
        let tasks = parse(toml);
        let pick = pick(&tasks, &NextBundleFilter::default(), "2026-05-14");
        let pick = pick.expect("healthy bundle picked");
        assert_eq!(pick.name, "healthy");
    }

    #[test]
    fn unmet_external_dep_skips_bundle() {
        let toml = r#"
schema_version = 2
project = "p"
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
        let tasks = parse(toml);
        let pick = pick(&tasks, &NextBundleFilter::default(), "2026-05-14");
        let pick = pick.expect("open bundle picked");
        assert_eq!(pick.name, "open");
    }

    #[test]
    fn force_pick_overrides_ranking() {
        let toml = r#"
schema_version = 2
project = "p"
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
        let tasks = parse(toml);
        let filter = NextBundleFilter {
            bundle: Some("low".to_string()),
            ..Default::default()
        };
        let pick = pick(&tasks, &filter, "2026-05-14");
        let pick = pick.expect("forced low bundle");
        assert_eq!(pick.name, "low");
        assert_eq!(pick.tasks[0].id, 2u32);
    }

    #[test]
    fn ties_broken_by_bundle_order() {
        let toml = r#"
schema_version = 2
project = "p"
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
        let tasks = parse(toml);
        let pick = pick(&tasks, &NextBundleFilter::default(), "2026-05-14");
        let pick = pick.expect("a bundle is picked");
        // Same sum_eff; bundle.order=1 (beta) wins over order=2 (alpha).
        assert_eq!(pick.name, "beta");
    }

    #[test]
    fn missing_bundle_returns_none() {
        let tasks = parse(FOCUS_VS_OTHER_PHASE);
        let filter = NextBundleFilter {
            bundle: Some("ghost".to_string()),
            ..Default::default()
        };
        assert!(pick(&tasks, &filter, "2026-05-14").is_none());
    }
}
