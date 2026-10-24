//! Catalog sync worker with single-flight guarding.
//!
//! The worker runs the blocking core [`SyncEngine`] on a Tauri blocking thread
//! and applies the outcome to [`AppState`] under the state lock. A single
//! [`SyncGuard`] flag keeps overlapping syncs from starting (SYNCA-01).

use std::sync::atomic::{AtomicBool, Ordering};

use cost_guard_core::catalog::CatalogSnapshot;
use cost_guard_core::sync::http::{HttpClient, UreqClient};
use cost_guard_core::sync::snapshot::SnapshotStore;
use cost_guard_core::sync::{SyncEngine, SyncOutcome, SyncStatus};
use tauri::{AppHandle, Manager};

use crate::runtime;
use crate::tray;

/// Managed single-flight guard for catalog sync.
#[derive(Debug, Default)]
pub struct SyncGuard(AtomicBool);

impl SyncGuard {
    /// Whether a sync is currently in flight.
    pub fn is_running(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    fn try_start(&self) -> bool {
        self.0
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn finish(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Start one sync on a blocking worker unless one is already running.
pub fn spawn(app: &AppHandle) {
    if !app.state::<SyncGuard>().try_start() {
        return;
    }
    eprintln!("cost-guard: sync started");
    // Disable the manual item while the worker runs.
    tray::request_refresh(app);

    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let outcome = run_sync(&worker);
        log_outcome(&outcome);
        runtime::with_state_mut(&worker, |state| {
            state.apply_sync_outcome(&outcome);
        });
        worker.state::<SyncGuard>().finish();
        tray::request_refresh(&worker);
    });
}

/// Run one sync through the core engine.
///
/// Extracted from [`spawn`] so unit tests can script HTTP responses.
pub fn sync_once(
    client: &dyn HttpClient,
    store: &SnapshotStore,
    bundled: &CatalogSnapshot,
) -> SyncOutcome {
    SyncEngine::new(client, store.clone(), bundled.clone()).sync()
}

fn run_sync(app: &AppHandle) -> SyncOutcome {
    let (data_dir, bundled) =
        runtime::with_state(app, |state| (state.data_dir.clone(), state.bundled.clone()));
    let client = UreqClient::default();
    sync_once(&client, &SnapshotStore::new(data_dir), &bundled)
}

/// Log sync failures without a modal dialog (SYNCA-04).
fn log_outcome(outcome: &SyncOutcome) {
    match &outcome.status {
        SyncStatus::Updated { source } => eprintln!("cost-guard: sync updated ({source:?})"),
        SyncStatus::NotModified => eprintln!("cost-guard: sync not modified"),
        SyncStatus::Fallback { source, reason } => {
            eprintln!("cost-guard: sync fell back to {source:?}: {reason}");
        }
    }
    for warning in &outcome.warnings {
        eprintln!("cost-guard: sync warning: {warning}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use cost_guard_core::catalog::{ModelEntry, Price};
    use cost_guard_core::sync::http::{HttpError, HttpResponse};
    use cost_guard_core::sync::{ActiveSource, MIRROR_URL};

    use crate::app_state::AppState;

    enum Reply {
        Ok(HttpResponse),
        Err(String),
    }

    struct StubClient(Reply);

    impl HttpClient for StubClient {
        fn get(&self, _url: &str, _etag: Option<&str>) -> Result<HttpResponse, HttpError> {
            match &self.0 {
                Reply::Ok(response) => Ok(response.clone()),
                Reply::Err(message) => Err(HttpError::Transport(message.clone())),
            }
        }
    }

    fn model(id: &str, input: f64) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            provider_id: "opencode-go".to_string(),
            name: id.to_string(),
            price: Price { input, output: 0.6 },
            cache_read: 0.003,
            context: 1_048_576,
            tool_call: true,
            attachment: true,
            reasoning: true,
            release_date: None,
        }
    }

    fn snapshot(models: Vec<ModelEntry>, etag: Option<&str>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema: 1,
            source: MIRROR_URL.to_string(),
            etag: etag.map(str::to_owned),
            fetched_at: "2026-10-24T08:00:00Z".to_string(),
            models,
        }
    }

    fn mirror(entries: &str) -> String {
        format!(r#"{{"opencode-go":{{"models":{{{entries}}}}}}}"#)
    }

    fn entry(id: &str, input: f64) -> String {
        format!(
            r#""{id}": {{
                "id": "{id}",
                "name": "{id}",
                "cost": {{ "input": {input}, "output": 0.6, "cache_read": 0.003 }},
                "limit": {{ "context": 1048576 }},
                "tool_call": true,
                "attachment": true,
                "reasoning": true
            }}"#
        )
    }

    fn seed(dir: &Path, snapshot: &CatalogSnapshot) {
        SnapshotStore::new(dir)
            .save(snapshot)
            .expect("seed snapshot");
    }

    fn state(dir: &Path) -> AppState {
        AppState::load(dir).expect("app state loads")
    }

    #[test]
    fn sync_guard_allows_only_one_sync_at_a_time() {
        let guard = SyncGuard::default();
        assert!(!guard.is_running());

        assert!(guard.try_start());
        assert!(guard.is_running());
        assert!(!guard.try_start(), "second start must be refused");

        guard.finish();
        assert!(!guard.is_running());
        assert!(guard.try_start(), "guard reopens after finishing");
    }

    #[test]
    fn updated_outcome_sets_the_badge_when_the_catalog_changed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let previous = snapshot(vec![model("model-a", 0.15)], Some("\"old\""));
        seed(dir.path(), &previous);
        let mut state = state(dir.path());
        let client = StubClient(Reply::Ok(HttpResponse {
            status: 200,
            body: mirror(&entry("model-a", 1.5)),
            etag: Some("\"new\"".to_string()),
        }));

        let outcome = sync_once(&client, &SnapshotStore::new(dir.path()), &state.bundled);
        assert_eq!(
            outcome.status,
            SyncStatus::Updated {
                source: ActiveSource::Remote
            }
        );

        assert!(state.apply_sync_outcome(&outcome));
        assert!(state.badge);
        assert_eq!(state.source, ActiveSource::Remote);
        assert_eq!(state.snapshot.models[0].price.input, 1.5);
        assert!(state.last_sync_at.is_some());
    }

    #[test]
    fn updated_outcome_keeps_the_badge_off_for_identical_models() {
        let dir = tempfile::tempdir().expect("tempdir");
        let previous = snapshot(vec![model("model-a", 0.15)], Some("\"old\""));
        seed(dir.path(), &previous);
        let mut state = state(dir.path());
        let client = StubClient(Reply::Ok(HttpResponse {
            status: 200,
            body: mirror(&entry("model-a", 0.15)),
            etag: Some("\"new\"".to_string()),
        }));

        let outcome = sync_once(&client, &SnapshotStore::new(dir.path()), &state.bundled);
        assert!(!state.apply_sync_outcome(&outcome));
        assert!(!state.badge);
        assert_eq!(state.source, ActiveSource::Remote);
    }

    #[test]
    fn not_modified_keeps_state_and_the_badge_off() {
        let dir = tempfile::tempdir().expect("tempdir");
        let previous = snapshot(vec![model("model-a", 0.15)], Some("\"old\""));
        seed(dir.path(), &previous);
        let mut state = state(dir.path());
        let client = StubClient(Reply::Ok(HttpResponse {
            status: 304,
            body: String::new(),
            etag: None,
        }));

        let outcome = sync_once(&client, &SnapshotStore::new(dir.path()), &state.bundled);

        assert_eq!(outcome.status, SyncStatus::NotModified);
        assert!(!state.apply_sync_outcome(&outcome));
        assert!(!state.badge);
        assert_eq!(state.snapshot, previous);
        assert_eq!(state.source, ActiveSource::Remote);
    }

    #[test]
    fn network_failure_reports_bundled_and_keeps_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = state(dir.path());
        let client = StubClient(Reply::Err("offline".to_string()));

        let outcome = sync_once(&client, &SnapshotStore::new(dir.path()), &state.bundled);

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::Bundled);
                assert!(reason.contains("offline"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert!(!state.apply_sync_outcome(&outcome));
        assert!(!state.badge);
        assert_eq!(state.source, ActiveSource::Bundled);
        assert_eq!(state.snapshot, state.bundled);
        assert_eq!(state.last_sync_at, None);
    }

    #[test]
    fn network_failure_reports_last_valid_when_a_snapshot_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let previous = snapshot(vec![model("model-a", 0.15)], Some("\"old\""));
        seed(dir.path(), &previous);
        let mut state = state(dir.path());
        let client = StubClient(Reply::Err("offline".to_string()));

        let outcome = sync_once(&client, &SnapshotStore::new(dir.path()), &state.bundled);

        match &outcome.status {
            SyncStatus::Fallback { source, .. } => assert_eq!(*source, ActiveSource::LastValid),
            other => panic!("expected fallback, got {other:?}"),
        }
        assert!(!state.apply_sync_outcome(&outcome));
        assert!(!state.badge);
        assert_eq!(state.source, ActiveSource::LastValid);
        assert_eq!(state.snapshot, previous);
    }
}
