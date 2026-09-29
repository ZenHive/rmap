//! `rmap render --html` / `--html --multi` — the static HTML board views.
//!
//! Pipeline: `schema::Tasks` → exported envelope (`export_compact_json_str`) →
//! owned view structs (`PhaseView` / `CardView` / `DagView`) → minijinja render
//! of `templates/roadmap.html.j2` or `templates/portfolio.html.j2` (inlined via
//! `include_str!`, so the templates ship inside the binary). Both pages go
//! through the same envelope-based view path, so the single-project page is a
//! one-repo portfolio in all but chrome. Output is a self-contained single
//! file: inline CSS + vanilla JS, no CDN, no framework, no build step.
//!
//! The `<script id="rmap-data">` island carries the full compact export so
//! agents read structured data without DOM scraping (the page's task detail
//! panel reads the same island). Every board task card carries six `data-*`
//! attributes (`data-id` / `data-status` / `data-eff` / `data-markers` /
//! `data-depends-on` / `data-phase`) — the agent selector contract. Rack cards
//! (the ready / active / hold digest) are copies and deliberately do NOT carry
//! the `task-card` class, so a selector sees each task once.

mod portfolio;
pub use portfolio::{ProjectInput, render_portfolio_str};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::export::export_compact_json_str;
use crate::schema::Tasks;
use crate::scoring::{format_efficiency, round_eff, tier_glyph};

/// Dependency-graph layout constants (SVG user units).
const NODE_W: f64 = 156.0;
const NODE_H: f64 = 44.0;
const H_GAP: f64 = 24.0;
const V_GAP: f64 = 60.0;
const PAD: f64 = 20.0;
/// Max characters of a task title shown inside a DAG node box.
const LABEL_MAX: usize = 20;

/// Which board lane a card sits in. Derived, never persisted: `ready` is a
/// `pending` task whose every in-repo `depends_on` is `done` (same predicate as
/// `next::is_unblocked`), `waiting` is a `pending` task with an unmet dep.
/// `superseded` shares the `done` lane and is stamped VOID by the template.
fn lane_of(status: &str, deps_met: bool) -> &'static str {
    match status {
        "done" | "superseded" => "done",
        "in_progress" => "active",
        "blocked" => "hold",
        _ if deps_met => "ready",
        _ => "waiting",
    }
}

/// Board lanes in reading order — operational first: what can move now, what
/// is moving, what is stuck, what waits on other cards, what is finished.
const LANES: [(&str, &str); 5] = [
    ("ready", "Ready"),
    ("active", "Active"),
    ("hold", "Hold"),
    ("waiting", "Waiting"),
    ("done", "Done"),
];

#[derive(Serialize, Clone)]
struct CardView {
    id: String,
    title: String,
    status: String,
    lane: &'static str,
    /// Efficiency rounded to 2 decimals — matches the island's `eff` so the
    /// `data-eff` attribute and the data island agree.
    eff: f64,
    eff_fmt: String,
    /// `hi` / `mid` / `lo` / `low` — the `scoring::tier_glyph` ladder as a CSS
    /// hook instead of an emoji.
    tier: &'static str,
    markers: Vec<String>,
    /// Space-joined dependency ids for the `data-depends-on` attribute.
    depends_on: String,
    deps: Vec<String>,
    phase: u32,
    unlocks: u64,
    blocked_reason: String,
    landing_ref: String,
    /// Short form of `landing_ref` for the card face (`PR 21`, else verbatim).
    landing_label: String,
    assignee: String,
    /// Index of the owning project in the page's island (always 0 on the
    /// single-project page) — the detail panel's lookup key.
    repo: usize,
    repo_name: String,
}

#[derive(Serialize)]
struct LaneView {
    key: &'static str,
    label: &'static str,
    cards: Vec<CardView>,
}

/// Proportional composition bar segment (`lane`, integer percent).
#[derive(Serialize)]
struct Segment {
    lane: &'static str,
    count: usize,
    pct: u32,
}

#[derive(Serialize)]
struct PhaseView {
    number: u32,
    name: String,
    /// The phase-level `[phases.N].status` — `data-phase-status` hook.
    status: String,
    done: usize,
    total: usize,
    pct: u32,
    lanes: Vec<LaneView>,
    segments: Vec<Segment>,
}

#[derive(Serialize)]
struct DagNode {
    id: String,
    label: String,
    status: String,
    lane: &'static str,
    repo: usize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Serialize)]
struct DagEdge {
    /// Cubic path from the dependency's bottom edge to the dependent's top.
    d: String,
}

#[derive(Serialize)]
struct DagView {
    svg_width: u32,
    svg_height: u32,
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
}

/// The shared design-system CSS, injected verbatim into both templates so they
/// speak one visual language. Not a minijinja template — substituted as a
/// `| safe` context value, never parsed.
const SHARED_STYLES: &str = include_str!("../templates/_styles.css");
/// The shared board script (lane filters, card pull-out detail panel). Injected
/// verbatim like the stylesheet.
const SHARED_SCRIPT: &str = include_str!("../templates/_board.js");

/// Build a minijinja env with the shared component macros registered and HTML
/// auto-escaping on. Callers add their page template and render.
fn build_env() -> anyhow::Result<minijinja::Environment<'static>> {
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);
    env.add_template(
        "components.html",
        include_str!("../templates/_components.html.j2"),
    )?;
    Ok(env)
}

/// Escape `</` so a task title containing `</script>` cannot break out of the
/// `<script type="application/json">` data island. `\/` is a valid JSON escape,
/// so the island still parses cleanly.
fn escape_island(json: &str) -> String {
    json.replace("</", "<\\/")
}

/// Page-level lane totals, shown as the fixed-digit counter row.
#[derive(Serialize, Default)]
struct Counts {
    total: usize,
    ready: usize,
    active: usize,
    hold: usize,
    waiting: usize,
    done: usize,
    pct: u32,
}

impl Counts {
    fn add(&mut self, lane: &str) {
        self.total += 1;
        match lane {
            "ready" => self.ready += 1,
            "active" => self.active += 1,
            "hold" => self.hold += 1,
            "waiting" => self.waiting += 1,
            _ => self.done += 1,
        }
    }

    fn absorb(&mut self, other: &Counts) {
        self.total += other.total;
        self.ready += other.ready;
        self.active += other.active;
        self.hold += other.hold;
        self.waiting += other.waiting;
        self.done += other.done;
    }

    fn finish(&mut self) {
        self.pct = pct_of(self.done, self.total);
    }

    fn segments(&self) -> Vec<Segment> {
        let counts = [
            ("ready", self.ready),
            ("active", self.active),
            ("hold", self.hold),
            ("waiting", self.waiting),
            ("done", self.done),
        ];
        counts
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(lane, count)| Segment {
                lane,
                count,
                pct: pct_of(count, self.total),
            })
            .collect()
    }
}

/// Dispatch racks: every ready / active / held card of the page, lifted out of
/// their phase boards so "what can move now" is answered before any scrolling.
/// Ready is ranked Eff desc, then unlocks desc (the `rmap ready` tiebreak).
#[derive(Serialize)]
struct Racks {
    ready: Vec<CardView>,
    active: Vec<CardView>,
    hold: Vec<CardView>,
}

impl Racks {
    fn collect<'a>(cards: impl Iterator<Item = &'a CardView>) -> Self {
        let mut racks = Racks {
            ready: Vec::new(),
            active: Vec::new(),
            hold: Vec::new(),
        };
        for card in cards {
            match card.lane {
                "ready" => racks.ready.push(card.clone()),
                "active" => racks.active.push(card.clone()),
                "hold" => racks.hold.push(card.clone()),
                _ => {}
            }
        }
        racks
            .ready
            .sort_by(|a, b| b.eff.total_cmp(&a.eff).then(b.unlocks.cmp(&a.unlocks)));
        racks.hold.sort_by_key(|c| std::cmp::Reverse(c.unlocks));
        racks
    }
}

/// Render the static HTML view for `tasks`. `today` is the `YYYY-MM-DD` stamp
/// shown in the header (callers pass `today_iso()`). The page is built from the
/// same exported envelope it embeds, so board, detail panel and island can
/// never disagree — and single-project and portfolio share one view path.
pub fn render_html_str(tasks: &Tasks, today: &str) -> anyhow::Result<String> {
    let island = export_compact_json_str(tasks)?;
    let data: PortfolioData = serde_json::from_str(&island).context("parse own export")?;
    let project = project_views(&data, 0, &tasks.project);
    let racks = Racks::collect(project.cards());
    let all_markers = collect_markers(&data);

    let mut env = build_env()?;
    env.add_template("roadmap.html", include_str!("../templates/roadmap.html.j2"))?;
    let tmpl = env.get_template("roadmap.html")?;
    let html = tmpl.render(minijinja::context! {
        project => tasks.project.as_str(),
        today => today,
        phases => project.phases,
        counts => project.counts,
        segments => project.counts.segments(),
        racks => racks,
        dag => project.dag,
        all_markers => all_markers,
        data_island => escape_island(&island),
        styles => SHARED_STYLES,
        script => SHARED_SCRIPT,
    })?;
    Ok(html)
}

/// Sorted, de-duplicated marker names across all tasks — drives the filter
/// switches. `BTreeSet` gives deterministic order.
fn collect_markers(data: &PortfolioData) -> Vec<String> {
    let set: BTreeSet<&str> = data
        .task
        .iter()
        .flat_map(|t| t.markers.iter().map(String::as_str))
        .collect();
    set.into_iter().map(String::from).collect()
}

/// Everything one project contributes to a page: phase boards, lane counts,
/// dependency graph.
struct ProjectViews {
    phases: Vec<PhaseView>,
    counts: Counts,
    dag: DagView,
}

impl ProjectViews {
    fn cards(&self) -> impl Iterator<Item = &CardView> {
        self.phases
            .iter()
            .flat_map(|p| p.lanes.iter().flat_map(|l| l.cards.iter()))
    }
}

/// Build the board for one exported envelope. `repo` is the island index the
/// detail panel resolves cards against; `repo_name` labels rack cards.
fn project_views(data: &PortfolioData, repo: usize, repo_name: &str) -> ProjectViews {
    let done_ids: HashSet<String> = data
        .task
        .iter()
        .filter(|t| t.status == "done")
        .map(|t| json_id_string(&t.id))
        .collect();
    let lane_by_id: HashMap<String, &'static str> = data
        .task
        .iter()
        .map(|t| {
            let met = t
                .depends_on
                .iter()
                .all(|d| done_ids.contains(&json_id_string(d)));
            (json_id_string(&t.id), lane_of(&t.status, met))
        })
        .collect();

    let mut phase_entries: Vec<_> = data.phases.iter().collect();
    phase_entries.sort_by_key(|(_, phase)| phase.order);

    let mut counts = Counts::default();
    let phases = phase_entries
        .into_iter()
        .map(|(key, phase)| {
            let number: u32 = key.parse().unwrap_or(0);
            let mut lanes: Vec<LaneView> = LANES
                .iter()
                .map(|&(key, label)| LaneView {
                    key,
                    label,
                    cards: Vec::new(),
                })
                .collect();
            let mut phase_counts = Counts::default();
            for task in data.task.iter().filter(|t| t.phase == number) {
                let id = json_id_string(&task.id);
                let lane = lane_by_id[&id];
                phase_counts.add(lane);
                let card = card_view(task, id, lane, repo, repo_name);
                if let Some(slot) = lanes.iter_mut().find(|l| l.key == lane) {
                    slot.cards.push(card);
                }
            }
            if let Some(ready) = lanes.iter_mut().find(|l| l.key == "ready") {
                ready
                    .cards
                    .sort_by(|a, b| b.eff.total_cmp(&a.eff).then(b.unlocks.cmp(&a.unlocks)));
            }
            counts.absorb(&phase_counts);
            phase_counts.finish();
            PhaseView {
                number,
                name: phase.name.clone(),
                status: phase.status.clone(),
                done: phase_counts.done,
                total: phase_counts.total,
                pct: phase_counts.pct,
                segments: phase_counts.segments(),
                lanes,
            }
        })
        .collect();
    counts.finish();

    let dag = build_dag(
        &data
            .task
            .iter()
            .map(|task| {
                let id = json_id_string(&task.id);
                DagNodeInput {
                    lane: lane_by_id[&id],
                    id,
                    title: task.title.clone(),
                    status: task.status.clone(),
                    depends_on: task.depends_on.iter().map(json_id_string).collect(),
                }
            })
            .collect::<Vec<_>>(),
        repo,
    );

    ProjectViews {
        phases,
        counts,
        dag,
    }
}

fn card_view(
    task: &PortfolioTask,
    id: String,
    lane: &'static str,
    repo: usize,
    repo_name: &str,
) -> CardView {
    let tier = match tier_glyph(task.eff) {
        "🎯" => "hi",
        "🚀" => "mid",
        "📋" => "lo",
        _ => "low",
    };
    let deps: Vec<String> = task.depends_on.iter().map(json_id_string).collect();
    CardView {
        id,
        title: task.title.clone(),
        status: task.status.clone(),
        lane,
        eff: round_eff(task.eff),
        eff_fmt: format_efficiency(task.eff),
        tier,
        markers: task.markers.clone(),
        depends_on: deps.join(" "),
        deps,
        phase: task.phase,
        unlocks: task.unlocks,
        blocked_reason: task.blocked_reason.clone(),
        landing_ref: task.landing_ref.clone(),
        landing_label: landing_label(&task.landing_ref),
        assignee: task.assignee.clone(),
        repo,
        repo_name: repo_name.to_string(),
    }
}

/// `https://…/pull/21` → `PR 21`, `…/merge_requests/7` → `MR 7`; anything
/// else is shown verbatim (the field is free text, never parsed elsewhere).
fn landing_label(landing_ref: &str) -> String {
    for (marker, prefix) in [("/pull/", "PR"), ("/merge_requests/", "MR")] {
        if let Some((_, rest)) = landing_ref.split_once(marker) {
            let number = rest.split(['/', '#', '?']).next().unwrap_or(rest);
            return format!("{prefix} {number}");
        }
    }
    landing_ref.to_string()
}

/// One node for the DAG layout — the source-agnostic input to [`build_dag`].
/// `depends_on` holds canonical id strings.
struct DagNodeInput {
    id: String,
    title: String,
    status: String,
    lane: &'static str,
    depends_on: Vec<String>,
}

/// Build the layered-DAG layout from in-repo `depends_on` edges. `cross_repo`
/// deps are skipped — those only matter in portfolio mode. `validate` already
/// guarantees the graph is acyclic. Each layer is centred on the widest one.
fn build_dag(nodes_in: &[DagNodeInput], repo: usize) -> DagView {
    let edges_in: Vec<(String, Vec<String>)> = nodes_in
        .iter()
        .map(|n| (n.id.clone(), n.depends_on.clone()))
        .collect();
    let layers = crate::topo::compute_layers_from_edges(&edges_in);

    // Group ids by layer, sorted within each layer for deterministic slotting.
    let mut by_layer: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for node in nodes_in {
        let layer = layers.get(&node.id).copied().unwrap_or(0);
        by_layer.entry(layer).or_default().push(node.id.clone());
    }
    for ids in by_layer.values_mut() {
        ids.sort();
    }
    let max_in_layer = by_layer.values().map(Vec::len).max().unwrap_or(0).max(1);

    let by_id: HashMap<&str, &DagNodeInput> = nodes_in.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut pos: HashMap<String, (f64, f64)> = HashMap::new();
    let mut nodes = Vec::new();
    for (layer, ids) in &by_layer {
        let offset = (max_in_layer - ids.len()) as f64 * (NODE_W + H_GAP) / 2.0;
        for (slot, id) in ids.iter().enumerate() {
            let x = PAD + offset + slot as f64 * (NODE_W + H_GAP);
            let y = PAD + *layer as f64 * (NODE_H + V_GAP);
            pos.insert(id.clone(), (x, y));
            let node = by_id[id.as_str()];
            nodes.push(DagNode {
                id: id.clone(),
                label: truncate_label(&node.title),
                status: node.status.clone(),
                lane: node.lane,
                repo,
                x,
                y,
                w: NODE_W,
                h: NODE_H,
            });
        }
    }

    // Edges run dep → dependent, i.e. downward (layer increases). Emit an edge
    // only when both endpoints resolved — defensive against any gap a future
    // validator change might leave open.
    let mut edges = Vec::new();
    for node in nodes_in {
        let Some(&(to_x, to_y)) = pos.get(&node.id) else {
            continue;
        };
        for dep in &node.depends_on {
            if let Some(&(from_x, from_y)) = pos.get(dep) {
                let (x1, y1) = (from_x + NODE_W / 2.0, from_y + NODE_H);
                let (x2, y2) = (to_x + NODE_W / 2.0, to_y);
                let bend = (y2 - y1) / 2.0;
                edges.push(DagEdge {
                    d: format!(
                        "M{x1},{y1} C{x1},{} {x2},{} {x2},{y2}",
                        y1 + bend,
                        y2 - bend
                    ),
                });
            }
        }
    }

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

// Exported-envelope slice both views read (the single-project page parses its
// own compact export into the same shape).

#[derive(Deserialize)]
struct PortfolioData {
    #[serde(default)]
    project: String,
    #[serde(default)]
    phases: BTreeMap<String, PhaseMeta>,
    #[serde(default)]
    task: Vec<PortfolioTask>,
}

#[derive(Deserialize)]
struct PhaseMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    order: u32,
    #[serde(default)]
    status: String,
}

/// The slice of an exported task the board needs. Every field is defaulted so
/// older `data.json` files (pre-`unlocks`, pre-`landing_ref`) still render.
#[derive(Deserialize)]
struct PortfolioTask {
    #[serde(default)]
    id: serde_json::Value,
    #[serde(default)]
    phase: u32,
    #[serde(default)]
    status: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    eff: f64,
    #[serde(default)]
    unlocks: u64,
    #[serde(default)]
    markers: Vec<String>,
    #[serde(default)]
    depends_on: Vec<serde_json::Value>,
    #[serde(default)]
    blocked_reason: String,
    #[serde(default)]
    landing_ref: String,
    #[serde(default)]
    assignee: String,
    #[serde(default)]
    cross_repo: Vec<CrossRepoData>,
}

#[derive(Deserialize)]
struct CrossRepoData {
    #[serde(default)]
    repo: String,
    #[serde(default)]
    task_id: serde_json::Value,
    #[serde(default)]
    relation: String,
}

fn json_id_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn pct_of(part: usize, total: usize) -> u32 {
    // Silencing warning as the solution proposed by clippy would run (part * 100) even when we
    // return 0. `unknown_lints` keeps older toolchains (without `manual_checked_ops`) compiling.
    #[allow(unknown_lints, clippy::manual_checked_ops)]
    if total == 0 {
        0
    } else {
        (part * 100 / total) as u32
    }
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

    fn views_of(tasks: &Tasks) -> ProjectViews {
        let island = export_compact_json_str(tasks).expect("export");
        let data: PortfolioData = serde_json::from_str(&island).expect("parse export");
        project_views(&data, 0, "demo")
    }

    fn lane_ids(views: &ProjectViews, lane: &str) -> Vec<String> {
        views
            .cards()
            .filter(|c| c.lane == lane)
            .map(|c| c.id.clone())
            .collect()
    }

    #[test]
    fn lanes_split_pending_by_dependency_state() {
        let body = format!(
            "{}{}{}{}",
            task_toml(1, "done", &[]),
            task_toml(2, "pending", &[1]),
            task_toml(3, "pending", &[4]),
            task_toml(4, "in_progress", &[]),
        );
        let views = views_of(&tasks_from(&body));
        assert_eq!(lane_ids(&views, "ready"), vec!["2"]);
        assert_eq!(lane_ids(&views, "waiting"), vec!["3"]);
        assert_eq!(lane_ids(&views, "active"), vec!["4"]);
        assert_eq!(lane_ids(&views, "done"), vec!["1"]);
        assert_eq!(views.counts.ready, 1);
        assert_eq!(views.counts.total, 4);
    }

    #[test]
    fn racks_rank_ready_by_eff_then_unlocks() {
        // Same scores → equal Eff; task 5 unlocks task 7, so it ranks first.
        let body = format!(
            "{}{}{}",
            task_toml(6, "pending", &[]),
            task_toml(5, "pending", &[]),
            task_toml(7, "pending", &[5]),
        );
        let views = views_of(&tasks_from(&body));
        let racks = Racks::collect(views.cards());
        let ids: Vec<_> = racks.ready.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["5", "6"]);
    }

    #[test]
    fn dag_single_node_sits_at_padding_origin() {
        let tasks = tasks_from(&task_toml(1, "pending", &[]));
        let dag = views_of(&tasks).dag;
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
        let dag = views_of(&tasks).dag;
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.edges.len(), 1);
        // Dependent (task 2) sits one layer below its dep (task 1).
        let y = |id: &str| dag.nodes.iter().find(|n| n.id == id).unwrap().y;
        assert!(y("2") > y("1"));
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
