use crate::embedding::embed_text;
use crate::vector_db::VectorDB;
use chrono::Local;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn run_indexer() -> anyhow::Result<()> {
    println!("📚 Meow indexer started…");

    let mut roots = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let downloads = home.join("Downloads");
        let pictures = home.join("OneDrive/Pictures");

        if downloads.exists() {
            println!("➡ Indexing Downloads: {}", downloads.display());
            roots.push(downloads);
        }

        if pictures.exists() {
            println!("➡ Indexing Pictures: {}", pictures.display());
            roots.push(pictures);
        }
    }

    let db = VectorDB::new("meow_vectors.db")?;

    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }

    println!("📁 Found {} files.", files.len());

    let mut indexed = 0;
    for path in files {
        if let Err(e) = index_file(&db, &path) {
            eprintln!("⚠️ Skipped {}: {}", path.display(), e);
            continue;
        }
        indexed += 1;
    }

    println!("✅ Indexed {} files.", indexed);
    Ok(())
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                out.push(path);
            } else if path.is_dir() {
                collect_files(&path, out);
            }
        }
    }
}

fn index_file(db: &VectorDB, path: &Path) -> anyhow::Result<()> {
    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()
        .map(|m| chrono::DateTime::<Local>::from(m).timestamp())
        .unwrap_or(0);

    let text = build_representation(path)?;
    let vec = embed_text(&text)?;

    db.store_embedding(&path.to_string_lossy(), &vec, modified)?;
    Ok(())
}

fn build_representation(path: &Path) -> anyhow::Result<String> {
    let mut repr = format!(
        "File name: {}. Full path: {}. ",
        path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        path.display()
    );

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let is_text = matches!(
        ext,
        "txt" | "md" | "json" | "log" | "rs" | "js" | "ts"
    );

    if is_text {
        if let Ok(mut f) = fs::File::open(path) {
            let mut buf = String::new();
            f.by_ref().take(8 * 1024).read_to_string(&mut buf).ok();
            repr.push_str(&buf);
        }
    }

    Ok(repr)
}
