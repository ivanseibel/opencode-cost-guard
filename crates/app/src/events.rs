//! Peak transition detection and notification dedupe.
//!
//! The tracker remembers the last observed window state, so the 30 s timer
//! reports a transition once and stays silent while the state holds
//! (NOTIF-01..03). Disabled toggles suppress their transition without freezing
//! the remembered state.

use cost_guard_core::config::Notifications;

/// A DeepSeek Pricing Window change worth one notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    EnteredPeak,
    ExitedPeak,
}

/// Remembers the last observed peak state between timer ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransitionTracker {
    last: Option<bool>,
}

impl TransitionTracker {
    /// A tracker that has not observed a state yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe the current peak state and return the transition, if any.
    ///
    /// The first observation only initializes the tracker. Later calls return
    /// at most one transition per state change, gated by `notifications`.
    pub fn observe(&mut self, peak: bool, notifications: &Notifications) -> Option<Transition> {
        let previous = self.last.replace(peak)?;

        match (previous, peak) {
            (false, true) if notifications.peak_enter => Some(Transition::EnteredPeak),
            (true, false) if notifications.peak_exit => Some(Transition::ExitedPeak),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> Notifications {
        Notifications {
            peak_enter: true,
            peak_exit: true,
        }
    }

    fn disabled() -> Notifications {
        Notifications {
            peak_enter: false,
            peak_exit: false,
        }
    }

    #[test]
    fn first_observation_initializes_without_notifying() {
        let mut tracker = TransitionTracker::new();

        assert_eq!(tracker.observe(false, &enabled()), None);
        assert_eq!(tracker.observe(true, &disabled()), None);
    }

    #[test]
    fn entering_peak_fires_once_and_repeated_ticks_are_silent() {
        let mut tracker = TransitionTracker::new();
        assert_eq!(tracker.observe(false, &enabled()), None);

        assert_eq!(
            tracker.observe(true, &enabled()),
            Some(Transition::EnteredPeak)
        );
        assert_eq!(tracker.observe(true, &enabled()), None);
        assert_eq!(tracker.observe(true, &enabled()), None);
    }

    #[test]
    fn leaving_peak_fires_once_and_repeated_ticks_are_silent() {
        let mut tracker = TransitionTracker::new();
        assert_eq!(tracker.observe(true, &enabled()), None);

        assert_eq!(
            tracker.observe(false, &enabled()),
            Some(Transition::ExitedPeak)
        );
        assert_eq!(tracker.observe(false, &enabled()), None);
        assert_eq!(tracker.observe(false, &enabled()), None);
    }

    #[test]
    fn disabled_peak_enter_suppresses_the_entry_only() {
        let mut tracker = TransitionTracker::new();
        let mut notifications = enabled();
        notifications.peak_enter = false;
        assert_eq!(tracker.observe(false, &notifications), None);

        assert_eq!(tracker.observe(true, &notifications), None);
        assert_eq!(tracker.observe(true, &notifications), None);
        assert_eq!(
            tracker.observe(false, &notifications),
            Some(Transition::ExitedPeak)
        );
    }

    #[test]
    fn disabled_peak_exit_suppresses_the_exit_only() {
        let mut tracker = TransitionTracker::new();
        let mut notifications = enabled();
        notifications.peak_exit = false;
        assert_eq!(tracker.observe(true, &notifications), None);

        assert_eq!(tracker.observe(false, &notifications), None);
        assert_eq!(
            tracker.observe(true, &notifications),
            Some(Transition::EnteredPeak)
        );
    }

    #[test]
    fn both_toggles_off_suppress_every_transition() {
        let mut tracker = TransitionTracker::new();

        assert_eq!(tracker.observe(false, &disabled()), None);
        assert_eq!(tracker.observe(true, &disabled()), None);
        assert_eq!(tracker.observe(false, &disabled()), None);
        assert_eq!(tracker.observe(true, &disabled()), None);
    }

    #[test]
    fn re_enabling_a_toggle_fires_on_the_next_change_only() {
        let mut tracker = TransitionTracker::new();
        assert_eq!(tracker.observe(false, &disabled()), None);
        assert_eq!(tracker.observe(true, &disabled()), None);

        assert_eq!(
            tracker.observe(false, &enabled()),
            Some(Transition::ExitedPeak)
        );
        assert_eq!(
            tracker.observe(true, &enabled()),
            Some(Transition::EnteredPeak)
        );
    }
}
