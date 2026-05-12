use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use rmap::delegate::format_delegate_prompt;
use rmap::diff::{diff_toml, format_diff};
use rmap::doctor::DoctorReport;
use rmap::export::{export_filtered_json_str, export_json_str, export_task_json_str};
use rmap::mutate::{
    CrossRepoSpec, MarkerOp, add_dependency_str, update_markers_str, update_status_many_str,
};
use rmap::next::{format_next_task, next_task};
use rmap::paths::{ResolvedPaths, resolve_paths};
use rmap::query::{TaskFilter, find_task, format_task, format_task_row, list_tasks};
use rmap::render::render_roadmap_str;
use rmap::schema_json::schema_json_str;
use rmap::stale::{find_stale, parse_duration};
use rmap::today_iso;
use rmap::validate::{validate_tasks_file, validate_tasks_str};

#[derive(Debug, Parser)]
#[command(name = "rmap")]
#[command(about = "Manage portable roadmap data")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        check_render: bool,
    },
    Render {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
        #[arg(long, conflicts_with = "stdout")]
        dry: bool,
        #[arg(long)]
        stdout: bool,
    },
    Status {
        id: String,
        new_status: String,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    Next {
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Show {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        phase: Option<u32>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Schema,
    Diff {
        #[arg(long)]
        against: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Delegate {
        id: String,
        #[arg(long)]
        to: DelegateTarget,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Aggregate health report: validation findings + stale + score-decay + render drift.
    ///
    /// Always exits 0 — informational. Use `rmap validate` for strict schema gating.
    /// Exception: if tasks.toml is unparseable, exits non-zero (nothing to analyze).
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
    },
    /// Add or remove markers on a task: `rmap mark 75 +cx -parallel`.
    ///
    /// Each `ops` token must start with `+` (add) or `-` (remove). Add is a no-op
    /// when the marker is already present; remove is a no-op when absent.
    /// The full file is re-validated after edit and any invalid marker name
    /// (per `VALID_MARKERS`) aborts the write.
    Mark {
        id: String,
        /// `+marker` to add, `-marker` to remove. One or more, applied left-to-right.
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        ops: Vec<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Add a dependency to a task.
    ///
    /// In-repo: `rmap depend 75 on 74`. Cross-repo: `rmap depend 75 --cross-repo
    /// ccxt_client:42:blocks` (relation defaults to `blocks` if omitted). Both
    /// forms can be combined in a single call. Cycles and unknown ids are
    /// rejected via re-validation.
    Depend {
        id: String,
        /// Literal `on` keyword separating the in-repo dependency. Required when
        /// the next positional is the target task id.
        on: Option<String>,
        /// Target task id when adding an in-repo dependency.
        other: Option<String>,
        /// Cross-repo dependency: `<repo>:<task_id>[:<relation>]`.
        #[arg(long)]
        cross_repo: Option<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// List in-progress tasks idle longer than the given duration.
    Stale {
        /// Duration threshold. Units: d (day), w (7d), m (30d), y (365d). Example: "30d", "2w", "6m", "1y".
        #[arg(long)]
        over: String,
        /// Emit JSON envelope.
        #[arg(long)]
        json: bool,
        /// Path to tasks.toml (overrides discovery).
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum DelegateTarget {
    Claude,
    Codex,
    Cursor,
}

impl std::fmt::Display for DelegateTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => formatter.write_str("claude"),
            Self::Codex => formatter.write_str("codex"),
            Self::Cursor => formatter.write_str("cursor"),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ExportCommands {
    Json {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            tasks_path,
            check_render,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            if check_render && !roadmap_is_current(&paths)? {
                eprintln!("ROADMAP.md is out of sync; run rmap render");
                return Ok(ExitCode::from(2));
            }
            if !check_render {
                validate_tasks_file(&paths.tasks_path)?;
            }

            println!("valid");
        }
        Commands::Render {
            tasks_path,
            roadmap_path,
            data_path,
            dry,
            stdout,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            render(paths, dry, stdout)?;
        }
        Commands::Status {
            id,
            new_status,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_status(paths, &id, &new_status)?;
        }
        Commands::Next {
            marker,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;

            let task = next_task(&tasks, marker.as_deref());
            if json {
                println!("{}", export_task_json_str(task)?);
            } else if let Some(task) = task {
                println!("{}", format_next_task(task));
            }
        }
        Commands::Show {
            id,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let task =
                find_task(&tasks, &id).ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;

            if json {
                println!("{}", export_task_json_str(Some(task))?);
            } else {
                println!("{}", format_task(task));
            }
        }
        Commands::List {
            status,
            marker,
            phase,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = TaskFilter {
                status,
                marker,
                phase,
            };
            let listed = list_tasks(&tasks, &filter);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &listed)?);
            } else {
                for task in listed {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Schema => {
            println!("{}", schema_json_str()?);
        }
        Commands::Diff {
            against,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let current = validate_tasks_file(&paths.tasks_path)?;
            let against = against.unwrap_or_else(|| current.default_branch.clone());
            let base_input = git_show_tasks(&paths, &against)?;
            let base = validate_tasks_str(
                format!("{}:{}", against, repo_relative_tasks_path(&paths)?),
                &base_input,
            )?;
            let diff = diff_toml(&base, &current);

            if json {
                println!("{}", serde_json::to_string_pretty(&diff)?);
            } else {
                println!("{}", format_diff(&diff));
            }
        }
        Commands::Delegate { id, to, tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let prompt = format_delegate_prompt(&tasks, &id, &to.to_string())
                .ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;

            print!("{prompt}");
        }
        Commands::Export {
            command: ExportCommands::Json { tasks_path },
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            println!("{}", export_json_str(&tasks)?);
        }
        Commands::Mark {
            id,
            ops,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_markers(paths, &id, &ops)?;
        }
        Commands::Depend {
            id,
            on,
            other,
            cross_repo,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            add_dependency(
                paths,
                &id,
                on.as_deref(),
                other.as_deref(),
                cross_repo.as_deref(),
            )?;
        }
        Commands::Stale {
            over,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let max_age = parse_duration(&over).map_err(|e| anyhow::anyhow!(e))?;
            let today = today_iso();
            let stale = find_stale(&tasks, max_age, &today);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &stale)?);
            } else {
                for task in stale {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Doctor {
            json,
            tasks_path,
            roadmap_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, None)?;
            let input = std::fs::read_to_string(&paths.tasks_path)
                .with_context(|| format!("read {}", paths.tasks_path.display()))?;
            // If tasks.toml won't parse at all, propagate the error (nothing to analyze).
            let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &input)?;
            let roadmap_input = std::fs::read_to_string(&paths.roadmap_path).ok();
            let today = today_iso();

            let report = DoctorReport::run(
                &tasks,
                &paths.tasks_path.display().to_string(),
                &input,
                roadmap_input.as_deref(),
                &today,
            );

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{report}");
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn render(paths: ResolvedPaths, dry: bool, stdout: bool) -> Result<()> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    if stdout {
        print!("{rendered_roadmap}");
        return Ok(());
    }

    if dry {
        println!("would render {}", paths.roadmap_path.display());
        println!("would export {}", paths.data_path.display());
        return Ok(());
    }

    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("rendered");

    Ok(())
}

fn update_markers(paths: ResolvedPaths, task_id: &str, op_tokens: &[String]) -> Result<()> {
    let ops: Vec<MarkerOp<'_>> = op_tokens
        .iter()
        .map(|token| MarkerOp::parse(token))
        .collect::<Result<_, _>>()?;

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = update_markers_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        &ops,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

fn add_dependency(
    paths: ResolvedPaths,
    task_id: &str,
    on_token: Option<&str>,
    other: Option<&str>,
    cross_repo_spec: Option<&str>,
) -> Result<()> {
    let in_repo: Option<&str> = match (on_token, other) {
        (Some("on"), Some(other_id)) => Some(other_id),
        (Some("on"), None) => {
            bail!("missing target task id after `on` (use `rmap depend <id> on <other-id>`)")
        }
        (Some(kw), _) => bail!(
            "unexpected positional {:?} — expected `on <task_id>`",
            kw
        ),
        // Clap fills optional positionals in declaration order, so `on` is always
        // None before `other`; the wildcard is defensive against future clap changes.
        (None, _) => None,
    };

    let cross_repo = cross_repo_spec.map(CrossRepoSpec::parse).transpose()?;

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = add_dependency_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        in_repo,
        cross_repo.as_ref(),
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

fn update_status(paths: ResolvedPaths, task_id: &str, new_status: &str) -> Result<()> {
    let ids: Vec<&str> = task_id
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if ids.is_empty() {
        anyhow::bail!("no task ids provided (got {:?})", task_id);
    }

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = update_status_many_str(
        paths.tasks_path.display().to_string(),
        &input,
        &ids,
        new_status,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

fn render_outputs(paths: &ResolvedPaths, tasks: &rmap::schema::Tasks) -> Result<(String, String)> {
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered_roadmap = render_roadmap_str(&roadmap, tasks)?;
    let rendered_data = export_json_str(tasks)?;

    Ok((rendered_roadmap, rendered_data))
}

fn roadmap_is_current(paths: &ResolvedPaths) -> Result<bool> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered = render_roadmap_str(&roadmap, &tasks)?;

    Ok(rendered == roadmap)
}

fn write_outputs(
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

fn git_show_tasks(paths: &ResolvedPaths, against: &str) -> Result<String> {
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

fn repo_relative_tasks_path(paths: &ResolvedPaths) -> Result<String> {
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

fn git_repo_root(project_root: &std::path::Path) -> Result<PathBuf> {
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
