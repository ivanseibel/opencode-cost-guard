//! Pure Pricing Window state machine.
//!
//! Peak applies only on provider-listed weekdays, inside a half-open window
//! `[start, end)` in UTC minutes, and never on a curated Chinese holiday.
//! Makeup weekends need no special case: they are weekend dates, and weekends
//! are not peak days. The function is pure: it reads no clock and keeps no
//! mutable state (SCHED-05).

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone, Timelike, Utc, Weekday};

use crate::overlay::{Overlays, PricingWindow};

/// Days scanned when looking for the next state transition.
const NEXT_TRANSITION_HORIZON_DAYS: u64 = 400;

/// Pricing state for one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowState {
    /// Peak pricing applies at the requested instant.
    pub peak: bool,
    /// First instant after the requested one where the state differs, in UTC.
    ///
    /// [`DateTime::<Utc>::MAX_UTC`] means no transition exists inside the
    /// search horizon (only reachable with a calendar that marks every
    /// weekday as a holiday).
    pub next_transition: DateTime<Utc>,
    /// Year missing from the curated holiday calendar, when applicable.
    pub coverage_gap: Option<i32>,
}

/// One provider's peak rule, borrowed from the Overlays.
struct ProviderRule<'a> {
    days: u8,
    windows: &'a [PricingWindow],
}

/// Return the pricing state for `now` (SCHED-01..06).
pub fn state_at(now: DateTime<Utc>, overlays: &Overlays) -> WindowState {
    let rules: Vec<ProviderRule<'_>> = overlays
        .windows
        .providers
        .iter()
        .map(|provider| ProviderRule {
            days: day_mask(&provider.days),
            windows: &provider.windows,
        })
        .collect();

    let (holidays, coverage_gap) = holidays_for(overlays, now.year());
    let peak = is_peak(now, &rules, &holidays);

    WindowState {
        peak,
        next_transition: next_transition(now, peak, &rules, &holidays),
        coverage_gap,
    }
}

fn day_mask(days: &[String]) -> u8 {
    days.iter()
        .filter_map(|day| parse_day(day))
        .fold(0, |mask, day| mask | 1 << day.num_days_from_monday())
}

fn parse_day(day: &str) -> Option<Weekday> {
    match day.trim().to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn holidays_for(overlays: &Overlays, year: i32) -> (BTreeSet<NaiveDate>, Option<i32>) {
    let Some(entry) = overlays
        .holidays
        .years
        .iter()
        .find(|entry| entry.year == year)
    else {
        return (BTreeSet::new(), Some(year));
    };

    let holidays = entry
        .holidays
        .iter()
        .filter_map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .collect();

    (holidays, None)
}

fn is_peak(now: DateTime<Utc>, rules: &[ProviderRule<'_>], holidays: &BTreeSet<NaiveDate>) -> bool {
    if holidays.contains(&now.date_naive()) {
        return false;
    }

    let weekday = 1u8 << now.weekday().num_days_from_monday();
    let minute = now.hour() * 60 + now.minute();

    rules.iter().any(|rule| {
        rule.days & weekday != 0
            && rule
                .windows
                .iter()
                .any(|window| minute >= window.start_minutes && minute < window.end_minutes)
    })
}

fn next_transition(
    now: DateTime<Utc>,
    peak: bool,
    rules: &[ProviderRule<'_>],
    holidays: &BTreeSet<NaiveDate>,
) -> DateTime<Utc> {
    let mut candidates = Vec::new();
    let today = now.date_naive();

    for offset in 0..=NEXT_TRANSITION_HORIZON_DAYS {
        let Some(day) = today.checked_add_days(Days::new(offset)) else {
            break;
        };
        candidates.push(midnight(day));
        for rule in rules {
            for window in rule.windows {
                candidates.push(minute_instant(day, window.start_minutes));
                candidates.push(minute_instant(day, window.end_minutes));
            }
        }
    }

    candidates.retain(|candidate| *candidate > now);
    candidates.sort_unstable();
    candidates.dedup();

    candidates
        .into_iter()
        .find(|candidate| is_peak(*candidate, rules, holidays) != peak)
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

fn midnight(day: NaiveDate) -> DateTime<Utc> {
    minute_instant(day, 0)
}

fn minute_instant(day: NaiveDate, minutes: u32) -> DateTime<Utc> {
    let day = if minutes >= 24 * 60 {
        day.checked_add_days(Days::new(u64::from(minutes / (24 * 60))))
            .expect("candidate day is within the supported calendar range")
    } else {
        day
    };
    let minutes = minutes % (24 * 60);
    let naive = day
        .and_hms_opt(minutes / 60, minutes % 60, 0)
        .expect("minutes are within a day");
    Utc.from_utc_datetime(&naive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::{
        CapabilitiesOverlay, HolidayYear, HolidaysOverlay, LimitsOverlay, ProviderWindows,
        WindowsOverlay,
    };

    fn instant(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .expect("valid instant")
    }

    fn synthetic_overlays() -> Overlays {
        let metadata = || {
            (
                "https://example.com/synthetic".to_string(),
                "2030-01-01".to_string(),
            )
        };

        Overlays {
            windows: WindowsOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                providers: vec![ProviderWindows {
                    provider: "deepseek".to_string(),
                    multiplier: 2.0,
                    days: ["Mon", "Tue", "Wed", "Thu", "Fri"]
                        .iter()
                        .map(|day| day.to_string())
                        .collect(),
                    windows: vec![
                        PricingWindow {
                            start_minutes: 60,
                            end_minutes: 240,
                        },
                        PricingWindow {
                            start_minutes: 360,
                            end_minutes: 600,
                        },
                    ],
                    source: metadata().0,
                    last_verified: metadata().1,
                }],
            },
            holidays: HolidaysOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                years: vec![
                    HolidayYear {
                        year: 2028,
                        holidays: Vec::new(),
                        makeup_weekends: Vec::new(),
                        source: metadata().0,
                        last_verified: metadata().1,
                    },
                    HolidayYear {
                        year: 2030,
                        holidays: vec!["2030-03-13".to_string()],
                        makeup_weekends: vec!["2030-03-09".to_string()],
                        source: metadata().0,
                        last_verified: metadata().1,
                    },
                ],
            },
            limits: LimitsOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                plan: None,
                models: Vec::new(),
            },
            capabilities: CapabilitiesOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                models: Vec::new(),
            },
        }
    }

    #[test]
    fn boundary_instants_follow_half_open_rule() {
        let overlays = synthetic_overlays();
        let cases = [
            (0, 59, 59, false),
            (1, 0, 0, true),
            (3, 59, 59, true),
            (4, 0, 0, false),
            (5, 59, 59, false),
            (6, 0, 0, true),
            (9, 59, 59, true),
            (10, 0, 0, false),
        ];

        for (hour, minute, second, expected) in cases {
            let state = state_at(instant(2030, 3, 12, hour, minute, second), &overlays);
            assert_eq!(
                state.peak, expected,
                "unexpected state at {hour:02}:{minute:02}:{second:02}"
            );
        }
    }

    #[test]
    fn window_ends_are_the_next_transition() {
        let overlays = synthetic_overlays();

        assert_eq!(
            state_at(instant(2030, 3, 12, 3, 59, 59), &overlays).next_transition,
            instant(2030, 3, 12, 4, 0, 0)
        );
        assert_eq!(
            state_at(instant(2030, 3, 12, 9, 59, 59), &overlays).next_transition,
            instant(2030, 3, 12, 10, 0, 0)
        );
    }

    #[test]
    fn weekend_is_off_peak() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 10, 2, 0, 0), &overlays);

        assert!(!state.peak);
        assert_eq!(state.next_transition, instant(2030, 3, 11, 1, 0, 0));
    }

    #[test]
    fn holiday_weekday_is_off_peak() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 13, 2, 0, 0), &overlays);

        assert!(!state.peak);
        assert_eq!(state.coverage_gap, None);
        assert_eq!(state.next_transition, instant(2030, 3, 14, 1, 0, 0));
    }

    #[test]
    fn makeup_weekend_is_off_peak() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 9, 2, 0, 0), &overlays);

        assert!(!state.peak);
        assert_eq!(state.next_transition, instant(2030, 3, 11, 1, 0, 0));
    }

    #[test]
    fn next_transition_from_off_peak() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 12, 0, 30, 0), &overlays);

        assert!(!state.peak);
        assert_eq!(state.next_transition, instant(2030, 3, 12, 1, 0, 0));
    }

    #[test]
    fn next_transition_inside_a_window() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 12, 2, 0, 0), &overlays);

        assert!(state.peak);
        assert_eq!(state.next_transition, instant(2030, 3, 12, 4, 0, 0));
    }

    #[test]
    fn next_transition_after_the_last_window() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2030, 3, 15, 10, 30, 0), &overlays);

        assert!(!state.peak);
        assert_eq!(state.next_transition, instant(2030, 3, 18, 1, 0, 0));
    }

    #[test]
    fn leap_day_follows_the_weekday_rule() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2028, 2, 29, 2, 0, 0), &overlays);

        assert!(state.peak);
        assert_eq!(state.coverage_gap, None);
        assert_eq!(state.next_transition, instant(2028, 2, 29, 4, 0, 0));
    }

    #[test]
    fn uncovered_year_is_flagged_and_computed_without_exclusions() {
        let overlays = synthetic_overlays();
        let state = state_at(instant(2031, 6, 4, 2, 0, 0), &overlays);

        assert_eq!(state.coverage_gap, Some(2031));
        assert!(state.peak);

        let covered = state_at(instant(2030, 6, 4, 2, 0, 0), &overlays);
        assert_eq!(covered.coverage_gap, None);
    }

    #[test]
    fn state_is_deterministic_and_ignores_wall_clock() {
        let overlays = synthetic_overlays();
        let fixed = instant(2030, 3, 12, 2, 0, 0);

        let first = state_at(fixed, &overlays);
        let second = state_at(fixed, &overlays);
        assert_eq!(first, second);

        // A historic instant still resolves from the rules, not from today.
        let historic = state_at(instant(2001, 1, 1, 2, 0, 0), &overlays);
        assert!(historic.peak);
        assert_eq!(historic.coverage_gap, Some(2001));
    }
}
