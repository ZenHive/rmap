use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rmap::export::export_json_str;
use rmap::mutate::update_status_str;
use rmap::next::{format_next_task, next_task};
use rmap::paths::{ResolvedPaths, resolve_paths};
use rmap::render::render_roadmap_str;
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
        tasks_path: Option<PathBuf>,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ExportCommands {
    Json {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            validate_tasks_file(&paths.tasks_path)?;
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
        Commands::Next { marker, tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let Some(task) = next_task(&tasks, marker.as_deref()) else {
                bail!("no matching pending unblocked task");
            };

            println!("{}", format_next_task(task));
        }
        Commands::Export {
            command: ExportCommands::Json { tasks_path },
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            println!("{}", export_json_str(&tasks)?);
        }
    }

    Ok(())
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
