//! Longest-path topological layering over the in-repo `depends_on` graph.
//!
//! `layer(n) = 0` when `n` has no in-repo dependency, else
//! `max(layer(dep)) + 1` over its in-repo deps. This is the single source of
//! truth for "how many dependency waves deep is this task," consumed by:
//!
//! - `render_html::build_dag` — vertical slotting of the DAG view.
//! - `export::exported_task` — the computed `dep_layer` field on every
//!   `--json` payload (like `eff`, never persisted).
//! - `graph_export` — `rmap export dot` and `rmap waves` over the same layers.
//!
//! It also hosts the reverse-edge traversal that backs the graph queries:
//! [`transitive_dependents`] (`rmap blocks` — what a task unblocks),
//! [`transitive_dependencies`] (`rmap deps` — what a task needs first), and
//! [`compute_unlocks`] (the computed `unlocks` leverage field on `--json`).
//! All share the canonical-keyed adjacency builders below and reuse the same
//! known-id filtering rule as the layering walk.
//!
//! The map is keyed by the canonical `TaskId` string (`TaskId::to_string()`),
//! which normalizes `Number(n)` ↔ `Text("n")` so a mixed-form file layers
//! correctly. `cross_repo` deps are intentionally ignored — only in-repo
//! `depends_on` edges shape the layout. `validate` already rejects dependency
//! cycles, but `layer_of` carries its own cycle guard because the layering is
//! reachable from public entry points that may run on an unvalidated `Tasks`.

use std::collections::{HashMap, HashSet};

use crate::schema::{Task, TaskId};

/// Longest-path layering: `layer(n) = 0` when `n` has no in-repo dep, else
/// `max(layer(dep)) + 1`. Memoized; safe on an acyclic graph and cycle-guarded
/// otherwise. Keyed by `task.id.to_string()` (canonical `TaskId` form).
pub fn compute_layers(tasks: &[&Task]) -> HashMap<String, usize> {
    let edges: Vec<(String, Vec<String>)> = tasks
        .iter()
        .map(|task| {
            (
                task.id.to_string(),
                task.depends_on.iter().map(TaskId::to_string).collect(),
            )
        })
        .collect();
    compute_layers_from_edges(&edges)
}

/// Layering over a pre-extracted `(id, depends_on)` edge list — the source-of-
/// truth shared by [`compute_layers`] (over `schema::Task`) and the portfolio
/// HTML view (over JSON-loaded tasks). Ids are canonical strings; deps naming
/// unknown ids are ignored, exactly like the in-repo `depends_on` walk.
pub fn compute_layers_from_edges(nodes: &[(String, Vec<String>)]) -> HashMap<String, usize> {
    let by_id: HashMap<&str, &Vec<String>> =
        nodes.iter().map(|(id, deps)| (id.as_str(), deps)).collect();
    let mut layers: HashMap<String, usize> = HashMap::new();
    let mut visiting: HashSet<String> = HashSet::new();
    for (id, _) in nodes {
        layer_of(id, &by_id, &mut layers, &mut visiting);
    }
    layers
}

fn layer_of(
    id: &str,
    by_id: &HashMap<&str, &Vec<String>>,
    layers: &mut HashMap<String, usize>,
    visiting: &mut HashSet<String>,
) -> usize {
    if let Some(&layer) = layers.get(id) {
        return layer;
    }
    let Some(deps) = by_id.get(id) else {
        return 0; // unknown id — treat as a root (validate should prevent this)
    };
    // Cycle defense: `validate` already rejects dependency cycles, but the
    // layering is reachable from public entry points that may be called on an
    // unvalidated `Tasks` — without this guard a cycle is infinite recursion.
    if !visiting.insert(id.to_string()) {
        return 0;
    }
    let mut layer = 0;
    for dep_id in deps.iter() {
        if by_id.contains_key(dep_id.as_str()) {
            layer = layer.max(layer_of(dep_id, by_id, layers, visiting) + 1);
        }
    }
    visiting.remove(id);
    layers.insert(id.to_string(), layer);
    layer
}

/// `id → depends_on` forward edges, canonical-keyed, restricted to known ids
/// (deps naming an unknown id are dropped — same rule as the layering walk).
pub fn forward_adjacency(tasks: &[&Task]) -> HashMap<String, Vec<String>> {
    let known: HashSet<String> = tasks.iter().map(|t| t.id.to_string()).collect();
    tasks
        .iter()
        .map(|task| {
            let deps = task
                .depends_on
                .iter()
                .map(TaskId::to_string)
                .filter(|dep| known.contains(dep))
                .collect();
            (task.id.to_string(), deps)
        })
        .collect()
}

/// `id → dependents` reverse edges (who lists `id` in their `depends_on`). The
/// inverse of [`forward_adjacency`]; the source of truth for "what does this
/// task unblock?" queries. Every known id gets an entry (possibly empty).
fn reverse_adjacency(tasks: &[&Task]) -> HashMap<String, Vec<String>> {
    let mut rev: HashMap<String, Vec<String>> = tasks
        .iter()
        .map(|t| (t.id.to_string(), Vec::new()))
        .collect();
    for (id, deps) in forward_adjacency(tasks) {
        for dep in deps {
            // `dep` is known (forward_adjacency filtered), so the entry exists.
            if let Some(dependents) = rev.get_mut(&dep) {
                dependents.push(id.clone());
            }
        }
    }
    rev
}

/// Set of ids reachable from `start` over `adjacency`, excluding `start`
/// itself. Iterative BFS — safe on a cyclic graph (the `seen` set is the
/// guard), so it never recurses unboundedly even on an unvalidated `Tasks`.
fn reachable_from(start: &str, adjacency: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = vec![start.to_string()];
    while let Some(node) = stack.pop() {
        if let Some(neighbors) = adjacency.get(&node) {
            for next in neighbors {
                if seen.insert(next.clone()) {
                    stack.push(next.clone());
                }
            }
        }
    }
    seen.remove(start); // a cycle could route back to start; never include it
    seen
}

/// Order an id set into `&Task` refs by `(dep_layer asc, numeric-or-lexical id)`
/// — nearest dependency wave first, deterministic within a wave.
fn ordered_tasks<'a>(tasks: &[&'a Task], ids: &HashSet<String>) -> Vec<&'a Task> {
    let layers = compute_layers(tasks);
    let mut out: Vec<&Task> = tasks
        .iter()
        .copied()
        .filter(|t| ids.contains(&t.id.to_string()))
        .collect();
    out.sort_by_key(|t| {
        let key = t.id.to_string();
        let layer = layers.get(&key).copied().unwrap_or(0);
        let numeric = key.parse::<u64>().unwrap_or(u64::MAX);
        (layer, numeric, key)
    });
    out
}

/// Every task that transitively depends on `id` — the subtree `id` unblocks
/// ("what frees up if I finish this?"). Empty when `id` is unknown or a leaf.
/// Ordered nearest-wave-first via [`ordered_tasks`].
pub fn transitive_dependents<'a>(tasks: &[&'a Task], id: &str) -> Vec<&'a Task> {
    let reachable = reachable_from(id, &reverse_adjacency(tasks));
    ordered_tasks(tasks, &reachable)
}

/// Every task `id` transitively depends on — its full prerequisite set ("what
/// must finish before this?"). Empty when `id` is unknown or has no deps.
/// Ordered nearest-wave-first via [`ordered_tasks`].
pub fn transitive_dependencies<'a>(tasks: &[&'a Task], id: &str) -> Vec<&'a Task> {
    let reachable = reachable_from(id, &forward_adjacency(tasks));
    ordered_tasks(tasks, &reachable)
}

/// Per-task transitive-dependent count keyed by canonical `TaskId` string — the
/// computed `unlocks` leverage field (number of tasks each task unblocks). Built
/// from the whole graph; like `eff` / `dep_layer`, never persisted.
pub fn compute_unlocks(tasks: &[&Task]) -> HashMap<String, usize> {
    let reverse = reverse_adjacency(tasks);
    tasks
        .iter()
        .map(|task| {
            let id = task.id.to_string();
            let count = reachable_from(&id, &reverse).len();
            (id, count)
        })
        .collect()
}

/// Ids reachable forward from any milestone-pinned task (following in-repo
/// `depends_on`), including the pinned seeds themselves. Empty when no tasks
/// are pinned to a known milestone.
pub fn milestone_reachable_ids(tasks: &[&Task]) -> HashSet<String> {
    let forward = forward_adjacency(tasks);
    let mut reachable = HashSet::new();
    for task in tasks {
        if task.milestone.is_none() {
            continue;
        }
        let id = task.id.to_string();
        reachable.insert(id.clone());
        reachable.extend(reachable_from(&id, &forward));
    }
    reachable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Tasks;

    const BASE: &str = "schema_version = 2\n\
        project = \"demo\"\n\
        default_branch = \"main\"\n\
        [phases.1]\n\
        name = \"Phase One\"\n\
        order = 1\n\
        status = \"in_progress\"\n";

    fn task_toml(id: u32, status: &str, deps: &[u32]) -> String {
        let deps_str = if deps.is_empty() {
            String::new()
        } else {
            let joined = deps
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("depends_on = [{joined}]\n")
        };
        format!(
            "[[task]]\n\
             id = {id}\n\
             phase = 1\n\
             bundle = \"b\"\n\
             status = \"{status}\"\n\
             title = \"Task {id}\"\n\
             scores = {{ d = 2, b = 3, u = 4 }}\n\
             {deps_str}"
        )
    }

    fn layers_from(body: &str) -> HashMap<String, usize> {
        let tasks: Tasks = toml::from_str(&format!("{BASE}{body}")).expect("valid tasks toml");
        let refs: Vec<&Task> = tasks.task.iter().collect();
        compute_layers(&refs)
    }

    #[test]
    fn layer_no_deps_is_zero() {
        let layers = layers_from(&task_toml(1, "pending", &[]));
        assert_eq!(layers.get("1"), Some(&0));
    }

    #[test]
    fn layer_chain_of_three() {
        let body = format!(
            "{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[2]),
        );
        let layers = layers_from(&body);
        assert_eq!(layers.get("1"), Some(&0));
        assert_eq!(layers.get("2"), Some(&1));
        assert_eq!(layers.get("3"), Some(&2));
    }

    #[test]
    fn layer_diamond_uses_longest_path() {
        // 1 → 2, 1 → 3, 2 → 4, 3 → 4
        let body = format!(
            "{}{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[1]),
            task_toml(4, "pending", &[2, 3]),
        );
        let layers = layers_from(&body);
        assert_eq!(layers.get("1"), Some(&0));
        assert_eq!(layers.get("2"), Some(&1));
        assert_eq!(layers.get("3"), Some(&1));
        assert_eq!(layers.get("4"), Some(&2));
    }

    fn tasks_from(body: &str) -> Tasks {
        toml::from_str(&format!("{BASE}{body}")).expect("valid tasks toml")
    }

    fn ids(tasks: &[&Task]) -> Vec<String> {
        tasks.iter().map(|t| t.id.to_string()).collect()
    }

    #[test]
    fn dependents_chain_is_whole_downstream_subtree() {
        // 1 ← 2 ← 3
        let body = format!(
            "{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[2]),
        );
        let tasks = tasks_from(&body);
        let refs: Vec<&Task> = tasks.task.iter().collect();
        // 1 unblocks 2 and 3 (nearest wave first).
        assert_eq!(ids(&transitive_dependents(&refs, "1")), vec!["2", "3"]);
        // 3 is a leaf — unblocks nothing.
        assert!(transitive_dependents(&refs, "3").is_empty());
    }

    #[test]
    fn dependencies_chain_is_whole_upstream_subtree() {
        let body = format!(
            "{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[2]),
        );
        let tasks = tasks_from(&body);
        let refs: Vec<&Task> = tasks.task.iter().collect();
        // 3 depends (transitively) on 1 and 2.
        assert_eq!(ids(&transitive_dependencies(&refs, "3")), vec!["1", "2"]);
        // 1 is a root — depends on nothing.
        assert!(transitive_dependencies(&refs, "1").is_empty());
    }

    #[test]
    fn diamond_dedups_dependents_and_unlocks() {
        // 1 → 2, 1 → 3, 2 → 4, 3 → 4: 1 reaches {2,3,4}, counted once.
        let body = format!(
            "{}{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[1]),
            task_toml(4, "pending", &[2, 3]),
        );
        let tasks = tasks_from(&body);
        let refs: Vec<&Task> = tasks.task.iter().collect();
        assert_eq!(ids(&transitive_dependents(&refs, "1")), vec!["2", "3", "4"]);
        let unlocks = compute_unlocks(&refs);
        assert_eq!(unlocks.get("1"), Some(&3)); // not 4 — 4 reached via two paths
        assert_eq!(unlocks.get("2"), Some(&1));
        assert_eq!(unlocks.get("4"), Some(&0));
    }

    #[test]
    fn traversal_on_unknown_id_is_empty() {
        let tasks = tasks_from(&task_toml(1, "pending", &[]));
        let refs: Vec<&Task> = tasks.task.iter().collect();
        assert!(transitive_dependents(&refs, "999").is_empty());
        assert!(transitive_dependencies(&refs, "999").is_empty());
    }
}
