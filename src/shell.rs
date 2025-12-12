use anyhow::Result;
use rustyline::{error::ReadlineError, DefaultEditor};
use crate::search::search_files;
use std::path::PathBuf;
use crate::ai::interpret_command;
use crate::engine::execute_action;
use crate::indexer::run_indexer;


fn clear_terminal() {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "cls"])
            .status()
            .unwrap();
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("clear")
            .status()
            .unwrap();
    }
}


pub fn run_shell() -> Result<()> {
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

                // Add to history (ignore return bool)
                let _ = rl.add_history_entry(input);

                // Exit commands
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    println!("👋 Bye, human.");
                    break;
                }

                // CLEAR SCREEN COMMANDS
                if input.eq_ignore_ascii_case("clear")
                    || input.eq_ignore_ascii_case("cls")
                    || input.eq_ignore_ascii_case("clean")
                {
                    clear_terminal();
                    println!("🐱  Meow shell refreshed.\n");
                    continue;
                }

                // INDEX COMMAND
                if input.eq_ignore_ascii_case("index") || input.eq_ignore_ascii_case("reindex") {
                    println!("📚 Building semantic index…");
                    match run_indexer() {
                        Ok(_) => println!("✅ Indexing finished.\n"),
                        Err(e) => println!("❌ Indexing failed: {e}"),
                    }
                    continue;
                }


                if input.starts_with("ai ") {
                    let query = input.replace("ai ", "");

                    match interpret_command(&query) {
                        Ok(action) => {
                            println!("🤖 AI interpreted:\n{:#?}", action);
                            match execute_action(action) {
                                Ok(_) => {},
                                Err(e) => println!("❌ Action execution failed: {e}"),
                            }
                        }
                        Err(err) => {
                            println!("❌ AI Error: {err}");
                        }
                    }

                    continue;
                }

                if input.starts_with("where is") || input.starts_with("find") {
                    let query: String = input
                        .replace("where is", "")
                        .replace("find", "")
                        .trim()
                        .to_string();

                    if query.is_empty() {
                        println!("😿 You must specify what to search for.");
                        continue;
                    }

                    println!("🔍 Searching for '{query}' ...");

                    // Search from current directory for now
                    let root = PathBuf::from(".");
                    let matches = search_files(&root, &query);

                    if matches.is_empty() {
                        println!("😺 No matches found!");
                    } else {
                        println!("😼 Found {} match(es):", matches.len());
                        for path in matches {
                            println!("• {}", path.display());
                        }
                    }

                    continue;
                }

            }

            Err(ReadlineError::Interrupted) => {
                // Ctrl+C
                println!("\n(Interrupted) Bye 🐾");
                break;
            }

            Err(ReadlineError::Eof) => {
                // Ctrl+D
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


