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
    for warning in &outcome.warnings {
        eprintln!("cost-guard: sync warning: {warning}");
    }
    if let SyncStatus::Fallback { source, reason } = &outcome.status {
        eprintln!("cost-guard: sync fell back to {source:?}: {reason}");
    }
}
