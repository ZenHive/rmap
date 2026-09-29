use crate::commands::output::{render_outputs, write_outputs};
use anyhow::{Context, Result, bail};
use notify::Watcher;
use rmap::paths::{ResolvedPaths, resolve_paths};
use rmap::render_html::{ProjectInput, render_html_str, render_portfolio_str};
use rmap::validate::validate_tasks_file;
use rmap::{today_iso, watch};
use std::path::PathBuf;

pub(crate) fn render(paths: ResolvedPaths, dry: bool, stdout: bool, html: bool) -> Result<()> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;

    if stdout {
        // `--stdout` emits to stdout and writes no files. With `--html` the
        // more specific request wins (the Unix convention rmap already uses
        // for `--bundle` over focus): print the HTML, not the markdown.
        if html {
            print!("{}", render_html_str(&tasks, &today_iso())?);
        } else {
            let (rendered_roadmap, _) = render_outputs(&paths, &tasks)?;
            print!("{rendered_roadmap}");
        }
        return Ok(());
    }

    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;
    let rendered_html = if html {
        Some(render_html_str(&tasks, &today_iso())?)
    } else {
        None
    };

    if dry {
        println!("would render {}", paths.roadmap_path.display());
        println!("would export {}", paths.data_path.display());
        if html {
            println!("would write {}", paths.html_path.display());
        }
        return Ok(());
    }

    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    if let Some(rendered_html) = rendered_html {
        write_html(&paths, rendered_html)?;
    }
    println!("rendered");

    Ok(())
}

/// Write the static HTML view, creating `roadmap/dist/` on first run.
fn write_html(paths: &ResolvedPaths, rendered_html: String) -> Result<()> {
    if let Some(dir) = paths.html_path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    std::fs::write(&paths.html_path, rendered_html)
        .with_context(|| format!("write {}", paths.html_path.display()))?;
    Ok(())
}

/// `rmap render --multi P1 P2 …` — the portfolio view. Each path is a project
/// root or a `data.json` path; both are normalized to the exported JSON
/// envelope. Requires `--html`; default output is `roadmap/dist/portfolio.html`
/// under the current project root (overridable with `--out`).
pub(crate) fn render_portfolio_cmd(
    inputs: &[PathBuf],
    html: bool,
    dry: bool,
    stdout: bool,
    out: Option<PathBuf>,
) -> Result<()> {
    if !html && !stdout {
        bail!("--multi renders the portfolio HTML view; pass --html (or --stdout)");
    }

    let projects: Vec<ProjectInput> = inputs
        .iter()
        .map(|path| load_project_input(path))
        .collect::<Result<_>>()?;

    let rendered = render_portfolio_str(&projects, &today_iso())?;

    if stdout {
        print!("{rendered}");
        return Ok(());
    }

    let out_path = match out {
        Some(path) => path,
        None => default_portfolio_path()?,
    };

    if dry {
        println!("would write {}", out_path.display());
        return Ok(());
    }

    if let Some(dir) = out_path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    std::fs::write(&out_path, rendered).with_context(|| format!("write {}", out_path.display()))?;
    println!("rendered {}", out_path.display());
    Ok(())
}

/// Resolve one `--multi` argument to a project envelope. A directory or a
/// `tasks.toml` path is validated and exported (canonical); a `data.json` path
/// is read verbatim; a directory with no `tasks.toml` falls back to its
/// `roadmap/data.json`.
fn load_project_input(path: &std::path::Path) -> Result<ProjectInput> {
    let display = path.display().to_string();

    let from_tasks = |tasks_path: &std::path::Path| -> Result<ProjectInput> {
        let tasks = validate_tasks_file(tasks_path)?;
        let json = rmap::export::export_compact_json_str(&tasks)
            .with_context(|| format!("export {}", tasks_path.display()))?;
        Ok(ProjectInput {
            label: tasks.project.clone(),
            path_display: display.clone(),
            data_json: json,
        })
    };
    let from_data_json = |data_path: &std::path::Path| -> Result<ProjectInput> {
        let json = std::fs::read_to_string(data_path)
            .with_context(|| format!("read {}", data_path.display()))?;
        Ok(ProjectInput {
            label: display.clone(),
            path_display: display.clone(),
            data_json: json,
        })
    };

    if path.is_dir() {
        let tasks_path = path.join("roadmap/tasks.toml");
        if tasks_path.is_file() {
            return from_tasks(&tasks_path);
        }
        let data_path = path.join("roadmap/data.json");
        if data_path.is_file() {
            return from_data_json(&data_path);
        }
        bail!(
            "no roadmap/tasks.toml or roadmap/data.json under {}",
            display
        );
    }

    match path.file_name().and_then(|n| n.to_str()) {
        Some("tasks.toml") => from_tasks(path),
        _ if path.extension().and_then(|e| e.to_str()) == Some("json") => from_data_json(path),
        _ => bail!(
            "{} is not a project root, tasks.toml, or data.json path",
            display
        ),
    }
}

/// Default portfolio output: `roadmap/dist/portfolio.html` under the current
/// project root, falling back to a cwd-relative path when not inside a project.
fn default_portfolio_path() -> Result<PathBuf> {
    match resolve_paths(None, None, None) {
        Ok(paths) => Ok(paths
            .html_path
            .parent()
            .unwrap_or(std::path::Path::new("roadmap/dist"))
            .join("portfolio.html")),
        Err(_) => Ok(PathBuf::from("roadmap/dist/portfolio.html")),
    }
}

/// Which outputs one `rerender_if_changed` pass wrote. `report_render` turns
/// this into either the `rendered` text line or a JSON `rendered` event whose
/// `outputs` array lists exactly the files that changed.
struct RenderOutcome {
    roadmap_changed: bool,
    data_changed: bool,
}

impl RenderOutcome {
    /// True when at least one output file was written this render.
    fn changed(&self) -> bool {
        self.roadmap_changed || self.data_changed
    }
}

/// `rmap watch`: re-render `ROADMAP.md` + `roadmap/data.json` whenever
/// `roadmap/tasks.toml` changes. Blocks until the process is interrupted.
///
/// The watcher is registered on the `roadmap/` directory (not the file) so it
/// survives editor atomic-save renames; events are filtered down to
/// `tasks.toml` by `watch::is_tasks_toml_event`, which also keeps our own
/// `data.json` writes from re-triggering the loop. Idempotency comes from
/// `watch::write_if_changed`. A failed render (e.g. mid-edit invalid TOML) is
/// reported and the loop continues.
///
/// `json` selects the output mode for render events (see `report_render`); the
/// startup line and watcher-infrastructure errors stay text on stderr either way.
pub(crate) fn watch_command(paths: ResolvedPaths, json: bool) -> Result<()> {
    let watch_dir = paths
        .tasks_path
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "cannot determine parent directory of {}",
                paths.tasks_path.display()
            )
        })?
        .to_path_buf();

    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx).context("create filesystem watcher")?;
    watcher
        .watch(&watch_dir, notify::RecursiveMode::NonRecursive)
        .with_context(|| format!("watch {}", watch_dir.display()))?;

    eprintln!("watching {} — Ctrl-C to stop", paths.tasks_path.display());

    // Initial render so the outputs are current before the first event — a
    // no-op when ROADMAP.md / data.json already match. A startup failure
    // (tasks.toml missing or invalid) is reported but does not abort: the loop
    // still starts and recovers on the next valid save.
    report_render(rerender_if_changed(&paths), &paths, json);

    for res in rx {
        match res {
            Ok(event) => {
                if watch::is_tasks_toml_event(&paths.tasks_path, &event) {
                    report_render(rerender_if_changed(&paths), &paths, json);
                }
            }
            Err(err) => eprintln!("rmap watch: watcher error: {err}"),
        }
    }

    Ok(())
}

/// Print the outcome of one watch-loop render attempt.
///
/// In text mode (`json == false`): `rendered` to stdout on a real write,
/// nothing on a no-op, the error to stderr on failure. In `--json` mode: one
/// compact JSON event line to stdout per render event — `rendered` (with the
/// changed `outputs`) on a write, `error` on a failure — and nothing on a
/// no-op. The stdout / stderr split is the agent contract: stdout carries only
/// render events, leaving stderr for the startup line and watcher-level errors.
fn report_render(result: Result<RenderOutcome>, paths: &ResolvedPaths, json: bool) {
    match result {
        Ok(outcome) => {
            if !outcome.changed() {
                return;
            }
            if json {
                let mut names: Vec<&str> = Vec::with_capacity(2);
                if outcome.roadmap_changed {
                    names.push(output_basename(&paths.roadmap_path));
                }
                if outcome.data_changed {
                    names.push(output_basename(&paths.data_path));
                }
                emit_json_line(&watch::render_event_line(&names));
            } else {
                println!("rendered");
            }
        }
        Err(err) => {
            if json {
                emit_json_line(&watch::error_event_line(&format!("{err:#}")));
            } else {
                eprintln!("rmap watch: {err:#}");
            }
        }
    }
}

/// Basename of an output path for the `--json` event `outputs` array — keeps
/// host-absolute paths out of the stable agent contract.
fn output_basename(path: &std::path::Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
}

/// Write one JSON event line to stdout and flush. The explicit flush matters:
/// Rust stdout is block-buffered when piped, and an event stream consumed by
/// `jq` must not be batched.
fn emit_json_line(line: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// Validate `tasks.toml`, re-render both outputs, and write each only when its
/// content changed. The returned `RenderOutcome` records which files were written.
fn rerender_if_changed(paths: &ResolvedPaths) -> Result<RenderOutcome> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let (rendered_roadmap, rendered_data) = render_outputs(paths, &tasks)?;
    let roadmap_changed = watch::write_if_changed(&paths.roadmap_path, &rendered_roadmap)?;
    let data_changed = watch::write_if_changed(&paths.data_path, &rendered_data)?;
    Ok(RenderOutcome {
        roadmap_changed,
        data_changed,
    })
}
