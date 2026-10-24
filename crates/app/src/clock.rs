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
    {
        let (fake, elapsed) = match fake_anchor() {
            Some((time, started)) => (Some(*time), started.elapsed()),
            None => (None, std::time::Duration::ZERO),
        };
        resolve_now(fake, elapsed, Utc::now())
    }

    #[cfg(not(debug_assertions))]
    Utc::now()
}

/// Combine an optional fake anchor with elapsed time, falling back to `wall`.
#[cfg(debug_assertions)]
fn resolve_now(
    fake: Option<DateTime<Utc>>,
    elapsed: std::time::Duration,
    wall: DateTime<Utc>,
) -> DateTime<Utc> {
    match fake {
        Some(anchor) => anchor + chrono::Duration::from_std(elapsed).unwrap_or_default(),
        None => wall,
    }
}

#[cfg(debug_assertions)]
fn fake_anchor() -> Option<&'static (DateTime<Utc>, std::time::Instant)> {
    use std::sync::OnceLock;

    static ANCHOR: OnceLock<Option<(DateTime<Utc>, std::time::Instant)>> = OnceLock::new();

    ANCHOR
        .get_or_init(|| {
            let value = std::env::var("COST_GUARD_FAKE_NOW").ok()?;
            parse_anchor(&value).map(|time| (time, std::time::Instant::now()))
        })
        .as_ref()
}

/// Parse one RFC 3339 anchor; invalid values warn and disable the fixture.
#[cfg(debug_assertions)]
fn parse_anchor(value: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(time) => Some(time.with_timezone(&Utc)),
        Err(error) => {
            eprintln!("cost-guard: ignoring invalid COST_GUARD_FAKE_NOW `{value}`: {error}");
            None
        }
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::time::Duration;

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid fixture timestamp")
            .with_timezone(&Utc)
    }

    fn wall() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 24, 12, 0, 0)
            .single()
            .expect("valid wall clock")
    }

    #[test]
    fn valid_anchor_parses_to_utc() {
        assert_eq!(
            parse_anchor("2026-10-26T00:59:45Z"),
            Some(instant("2026-10-26T00:59:45Z"))
        );
    }

    #[test]
    fn offset_anchor_is_normalized_to_utc() {
        assert_eq!(
            parse_anchor("2026-10-26T08:59:45+08:00"),
            Some(instant("2026-10-26T00:59:45Z"))
        );
    }

    #[test]
    fn invalid_anchor_warns_and_falls_back_to_the_wall_clock() {
        assert_eq!(parse_anchor("not-a-time"), None);
        assert_eq!(resolve_now(None, Duration::ZERO, wall()), wall());
    }

    #[test]
    fn empty_anchor_warns_and_falls_back_to_the_wall_clock() {
        assert_eq!(parse_anchor(""), None);
        assert_eq!(
            resolve_now(None, Duration::from_secs(30), wall()),
            wall(),
            "elapsed time never moves an absent anchor"
        );
    }

    #[test]
    fn valid_anchor_advances_with_elapsed_time() {
        let anchor = instant("2026-10-26T00:59:45Z");

        assert_eq!(
            resolve_now(Some(anchor), Duration::from_secs(45), wall()),
            instant("2026-10-26T01:00:30Z")
        );
    }
}
