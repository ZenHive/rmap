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
    // input label, its `project` field, or its source-path basename. The first
    // project in input order wins every collision, regardless of alias kind.
    // Share this lookup with the detail panel without changing the envelopes.
    let mut repo_aliases: BTreeMap<String, usize> = BTreeMap::new();
    let slugs: Vec<String> = parsed
        .iter()
        .map(|(i, input, data)| slugify(&display_name(input, data), *i))
        .collect();
    for (index, input, data) in &parsed {
        for key in [
            data.project.to_lowercase(),
            input.label.to_lowercase(),
            path_basename(&input.path_display).to_lowercase(),
        ] {
            if !key.is_empty() {
                repo_aliases.entry(key).or_insert(*index);
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
                let Some(target_index) = repo_aliases.get(&cr.repo.to_lowercase()) else {
                    continue;
                };
                let target = &slugs[*target_index];
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
    let data_island = escape_island(&format!(
        "{{\"projects\":[{}],\"repo_aliases\":{}}}",
        raw.join(","),
        serde_json::to_string(&repo_aliases)?
    ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn island(html: &str, id: &str) -> Value {
        let start = format!("<script id=\"{id}\" type=\"application/json\">");
        let json = html
            .split_once(&start)
            .unwrap()
            .1
            .split_once("</script>")
            .unwrap()
            .0;
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn portfolio_aliases_share_first_input_wins_resolution_with_cords() {
        let aliases = ["PROJECT", "Label", "basename", "__proto__", "missing", ""];
        let inputs = vec![
            ProjectInput {
                label: "Label".into(),
                path_display: "/repos/basename".into(),
                data_json: json!({"project": "Project", "custom": {"preserved": true}}).to_string(),
            },
            ProjectInput {
                label: "__proto__".into(),
                path_display: "/other/LABEL".into(),
                data_json: json!({"project": "PROJECT"}).to_string(),
            },
            ProjectInput {
                label: "".into(),
                path_display: "".into(),
                data_json: json!({"project": "basename"}).to_string(),
            },
            ProjectInput {
                label: "Source".into(),
                path_display: "/repos/source".into(),
                data_json: json!({"project": "Source", "task": [{
                    "id": 1, "cross_repo": aliases.map(|repo| json!({
                        "repo": repo, "task_id": 1, "relation": "blocks"
                    }))
                }]})
                .to_string(),
            },
        ];
        let html = render_portfolio_str(&inputs, "2026-09-29").unwrap();
        let data = island(&html, "rmap-data");
        assert_eq!(
            data["repo_aliases"],
            json!({
                "project": 0, "label": 0, "basename": 0, "__proto__": 1, "source": 3
            })
        );
        for (i, input) in inputs.iter().enumerate() {
            assert_eq!(
                data["projects"][i],
                serde_json::from_str::<Value>(&input.data_json).unwrap()
            );
            assert!(html.contains(input.data_json.as_str()));
        }
        assert_eq!(
            island(&html, "rmap-relations"),
            json!([
                {"source": "source-3", "target": "project-0", "relation": "blocks"},
                {"source": "source-3", "target": "project-1", "relation": "blocks"}
            ])
        );
        assert_eq!(html, render_portfolio_str(&inputs, "2026-09-29").unwrap());
    }
}
