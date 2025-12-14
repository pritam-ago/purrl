use anyhow::Result;
use rustyline::{error::ReadlineError, DefaultEditor};

use crate::ai::interpret_command;
use crate::engine::execute_action;
use crate::indexer::run_indexer;
use crate::types::SearchResults;

use std::process::Command;

fn open_path(path: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }

    Ok(())
}

fn clear_terminal() {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "cls"])
            .status()
            .unwrap();
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("clear").status().unwrap();
    }
}

pub fn run_shell() -> Result<()> {
    let mut last_results: Option<SearchResults> = None;
    let mut rl = DefaultEditor::new()?;

    println!("🐱  Meow shell activated.");
    println!("Type 'exit' or 'quit' to leave.\n");

    loop {
        let line = rl.readline("meow> ");

        match line {
            Ok(input) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                // Exit
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    println!("👋 Bye, human.");
                    break;
                }

                // Clear
                if matches!(input, "clear" | "cls" | "clean") {
                    clear_terminal();
                    println!("🐱  Meow shell refreshed.\n");
                    continue;
                }

                // Index
                if matches!(input, "index" | "reindex") {
                    println!("📚 Building semantic index…");
                    match run_indexer() {
                        Ok(_) => println!("✅ Indexing finished.\n"),
                        Err(e) => println!("❌ Indexing failed: {e}"),
                    }
                    continue;
                }

                // open <n>
                if input.to_lowercase().starts_with("open ") {
                    let arg = input[5..].trim();

                    if let Ok(n) = arg.parse::<usize>() {
                        let Some(results) = &last_results else {
                            println!("❌ No previous results. Run a search first.");
                            continue;
                        };

                        if n == 0 || n > results.items.len() {
                            println!(
                                "❌ Invalid index. Choose 1..{}",
                                results.items.len()
                            );
                            continue;
                        }

                        let path = &results.items[n - 1];
                        open_path(path)?;
                        println!("✅ Opened: {}", path);
                    } else {
                        println!("❌ Usage: open <number>");
                    }

                    continue;
                }

                // AI command
                if input.starts_with("ai ") {
                    let query = input.trim_start_matches("ai ").to_string();

                    match interpret_command(&query) {
                        Ok(action) => {
                            println!("🤖 AI interpreted:\n{:#?}", action);

                            match execute_action(action) {
                                Ok(Some(results)) => {
                                    last_results = Some(results);
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    println!("❌ Action execution failed: {e}");
                                }
                            }
                        }
                        Err(err) => {
                            println!("❌ AI Error: {err}");
                        }
                    }

                    continue;
                }

                println!("❓ Unknown command. Try `ai find ...`, `index`, `open <n>`");
            }

            Err(ReadlineError::Interrupted) => {
                println!("\n(Interrupted) Bye 🐾");
                break;
            }

            Err(ReadlineError::Eof) => {
                println!("\n(EOF) Bye 🐾");
                break;
            }

            Err(err) => {
                eprintln!("Error reading line: {err}");
                break;
            }
        }
    }

    Ok(())
}
