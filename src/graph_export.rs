//! Read-only graph export surfaces over the in-repo `depends_on` graph.
//!
//! - [`build_waves`] / [`format_waves`] — parallel dispatch schedule by `dep_layer`.
//! - [`build_dot`] / [`format_dot`] — Graphviz DOT digraph for external tooling.

use std::collections::BTreeMap;

use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, tier_glyph};
use crate::topo::{compute_layers, forward_adjacency};

/// Tasks grouped by longest-path `dep_layer` — wave 0 is the roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waves {
    pub layers: BTreeMap<usize, Vec<String>>,
}

/// One node in the dependency digraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotNode {
    pub id: String,
    pub label: String,
    pub fillcolor: &'static str,
}

/// One directed edge: dependency → dependent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotEdge {
    pub from: String,
    pub to: String,
}

/// Pure digraph builder output — nodes and in-repo edges only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotGraph {
    pub name: String,
    pub nodes: Vec<DotNode>,
    pub edges: Vec<DotEdge>,
}

/// Group every task id by [`compute_layers`] depth.
pub fn build_waves(tasks: &Tasks) -> Waves {
    let refs: Vec<&Task> = tasks.task.iter().collect();
    let layers = compute_layers(&refs);
    let mut by_layer: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for task in &tasks.task {
        let id = task.id.to_string();
        let layer = layers.get(&id).copied().unwrap_or(0);
        by_layer.entry(layer).or_default().push(id);
    }
    for ids in by_layer.values_mut() {
        sort_ids(ids);
    }
    Waves { layers: by_layer }
}

/// Human-readable wave schedule: `wave 0: [1, 3]` per line.
pub fn format_waves(waves: &Waves) -> String {
    if waves.layers.is_empty() {
        return String::new();
    }
    waves
        .layers
        .iter()
        .map(|(layer, ids)| format!("wave {layer}: [{ids}]", ids = format_id_list(ids)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// JSON map of layer index → sorted task-id strings.
pub fn format_waves_json(waves: &Waves) -> String {
    let map: BTreeMap<String, &Vec<String>> = waves
        .layers
        .iter()
        .map(|(layer, ids)| (layer.to_string(), ids))
        .collect();
    serde_json::to_string(&map).expect("waves JSON serializes")
}

/// Build a status/Eff-styled DOT digraph over in-repo `depends_on` edges.
pub fn build_dot(tasks: &Tasks) -> DotGraph {
    let refs: Vec<&Task> = tasks.task.iter().collect();
    let adjacency = forward_adjacency(&refs);
    let by_id: BTreeMap<String, &Task> = tasks.task.iter().map(|t| (t.id.to_string(), t)).collect();

    let mut nodes = Vec::with_capacity(by_id.len());
    for (id, task) in &by_id {
        let eff = efficiency(task);
        nodes.push(DotNode {
            id: id.clone(),
            label: format!(
                "{}\\n{} Eff:{} {}",
                dot_escape(id),
                task.status,
                format_efficiency(eff),
                tier_glyph(eff)
            ),
            fillcolor: status_fillcolor(&task.status),
        });
    }
    nodes.sort_by(|a, b| cmp_ids(&a.id, &b.id));

    let mut edges = Vec::new();
    for (id, deps) in adjacency {
        for dep in deps {
            edges.push(DotEdge {
                from: dep,
                to: id.clone(),
            });
        }
    }
    edges.sort_by(|a, b| cmp_ids(&a.from, &b.from).then_with(|| cmp_ids(&a.to, &b.to)));

    DotGraph {
        name: dot_escape(&tasks.project),
        nodes,
        edges,
    }
}

/// Render a [`DotGraph`] as a Graphviz DOT digraph string.
pub fn format_dot(graph: &DotGraph) -> String {
    let mut out = String::from("digraph \"");
    out.push_str(&graph.name);
    out.push_str("\" {\n  graph [rankdir=TB];\n  node [shape=box style=\"filled,rounded\"];\n");
    for node in &graph.nodes {
        out.push_str(&format!(
            "  \"{}\" [label=\"{}\" fillcolor=\"{}\"];\n",
            dot_escape(&node.id),
            node.label,
            node.fillcolor
        ));
    }
    for edge in &graph.edges {
        out.push_str(&format!(
            "  \"{}\" -> \"{}\";\n",
            dot_escape(&edge.from),
            dot_escape(&edge.to)
        ));
    }
    out.push('}');
    out
}

fn status_fillcolor(status: &str) -> &'static str {
    match status {
        "done" => "#34d399",
        "in_progress" => "#fbbf24",
        "blocked" => "#fb7185",
        _ => "#94a3b8", // pending, superseded, unknown
    }
}

fn sort_ids(ids: &mut [String]) {
    ids.sort_by(|a, b| cmp_ids(a, b));
}

fn cmp_ids(a: &str, b: &str) -> std::cmp::Ordering {
    let na = a.parse::<u64>().unwrap_or(u64::MAX);
    let nb = b.parse::<u64>().unwrap_or(u64::MAX);
    (na, a).cmp(&(nb, b))
}

fn format_id_list(ids: &[String]) -> String {
    ids.join(", ")
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
        let mut block = format!(
            "[[task]]\n\
             id = {id}\n\
             phase = 1\n\
             bundle = \"b\"\n\
             status = \"{status}\"\n\
             title = \"Task {id}\"\n\
             scores = {{ d = 2, b = 3, u = 4 }}\n\
             {deps_str}"
        );
        if status == "done" {
            block.push_str("implemented = \"fixture\"\n");
        }
        block
    }

    fn tasks_from(body: &str) -> Tasks {
        toml::from_str(&format!("{BASE}{body}")).expect("valid tasks toml")
    }

    #[test]
    fn build_waves_groups_by_dep_layer() {
        let body = format!(
            "{}{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[1]),
            task_toml(4, "pending", &[2, 3]),
        );
        let waves = build_waves(&tasks_from(&body));
        assert_eq!(waves.layers.get(&0), Some(&vec!["1".into()]));
        assert_eq!(waves.layers.get(&1), Some(&vec!["2".into(), "3".into()]));
        assert_eq!(waves.layers.get(&2), Some(&vec!["4".into()]));
    }

    #[test]
    fn format_waves_human_lines() {
        let body = format!(
            "{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
        );
        let waves = build_waves(&tasks_from(&body));
        assert_eq!(format_waves(&waves), "wave 0: [1]\nwave 1: [2]");
    }

    #[test]
    fn format_waves_json_is_layer_map() {
        let body = format!(
            "{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
        );
        let waves = build_waves(&tasks_from(&body));
        let json: serde_json::Value =
            serde_json::from_str(&format_waves_json(&waves)).expect("valid json");
        assert_eq!(json["0"], serde_json::json!(["1"]));
        assert_eq!(json["1"], serde_json::json!(["2"]));
    }

    #[test]
    fn build_dot_emits_in_repo_edges_only() {
        let body = format!(
            "{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
        );
        let graph = build_dot(&tasks_from(&body));
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(
            graph.edges,
            vec![DotEdge {
                from: "1".into(),
                to: "2".into()
            }]
        );
    }

    #[test]
    fn format_dot_is_graphviz_digraph_with_styled_nodes() {
        let body = format!(
            "{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
        );
        let dot = format_dot(&build_dot(&tasks_from(&body)));
        assert!(dot.starts_with("digraph \"demo\" {"));
        assert!(dot.contains("fillcolor=\"#34d399\""));
        assert!(dot.contains("fillcolor=\"#94a3b8\""));
        assert!(dot.contains("\"1\" -> \"2\";"));
        assert!(dot.ends_with('}'));
    }
}
