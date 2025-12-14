use crate::ai::AiAction;
use crate::embedding::embed_text;
use crate::similarity::cosine_similarity;
use crate::types::SearchResults;
use crate::vector_db::VectorDB;

use chrono::{Duration, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_action(action: AiAction) -> anyhow::Result<Option<SearchResults>> {
    match action.intent.as_str() {
        "search" => {
            let res = execute_search(action)?;
            Ok(Some(res))
        }
        "open" => {
            println!("(open not implemented yet)");
            Ok(None)
        }
        "read" => {
            println!("(read not implemented yet)");
            Ok(None)
        }
        "summarize" => {
            println!("(summarize not implemented yet)");
            Ok(None)
        }
        "delete" => {
            println!("(delete not implemented yet)");
            Ok(None)
        }
        _ => {
            println!("❌ Unknown intent: {}", action.intent);
            Ok(None)
        }
    }
}

fn execute_search(action: AiAction) -> anyhow::Result<SearchResults> {
    println!("🔍 Executing AI search...");

    // guard
    let raw_query = match &action.query {
        Some(q) if !q.trim().is_empty() => q.as_str(),
        _ => {
            println!("❌ Cannot search without a query.");
            return Ok(SearchResults { items: vec![] });
        }
    };

    // (optional) cleaned query for embeddings
    let cleaned = clean_query(raw_query);
    let final_query = if cleaned.trim().is_empty() {
        raw_query.to_string()
    } else {
        cleaned
    };

    // root selection (you currently infer Pictures/Downloads etc.)
    let root = resolve_folder_hint(&action.folder_hint, &action.query);
    println!("📁 Searching in: {}", root.display());

    // Walk filesystem (used for time filter; semantic matches will come from DB)
    let mut files = Vec::new();
    walk(&root, &mut files);

    // time filter (optional)
    if let Some(time) = &action.time_filter {
        files.retain(|p| file_matches_time(p, time));
    }

    // --- semantic ---
    let db = VectorDB::new("meow_vectors.db")?;

    println!("🧠 Generating query embedding...");
    let query_vec = embed_text(&final_query)?;

    println!("📦 Loading file embeddings...");
    let mut vectors = db.load_all()?;
    println!("📦 Loaded {} vectors from DB", vectors.len());

    // IMPORTANT: if you want semantic to respect the chosen root/time-filter,
    // filter DB vectors to only those file paths in `files`.
    // (Otherwise it will rank ALL indexed files.)
    if !files.is_empty() {
        use std::collections::HashSet;
        let allowed: HashSet<String> = files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        vectors.retain(|(path, _)| allowed.contains(path));
    }

    // score
    let mut scored: Vec<(String, f32)> = vectors
        .into_iter()
        .map(|(path, vec)| {
            let semantic = cosine_similarity(&query_vec, &vec);
            (path, if semantic.is_nan() { 0.0 } else { semantic })
        })
        .collect();

    scored.sort_by(|a, b| b.1.total_cmp(&a.1));

    // top 10
    let top = scored.into_iter().take(10).collect::<Vec<_>>();

    println!("\n😼 Top matches:");
    let mut items = Vec::new();
    for (i, (path, score)) in top.iter().enumerate() {
        println!("[{}] {:.4} → {}", i + 1, score, path);
        items.push(path.clone());
    }

    Ok(SearchResults { items })
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

    // simple inference
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
