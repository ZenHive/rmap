use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rmap::render::render_roadmap_file;
use rmap::validate::validate_tasks_file;

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
        path: PathBuf,
    },
    Render {
        tasks_path: PathBuf,
        roadmap_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            validate_tasks_file(&path)?;
            println!("valid");
        }
        Commands::Render {
            tasks_path,
            roadmap_path,
        } => {
            let tasks = validate_tasks_file(&tasks_path)?;
            render_roadmap_file(&roadmap_path, &tasks)?;
            println!("rendered");
        }
    }

    Ok(())
}
