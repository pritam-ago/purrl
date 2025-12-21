mod shell;
mod ai;
mod engine;
mod embedding;
mod vector_db;
mod similarity;
mod indexer;
mod types;
mod ai_decider;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// meow — AI-augmented filesystem shell (MVP)
#[derive(Parser, Debug)]
#[command(
    name = "meow",
    version,
    about = "A curious cat that explores your filesystem.",
    long_about = None
)]
struct Cli {
    /// Optional subcommand. If omitted, starts interactive meow shell.
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the interactive Meow shell
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

    match cli.command {
        Some(Commands::Shell) | None => {
            // Default is interactive shell
            shell::run_shell()?;
        }
        Some(Commands::Hello { name }) => {
            println!("Meow, {name}!");
        }
        Some(Commands::Index) => {
            indexer::run_indexer()?;
        }
    }

    Ok(())
}
