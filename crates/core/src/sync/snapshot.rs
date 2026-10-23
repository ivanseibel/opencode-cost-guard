//! Atomic current/previous Catalog snapshot files.
//!
//! Exactly two files are retained: `catalog.current.json` and
//! `catalog.previous.json` (ADR-0003). A file that fails to parse, including a
//! truncated write, is treated as absent (SYNC-08).

use std::path::PathBuf;

use crate::catalog::{CatalogSnapshot, parse_snapshot};

/// File name of the active snapshot.
pub const CURRENT_FILE: &str = "catalog.current.json";
/// File name of the snapshot replaced by the most recent save.
pub const PREVIOUS_FILE: &str = "catalog.previous.json";

/// Snapshot persistence rooted at a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotStore {
    dir: PathBuf,
}

impl SnapshotStore {
    /// Create a store rooted at `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Load the current snapshot; an unreadable or invalid file is absent.
    pub fn load_current(&self) -> Option<CatalogSnapshot> {
        self.load(CURRENT_FILE)
    }

    /// Load the previous snapshot; an unreadable or invalid file is absent.
    pub fn load_previous(&self) -> Option<CatalogSnapshot> {
        self.load(PREVIOUS_FILE)
    }

    /// Persist `snapshot` as current, rotating a valid current to previous.
    pub fn save(&self, snapshot: &CatalogSnapshot) -> Result<(), SnapshotError> {
        if let Some(current) = self.load_current() {
            self.write(PREVIOUS_FILE, &current)?;
        }
        self.write(CURRENT_FILE, snapshot)?;
        Ok(())
    }

    fn load(&self, name: &str) -> Option<CatalogSnapshot> {
        let text = std::fs::read_to_string(self.dir.join(name)).ok()?;
        parse_snapshot(&text).ok()
    }

    fn write(&self, name: &str, snapshot: &CatalogSnapshot) -> Result<(), SnapshotError> {
        let path = self.dir.join(name);
        let temp = self.dir.join(format!("{name}.tmp"));
        let json = serde_json::to_string_pretty(snapshot)?;

        std::fs::write(&temp, json).map_err(|source| SnapshotError::Io {
            path: temp.clone(),
            source,
        })?;
        std::fs::rename(&temp, &path).map_err(|source| SnapshotError::Io { path, source })?;

        Ok(())
    }
}

/// Snapshot persistence errors.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("failed to write snapshot {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize snapshot: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelEntry, Price};
    use std::path::Path;

    fn snapshot(etag: &str) -> CatalogSnapshot {
        CatalogSnapshot {
            schema: 1,
            source: "https://models.opencode.ai/api.json".to_string(),
            etag: Some(etag.to_string()),
            fetched_at: "2026-10-23T22:38:03Z".to_string(),
            models: vec![ModelEntry {
                id: "deepseek-v4.1-flash".to_string(),
                provider_id: "opencode-go".to_string(),
                name: "DeepSeek V4.1 Flash".to_string(),
                price: Price {
                    input: 0.15,
                    output: 0.6,
                },
                cache_read: 0.003,
                context: 1_048_576,
                tool_call: true,
                attachment: true,
                reasoning: true,
                release_date: Some("2026-10-17".to_string()),
            }],
        }
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("read tempdir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn save_rotates_current_to_previous_and_retains_two_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SnapshotStore::new(dir.path());

        let first = snapshot("\"first\"");
        store.save(&first).expect("save first");

        assert_eq!(store.load_current(), Some(first.clone()));
        assert_eq!(store.load_previous(), None);

        let second = snapshot("\"second\"");
        store.save(&second).expect("save second");

        assert_eq!(store.load_current(), Some(second));
        assert_eq!(store.load_previous(), Some(first));
        assert_eq!(
            file_names(dir.path()),
            vec![CURRENT_FILE.to_string(), PREVIOUS_FILE.to_string()]
        );
    }

    #[test]
    fn truncated_or_invalid_files_are_treated_as_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SnapshotStore::new(dir.path());

        std::fs::write(dir.path().join(CURRENT_FILE), "{\"schema\": 1, \"sour")
            .expect("write truncated current");
        std::fs::write(dir.path().join(PREVIOUS_FILE), "not json").expect("write invalid previous");

        assert_eq!(store.load_current(), None);
        assert_eq!(store.load_previous(), None);
    }

    #[test]
    fn invalid_current_is_not_rotated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SnapshotStore::new(dir.path());

        std::fs::write(dir.path().join(CURRENT_FILE), "truncated").expect("write invalid current");

        let saved = snapshot("\"saved\"");
        store.save(&saved).expect("save over invalid current");

        assert_eq!(store.load_current(), Some(saved));
        assert_eq!(store.load_previous(), None);
    }

    #[test]
    fn atomic_write_leaves_no_temp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SnapshotStore::new(dir.path());

        store.save(&snapshot("\"first\"")).expect("save first");
        assert!(!dir.path().join(format!("{CURRENT_FILE}.tmp")).exists());

        std::fs::write(
            dir.path().join(format!("{CURRENT_FILE}.tmp")),
            "stale interrupted write",
        )
        .expect("write stale temp");

        store.save(&snapshot("\"second\"")).expect("save second");
        assert!(!dir.path().join(format!("{CURRENT_FILE}.tmp")).exists());
        assert!(!dir.path().join(format!("{PREVIOUS_FILE}.tmp")).exists());
    }
}
