use crate::ai::AiAction;
use crate::embedding::embed_text;
use crate::similarity::cosine_similarity;
use crate::vector_db::VectorDB;
use chrono::{Duration, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_action(action: AiAction) -> anyhow::Result<()> {
    match action.intent.as_str() {
        "search" => {
            execute_search(action)?;
            Ok(())
        }
        "open" => {
            println!("(open not implemented yet)");
            Ok(())
        }
        "read" => {
            println!("(read not implemented yet)");
            Ok(())
        }
        "summarize" => {
            println!("(summarize not implemented yet)");
            Ok(())
        }
        "delete" => {
            println!("(delete not implemented yet)");
            Ok(())
        }
        _ => {
            println!("❌ Unknown intent: {}", action.intent);
            Ok(())
        }
    }
}

fn execute_search(action: AiAction) -> anyhow::Result<()> {
    println!("🔍 Executing AI search...");

    // ---------- HARD GUARD ----------
    let raw_query = match &action.query {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            println!("❌ Cannot run semantic search without a query.");
            return Ok(());
        }
    };

    let cleaned_query = clean_query(raw_query);
    println!("🧹 Cleaned query: \"{}\"", cleaned_query);

    // ---------- FOLDER ----------
    let root = resolve_folder_hint(&action.folder_hint, &action.query);
    println!("📁 Searching in: {}", root.display());

    // ---------- FILE WALK ----------
    let mut files = Vec::new();
    walk(&root, &mut files);

    // ---------- TIME FILTER ----------
    if let Some(time) = &action.time_filter {
        files.retain(|p| file_matches_time(p, time));
    }

    // ---------- SEMANTIC SEARCH ----------
    let db = VectorDB::new("meow_vectors.db")?;

    println!("🧠 Generating query embedding...");
    let query_vec = embed_text(&cleaned_query)?;

    println!("📦 Loading file embeddings...");
    let vectors = db.load_all()?; // (path, vec)
    println!("📦 Loaded {} vectors from DB", vectors.len());


    let mut scored: Vec<(String, f32)> = vectors
        .into_iter()
        .map(|(path, vec)| {
            let mut score = cosine_similarity(&query_vec, &vec);
            if score.is_nan() {
                score = 0.0;
            }
            (path, score)
        })
        .collect();

    // ---------- REAL SORT ----------
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));

    println!("\n😼 Top semantic matches:");
    for (path, score) in scored.iter().take(10) {
        println!("{:.4} → {}", score, path);
    }

    Ok(())
}

fn clean_query(q: &str) -> String {
    let stopwords = [
        "find", "the", "file", "that", "i", "downloaded", "download",
        "yesterday", "today", "please", "can", "you", "my", "a", "an",
        "is", "was", "me"
    ];

    q.split_whitespace()
        .filter(|w| !stopwords.contains(&w.to_lowercase().as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                out.push(path);
            } else if path.is_dir() {
                walk(&path, out);
            }
        }
    }
}

fn resolve_folder_hint(hint: &Option<String>, query: &Option<String>) -> PathBuf {
    let home = dirs::home_dir().unwrap_or(PathBuf::from("."));

    if let Some(h) = hint {
        return match h.to_lowercase().as_str() {
            "downloads" => home.join("Downloads"),
            "pictures" => home.join("Pictures"),
            _ => PathBuf::from("."),
        };
    }

    if let Some(q) = query {
        let q = q.to_lowercase();
        if q.contains("download") {
            return home.join("Downloads");
        }
        if q.contains("photo") || q.contains("picture") || q.contains("image") {
            return home.join("Pictures");
        }
    }

    PathBuf::from(".")
}

fn file_matches_time(path: &PathBuf, filter: &str) -> bool {
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            let modified_dt = chrono::DateTime::<Local>::from(modified);
            let now = Local::now();

            match filter {
                "yesterday" => {
                    let y = now - Duration::days(1);
                    return modified_dt.date_naive() == y.date_naive();
                }
                "today" => {
                    return modified_dt.date_naive() == now.date_naive();
                }
                _ => return true,
            }
        }
    }
    false
}
