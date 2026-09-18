use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Name of the index file inside the data directory.
const DB_FILE_NAME: &str = "vectors.db";

/// The pre-purrl index, which used to be written into the working directory.
const LEGACY_DB_FILE_NAME: &str = "meow_vectors.db";

/// Where the index lives by default: `<data_dir>/purrl/vectors.db`.
///
/// Creates the `purrl` directory if it is missing. Deliberately never the
/// working directory: an index of your personal files does not belong in
/// whatever repository you happened to be standing in when you ran `index`.
pub fn default_db_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| {
            anyhow::anyhow!("could not determine a data directory for this platform; pass --db <PATH>")
        })?
        .join("purrl");

    std::fs::create_dir_all(&dir)?;

    Ok(dir.join(DB_FILE_NAME))
}

/// Resolve the index path: an explicit `--db` wins, otherwise the default.
pub fn resolve_db_path(flag: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match flag {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            Ok(path)
        }
        None => default_db_path(),
    }
}

/// Say once, on startup, that a leftover `meow_vectors.db` in the working
/// directory is no longer read. Does not migrate it and does not delete it --
/// that is the user's call, on their own data.
pub fn legacy_db_notice(db_path: &Path) {
    if Path::new(LEGACY_DB_FILE_NAME).exists() {
        println!(
            "note: the index now lives at {} — {} in this directory is unused and can be deleted.",
            db_path.display(),
            LEGACY_DB_FILE_NAME
        );
    }
}

pub struct VectorDB {
    conn: Connection,
}

impl VectorDB {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS embeddings (
                path TEXT PRIMARY KEY,
                vector BLOB,
                modified INTEGER
            );",
        )?;

        Ok(Self { conn })
    }

    pub fn store_embedding(
        &self,
        file_path: &str,
        vector: &[f32],
        modified: i64,
    ) -> anyhow::Result<()> {
        let blob = bincode::serialize(vector)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings (path, vector, modified)
             VALUES (?1, ?2, ?3)",
            params![file_path, blob, modified],
        )?;
        Ok(())
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare("SELECT path, vector FROM embeddings")?;

        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let vector: Vec<f32> = bincode::deserialize(&blob).unwrap();
            Ok((path, vector))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}

// Every test here runs against its own TempDir. None of them call
// default_db_path(), so the real data directory is never touched, and none of
// them need Ollama.
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_db() -> (TempDir, VectorDB) {
        let dir = TempDir::new().expect("temp dir");
        let db = VectorDB::new(&dir.path().join("vectors.db")).expect("open db");
        (dir, db)
    }

    #[test]
    fn a_vector_survives_the_round_trip_exactly() {
        let (_dir, db) = temp_db();

        let vector = vec![0.1, -0.2, 0.000_003, 1.0, -1.0, f32::MIN_POSITIVE];
        db.store_embedding("/home/ada/Downloads/report.pdf", &vector, 1_700_000_000)
            .unwrap();

        let loaded = db.load_all().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, "/home/ada/Downloads/report.pdf");
        // Exact equality, not approximate: bincode round-trips the f32 bits.
        assert_eq!(loaded[0].1, vector);
    }

    #[test]
    fn a_full_width_embedding_round_trips() {
        let (_dir, db) = temp_db();

        // 768 is what nomic-embed-text actually returns.
        let vector: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
        db.store_embedding("/home/ada/Pictures/cat.png", &vector, 0)
            .unwrap();

        let loaded = db.load_all().unwrap();
        assert_eq!(loaded[0].1.len(), 768);
        assert_eq!(loaded[0].1, vector);
    }

    #[test]
    fn reindexing_a_path_replaces_it_instead_of_duplicating_it() {
        let (_dir, db) = temp_db();

        db.store_embedding("/home/ada/Downloads/a.pdf", &[1.0, 2.0], 1)
            .unwrap();
        db.store_embedding("/home/ada/Downloads/a.pdf", &[9.0, 9.0], 2)
            .unwrap();

        let loaded = db.load_all().unwrap();

        assert_eq!(loaded.len(), 1, "path is the primary key");
        assert_eq!(loaded[0].1, vec![9.0, 9.0]);
    }

    #[test]
    fn several_files_are_all_returned() {
        let (_dir, db) = temp_db();

        for i in 0..5 {
            db.store_embedding(&format!("/home/ada/f{i}.txt"), &[i as f32], i as i64)
                .unwrap();
        }

        let loaded = db.load_all().unwrap();
        assert_eq!(loaded.len(), 5);

        let mut paths: Vec<String> = loaded.into_iter().map(|(p, _)| p).collect();
        paths.sort();
        assert_eq!(paths[0], "/home/ada/f0.txt");
        assert_eq!(paths[4], "/home/ada/f4.txt");
    }

    #[test]
    fn a_fresh_index_is_empty_rather_than_missing() {
        let (_dir, db) = temp_db();
        assert!(db.load_all().unwrap().is_empty());
    }

    #[test]
    fn an_empty_vector_round_trips() {
        let (_dir, db) = temp_db();

        db.store_embedding("/home/ada/weird.bin", &[], 0).unwrap();

        let loaded = db.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].1.is_empty());
    }

    #[test]
    fn windows_paths_survive_storage_verbatim() {
        let (_dir, db) = temp_db();

        // The exact shape found in the committed index: a path built by
        // joining "OneDrive/Pictures" onto a Windows home directory.
        let path = r"C:\Users\ada\OneDrive/Pictures\logo_icon.png";
        db.store_embedding(path, &[0.5, 0.5], 0).unwrap();

        let loaded = db.load_all().unwrap();
        assert_eq!(loaded[0].0, path, "separators must not be normalised");
    }

    #[test]
    fn reopening_the_same_file_keeps_the_data() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vectors.db");

        {
            let db = VectorDB::new(&path).unwrap();
            db.store_embedding("/home/ada/persisted.txt", &[1.0, 2.0, 3.0], 42)
                .unwrap();
        }

        let reopened = VectorDB::new(&path).unwrap();
        let loaded = reopened.load_all().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].1, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn opening_creates_the_file_where_it_was_asked_to() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("vectors.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let _db = VectorDB::new(&path).unwrap();

        assert!(path.exists(), "index should exist at {}", path.display());
    }

    #[test]
    fn an_explicit_db_flag_is_used_verbatim() {
        let dir = TempDir::new().unwrap();
        let wanted = dir.path().join("custom.db");

        let resolved = resolve_db_path(Some(wanted.clone())).unwrap();

        assert_eq!(resolved, wanted);
    }

    #[test]
    fn an_explicit_db_flag_gets_its_parent_directory_created() {
        let dir = TempDir::new().unwrap();
        let wanted = dir.path().join("does").join("not").join("exist").join("x.db");

        let resolved = resolve_db_path(Some(wanted.clone())).unwrap();

        assert_eq!(resolved, wanted);
        assert!(wanted.parent().unwrap().is_dir());
        // Resolving prepares the directory; it does not create the index.
        assert!(!wanted.exists());
    }

    #[test]
    fn a_bare_filename_flag_does_not_panic_on_an_empty_parent() {
        // Path::parent() of "x.db" is Some(""), which create_dir_all would
        // choke on. Guarded, so this must simply pass the name through.
        let resolved = resolve_db_path(Some(PathBuf::from("x.db"))).unwrap();
        assert_eq!(resolved, PathBuf::from("x.db"));
    }
}
