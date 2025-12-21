use crate::ai::AiAction;
use crate::embedding::embed_text;
use crate::similarity::cosine_similarity;
use crate::types::SearchResults;
use crate::vector_db::VectorDB;
use crate::ai_decider::{decide_best, Candidate};


use chrono::{Duration, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_action(action: AiAction) -> anyhow::Result<Option<SearchResults>> {
    match action.intent.as_str() {
        "search" | "find" => {
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
            println!("Unknown intent: {}", action.intent);
            Ok(None)
        }
    }
}

fn execute_search(action: AiAction) -> anyhow::Result<SearchResults> {
    println!("Executing AI search...");

    // ---- Guard: query must exist ----
    let raw_query = match &action.query {
        Some(q) if !q.trim().is_empty() => q.as_str(),
        _ => {
            println!("Cannot search without a query.");
            return Ok(SearchResults { items: vec![] });
        }
    };

    // Cleaned query for embeddings
    let cleaned = clean_query(raw_query);
    let final_query = if cleaned.trim().is_empty() {
        raw_query.to_string()
    } else {
        cleaned
    };

    // ---- Folder resolution ----
    let root = resolve_folder_hint(&action.folder_hint, &action.query);
    println!("Searching in: {}", root.display());

    // Walk filesystem (for filtering only)
    let mut files = Vec::new();
    walk(&root, &mut files);

    if let Some(time) = &action.time_filter {
        files.retain(|p| file_matches_time(p, time));
    }

    // ---- Load embeddings ----
    let db = VectorDB::new("meow_vectors.db")?;

    println!("Generating query embedding...");
    let query_vec = embed_text(&final_query)?;

    println!("Loading file embeddings...");
    let mut vectors = db.load_all()?;
    println!("Loaded {} vectors from DB", vectors.len());

    // Restrict vectors to files under selected root
    if !files.is_empty() {
        use std::collections::HashSet;
        let allowed: HashSet<String> = files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        vectors.retain(|(path, _)| allowed.contains(path));
    }

    // ---- Score (semantic only; hybrid already handled elsewhere if needed) ----
    let mut scored: Vec<(String, f32)> = vectors
        .into_iter()
        .map(|(path, vec)| {
            let s = cosine_similarity(&query_vec, &vec);
            (path, if s.is_nan() { 0.0 } else { s })
        })
        .collect();

    if scored.is_empty() {
        println!("No matches found.");
        return Ok(SearchResults { items: vec![] });
    }

    scored.sort_by(|a, b| b.1.total_cmp(&a.1));

    // ---- Build candidates (max 10) ----
    let top = scored.into_iter().take(10).collect::<Vec<_>>();

    let candidates: Vec<Candidate> = top
        .iter()
        .enumerate()
        .map(|(i, (path, score))| {
            let p = std::path::Path::new(path);

            Candidate {
                idx: i + 1,
                path: path.clone(),
                file_name: p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string(),
                ext: p.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase(),
                folder: p.parent()
                    .and_then(|pp| pp.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
                score: *score,
            }
        })
        .collect();

    // ---- Decide if second AI should be used ----
    let mut sure_pick: Option<usize> = None;

    if candidates.len() >= 2 {
        let best = candidates[0].score;
        let second = candidates[1].score;

        let ambiguous = best < 0.75 && (best - second) < 0.08;

        if ambiguous {
            sure_pick = decide_best(raw_query, &candidates).ok().flatten();
        }
    }

    // ---- Output & result ordering ----
    let mut items = Vec::new();

    if let Some(idx) = sure_pick {
        let picked = &candidates[idx - 1];
        println!("\n🎯 Best match (AI confirmed):");
        println!("★ {:.4} → {}", picked.score, picked.path);

        // Ensure sure-shot is first (open 1)
        items.push(picked.path.clone());
    }

    println!("\n😼 Results:");
    for c in &candidates {
        // Skip duplicate if already added as sure-shot
        if sure_pick == Some(c.idx) {
            continue;
        }

        println!("[{}] {:.4} → {}", c.idx, c.score, c.path);
        items.push(c.path.clone());
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
