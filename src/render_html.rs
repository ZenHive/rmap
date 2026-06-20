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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use anyhow::Context;
use serde::{Deserialize, Serialize};

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

/// The shared design-system CSS, injected verbatim into both the single-project
/// and portfolio templates so they speak one visual language. Not a minijinja
/// template — substituted as a `| safe` context value, never parsed.
const SHARED_STYLES: &str = include_str!("../templates/_styles.css");

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

/// Render the static HTML view for `tasks`. `today` is the `YYYY-MM-DD` stamp
/// shown in the footer (callers pass `today_iso()`).
pub fn render_html_str(tasks: &Tasks, today: &str) -> anyhow::Result<String> {
    let phases = build_phase_views(tasks);
    let dag = build_dag(&dag_inputs(tasks.task.iter()));
    let all_markers = collect_markers(tasks);
    let data_island = escape_island(&export_compact_json_str(tasks)?);

    let mut env = build_env()?;
    env.add_template("roadmap.html", include_str!("../templates/roadmap.html.j2"))?;
    let tmpl = env.get_template("roadmap.html")?;
    let html = tmpl.render(minijinja::context! {
        project => tasks.project.as_str(),
        today => today,
        phases => phases,
        dag => dag,
        all_markers => all_markers,
        data_island => data_island,
        styles => SHARED_STYLES,
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
            let pct = pct_of(done, total);
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

/// One node for the DAG layout — the source-agnostic input to [`build_dag`],
/// extracted from either a `schema::Task` (single-project) or a JSON-loaded
/// portfolio task. `depends_on` holds canonical id strings.
struct DagNodeInput {
    id: String,
    title: String,
    status: String,
    depends_on: Vec<String>,
}

/// Map a borrowed `schema::Task` iterator into [`DagNodeInput`]s.
fn dag_inputs<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Vec<DagNodeInput> {
    tasks
        .into_iter()
        .map(|task| DagNodeInput {
            id: task.id.to_string(),
            title: task.title.clone(),
            status: task.status.clone(),
            depends_on: task.depends_on.iter().map(|d| d.to_string()).collect(),
        })
        .collect()
}

/// Build the layered-DAG layout from in-repo `depends_on` edges. `cross_repo`
/// deps are skipped — those only matter in portfolio mode. `validate` already
/// guarantees the graph is acyclic.
fn build_dag(nodes_in: &[DagNodeInput]) -> DagView {
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

    let by_id: HashMap<&str, &DagNodeInput> = nodes_in.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut pos: HashMap<String, (f64, f64)> = HashMap::new();
    let mut nodes = Vec::new();
    for (layer, ids) in &by_layer {
        for (slot, id) in ids.iter().enumerate() {
            let x = PAD + slot as f64 * (NODE_W + H_GAP);
            let y = PAD + *layer as f64 * (NODE_H + V_GAP);
            pos.insert(id.clone(), (x, y));
            let node = by_id[id.as_str()];
            nodes.push(DagNode {
                id: id.clone(),
                label: truncate_label(&node.title),
                status: node.status.clone(),
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
    for node in nodes_in {
        let Some(&(to_x, to_y)) = pos.get(&node.id) else {
            continue;
        };
        for dep in &node.depends_on {
            if let Some(&(from_x, from_y)) = pos.get(dep) {
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

// ───────────────────────────────────────────────────────────────────────────
// Portfolio (multi-project) view — `rmap render --html --multi`.
//
// Each project is normalized to its exported JSON envelope before reaching this
// module (project root → validate+export, data.json path → file read); the CLI
// owns that filesystem step so the renderer stays pure. The envelope is parsed
// for the per-repo views AND embedded verbatim in the aggregate `rmap-data`
// island so agents read every project's full structured data from one element.
// Repos render as rows; clicking a row expands its single-project layout (same
// component macros). Cross-repo relations (`cross_repo`) become gutter arrows.
// ───────────────────────────────────────────────────────────────────────────

/// One project's pre-loaded input to the portfolio renderer. `data_json` is the
/// project's exported envelope (compact or pretty — both deserialize).
pub struct ProjectInput {
    /// Display name (defaults to the envelope's `project`, else the path).
    pub label: String,
    /// Source path shown under the repo name.
    pub path_display: String,
    /// The exported JSON envelope — parsed for views, embedded for the island.
    pub data_json: String,
}

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
    markers: Vec<String>,
    #[serde(default)]
    depends_on: Vec<serde_json::Value>,
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

#[derive(Serialize)]
struct OwnedTaskView {
    id: String,
    title: String,
    status: String,
    eff: f64,
    eff_fmt: String,
    tier_glyph: &'static str,
    markers: Vec<String>,
    depends_on: String,
    phase: u32,
}

#[derive(Serialize)]
struct OwnedPhaseView {
    number: u32,
    name: String,
    status: String,
    done: usize,
    total: usize,
    pct: u32,
    pending: Vec<OwnedTaskView>,
    in_progress: Vec<OwnedTaskView>,
    done_tasks: Vec<OwnedTaskView>,
}

#[derive(Serialize)]
struct RelTag {
    relation: String,
    repo: String,
    task_id: String,
}

#[derive(Serialize)]
struct RepoView {
    project: String,
    slug: String,
    path: String,
    total: usize,
    done: usize,
    in_progress: usize,
    pending: usize,
    blocked: usize,
    pct: u32,
    pct_done: u32,
    pct_in_progress: u32,
    pct_blocked: u32,
    pct_pending: u32,
    /// `1` when this repo is an endpoint of a drawn cross-repo arrow — toggles
    /// the gutter anchor highlight.
    has_rel: u32,
    phases: Vec<OwnedPhaseView>,
    dag: DagView,
    rel_tags: Vec<RelTag>,
}

#[derive(Serialize)]
struct PortfolioMetrics {
    repo_count: usize,
    total_tasks: usize,
    done: usize,
    in_progress: usize,
    pending: usize,
    blocked: usize,
    pct: u32,
}

/// A drawn cross-repo arrow (source row → target row), consumed by the
/// portfolio JS overlay. `relation` is the CSS class (`blocks` / `related`).
#[derive(Serialize, Hash, PartialEq, Eq)]
struct RelEdge {
    source: String,
    target: String,
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

/// Sanitize a label into a DOM-safe, deterministic, unique slug (`<clean>-<i>`).
fn slugify(label: &str, index: usize) -> String {
    let mut clean = String::new();
    let mut last_dash = false;
    for ch in label.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            clean.push(ch);
            last_dash = false;
        } else if !last_dash {
            clean.push('-');
            last_dash = true;
        }
    }
    let clean = clean.trim_matches('-');
    let base = if clean.is_empty() { "repo" } else { clean };
    format!("{base}-{index}")
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

/// Render the multi-project portfolio view from pre-loaded project envelopes.
/// `today` is the footer stamp. Each `ProjectInput.data_json` is parsed for its
/// views and embedded verbatim in the aggregate `rmap-data` island.
pub fn render_portfolio_str(projects: &[ProjectInput], today: &str) -> anyhow::Result<String> {
    let parsed: Vec<(usize, &ProjectInput, PortfolioData)> = projects
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let data: PortfolioData = serde_json::from_str(&input.data_json)
                .with_context(|| format!("parse project envelope: {}", input.path_display))?;
            Ok((i, input, data))
        })
        .collect::<anyhow::Result<_>>()?;

    // Resolve cross_repo.repo references: a repo name can match a project's
    // display name, its `project` field, or its source-path basename.
    let mut slug_by_key: HashMap<String, String> = HashMap::new();
    let slugs: Vec<String> = parsed
        .iter()
        .map(|(i, input, data)| {
            let label = if data.project.is_empty() {
                input.label.clone()
            } else {
                data.project.clone()
            };
            slugify(&label, *i)
        })
        .collect();
    for ((_, input, data), slug) in parsed.iter().zip(&slugs) {
        for key in [
            data.project.to_lowercase(),
            input.label.to_lowercase(),
            path_basename(&input.path_display).to_lowercase(),
        ] {
            if !key.is_empty() {
                slug_by_key.entry(key).or_insert_with(|| slug.clone());
            }
        }
    }

    let mut repos = Vec::with_capacity(parsed.len());
    let mut edges: Vec<RelEdge> = Vec::new();
    let mut edge_seen: HashSet<RelEdge> = HashSet::new();
    let mut endpoints: HashSet<String> = HashSet::new();
    let mut metrics = PortfolioMetrics {
        repo_count: parsed.len(),
        total_tasks: 0,
        done: 0,
        in_progress: 0,
        pending: 0,
        blocked: 0,
        pct: 0,
    };

    for ((_, input, data), slug) in parsed.iter().zip(&slugs) {
        let project = if data.project.is_empty() {
            input.label.clone()
        } else {
            data.project.clone()
        };

        let (mut done, mut in_progress, mut blocked, mut total) = (0usize, 0usize, 0usize, 0usize);
        for task in &data.task {
            total += 1;
            match task.status.as_str() {
                "done" => done += 1,
                "in_progress" => in_progress += 1,
                "blocked" => blocked += 1,
                _ => {}
            }
        }
        let pending = total - done - in_progress - blocked;
        metrics.total_tasks += total;
        metrics.done += done;
        metrics.in_progress += in_progress;
        metrics.blocked += blocked;
        metrics.pending += pending;

        // Cross-repo relations → tags (all declared) + arrows (resolved only).
        let mut rel_tags = Vec::new();
        for task in &data.task {
            for cr in &task.cross_repo {
                rel_tags.push(RelTag {
                    relation: cr.relation.clone(),
                    repo: cr.repo.clone(),
                    task_id: json_id_string(&cr.task_id),
                });
                if let Some(target) = slug_by_key.get(&cr.repo.to_lowercase()) {
                    if target == slug {
                        continue;
                    }
                    let edge = match cr.relation.as_str() {
                        // P blocks target: arrow P → target.
                        "blocks" => RelEdge {
                            source: slug.clone(),
                            target: target.clone(),
                            relation: "blocks".into(),
                        },
                        // target blocks P: arrow target → P.
                        "blocked_by" => RelEdge {
                            source: target.clone(),
                            target: slug.clone(),
                            relation: "blocks".into(),
                        },
                        // related: undirected, drawn once.
                        _ => RelEdge {
                            source: slug.clone(),
                            target: target.clone(),
                            relation: "related".into(),
                        },
                    };
                    if edge_seen.insert(RelEdge {
                        source: edge.source.clone(),
                        target: edge.target.clone(),
                        relation: edge.relation.clone(),
                    }) {
                        endpoints.insert(edge.source.clone());
                        endpoints.insert(edge.target.clone());
                        edges.push(edge);
                    }
                }
            }
        }

        repos.push(RepoView {
            project,
            slug: slug.clone(),
            path: input.path_display.clone(),
            total,
            done,
            in_progress,
            pending,
            blocked,
            pct: pct_of(done, total),
            pct_done: pct_of(done, total),
            pct_in_progress: pct_of(in_progress, total),
            pct_blocked: pct_of(blocked, total),
            pct_pending: pct_of(pending, total),
            has_rel: 0, // filled after all edges are known
            phases: build_owned_phase_views(data),
            dag: build_dag(&portfolio_dag_inputs(data)),
            rel_tags,
        });
    }

    for repo in &mut repos {
        if endpoints.contains(&repo.slug) {
            repo.has_rel = 1;
        }
    }
    metrics.pct = pct_of(metrics.done, metrics.total_tasks);

    // Aggregate island: every project's envelope verbatim, in one array.
    let raw: Vec<&str> = projects.iter().map(|p| p.data_json.trim()).collect();
    let data_island = escape_island(&format!("{{\"projects\":[{}]}}", raw.join(",")));
    let relations_json = escape_island(&serde_json::to_string(&edges)?);

    let mut env = build_env()?;
    env.add_template(
        "portfolio.html",
        include_str!("../templates/portfolio.html.j2"),
    )?;
    let tmpl = env.get_template("portfolio.html")?;
    let html = tmpl.render(minijinja::context! {
        today => today,
        repos => repos,
        metrics => metrics,
        data_island => data_island,
        relations_json => relations_json,
        styles => SHARED_STYLES,
    })?;
    Ok(html)
}

fn path_basename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string()
}

fn owned_task_view(task: &PortfolioTask) -> OwnedTaskView {
    OwnedTaskView {
        id: json_id_string(&task.id),
        title: task.title.clone(),
        status: task.status.clone(),
        eff: task.eff,
        eff_fmt: format_efficiency(task.eff),
        tier_glyph: tier_glyph(task.eff),
        markers: task.markers.clone(),
        depends_on: task
            .depends_on
            .iter()
            .map(json_id_string)
            .collect::<Vec<_>>()
            .join(" "),
        phase: task.phase,
    }
}

fn build_owned_phase_views(data: &PortfolioData) -> Vec<OwnedPhaseView> {
    let mut phase_entries: Vec<_> = data.phases.iter().collect();
    phase_entries.sort_by_key(|(_, phase)| phase.order);
    phase_entries
        .into_iter()
        .map(|(key, phase)| {
            let number: u32 = key.parse().unwrap_or(0);
            let mut pending = Vec::new();
            let mut in_progress = Vec::new();
            let mut done_tasks = Vec::new();
            for task in &data.task {
                if task.phase != number {
                    continue;
                }
                let view = owned_task_view(task);
                match task.status.as_str() {
                    "done" => done_tasks.push(view),
                    "in_progress" => in_progress.push(view),
                    _ => pending.push(view),
                }
            }
            let total = pending.len() + in_progress.len() + done_tasks.len();
            let done = done_tasks.len();
            OwnedPhaseView {
                number,
                name: phase.name.clone(),
                status: phase.status.clone(),
                done,
                total,
                pct: pct_of(done, total),
                pending,
                in_progress,
                done_tasks,
            }
        })
        .collect()
}

fn portfolio_dag_inputs(data: &PortfolioData) -> Vec<DagNodeInput> {
    data.task
        .iter()
        .map(|task| DagNodeInput {
            id: json_id_string(&task.id),
            title: task.title.clone(),
            status: task.status.clone(),
            depends_on: task.depends_on.iter().map(json_id_string).collect(),
        })
        .collect()
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
        let dag = build_dag(&dag_inputs(tasks.task.iter()));
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
        let dag = build_dag(&dag_inputs(tasks.task.iter()));
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
