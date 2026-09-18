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
