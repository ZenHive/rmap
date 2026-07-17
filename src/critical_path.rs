//! Longest dependency chain (critical path) through the in-repo `depends_on` DAG.
//!
//! Backs `rmap critical-path`. Reuses [`crate::topo::compute_layers`] for depth,
//! then backtracks from the deepest task to emit the ordered root→leaf sequence.
//! `--milestone` scopes to that release line's pinned tasks plus every transitive
//! in-repo dependency.

use std::collections::{HashMap, HashSet};

use crate::query::{format_task_row, matches_milestone};
use crate::schema::{Task, Tasks};
use crate::topo::compute_layers;

#[derive(Debug, Default)]
pub struct CriticalPathFilter {
    pub milestone: Option<String>,
}

/// Returns the longest in-repo dependency chain as tasks ordered root → leaf.
/// Path length is task count (not Eff-weighted). An empty graph yields `[]`.
pub fn critical_path<'a>(tasks: &'a Tasks, filter: &CriticalPathFilter) -> Vec<&'a Task> {
    let scoped = scoped_tasks(tasks, filter);
    if scoped.is_empty() {
        return Vec::new();
    }

    let by_id: HashMap<String, &Task> = scoped
        .iter()
        .map(|task| (task.id.to_string(), *task))
        .collect();
    let scope: HashSet<String> = by_id.keys().cloned().collect();

    let layers = compute_layers(&scoped);
    let max_layer = layers.values().copied().max().unwrap_or(0);

    let Some(leaf) = scoped
        .iter()
        .filter(|task| layers.get(&task.id.to_string()) == Some(&max_layer))
        .min_by_key(|task| task_index(tasks, task.id.to_string()))
        .copied()
    else {
        return Vec::new();
    };

    let mut chain = vec![leaf];
    let mut current_layer = max_layer;

    while current_layer > 0 {
        let current = chain.last().copied().expect("non-empty chain");
        let target_layer = current_layer - 1;
        let Some(pred) = current
            .depends_on
            .iter()
            .map(|dep| dep.to_string())
            .filter(|dep| scope.contains(dep))
            .filter(|dep| layers.get(dep) == Some(&target_layer))
            .min_by_key(|dep| task_index(tasks, dep.clone()))
        else {
            break;
        };

        let pred_task = by_id.get(&pred).copied().expect("scoped pred");
        chain.push(pred_task);
        current_layer = target_layer;
    }

    chain.reverse();
    chain
}

fn scoped_tasks<'a>(tasks: &'a Tasks, filter: &CriticalPathFilter) -> Vec<&'a Task> {
    match filter.milestone.as_deref() {
        None => tasks.task.iter().collect(),
        Some(milestone) => {
            let ids = milestone_closure(tasks, milestone);
            tasks
                .task
                .iter()
                .filter(|task| ids.contains(&task.id.to_string()))
                .collect()
        }
    }
}

fn milestone_closure(tasks: &Tasks, milestone: &str) -> HashSet<String> {
    let by_id: HashMap<String, &Task> = tasks
        .task
        .iter()
        .map(|task| (task.id.to_string(), task))
        .collect();

    let mut included = HashSet::new();
    let mut stack: Vec<String> = tasks
        .task
        .iter()
        .filter(|task| matches_milestone(task, Some(milestone)))
        .map(|task| task.id.to_string())
        .collect();

    while let Some(id) = stack.pop() {
        if !included.insert(id.clone()) {
            continue;
        }
        if let Some(task) = by_id.get(&id) {
            for dep in &task.depends_on {
                let dep_id = dep.to_string();
                if by_id.contains_key(&dep_id) {
                    stack.push(dep_id);
                }
            }
        }
    }

    included
}

fn task_index(tasks: &Tasks, id: String) -> usize {
    tasks
        .task
        .iter()
        .position(|task| task.id.to_string() == id)
        .unwrap_or(usize::MAX)
}

/// Human view: header plus one row per hop with status and Eff glyphs.
pub fn format_critical_path_human(chain: &[&Task]) -> String {
    if chain.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("critical path ({} tasks):", chain.len())];
    for (index, task) in chain.iter().enumerate() {
        let prefix = if index == 0 { "  " } else { "→ " };
        lines.push(format!("{prefix}{}", format_task_row(task)));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Scores, Task, TaskId, Tasks};
    use std::collections::BTreeMap;

    fn make_task(id: u32, status: &str, deps: &[u32], milestone: Option<&str>) -> Task {
        Task {
            id: TaskId::Number(id),
            phase: 1,
            bundle: "b".to_string(),
            milestone: milestone.map(String::from),
            status: status.to_string(),
            title: format!("Task {id}"),
            scores: Scores { d: 2, b: 3, u: 4 },
            markers: vec![],
            depends_on: deps.iter().copied().map(TaskId::Number).collect(),
            linear_id: None,
            assignee: None,
            module: None,
            branch: None,
            model: None,
            acceptance_criteria: vec![],
            out_of_scope: vec![],
            files_to_modify: vec![],
            touches: vec![],
            domains: vec![],
            shipped_in: None,
            body: None,
            created_at: None,
            started_at: None,
            done_at: None,
            scored_at: None,
            blocked_reason: None,
            implemented: None,
            delivered_by: None,
            verified: None,
            verified_by: None,
            verification_ref: None,
            attempts: vec![],
            cross_repo: vec![],
        }
    }

    fn make_tasks(task: Vec<Task>) -> Tasks {
        Tasks {
            schema_version: 2,
            project: "demo".to_string(),
            default_branch: "main".to_string(),
            vision: None,
            focus: None,
            linear: None,
            phases: BTreeMap::new(),
            bundles: BTreeMap::new(),
            milestones: BTreeMap::new(),
            task,
        }
    }

    fn ids(chain: &[&Task]) -> Vec<u32> {
        chain
            .iter()
            .map(|task| match task.id {
                TaskId::Number(n) => n,
                TaskId::Text(ref s) => s.parse().unwrap_or(0),
            })
            .collect()
    }

    #[test]
    fn critical_path_chain_of_three() {
        let tasks = make_tasks(vec![
            make_task(1, "done", &[], None),
            make_task(2, "pending", &[1], None),
            make_task(3, "pending", &[2], None),
        ]);
        let chain = critical_path(&tasks, &CriticalPathFilter::default());
        assert_eq!(ids(&chain), vec![1, 2, 3]);
    }

    #[test]
    fn critical_path_diamond_picks_longer_arm() {
        // 1 → 2 → 3 → 5
        // 1 → 4 → 5
        let tasks = make_tasks(vec![
            make_task(1, "done", &[], None),
            make_task(2, "pending", &[1], None),
            make_task(3, "pending", &[2], None),
            make_task(4, "pending", &[1], None),
            make_task(5, "pending", &[3, 4], None),
        ]);
        let chain = critical_path(&tasks, &CriticalPathFilter::default());
        assert_eq!(ids(&chain), vec![1, 2, 3, 5]);
    }

    #[test]
    fn critical_path_milestone_scopes_to_pinned_and_transitive_deps() {
        let tasks = make_tasks(vec![
            make_task(1, "done", &[], None),
            make_task(2, "done", &[1], None),
            make_task(3, "pending", &[2], Some("v1")),
            make_task(4, "pending", &[1], None),
        ]);
        let filter = CriticalPathFilter {
            milestone: Some("v1".to_string()),
        };
        let chain = critical_path(&tasks, &filter);
        assert_eq!(ids(&chain), vec![1, 2, 3]);
    }

    #[test]
    fn critical_path_single_root_task() {
        let tasks = make_tasks(vec![make_task(1, "pending", &[], None)]);
        let chain = critical_path(&tasks, &CriticalPathFilter::default());
        assert_eq!(ids(&chain), vec![1]);
    }

    #[test]
    fn critical_path_empty_when_milestone_has_no_tasks() {
        let tasks = make_tasks(vec![make_task(1, "pending", &[], None)]);
        let filter = CriticalPathFilter {
            milestone: Some("missing".to_string()),
        };
        let chain = critical_path(&tasks, &filter);
        assert!(chain.is_empty());
    }
}
