//! Catalog sync: HTTP fetch, snapshots, fallback chain and change detection.
//!
//! One [`SyncEngine::sync`] call performs at most one conditional GET. The
//! fallback chain is remote → last valid snapshot → bundled snapshot, and the
//! outcome always names the active source (SYNC-03).

use chrono::{SecondsFormat, Utc};

use crate::catalog::{CatalogSnapshot, parse_mirror};
use crate::sync::http::{HttpClient, HttpResponse};
use crate::sync::snapshot::SnapshotStore;

pub mod http;
pub mod snapshot;

/// Anonymous mirror URL holding the Roster.
pub const MIRROR_URL: &str = "https://models.opencode.ai/api.json";

/// Where the active Catalog data came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSource {
    Remote,
    LastValid,
    Bundled,
}

/// Result of one sync attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// A validated remote payload became the current snapshot.
    Updated { source: ActiveSource },
    /// The remote reported 304 and the stored snapshot is unchanged.
    NotModified,
    /// The remote path failed; `source` names the data now active.
    Fallback {
        source: ActiveSource,
        reason: String,
    },
}

/// Sync status plus non-fatal findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    pub status: SyncStatus,
    /// Skipped roster entries, one message each.
    pub warnings: Vec<String>,
}

/// Orchestrates fetch, validation, persistence and fallback.
pub struct SyncEngine<'a> {
    client: &'a dyn HttpClient,
    store: SnapshotStore,
    bundled: CatalogSnapshot,
}

impl<'a> SyncEngine<'a> {
    /// Build an engine for one client, store directory and bundled fallback.
    pub fn new(client: &'a dyn HttpClient, store: SnapshotStore, bundled: CatalogSnapshot) -> Self {
        Self {
            client,
            store,
            bundled,
        }
    }

    /// Run one sync attempt (SYNC-01..05, SYNC-07, SYNC-09, SYNC-10).
    pub fn sync(&self) -> SyncOutcome {
        let stored = self.store.load_current();
        let etag = stored
            .as_ref()
            .and_then(|snapshot| snapshot.etag.as_deref());

        let response = match self.client.get(MIRROR_URL, etag) {
            Ok(response) => response,
            Err(error) => return self.fallback(stored.is_some(), format!("fetch failed: {error}")),
        };

        match response.status {
            200 => self.apply_remote(response, stored.is_some()),
            304 if stored.is_some() => SyncOutcome {
                status: SyncStatus::NotModified,
                warnings: Vec::new(),
            },
            304 => self.fallback(false, "304 response with no valid snapshot".to_string()),
            status => self.fallback(stored.is_some(), format!("unexpected HTTP status {status}")),
        }
    }

    /// The bundled snapshot used as the last fallback.
    pub fn bundled(&self) -> &CatalogSnapshot {
        &self.bundled
    }

    fn apply_remote(&self, response: HttpResponse, has_last_valid: bool) -> SyncOutcome {
        let parsed = match parse_mirror(&response.body) {
            Ok(parsed) => parsed,
            Err(error) => {
                return self.fallback(has_last_valid, format!("invalid payload: {error}"));
            }
        };

        if parsed.models.is_empty() {
            return self.fallback(has_last_valid, "invalid payload: empty roster".to_string());
        }

        let warnings: Vec<String> = parsed
            .skipped
            .iter()
            .map(|entry| format!("skipped model `{}`: {}", entry.id, entry.reason()))
            .collect();

        let snapshot = CatalogSnapshot {
            schema: 1,
            source: MIRROR_URL.to_string(),
            etag: response.etag,
            fetched_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            models: parsed.models,
        };

        if let Err(error) = self.store.save(&snapshot) {
            let mut outcome = self.fallback(
                has_last_valid,
                format!("failed to persist snapshot: {error}"),
            );
            outcome.warnings = warnings;
            return outcome;
        }

        SyncOutcome {
            status: SyncStatus::Updated {
                source: ActiveSource::Remote,
            },
            warnings,
        }
    }

    fn fallback(&self, has_last_valid: bool, reason: String) -> SyncOutcome {
        let source = if has_last_valid {
            ActiveSource::LastValid
        } else {
            ActiveSource::Bundled
        };

        SyncOutcome {
            status: SyncStatus::Fallback { source, reason },
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelEntry, Price};
    use crate::sync::http::HttpError;
    use std::cell::RefCell;
    use std::path::Path;

    struct StubClient {
        response: RefCell<Option<Result<HttpResponse, HttpError>>>,
        calls: RefCell<Vec<(String, Option<String>)>>,
    }

    impl StubClient {
        fn new(response: Result<HttpResponse, HttpError>) -> Self {
            Self {
                response: RefCell::new(Some(response)),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(String, Option<String>)> {
            self.calls.borrow().clone()
        }
    }

    impl HttpClient for StubClient {
        fn get(&self, url: &str, etag: Option<&str>) -> Result<HttpResponse, HttpError> {
            self.calls
                .borrow_mut()
                .push((url.to_string(), etag.map(str::to_owned)));
            self.response
                .borrow_mut()
                .take()
                .expect("one scripted response")
        }
    }

    fn model(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            provider_id: "opencode-go".to_string(),
            name: id.to_string(),
            price: Price {
                input: 0.15,
                output: 0.6,
            },
            cache_read: 0.003,
            context: 1_048_576,
            tool_call: true,
            attachment: true,
            reasoning: true,
            release_date: None,
        }
    }

    fn snapshot(etag: &str, model_id: &str) -> CatalogSnapshot {
        CatalogSnapshot {
            schema: 1,
            source: MIRROR_URL.to_string(),
            etag: Some(etag.to_string()),
            fetched_at: "2026-10-23T00:00:00Z".to_string(),
            models: vec![model(model_id)],
        }
    }

    fn mirror(entries: &str) -> String {
        format!(r#"{{"opencode-go":{{"models":{{{entries}}}}}}}"#)
    }

    fn valid_entry(id: &str) -> String {
        format!(
            r#""{id}": {{
                "id": "{id}",
                "name": "{id}",
                "cost": {{ "input": 0.15, "output": 0.6, "cache_read": 0.003 }},
                "limit": {{ "context": 1048576 }},
                "tool_call": true,
                "attachment": true
            }}"#
        )
    }

    fn store(dir: &Path) -> SnapshotStore {
        SnapshotStore::new(dir)
    }

    fn current_bytes(dir: &Path) -> Option<Vec<u8>> {
        std::fs::read(dir.join(crate::sync::snapshot::CURRENT_FILE)).ok()
    }

    #[test]
    fn valid_200_updates_current_and_reports_remote() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let client = StubClient::new(Ok(HttpResponse {
            status: 200,
            body: mirror(&valid_entry("model-a")),
            etag: Some("\"new-etag\"".to_string()),
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        assert_eq!(
            outcome.status,
            SyncStatus::Updated {
                source: ActiveSource::Remote
            }
        );
        assert!(outcome.warnings.is_empty());

        let current = store.load_current().expect("current snapshot stored");
        assert_eq!(current.models.len(), 1);
        assert_eq!(current.models[0].id, "model-a");
        assert_eq!(current.source, MIRROR_URL);
        assert_eq!(current.etag.as_deref(), Some("\"new-etag\""));
        chrono::DateTime::parse_from_rfc3339(&current.fetched_at).expect("fetched_at is RFC 3339");

        assert_eq!(
            client.calls(),
            vec![(MIRROR_URL.to_string(), None)],
            "first sync has no ETag to send"
        );
    }

    #[test]
    fn not_modified_leaves_snapshots_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let last_valid = snapshot("\"old-etag\"", "last-valid");
        store.save(&last_valid).expect("seed last valid");
        let before = current_bytes(dir.path());

        let client = StubClient::new(Ok(HttpResponse {
            status: 304,
            body: String::new(),
            etag: None,
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        assert_eq!(outcome.status, SyncStatus::NotModified);
        assert_eq!(current_bytes(dir.path()), before);
        assert_eq!(store.load_current(), Some(last_valid));
        assert_eq!(
            client.calls(),
            vec![(MIRROR_URL.to_string(), Some("\"old-etag\"".to_string()))]
        );
    }

    #[test]
    fn network_error_falls_back_to_last_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let last_valid = snapshot("\"old-etag\"", "last-valid");
        store.save(&last_valid).expect("seed last valid");
        let before = current_bytes(dir.path());

        let client = StubClient::new(Err(HttpError::Transport("boom".to_string())));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::LastValid);
                assert!(reason.contains("boom"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert_eq!(current_bytes(dir.path()), before);
        assert_eq!(store.load_current(), Some(last_valid));
    }

    #[test]
    fn non_2xx_falls_back_to_last_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let last_valid = snapshot("\"old-etag\"", "last-valid");
        store.save(&last_valid).expect("seed last valid");

        let client = StubClient::new(Ok(HttpResponse {
            status: 500,
            body: "server exploded".to_string(),
            etag: None,
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::LastValid);
                assert!(reason.contains("500"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert_eq!(store.load_current(), Some(last_valid));
    }

    #[test]
    fn invalid_payload_does_not_overwrite_last_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let last_valid = snapshot("\"old-etag\"", "last-valid");
        store.save(&last_valid).expect("seed last valid");
        let before = current_bytes(dir.path());

        let client = StubClient::new(Ok(HttpResponse {
            status: 200,
            body: "not json".to_string(),
            etag: Some("\"new\"".to_string()),
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::LastValid);
                assert!(reason.contains("invalid payload"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert_eq!(current_bytes(dir.path()), before);
        assert_eq!(store.load_current(), Some(last_valid));
    }

    #[test]
    fn empty_roster_is_treated_as_invalid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let last_valid = snapshot("\"old-etag\"", "last-valid");
        store.save(&last_valid).expect("seed last valid");

        let client = StubClient::new(Ok(HttpResponse {
            status: 200,
            body: mirror(""),
            etag: Some("\"new\"".to_string()),
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::LastValid);
                assert!(reason.contains("empty roster"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert_eq!(store.load_current(), Some(last_valid));
    }

    #[test]
    fn not_modified_without_snapshot_falls_back_to_bundled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let client = StubClient::new(Ok(HttpResponse {
            status: 304,
            body: String::new(),
            etag: None,
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::Bundled);
                assert!(reason.contains("304"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert!(current_bytes(dir.path()).is_none());
    }

    #[test]
    fn network_error_without_snapshot_falls_back_to_bundled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let client = StubClient::new(Err(HttpError::Transport("offline".to_string())));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        match &outcome.status {
            SyncStatus::Fallback { source, reason } => {
                assert_eq!(*source, ActiveSource::Bundled);
                assert!(reason.contains("offline"), "{reason}");
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        assert!(current_bytes(dir.path()).is_none());
    }

    #[test]
    fn skipped_entries_are_surfaced_as_warnings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let broken = r#""broken": { "id": "broken", "cost": { "output": 1.0 } }"#;
        let body = mirror(&format!("{},{}", valid_entry("model-a"), broken));

        let client = StubClient::new(Ok(HttpResponse {
            status: 200,
            body,
            etag: None,
        }));
        let engine = SyncEngine::new(&client, store.clone(), snapshot("\"bundled\"", "bundled"));

        let outcome = engine.sync();

        assert_eq!(
            outcome.status,
            SyncStatus::Updated {
                source: ActiveSource::Remote
            }
        );
        assert_eq!(outcome.warnings.len(), 1);
        assert!(
            outcome.warnings[0].contains("broken"),
            "{:?}",
            outcome.warnings
        );
        assert!(
            outcome.warnings[0].contains("missing required fields"),
            "{:?}",
            outcome.warnings
        );

        let current = store.load_current().expect("current snapshot stored");
        assert_eq!(current.models.len(), 1);
        assert_eq!(current.models[0].id, "model-a");
    }
}
