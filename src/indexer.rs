use crate::embedding::embed_text;
use crate::vector_db::VectorDB;
use chrono::Local;
use std::fs;
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
    let file_name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace(['_', '-', '.'], " ");

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let mut text = String::new();

    // Base semantic description
    text.push_str(&format!(
        "This is a {} file named {} located in {} folder. ",
        ext, file_name, folder
    ));

    // Add semantic hints by file type
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            text.push_str("This file is an image or photo. ");
            if file_name.contains("logo") {
                text.push_str("It is likely a logo image. ");
            }
            if file_name.contains("screenshot") {
                text.push_str("It may be a screenshot. ");
            }
            if file_name.contains("icon") {
                text.push_str("It may be an icon or logo. ");
            }
        }

        "pdf" => {
            text.push_str("This file is a PDF document. ");
        }

        "txt" | "md" => {
            text.push_str("This file is a text document. ");
        }

        "exe" | "msi" => {
            text.push_str("This file is an installer or application. ");
        }

        _ => {}
    }

    Ok(text)
}

