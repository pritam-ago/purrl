use crate::ai::AiAction;
use crate::embedding::embed_text;
use crate::similarity::cosine_similarity;
use crate::types::SearchResults;
use crate::vector_db::VectorDB;
use crate::ai_decider::{decide_best, Candidate};
use crate::shell::open_path;


use chrono::{Duration, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_action(action: AiAction, db_path: &Path) -> anyhow::Result<Option<SearchResults>> {
    match action.intent.as_str() {
        "search" | "find" => {
            let res = execute_search(action, db_path)?;
            Ok(Some(res))
        }
        "open" => {
            // The natural end of a search: run the same search, then open the
            // top result. execute_search already puts the AI-confirmed pick
            // first when there is one.
            let res = execute_search(action, db_path)?;

            if let Some(best) = res.items.first() {
                open_path(best)?;
                println!("\nOpened: {best}");
            }

            Ok(Some(res))
        }
        "delete" => {
            // The intent is gone from the prompt, but llama3 is unconstrained
            // and still emits it now and then. Answer it explicitly rather
            // than letting a destructive request look like a parse failure.
            println!("purrl finds files. It never deletes them.");
            Ok(None)
        }
        other => {
            println!(
                "purrl can't do '{other}'. Try `ai find <something>`, then `open <n>`."
            );
            Ok(None)
        }
    }
}

fn execute_search(action: AiAction, db_path: &Path) -> anyhow::Result<SearchResults> {
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
    let db = VectorDB::new(db_path)?;

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

    // ---- Score (semantic only) ----
    let scored = score_and_rank(&query_vec, vectors);

    if scored.is_empty() {
        println!("No matches found.");
        return Ok(SearchResults { items: vec![] });
    }

    // ---- Build candidates (max 10) ----
    let top = scored.into_iter().take(10).collect::<Vec<_>>();

    let candidates: Vec<Candidate> = top
        .iter()
        .enumerate()
        .map(|(i, (path, score))| make_candidate(i + 1, path, *score))
        .collect();

    // ---- Decide if second AI should be used ----
    let mut sure_pick: Option<usize> = None;

    if candidates.len() >= 2 {
        let best = candidates[0].score;
        let second = candidates[1].score;

        if is_ambiguous(best, second) {
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


/// Cosine similarity against every stored vector, NaN coerced to zero, best
/// first. Lifted out of execute_search so it can be tested; the arithmetic is
/// unchanged.
fn score_and_rank(query_vec: &[f32], vectors: Vec<(String, Vec<f32>)>) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = vectors
        .into_iter()
        .map(|(path, vec)| {
            let s = cosine_similarity(query_vec, &vec);
            (path, if s.is_nan() { 0.0 } else { s })
        })
        .collect();

    scored.sort_by(|a, b| b.1.total_cmp(&a.1));

    scored
}

/// Whether the top two hits are close enough to be worth a second opinion from
/// the decider model. Lifted out of execute_search; thresholds unchanged.
fn is_ambiguous(best: f32, second: f32) -> bool {
    best < 0.75 && (best - second) < 0.08
}

/// Split a scored path into the fields the decider prompt shows.
fn make_candidate(idx: usize, path: &str, score: f32) -> Candidate {
    let p = Path::new(path);

    Candidate {
        idx,
        path: path.to_string(),
        file_name: p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string(),
        ext: p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase(),
        folder: p
            .parent()
            .and_then(|pp| pp.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        score,
    }
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

// These cover the pure ranking logic only. Nothing here talks to Ollama:
// embed_text, interpret_command and decide_best are never called, which is why
// cargo test passes with no server running.
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn v(path: &str, vec: &[f32]) -> (String, Vec<f32>) {
        (path.to_string(), vec.to_vec())
    }

    // ---- score_and_rank ----

    #[test]
    fn ranks_best_match_first() {
        let query = [1.0, 0.0];
        let vectors = vec![
            v("orthogonal", &[0.0, 1.0]),
            v("exact", &[1.0, 0.0]),
            v("diagonal", &[1.0, 1.0]),
        ];

        let ranked = score_and_rank(&query, vectors);

        let names: Vec<&str> = ranked.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["exact", "diagonal", "orthogonal"]);

        assert!((ranked[0].1 - 1.0).abs() < EPS);
        assert!((ranked[1].1 - 0.707_106_8).abs() < 1e-4);
        assert!(ranked[2].1.abs() < EPS);
    }

    #[test]
    fn scores_are_descending() {
        let query = [1.0, 1.0, 1.0];
        let vectors = vec![
            v("a", &[1.0, 0.0, 0.0]),
            v("b", &[1.0, 1.0, 1.0]),
            v("c", &[0.0, 0.0, 1.0]),
            v("d", &[1.0, 1.0, 0.0]),
        ];

        let ranked = score_and_rank(&query, vectors);

        for pair in ranked.windows(2) {
            assert!(
                pair[0].1 >= pair[1].1,
                "not descending: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn nan_scores_become_zero() {
        let query = [1.0, 0.0];
        let vectors = vec![v("poisoned", &[f32::NAN, 1.0]), v("opposed", &[-1.0, 0.0])];

        let ranked = score_and_rank(&query, vectors);

        // The NaN is coerced to 0.0, which also means it outranks a genuinely
        // opposed vector scoring -1.0. Documented, not endorsed.
        assert_eq!(ranked[0].0, "poisoned");
        assert!(ranked[0].1.abs() < EPS);
        assert!(!ranked[0].1.is_nan());
        assert!((ranked[1].1 + 1.0).abs() < EPS);
    }

    #[test]
    fn zero_vectors_score_zero_rather_than_erroring() {
        let query = [0.0, 0.0];
        let vectors = vec![v("anything", &[1.0, 2.0])];

        let ranked = score_and_rank(&query, vectors);

        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].1.abs() < EPS);
    }

    #[test]
    fn empty_input_ranks_to_nothing() {
        assert!(score_and_rank(&[1.0, 0.0], vec![]).is_empty());
    }

    #[test]
    fn every_vector_is_scored_and_kept() {
        // score_and_rank does not truncate; execute_search takes the top 10.
        let query = [1.0, 0.0];
        let vectors: Vec<(String, Vec<f32>)> = (0..25)
            .map(|i| v(&format!("f{i}"), &[1.0, i as f32]))
            .collect();

        assert_eq!(score_and_rank(&query, vectors).len(), 25);
    }

    #[test]
    fn equal_scores_keep_their_original_order() {
        let query = [1.0, 0.0];
        let vectors = vec![
            v("first", &[1.0, 0.0]),
            v("second", &[2.0, 0.0]),
            v("third", &[3.0, 0.0]),
        ];

        // All three are collinear with the query, so all score 1.0 and the
        // stable sort must leave them alone.
        let ranked = score_and_rank(&query, vectors);
        let names: Vec<&str> = ranked.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    // ---- is_ambiguous: the gate on the second AI call ----

    #[test]
    fn close_and_uncertain_results_are_ambiguous() {
        assert!(is_ambiguous(0.70, 0.65));
        assert!(is_ambiguous(0.40, 0.40));
    }

    #[test]
    fn a_confident_top_score_is_never_ambiguous() {
        // Above the 0.75 confidence floor, closeness stops mattering.
        assert!(!is_ambiguous(0.90, 0.899));
        assert!(!is_ambiguous(0.75, 0.75));
    }

    #[test]
    fn a_clear_winner_is_never_ambiguous() {
        // Below the confidence floor, but with a comfortable gap.
        assert!(!is_ambiguous(0.70, 0.60));
        assert!(!is_ambiguous(0.70, 0.10));
    }

    #[test]
    fn ambiguity_boundaries_are_exclusive() {
        // best < 0.75, strictly.
        assert!(is_ambiguous(0.749, 0.749));
        assert!(!is_ambiguous(0.750, 0.750));

        // gap < 0.08, strictly -- but in f32 the boundary does not sit where
        // decimal arithmetic suggests. 0.70 - 0.62 computes to 0.079999976,
        // just under the threshold, so that pair is ambiguous despite looking
        // like an exact 0.08 gap. Pinned because it is exactly the sort of
        // edge a later refactor would move without noticing.
        assert!((0.70_f32 - 0.62_f32) < 0.08);
        assert!(is_ambiguous(0.70, 0.62));

        // Clear of the threshold in either direction.
        assert!(!is_ambiguous(0.70, 0.61));
        assert!(is_ambiguous(0.70, 0.63));
    }

    // ---- make_candidate ----

    #[test]
    fn candidate_splits_a_path_into_name_ext_and_folder() {
        let c = make_candidate(3, "/home/ada/Downloads/hostel_fees.pdf", 0.42);

        assert_eq!(c.idx, 3);
        assert_eq!(c.path, "/home/ada/Downloads/hostel_fees.pdf");
        assert_eq!(c.file_name, "hostel_fees.pdf");
        assert_eq!(c.ext, "pdf");
        assert_eq!(c.folder, "Downloads");
        assert!((c.score - 0.42).abs() < EPS);
    }

    #[test]
    fn candidate_lowercases_the_extension() {
        let c = make_candidate(1, "/home/ada/Pictures/LOGO.PNG", 1.0);
        assert_eq!(c.ext, "png");
        assert_eq!(c.file_name, "LOGO.PNG");
    }

    #[test]
    fn candidate_handles_a_missing_extension() {
        let c = make_candidate(1, "/home/ada/Downloads/Makefile", 1.0);
        assert_eq!(c.ext, "");
        assert_eq!(c.file_name, "Makefile");
    }

    #[test]
    fn candidate_takes_the_last_extension_only() {
        let c = make_candidate(1, "/home/ada/archive.tar.gz", 1.0);
        assert_eq!(c.ext, "gz");
        assert_eq!(c.file_name, "archive.tar.gz");
    }

    #[test]
    fn candidate_with_no_parent_folder_reports_an_empty_folder() {
        let c = make_candidate(1, "loose_file.txt", 1.0);
        assert_eq!(c.file_name, "loose_file.txt");
        assert_eq!(c.folder, "");
    }

    // ---- clean_query ----

    #[test]
    fn clean_query_drops_stopwords() {
        assert_eq!(clean_query("find the file that i downloaded"), "");
        assert_eq!(clean_query("find my ubuntu iso"), "ubuntu iso");
    }

    #[test]
    fn clean_query_is_case_insensitive() {
        assert_eq!(clean_query("Find The Logo"), "Logo");
    }

    #[test]
    fn clean_query_can_strip_everything() {
        // Which is why execute_search falls back to the raw query when the
        // cleaned one comes back empty.
        assert!(
            clean_query("can you find me the file please")
                .trim()
                .is_empty()
        );
    }

    #[test]
    fn clean_query_keeps_unknown_words_untouched() {
        assert_eq!(
            clean_query("qrcode screenshot 2025"),
            "qrcode screenshot 2025"
        );
    }

    // ---- resolve_folder_hint ----

    #[test]
    fn explicit_hints_win() {
        let downloads = resolve_folder_hint(&Some("downloads".into()), &None);
        assert!(downloads.ends_with("Downloads"));

        let pictures = resolve_folder_hint(&Some("Pictures".into()), &None);
        assert!(pictures.ends_with("Pictures"));
    }

    #[test]
    fn an_unrecognised_hint_falls_back_to_the_current_directory() {
        let p = resolve_folder_hint(&Some("documents".into()), &None);
        assert_eq!(p, Path::new("."));
    }

    #[test]
    fn a_hint_beats_the_query_text() {
        let p = resolve_folder_hint(&Some("downloads".into()), &Some("my photos".into()));
        assert!(p.ends_with("Downloads"));
    }

    #[test]
    fn the_folder_is_inferred_from_the_query_when_no_hint_is_given() {
        assert!(
            resolve_folder_hint(&None, &Some("the iso I downloaded".into()))
                .ends_with("Downloads")
        );
        assert!(
            resolve_folder_hint(&None, &Some("that photo of the cat".into()))
                .ends_with("Pictures")
        );
        assert!(
            resolve_folder_hint(&None, &Some("my profile picture".into()))
                .ends_with("Pictures")
        );
        assert!(resolve_folder_hint(&None, &Some("a nice image".into())).ends_with("Pictures"));
    }

    #[test]
    fn with_neither_hint_nor_clue_the_search_starts_here() {
        assert_eq!(resolve_folder_hint(&None, &None), Path::new("."));
        assert_eq!(
            resolve_folder_hint(&None, &Some("hostel fees".into())),
            Path::new(".")
        );
    }
}
