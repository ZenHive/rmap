use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use rmap::delegate::format_delegate_prompt;
use rmap::diff::{diff_toml, format_diff};
use rmap::export::{export_filtered_json_str, export_json_str, export_task_json_str};
use rmap::mutate::update_status_str;
use rmap::next::{format_next_task, next_task};
use rmap::paths::{ResolvedPaths, resolve_paths};
use rmap::query::{TaskFilter, find_task, format_task, format_task_row, list_tasks};
use rmap::render::render_roadmap_str;
use rmap::schema_json::schema_json_str;
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
    Schema {
        #[arg(long)]
        json: bool,
    },
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
        Commands::Schema { json } => {
            if !json {
                bail!("schema currently supports --json only");
            }

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

fn update_status(paths: ResolvedPaths, task_id: &str, new_status: &str) -> Result<()> {
    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;
    let updated = update_status_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
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
