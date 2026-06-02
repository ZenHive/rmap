//! Longest-path topological layering over the in-repo `depends_on` graph.
//!
//! `layer(n) = 0` when `n` has no in-repo dependency, else
//! `max(layer(dep)) + 1` over its in-repo deps. This is the single source of
//! truth for "how many dependency waves deep is this task," consumed by:
//!
//! - `render_html::build_dag` — vertical slotting of the DAG view.
//! - `export::exported_task` — the computed `dep_layer` field on every
//!   `--json` payload (like `eff`, never persisted).
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
}
