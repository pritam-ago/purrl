mod ai;
mod ai_decider;
mod embedding;
mod engine;
mod indexer;
mod shell;
mod similarity;
mod types;
mod vector_db;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// purrl — AI-augmented filesystem shell (MVP)
#[derive(Parser, Debug)]
#[command(
    name = "purrl",
    version,
    about = "A curious cat that explores your filesystem.",
    long_about = None
)]
struct Cli {
    /// Path to the vector index. Defaults to <data_dir>/purrl/vectors.db.
    #[arg(long, global = true, value_name = "PATH")]
    db: Option<PathBuf>,

    /// Optional subcommand. If omitted, starts the interactive purrl shell.
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the interactive purrl shell
    Shell,
    /// Just print a one-off message (for testing)
    #[command(alias = "hi")]
    Hello {
        /// Optional name
        #[arg(default_value = "human")]
        name: String,
    },
    Index,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = vector_db::resolve_db_path(cli.db)?;
    vector_db::legacy_db_notice(&db_path);

    match cli.command {
        Some(Commands::Shell) | None => {
            // Default is interactive shell
            shell::run_shell(&db_path)?;
        }
        Some(Commands::Hello { name }) => {
            println!("Purrl, {name}!");
        }
        Some(Commands::Index) => {
            indexer::run_indexer(&db_path)?;
        }
    }

    Ok(())
}
