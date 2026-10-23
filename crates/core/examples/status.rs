//! Offline demo: print the current status and one recommendation per profile.
//!
//! Usage:
//!
//! ```text
//! cargo run -p cost-guard-core --example status
//! cargo run -p cost-guard-core --example status -- /path/to/data
//! ```
//!
//! The optional directory may contain `config.json` and the snapshot store
//! files. Without a valid current snapshot the example runs from the bundled
//! snapshot and the report names it as the active source (DEMO-02). The example
//! never performs a network call.

use std::path::PathBuf;

use chrono::Utc;
use cost_guard_core::catalog::parse_snapshot;
use cost_guard_core::config::Config;
use cost_guard_core::report::build_report;
use cost_guard_core::sync::ActiveSource;
use cost_guard_core::sync::snapshot::SnapshotStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let config = Config::load(&dir)?;

    let (snapshot, source) = match SnapshotStore::new(dir.as_path()).load_current() {
        Some(snapshot) => (snapshot, ActiveSource::LastValid),
        None => (
            parse_snapshot(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/bundled/opencode-go.json"
            )))?,
            ActiveSource::Bundled,
        ),
    };

    let report = build_report(Utc::now(), source, &snapshot, &config)?;
    print!("{report}");

    Ok(())
}
