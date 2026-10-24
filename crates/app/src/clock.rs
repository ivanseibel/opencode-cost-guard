//! Clock source with a debug-only fake time anchor.
//!
//! Release builds always read the wall clock. Debug builds accept
//! `COST_GUARD_FAKE_NOW` as an RFC 3339 instant and advance from it with real
//! elapsed time, so smoke runs can cross a peak boundary without waiting for
//! the real schedule.

use chrono::{DateTime, Utc};

/// Current time, honouring the debug fake-clock anchor.
pub fn now() -> DateTime<Utc> {
    #[cfg(debug_assertions)]
    if let Some(anchor) = fake_anchor() {
        let elapsed = chrono::Duration::from_std(anchor.1.elapsed()).unwrap_or_default();
        return anchor.0 + elapsed;
    }

    Utc::now()
}

#[cfg(debug_assertions)]
fn fake_anchor() -> Option<&'static (DateTime<Utc>, std::time::Instant)> {
    use std::sync::OnceLock;

    static ANCHOR: OnceLock<Option<(DateTime<Utc>, std::time::Instant)>> = OnceLock::new();

    ANCHOR
        .get_or_init(|| {
            let value = std::env::var("COST_GUARD_FAKE_NOW").ok()?;
            match DateTime::parse_from_rfc3339(&value) {
                Ok(time) => Some((time.with_timezone(&Utc), std::time::Instant::now())),
                Err(error) => {
                    eprintln!(
                        "cost-guard: ignoring invalid COST_GUARD_FAKE_NOW `{value}`: {error}"
                    );
                    None
                }
            }
        })
        .as_ref()
}
