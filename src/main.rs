use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
    Validate { path: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            validate_tasks_file(&path)?;
            println!("valid");
        }
    }

    Ok(())
}
