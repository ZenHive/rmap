//! `rmap render --html` — the single-project static HTML view.
//!
//! Pipeline: `schema::Tasks` → view structs (`PhaseView` / `TaskView` / `DagView`)
//! → minijinja render of `templates/roadmap.html.j2` (inlined via `include_str!`,
//! so the template ships inside the binary). The output is a self-contained
//! single file: inline CSS + vanilla JS, no CDN, no framework, no build step.
//!
//! The `<script id="rmap-data">` island carries the full compact `data.json`
//! so agents read structured data without DOM scraping. Every task card carries
//! six `data-*` attributes (`data-id` / `data-status` / `data-eff` /
//! `data-markers` / `data-depends-on` / `data-phase`) — the agent selector
//! contract.
//!
//! Out of scope here (deliberate): `--multi`/portfolio view is Task 9; this
//! module is single-project only.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;

use crate::export::export_compact_json_str;
use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, round_eff, tier_glyph};

/// Dependency-graph layout constants (SVG user units).
const NODE_W: f64 = 140.0;
const NODE_H: f64 = 40.0;
const H_GAP: f64 = 24.0;
const V_GAP: f64 = 60.0;
const PAD: f64 = 20.0;
/// Max characters of a task title shown inside a DAG node box.
const LABEL_MAX: usize = 18;

#[derive(Serialize)]
struct TaskView<'a> {
    id: String,
    title: &'a str,
    status: &'a str,
    /// Efficiency rounded to 2 decimals — matches `data.json` `eff` so the
    /// `data-eff` attribute and the data island agree.
    eff: f64,
    /// Human-formatted efficiency, matches every other render surface.
    eff_fmt: String,
    tier_glyph: &'static str,
    markers: &'a [String],
    /// Space-joined dependency ids for the `data-depends-on` attribute.
    depends_on: String,
    phase: u32,
    bundle: &'a str,
}

#[derive(Serialize)]
struct PhaseView<'a> {
    number: u32,
    name: &'a str,
    /// The phase-level `[phases.N].status` field — rendered as a header badge
    /// and a `data-phase-status` attribute so a `done` phase reads as distinct
    /// from an active one (not just "all columns happen to be in Done").
    status: &'a str,
    /// Count of `done` tasks in the phase (shown in the header).
    done: usize,
    total: usize,
    /// Integer completion percentage for the progress bar.
    pct: u32,
    pending: Vec<TaskView<'a>>,
    in_progress: Vec<TaskView<'a>>,
    /// `done`-status task cards. Named `done_tasks` so it does not collide with
    /// the `done` count field in the template context.
    done_tasks: Vec<TaskView<'a>>,
}

#[derive(Serialize)]
struct DagNode {
    id: String,
    label: String,
    status: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label_x: f64,
    label_y: f64,
}

#[derive(Serialize)]
struct DagEdge {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

#[derive(Serialize)]
struct DagView {
    svg_width: u32,
    svg_height: u32,
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
}

/// Render the static HTML view for `tasks`. `today` is the `YYYY-MM-DD` stamp
/// shown in the footer (callers pass `today_iso()`).
pub fn render_html_str(tasks: &Tasks, today: &str) -> anyhow::Result<String> {
    let phases = build_phase_views(tasks);
    let task_refs: Vec<&Task> = tasks.task.iter().collect();
    let dag = build_dag(&task_refs);
    let all_markers = collect_markers(tasks);
    // Escape `</` so a task title containing `</script>` cannot break out of
    // the `<script type="application/json">` data island. `\/` is a valid JSON
    // escape, so the island still parses cleanly.
    let data_island = export_compact_json_str(tasks)?.replace("</", "<\\/");

    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);
    env.add_template("roadmap.html", include_str!("../templates/roadmap.html.j2"))?;
    let tmpl = env.get_template("roadmap.html")?;
    let html = tmpl.render(minijinja::context! {
        project => tasks.project.as_str(),
        today => today,
        phases => phases,
        dag => dag,
        all_markers => all_markers,
        data_island => data_island,
    })?;
    Ok(html)
}

/// One `PhaseView` per declared `[phases.*]` entry, sorted by `phase.order`.
/// A phase with no tasks still gets a view (empty columns) — it is declared.
/// `blocked` tasks land in the pending column; their `data-status="blocked"`
/// (amber border, blocked filter toggle) carries the distinction.
fn build_phase_views(tasks: &Tasks) -> Vec<PhaseView<'_>> {
    let mut phase_entries: Vec<_> = tasks.phases.iter().collect();
    phase_entries.sort_by_key(|(_, phase)| phase.order);

    phase_entries
        .into_iter()
        .map(|(key, phase)| {
            let number: u32 = key.parse().unwrap_or(0);
            let mut pending = Vec::new();
            let mut in_progress = Vec::new();
            let mut done_tasks = Vec::new();
            for task in &tasks.task {
                if task.phase != number {
                    continue;
                }
                let view = task_view(task);
                match task.status.as_str() {
                    "done" => done_tasks.push(view),
                    "in_progress" => in_progress.push(view),
                    _ => pending.push(view), // pending + blocked share the column
                }
            }
            let total = pending.len() + in_progress.len() + done_tasks.len();
            let done = done_tasks.len();
            let pct = if total == 0 {
                0
            } else {
                (done * 100 / total) as u32
            };
            PhaseView {
                number,
                name: &phase.name,
                status: phase.status.as_str(),
                done,
                total,
                pct,
                pending,
                in_progress,
                done_tasks,
            }
        })
        .collect()
}

fn task_view(task: &Task) -> TaskView<'_> {
    let raw_eff = efficiency(task);
    TaskView {
        id: task.id.to_string(),
        title: &task.title,
        status: &task.status,
        eff: round_eff(raw_eff),
        eff_fmt: format_efficiency(raw_eff),
        tier_glyph: tier_glyph(raw_eff),
        markers: &task.markers,
        depends_on: task
            .depends_on
            .iter()
            .map(|dep| dep.to_string())
            .collect::<Vec<_>>()
            .join(" "),
        phase: task.phase,
        bundle: &task.bundle,
    }
}

/// Sorted, de-duplicated marker names across all tasks — drives the filter-bar
/// chips. `BTreeSet` gives deterministic order.
fn collect_markers(tasks: &Tasks) -> Vec<String> {
    let mut set: BTreeSet<&str> = BTreeSet::new();
    for task in &tasks.task {
        for marker in &task.markers {
            set.insert(marker.as_str());
        }
    }
    set.into_iter().map(String::from).collect()
}

/// Build the layered-DAG layout from in-repo `depends_on` edges. `cross_repo`
/// deps are skipped — those only matter in portfolio mode. `validate` already
/// guarantees the graph is acyclic.
fn build_dag(tasks: &[&Task]) -> DagView {
    let layers = crate::topo::compute_layers(tasks);

    // Group ids by layer, sorted within each layer for deterministic slotting.
    let mut by_layer: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for task in tasks {
        let id = task.id.to_string();
        let layer = layers.get(&id).copied().unwrap_or(0);
        by_layer.entry(layer).or_default().push(id);
    }
    for ids in by_layer.values_mut() {
        ids.sort();
    }

    let by_id: HashMap<String, &Task> = tasks
        .iter()
        .map(|task| (task.id.to_string(), *task))
        .collect();

    let mut pos: HashMap<String, (f64, f64)> = HashMap::new();
    let mut nodes = Vec::new();
    for (layer, ids) in &by_layer {
        for (slot, id) in ids.iter().enumerate() {
            let x = PAD + slot as f64 * (NODE_W + H_GAP);
            let y = PAD + *layer as f64 * (NODE_H + V_GAP);
            pos.insert(id.clone(), (x, y));
            let task = by_id[id];
            nodes.push(DagNode {
                id: id.clone(),
                label: truncate_label(&task.title),
                status: task.status.clone(),
                x,
                y,
                w: NODE_W,
                h: NODE_H,
                label_x: x + NODE_W / 2.0,
                label_y: y + NODE_H / 2.0 + 4.0,
            });
        }
    }

    // Edges point dep → dependent, i.e. downward (layer increases). Emit an
    // edge only when both endpoints resolved — defensive against any gap a
    // future validator change might leave open.
    let mut edges = Vec::new();
    for task in tasks {
        let Some(&(to_x, to_y)) = pos.get(&task.id.to_string()) else {
            continue;
        };
        for dep in &task.depends_on {
            if let Some(&(from_x, from_y)) = pos.get(&dep.to_string()) {
                edges.push(DagEdge {
                    x1: from_x + NODE_W / 2.0,
                    y1: from_y + NODE_H,
                    x2: to_x + NODE_W / 2.0,
                    y2: to_y,
                });
            }
        }
    }

    let max_in_layer = by_layer.values().map(Vec::len).max().unwrap_or(0).max(1);
    let layer_count = by_layer.keys().max().map_or(0, |max| max + 1);
    let svg_width = (PAD * 2.0 + max_in_layer as f64 * (NODE_W + H_GAP) - H_GAP).ceil() as u32;
    let svg_height = if layer_count == 0 {
        (PAD * 2.0) as u32
    } else {
        (PAD * 2.0 + layer_count as f64 * (NODE_H + V_GAP) - V_GAP).ceil() as u32
    };

    DagView {
        svg_width,
        svg_height,
        nodes,
        edges,
    }
}

/// Truncate a title to fit inside a DAG node box, on a char boundary.
fn truncate_label(title: &str) -> String {
    if title.chars().count() <= LABEL_MAX {
        return title.to_string();
    }
    let head: String = title.chars().take(LABEL_MAX).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "schema_version = 1\n\
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

    fn tasks_from(body: &str) -> Tasks {
        toml::from_str(&format!("{BASE}{body}")).expect("valid tasks toml")
    }

    #[test]
    fn dag_single_node_sits_at_padding_origin() {
        let tasks = tasks_from(&task_toml(1, "pending", &[]));
        let refs: Vec<&Task> = tasks.task.iter().collect();
        let dag = build_dag(&refs);
        assert_eq!(dag.nodes.len(), 1);
        assert_eq!(dag.edges.len(), 0);
        assert_eq!(dag.nodes[0].x, PAD);
        assert_eq!(dag.nodes[0].y, PAD);
    }

    #[test]
    fn dag_emits_one_edge_per_dependency() {
        let body = format!(
            "{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
        );
        let tasks = tasks_from(&body);
        let refs: Vec<&Task> = tasks.task.iter().collect();
        let dag = build_dag(&refs);
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.edges.len(), 1);
        // Dependent (task 2) sits one layer below its dep (task 1).
        assert!(dag.edges[0].y2 > dag.edges[0].y1);
    }

    #[test]
    fn render_html_str_on_empty_roadmap_succeeds() {
        let tasks: Tasks = toml::from_str(BASE).expect("valid tasks toml");
        let html = render_html_str(&tasks, "2026-05-14").expect("render html");
        assert!(html.contains("id=\"rmap-data\""));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn render_html_str_carries_data_island_and_attrs() {
        let tasks = tasks_from(&task_toml(7, "pending", &[]));
        let html = render_html_str(&tasks, "2026-05-14").expect("render html");
        assert!(html.contains("id=\"rmap-data\""));
        assert!(html.contains("data-id=\"7\""));
        assert!(html.contains("data-status=\"pending\""));
        assert!(html.contains("data-eff="));
        assert!(html.contains("data-markers="));
        assert!(html.contains("data-depends-on="));
        assert!(html.contains("data-phase=\"1\""));
        assert!(html.contains("@media print"));
        assert!(html.contains("<svg"));
    }

    #[test]
    fn render_html_str_carries_phase_status_badge_and_attr() {
        // BASE declares `[phases.1]` with `status = "in_progress"`.
        let tasks = tasks_from(&task_toml(1, "done", &[]));
        let html = render_html_str(&tasks, "2026-05-14").expect("render html");
        assert!(html.contains("data-phase-status=\"in_progress\""));
        assert!(html.contains("phase-status-in_progress"));
    }

    #[test]
    fn truncate_label_keeps_short_titles_and_clips_long_ones() {
        assert_eq!(truncate_label("short"), "short");
        let long = "a really long task title that overflows";
        let clipped = truncate_label(long);
        assert!(clipped.ends_with('…'));
        assert_eq!(clipped.chars().count(), LABEL_MAX + 1);
    }
}
