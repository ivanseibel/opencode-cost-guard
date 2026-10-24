//! 30 s state tick, peak notifications and sync scheduling.
//!
//! Every tick re-reads the Pricing Window state, feeds the transition tracker
//! and asks for a tray refresh; the tray itself skips unchanged presentations
//! (TRAY-03). Notifications fire at most once per transition (NOTIF-01..03).
//! Catalog sync is due on the periodic path when a sync has never completed or
//! the configured interval elapsed (SYNCA-01); `setup` additionally spawns one
//! unconditional sync on every launch.

use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cost_guard_core::config::Notifications;
use cost_guard_core::recommend::Engine;
use cost_guard_core::schedule::state_for;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tokio::time::MissedTickBehavior;

use crate::app_state::AppState;
use crate::events::{Transition, TransitionTracker};
use crate::menu::{DEEPSEEK_VENDOR, profile_label};
use crate::runtime;
use crate::{clock, sync_job, tray};

/// State tick cadence.
pub const TICK: Duration = Duration::from_secs(30);

/// Reason a Catalog Sync is being considered (SYNCA-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    /// Unconditional sync requested from `setup` on every launch.
    Startup,
    /// Periodic path driven by [`should_sync`].
    Interval,
}

/// Whether a sync is due at `now`, given the last successful fetch.
pub fn should_sync(
    now: DateTime<Utc>,
    last_sync: Option<DateTime<Utc>>,
    interval: ChronoDuration,
) -> bool {
    match last_sync {
        None => true,
        Some(last) => now.signed_duration_since(last) >= interval,
    }
}

/// Whether `trigger` requests a sync.
///
/// Startup ignores the stored snapshot age: a launch always refreshes the
/// catalog once. The periodic path keeps the interval gate.
pub fn sync_requested(
    trigger: SyncTrigger,
    now: DateTime<Utc>,
    last_sync: Option<DateTime<Utc>>,
    interval: ChronoDuration,
) -> bool {
    match trigger {
        SyncTrigger::Startup => true,
        SyncTrigger::Interval => should_sync(now, last_sync, interval),
    }
}

/// Start the repeating state tick. Called once from `setup`.
pub fn spawn(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(TICK);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut tracker = TransitionTracker::new();

        loop {
            interval.tick().await;
            tick(&handle, clock::now(), &mut tracker);
        }
    });
}

/// One tick: detect transitions, notify, schedule sync and refresh the tray.
fn tick(app: &AppHandle, now: DateTime<Utc>, tracker: &mut TransitionTracker) {
    let tick = runtime::with_state(app, |state| Tick {
        peak: state_for(DEEPSEEK_VENDOR, now, &state.overlays).is_some_and(|window| window.peak),
        notifications: state.config.notifications,
        sync_due: sync_requested(
            SyncTrigger::Interval,
            now,
            state.last_sync_at,
            ChronoDuration::hours(i64::from(state.config.sync_interval_hours)),
        ),
        body: notification_body(state, now),
    });

    if let Some(transition) = tracker.observe(tick.peak, &tick.notifications) {
        eprintln!("cost-guard: peak transition detected: {transition:?}");
        send_notification(app, transition, &tick.body);
    }
    if tick.sync_due {
        sync_job::spawn(app);
    }
    tray::request_refresh(app);
    tray::ensure_visible(app);
}

struct Tick {
    peak: bool,
    notifications: Notifications,
    sync_due: bool,
    body: String,
}

/// Notification body naming the active profile's recommendation (NOTIF-01/02).
fn notification_body(state: &AppState, now: DateTime<Utc>) -> String {
    let recommendation = Engine::new(&state.snapshot, &state.overlays, &state.config)
        .recommend(state.config.default_profile, now);

    match (&recommendation.pick, &recommendation.no_candidate) {
        (Some(pick), _) => format!(
            "Now recommending {} for {}.",
            pick.name,
            profile_label(state.config.default_profile)
        ),
        (None, Some(no_candidate)) => format!("No candidate: {}", no_candidate.message),
        (None, None) => "No recommendation available.".to_string(),
    }
}

fn send_notification(app: &AppHandle, transition: Transition, body: &str) {
    let title = match transition {
        Transition::EnteredPeak => "DeepSeek peak started",
        Transition::ExitedPeak => "DeepSeek peak ended",
    };
    match app.notification().builder().title(title).body(body).show() {
        Ok(()) => eprintln!("cost-guard: peak notification sent: {title} | {body}"),
        Err(error) => eprintln!("cost-guard: failed to send peak notification: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn instant(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 26, hour, 0, 0)
            .single()
            .expect("valid instant")
    }

    fn six_hours() -> ChronoDuration {
        ChronoDuration::hours(6)
    }

    #[test]
    fn missing_last_sync_is_due() {
        assert!(should_sync(instant(12), None, six_hours()));
    }

    #[test]
    fn recent_sync_is_not_due() {
        assert!(!should_sync(instant(5), Some(instant(0)), six_hours()));
    }

    #[test]
    fn exact_interval_is_due() {
        assert!(should_sync(instant(6), Some(instant(0)), six_hours()));
    }

    #[test]
    fn overdue_sync_is_due() {
        assert!(should_sync(instant(7), Some(instant(0)), six_hours()));
    }

    #[test]
    fn one_hour_interval_respects_the_boundary() {
        let interval = ChronoDuration::hours(1);
        assert!(!should_sync(
            instant(0) + ChronoDuration::minutes(59),
            Some(instant(0)),
            interval
        ));
        assert!(should_sync(instant(1), Some(instant(0)), interval));
    }

    #[test]
    fn startup_syncs_even_with_a_fresh_snapshot() {
        let now = instant(12);
        let fresh = Some(now - ChronoDuration::minutes(1));

        assert!(
            !should_sync(now, fresh, six_hours()),
            "a one-minute-old snapshot is not due on the periodic path"
        );
        assert!(
            sync_requested(SyncTrigger::Startup, now, fresh, six_hours()),
            "startup must request one sync regardless of snapshot age"
        );
        assert!(!sync_requested(
            SyncTrigger::Interval,
            now,
            fresh,
            six_hours()
        ));
    }
}
