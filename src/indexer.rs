use crate::embedding::embed_text;
use crate::vector_db::VectorDB;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_indexer(db_path: &Path) -> anyhow::Result<()> {
    println!("purrl indexer started…");

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

    let db = VectorDB::new(db_path)?;

    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }

    println!("Found {} files.", files.len());

    let total = files.len() as u64;

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} Indexing: [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
        )
        .unwrap()
        .progress_chars("█░"),
    );

    let mut indexed = 0usize;

    for path in files {
        if let Err(e) = index_file(&db, &path) {
            eprintln!("\nSkipped {}: {}", path.display(), e);
            pb.inc(1);
            continue;
        }

        indexed += 1;
        pb.inc(1);
    }

    pb.finish_with_message("Indexing complete");

    println!("Indexed {} files.", indexed);
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
            if file_name.contains("wallpaper") {
                text.push_str("It may be a wallpaper or background image. ");
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

// build_representation only parses paths -- it never opens the file -- so
// these tests need neither Ollama nor a filesystem.
#[cfg(test)]
mod tests {
    use super::*;

    /// The exact path shape found in the index that was committed to this
    /// repo: a Windows home directory joined with the literal "OneDrive/Pictures",
    /// which leaves one forward slash in the middle of a backslash path.
    const MIXED: &str = r"C:\Users\ada\OneDrive/Pictures\logo_icon.png";

    #[test]
    fn a_plain_path_becomes_a_sentence() {
        let text = build_representation(Path::new("/home/ada/Downloads/hostel_fees.pdf")).unwrap();

        assert!(
            text.starts_with("This is a pdf file named hostel fees located in Downloads folder."),
            "{text}"
        );
        assert!(text.contains("This file is a PDF document."), "{text}");
    }

    #[test]
    fn underscores_hyphens_and_dots_become_spaces() {
        let text = build_representation(Path::new(
            "/home/ada/Downloads/ubuntu-24.04.desktop_amd64.iso",
        ))
        .unwrap();

        assert!(text.contains("named ubuntu 24 04 desktop amd64 "), "{text}");
    }

    #[test]
    fn the_extension_is_lowercased() {
        let text = build_representation(Path::new("/home/ada/Pictures/LOGO.PNG")).unwrap();

        assert!(text.starts_with("This is a png file"), "{text}");
        assert!(text.contains("This file is an image or photo."), "{text}");
    }

    #[test]
    fn the_filename_hints_are_case_sensitive() {
        // LOGO.PNG is an image, but "logo" never matches, so the logo hint is
        // silently missed. Documented, not endorsed -- fixing it would change
        // what gets embedded.
        let shouty = build_representation(Path::new("/home/ada/Pictures/LOGO.PNG")).unwrap();
        assert!(!shouty.contains("likely a logo image"), "{shouty}");

        let quiet = build_representation(Path::new("/home/ada/Pictures/logo.png")).unwrap();
        assert!(quiet.contains("likely a logo image"), "{quiet}");
    }

    #[test]
    fn image_hints_fire_on_the_cleaned_name() {
        let shot = build_representation(Path::new("/home/ada/Pictures/screenshot_2025-01-02.png"))
            .unwrap();
        assert!(shot.contains("may be a screenshot"), "{shot}");

        let paper =
            build_representation(Path::new("/home/ada/Pictures/wallpaper-blue.jpg")).unwrap();
        assert!(paper.contains("wallpaper or background image"), "{paper}");

        let icon = build_representation(Path::new("/home/ada/Pictures/app_icon.webp")).unwrap();
        assert!(icon.contains("may be an icon or logo"), "{icon}");
    }

    #[test]
    fn text_and_installer_extensions_get_their_own_sentence() {
        let md = build_representation(Path::new("/home/ada/notes.md")).unwrap();
        assert!(md.contains("This file is a text document."), "{md}");

        let exe = build_representation(Path::new("/home/ada/Downloads/setup.exe")).unwrap();
        assert!(
            exe.contains("This file is an installer or application."),
            "{exe}"
        );
    }

    #[test]
    fn an_unrecognised_extension_gets_no_hint() {
        let text = build_representation(Path::new("/home/ada/Downloads/disk.iso")).unwrap();

        assert_eq!(
            text, "This is a iso file named disk located in Downloads folder. ",
            "unknown extensions should stop after the base sentence"
        );
    }

    #[test]
    fn a_file_with_no_extension_leaves_a_gap() {
        let text = build_representation(Path::new("/home/ada/Downloads/Makefile")).unwrap();

        // The template interpolates an empty extension, so there is a double
        // space. Pinned because it is what actually gets embedded.
        assert_eq!(
            text,
            "This is a  file named Makefile located in Downloads folder. "
        );
    }

    #[test]
    fn a_file_with_no_parent_folder_reports_an_empty_folder() {
        let text = build_representation(Path::new("loose.txt")).unwrap();

        assert!(
            text.starts_with("This is a txt file named loose located in  folder."),
            "{text}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn mixed_separators_parse_as_intended_on_windows() {
        let text = build_representation(Path::new(MIXED)).unwrap();

        // Windows accepts both separators, so this parses the way the indexer
        // assumed it would.
        assert!(text.contains("named logo icon "), "{text}");
        assert!(text.contains("located in Pictures folder"), "{text}");
        assert!(text.starts_with("This is a png file"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn mixed_separators_are_mangled_on_unix() {
        let text = build_representation(Path::new(MIXED)).unwrap();

        // A backslash is an ordinary filename character here, so the only
        // separator is the single forward slash: the "folder" swallows the
        // whole Windows prefix and the "name" keeps the Pictures component.
        // This is what an index built on Windows and read on Linux would mean.
        assert!(text.contains(r"named Pictures\logo icon "), "{text}");
        assert!(
            text.contains(r"located in C:\Users\ada\OneDrive folder"),
            "{text}"
        );
        assert!(text.starts_with("This is a png file"), "{text}");
    }

    #[test]
    fn the_extension_is_found_either_way() {
        // Whatever the separators do to the name, the extension survives, so
        // the file-type hints keep working across platforms.
        let text = build_representation(Path::new(MIXED)).unwrap();

        assert!(text.starts_with("This is a png file"), "{text}");
        assert!(text.contains("This file is an image or photo."), "{text}");
        assert!(text.contains("may be an icon or logo"), "{text}");
    }
}
