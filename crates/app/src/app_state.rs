//! App state: config, overlays and catalog snapshots with bundled fallback.
//!
//! Everything the tray, menu and timers read lives here. Startup never aborts
//! on bad user data: a broken config or snapshot falls back to defaults and
//! bundled data while recording a message for the menu (APP-03, SYNCA-03).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use cost_guard_core::catalog::{CatalogError, CatalogSnapshot, parse_snapshot};
use cost_guard_core::config::{Config, ConfigError, TaskProfile};
use cost_guard_core::overlay::{OverlayError, Overlays};
use cost_guard_core::sync::changes::detect_changes;
use cost_guard_core::sync::snapshot::SnapshotStore;
use cost_guard_core::sync::{ActiveSource, SyncOutcome, SyncStatus};

const BUNDLED_SNAPSHOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/bundled/opencode-go.json"
));

/// Startup failures that bundled data cannot recover from.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to load bundled overlays: {0}")]
    Overlays(#[from] OverlayError),
    #[error("failed to parse the bundled catalog snapshot: {0}")]
    BundledSnapshot(#[from] CatalogError),
}

/// Mutable state shared by the tray, menu, timers and settings commands.
#[derive(Debug, Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub config: Config,
    pub overlays: Overlays,
    pub snapshot: CatalogSnapshot,
    /// Snapshot shipped in the binary; the last resort for every load path.
    pub bundled: CatalogSnapshot,
    pub source: ActiveSource,
    /// Fetch time of the active snapshot; `None` while bundled data is active.
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Unacknowledged Catalog Change badge (TRAY-02).
    pub badge: bool,
    /// Non-fatal startup message, such as an invalid config file.
    pub error: Option<String>,
}

impl AppState {
    /// Load config, overlays and the newest usable snapshot.
    ///
    /// Missing config yields defaults. An invalid config yields defaults in
    /// memory plus [`AppState::error`], leaving the file untouched. A missing
    /// or invalid snapshot falls back to bundled data and reports the source.
    pub fn load(data_dir: impl Into<PathBuf>) -> Result<Self, AppError> {
        let data_dir = data_dir.into();
        let (config, error) = match Config::load(&data_dir) {
            Ok(config) => (config, None),
            Err(error) => (Config::default(), Some(error.to_string())),
        };
        let overlays = Overlays::bundled()?;
        let bundled = parse_snapshot(BUNDLED_SNAPSHOT)?;
        let store = SnapshotStore::new(data_dir.clone());

        let (snapshot, source, last_sync_at) = match store.load_current() {
            Some(snapshot) => {
                let last_sync_at = snapshot_time(&snapshot);
                (snapshot, ActiveSource::Remote, last_sync_at)
            }
            None => (bundled.clone(), ActiveSource::Bundled, None),
        };

        Ok(Self {
            data_dir,
            config,
            overlays,
            snapshot,
            bundled,
            source,
            last_sync_at,
            badge: false,
            error,
        })
    }

    /// Reload the active snapshot from disk, falling back to bundled data.
    pub fn refresh_snapshot(&mut self) -> ActiveSource {
        let store = SnapshotStore::new(self.data_dir.clone());
        match store.load_current() {
            Some(snapshot) => {
                self.last_sync_at = snapshot_time(&snapshot);
                self.snapshot = snapshot;
                self.source = ActiveSource::Remote;
            }
            None => {
                self.snapshot = self.bundled.clone();
                self.last_sync_at = None;
                self.source = ActiveSource::Bundled;
            }
        }
        self.source
    }

    /// Apply a finished sync: refresh the snapshot, source and change badge.
    ///
    /// Returns whether the badge turned on. A change never clears the badge;
    /// only [`AppState::acknowledge_changes`] does (SYNCA-02).
    pub fn apply_sync_outcome(&mut self, outcome: &SyncOutcome) -> bool {
        match &outcome.status {
            SyncStatus::Updated { source } => {
                let store = SnapshotStore::new(self.data_dir.clone());
                let Some(current) = store.load_current() else {
                    // The engine reported an update it could not persist.
                    // Keep the previous snapshot; the next sync retries.
                    self.source = *source;
                    return false;
                };

                let changes = detect_changes(&self.snapshot, &current);
                self.source = *source;
                self.last_sync_at = snapshot_time(&current);
                self.snapshot = current;

                let was_set = self.badge;
                if !changes.is_empty() {
                    self.badge = true;
                }
                !was_set && self.badge
            }
            SyncStatus::NotModified => {
                // Remote data is unchanged and still active; the snapshot's
                // fetch time remains the last successful transfer.
                self.source = ActiveSource::Remote;
                false
            }
            SyncStatus::Fallback { source, .. } => {
                let (snapshot, last_sync_at) = self.active_data(*source);
                self.snapshot = snapshot;
                self.last_sync_at = last_sync_at;
                self.source = *source;
                false
            }
        }
    }

    /// Select a Task Profile and persist it through core [`Config`].
    ///
    /// The in-memory config changes only after the file is written.
    pub fn set_profile(&mut self, profile: TaskProfile) -> Result<(), ConfigError> {
        let mut config = self.config.clone();
        config.default_profile = profile;
        config.save(&self.data_dir)?;
        self.config = config;
        Ok(())
    }

    /// Dismiss the Catalog Change badge; returns whether it was set.
    pub fn acknowledge_changes(&mut self) -> bool {
        let was_set = self.badge;
        self.badge = false;
        was_set
    }

    /// Snapshot and fetch time for one active source, using the best data on disk.
    fn active_data(&self, source: ActiveSource) -> (CatalogSnapshot, Option<DateTime<Utc>>) {
        if source == ActiveSource::Bundled {
            return (self.bundled.clone(), None);
        }

        let store = SnapshotStore::new(self.data_dir.clone());
        match store.load_current() {
            Some(snapshot) => {
                let last_sync_at = snapshot_time(&snapshot);
                (snapshot, last_sync_at)
            }
            None => (self.bundled.clone(), None),
        }
    }
}

/// Parse the RFC 3339 fetch time stored with a snapshot.
fn snapshot_time(snapshot: &CatalogSnapshot) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&snapshot.fetched_at)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn load(dir: &Path) -> AppState {
        AppState::load(dir.to_path_buf()).expect("app state loads")
    }

    fn updated() -> SyncOutcome {
        SyncOutcome {
            status: SyncStatus::Updated {
                source: ActiveSource::Remote,
            },
            warnings: Vec::new(),
        }
    }

    fn fallback(source: ActiveSource) -> SyncOutcome {
        SyncOutcome {
            status: SyncStatus::Fallback {
                source,
                reason: "fetch failed: offline".to_string(),
            },
            warnings: Vec::new(),
        }
    }

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid fixture timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn missing_files_load_defaults_and_the_bundled_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = load(dir.path());

        assert_eq!(state.config, Config::default());
        assert_eq!(state.source, ActiveSource::Bundled);
        assert_eq!(state.snapshot, state.bundled);
        assert!(!state.snapshot.models.is_empty());
        assert_eq!(state.last_sync_at, None);
        assert!(!state.badge);
        assert_eq!(state.error, None);
        assert!(!dir.path().join("config.json").exists());
    }

    #[test]
    fn stored_snapshot_is_active_and_reports_remote() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut stored = load(dir.path()).bundled;
        stored.etag = Some("\"stored\"".to_string());
        stored.fetched_at = "2026-10-24T08:30:00Z".to_string();
        SnapshotStore::new(dir.path())
            .save(&stored)
            .expect("save stored snapshot");

        let state = load(dir.path());

        assert_eq!(state.snapshot, stored);
        assert_eq!(state.source, ActiveSource::Remote);
        assert_eq!(state.last_sync_at, Some(instant("2026-10-24T08:30:00Z")));
        assert_eq!(state.error, None);
    }

    #[test]
    fn invalid_config_keeps_defaults_and_reports_the_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{"syncIntervalHours": 0}"#).expect("write invalid config");
        let before = fs::read(&path).expect("read fixture");

        let state = load(dir.path());

        assert_eq!(state.config, Config::default());
        let error = state.error.expect("error string");
        assert!(error.contains("syncIntervalHours"), "{error}");
        assert_eq!(fs::read(&path).expect("read fixture"), before);
    }

    #[test]
    fn set_profile_persists_through_core_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());

        state
            .set_profile(TaskProfile::LargeRefactor)
            .expect("persist profile");

        assert_eq!(state.config.default_profile, TaskProfile::LargeRefactor);
        let reloaded = Config::load(dir.path()).expect("reload config");
        assert_eq!(reloaded.default_profile, TaskProfile::LargeRefactor);
    }

    #[test]
    fn set_profile_keeps_memory_when_the_write_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        fs::remove_dir_all(dir.path()).expect("remove data dir");

        state
            .set_profile(TaskProfile::VisualUi)
            .expect_err("save into a missing directory fails");

        assert_eq!(state.config.default_profile, TaskProfile::SimpleCoding);
    }

    #[test]
    fn acknowledge_changes_clears_the_badge_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        state.badge = true;

        assert!(state.acknowledge_changes());
        assert!(!state.badge);
        assert!(!state.acknowledge_changes());
    }

    #[test]
    fn updated_outcome_switches_source_and_sets_the_badge_on_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        let mut changed = state.bundled.clone();
        changed.fetched_at = "2026-10-24T09:00:00Z".to_string();
        changed.models[0].price.input += 1.5;
        SnapshotStore::new(dir.path())
            .save(&changed)
            .expect("save changed snapshot");

        assert!(state.apply_sync_outcome(&updated()));

        assert!(state.badge);
        assert_eq!(state.source, ActiveSource::Remote);
        assert_eq!(state.snapshot, changed);
        assert_eq!(state.last_sync_at, Some(instant("2026-10-24T09:00:00Z")));
    }

    #[test]
    fn identical_update_leaves_the_badge_off() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        let unchanged = state.snapshot.clone();
        SnapshotStore::new(dir.path())
            .save(&unchanged)
            .expect("save identical snapshot");

        assert!(!state.apply_sync_outcome(&updated()));

        assert!(!state.badge);
        assert_eq!(state.snapshot, unchanged);
        assert_eq!(state.source, ActiveSource::Remote);
    }

    #[test]
    fn not_modified_keeps_the_snapshot_and_badge() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        state.source = ActiveSource::LastValid;
        state.badge = true;
        let snapshot = state.snapshot.clone();

        let outcome = SyncOutcome {
            status: SyncStatus::NotModified,
            warnings: Vec::new(),
        };
        assert!(!state.apply_sync_outcome(&outcome));

        assert_eq!(state.snapshot, snapshot);
        assert_eq!(state.source, ActiveSource::Remote);
        assert!(state.badge);
    }

    #[test]
    fn fallback_names_the_source_and_keeps_the_change_badge() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        state.badge = true;

        assert!(!state.apply_sync_outcome(&fallback(ActiveSource::Bundled)));

        assert_eq!(state.source, ActiveSource::Bundled);
        assert_eq!(state.snapshot, state.bundled);
        assert_eq!(state.last_sync_at, None);
        assert!(state.badge);

        let stored = state.bundled.clone();
        SnapshotStore::new(dir.path())
            .save(&stored)
            .expect("save stored snapshot");
        assert!(!state.apply_sync_outcome(&fallback(ActiveSource::LastValid)));
        assert_eq!(state.source, ActiveSource::LastValid);
        assert_eq!(state.snapshot, stored);
        assert_eq!(state.last_sync_at, snapshot_time(&stored));
    }

    #[test]
    fn refresh_snapshot_picks_up_a_new_file_and_falls_back_when_it_is_gone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = load(dir.path());
        assert_eq!(state.refresh_snapshot(), ActiveSource::Bundled);

        let mut stored = state.bundled.clone();
        stored.fetched_at = "2026-10-24T10:00:00Z".to_string();
        SnapshotStore::new(dir.path())
            .save(&stored)
            .expect("save stored snapshot");

        assert_eq!(state.refresh_snapshot(), ActiveSource::Remote);
        assert_eq!(state.snapshot, stored);
        assert_eq!(state.last_sync_at, Some(instant("2026-10-24T10:00:00Z")));

        fs::remove_file(dir.path().join("catalog.current.json")).expect("remove snapshot");
        assert_eq!(state.refresh_snapshot(), ActiveSource::Bundled);
        assert_eq!(state.snapshot, state.bundled);
        assert_eq!(state.last_sync_at, None);
    }
}
