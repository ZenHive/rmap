use anyhow::{Context, Result, bail};
use rmap::export::export_json_str;
use rmap::next_bundle::BundlePick;
use rmap::paths::ResolvedPaths;
use rmap::query::format_task_row;
use rmap::render::render_roadmap_str;
use rmap::validate::validate_tasks_file;
use std::path::PathBuf;

/// Human-readable `rmap next-bundle` output. Header `bundle <name>  phase <N> —
/// <phase_name>  [<done>/<total>]  — <description>`, then one task per line
/// using `format_task_row` (mirroring `rmap list`) with a two-space indent.

pub(crate) fn format_next_bundle_human(
    tasks: &rmap::schema::Tasks,
    pick: &BundlePick<'_>,
) -> String {
    let phase_name = tasks
        .phases
        .get(&pick.bundle.phase.to_string())
        .map(|phase| phase.name.as_str())
        .unwrap_or("");
    let (done_count, total_count) = tasks
        .task
        .iter()
        .filter(|task| task.bundle == pick.name)
        .fold((0u32, 0u32), |(d, t), task| {
            (d + u32::from(task.status == "done"), t + 1)
        });

    let mut out = format!(
        "bundle {}  phase {} — {}  [{}/{}]  — {}\n",
        pick.name, pick.bundle.phase, phase_name, done_count, total_count, pick.bundle.description,
    );
    for task in &pick.tasks {
        out.push_str("  ");
        out.push_str(&format_task_row(task));
        out.push('\n');
    }
    out
}

pub(crate) fn render_outputs(
    paths: &ResolvedPaths,
    tasks: &rmap::schema::Tasks,
) -> Result<(String, String)> {
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered_roadmap = render_roadmap_str(&roadmap, tasks)?;
    let rendered_data = export_json_str(tasks)?;

    Ok((rendered_roadmap, rendered_data))
}

pub(crate) fn roadmap_is_current(paths: &ResolvedPaths) -> Result<bool> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered = render_roadmap_str(&roadmap, &tasks)?;

    Ok(rendered == roadmap)
}

pub(crate) fn write_outputs(
    paths: &ResolvedPaths,
    rendered_roadmap: String,
    rendered_data: String,
) -> Result<()> {
    std::fs::write(&paths.roadmap_path, rendered_roadmap)
        .with_context(|| format!("write {}", paths.roadmap_path.display()))?;
    std::fs::write(&paths.data_path, rendered_data)
        .with_context(|| format!("write {}", paths.data_path.display()))?;

    Ok(())
}

pub(crate) fn git_show_tasks(paths: &ResolvedPaths, against: &str) -> Result<String> {
    let repo_path = repo_relative_tasks_path(paths)?;
    let object = format!("{against}:{repo_path}");
    let output = std::process::Command::new("git")
        .arg("show")
        .arg(&object)
        .current_dir(&paths.project_root)
        .output()
        .with_context(|| format!("run git show {object}"))?;

    if !output.status.success() {
        bail!(
            "could not read {} from git ref {}\n{}",
            repo_path,
            against,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8(output.stdout).context("git show returned non-UTF-8 tasks.toml")
}

pub(crate) fn repo_relative_tasks_path(paths: &ResolvedPaths) -> Result<String> {
    let repo_root = git_repo_root(&paths.project_root)?;
    let tasks_path = std::fs::canonicalize(&paths.tasks_path)
        .with_context(|| format!("canonicalize {}", paths.tasks_path.display()))?;
    let relative = tasks_path.strip_prefix(&repo_root).with_context(|| {
        format!(
            "{} is not inside git repository {}",
            tasks_path.display(),
            repo_root.display()
        )
    })?;

    Ok(relative.to_string_lossy().to_string())
}

pub(crate) fn git_repo_root(project_root: &std::path::Path) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(project_root)
        .output()
        .context("run git rev-parse --show-toplevel")?;

    if !output.status.success() {
        bail!(
            "could not find git repository for {}\n{}",
            project_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let path = String::from_utf8(output.stdout).context("git rev-parse returned non-UTF-8 path")?;
    std::fs::canonicalize(path.trim()).with_context(|| format!("canonicalize {}", path.trim()))
}
