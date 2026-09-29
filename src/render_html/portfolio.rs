use super::*;

// ───────────────────────────────────────────────────────────────────────────
// Portfolio (multi-project) view — `rmap render --html --multi`.
//
// Each project is normalized to its exported JSON envelope before reaching this
// module (project root → validate+export, data.json path → file read); the CLI
// owns that filesystem step so the renderer stays pure. The envelope is parsed
// for the per-repo views AND embedded verbatim in the aggregate `rmap-data`
// island so agents read every project's full structured data from one element.
// Repos render as rails; clicking a rail expands its board (same component
// macros). Cross-repo relations (`cross_repo`) become gutter cords.
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
    /// Island index — the detail panel's lookup key for this repo's cards.
    index: usize,
    counts: Counts,
    segments: Vec<Segment>,
    /// 1 when this repo is an endpoint of a resolved cross-repo edge (lights
    /// its gutter anchor). `u32` so the template can emit it as `data-has-rel`.
    has_rel: u32,
    phases: Vec<PhaseView>,
    dag: DagView,
    rel_tags: Vec<RelTag>,
}

/// Resolved cross-repo edge (source/target are repo slugs) — drives the
/// portfolio JS overlay. `relation` is the CSS class (`blocks` / `related`).
#[derive(Serialize, Hash, PartialEq, Eq)]
struct RelEdge {
    source: String,
    target: String,
    relation: String,
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
/// `today` is the header stamp. Each `ProjectInput.data_json` is parsed for its
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
        .map(|(i, input, data)| slugify(&display_name(input, data), *i))
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
    let mut totals = Counts::default();
    let mut rack_cards: Vec<CardView> = Vec::new();

    for ((index, input, data), slug) in parsed.iter().zip(&slugs) {
        let project = display_name(input, data);
        let views = project_views(data, *index, &project);
        totals.absorb(&views.counts);
        rack_cards.extend(views.cards().cloned());

        // Cross-repo relations → tags (all declared) + cords (resolved only).
        let mut rel_tags = Vec::new();
        for task in &data.task {
            for cr in &task.cross_repo {
                rel_tags.push(RelTag {
                    relation: cr.relation.clone(),
                    repo: cr.repo.clone(),
                    task_id: json_id_string(&cr.task_id),
                });
                let Some(target) = slug_by_key.get(&cr.repo.to_lowercase()) else {
                    continue;
                };
                if target == slug {
                    continue;
                }
                let (source, target, relation) = match cr.relation.as_str() {
                    // P blocks target: cord P → target.
                    "blocks" => (slug.clone(), target.clone(), "blocks"),
                    // target blocks P: cord target → P.
                    "blocked_by" => (target.clone(), slug.clone(), "blocks"),
                    // related: undirected, drawn once.
                    _ => (slug.clone(), target.clone(), "related"),
                };
                let edge = RelEdge {
                    source,
                    target,
                    relation: relation.into(),
                };
                if !edge_seen.contains(&edge) {
                    endpoints.insert(edge.source.clone());
                    endpoints.insert(edge.target.clone());
                    edge_seen.insert(RelEdge {
                        source: edge.source.clone(),
                        target: edge.target.clone(),
                        relation: edge.relation.clone(),
                    });
                    edges.push(edge);
                }
            }
        }

        let counts = views.counts;
        repos.push(RepoView {
            project,
            slug: slug.clone(),
            path: input.path_display.clone(),
            index: *index,
            segments: counts.segments(),
            counts,
            has_rel: 0, // filled after all edges are known
            phases: views.phases,
            dag: views.dag,
            rel_tags,
        });
    }

    for repo in &mut repos {
        if endpoints.contains(&repo.slug) {
            repo.has_rel = 1;
        }
    }
    totals.finish();
    let racks = Racks::collect(rack_cards.iter());

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
        counts => totals,
        segments => totals.segments(),
        racks => racks,
        data_island => data_island,
        relations_json => relations_json,
        styles => SHARED_STYLES,
        script => SHARED_SCRIPT,
    })?;
    Ok(html)
}

fn display_name(input: &ProjectInput, data: &PortfolioData) -> String {
    if data.project.is_empty() {
        input.label.clone()
    } else {
        data.project.clone()
    }
}

fn path_basename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string()
}
