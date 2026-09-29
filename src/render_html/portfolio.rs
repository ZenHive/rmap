use super::*;

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
        include_str!("../../templates/portfolio.html.j2"),
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
