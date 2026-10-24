//! Pure string builders for the tray title and native menu.
//!
//! No Tauri types cross this boundary, so every label the user sees can be
//! asserted exactly in unit tests (TRAY-01, MENU-01, MENU-03, MENU-04,
//! MENU-06).

use chrono::{DateTime, FixedOffset, Utc};
use cost_guard_core::catalog::ModelEntry;
use cost_guard_core::overlay::Warning;
use cost_guard_core::recommend::Recommendation;
use cost_guard_core::sync::ActiveSource;

/// Short tray title for the current DeepSeek window, with the change badge suffix.
///
/// Kept deliberately compact (icon + two words) so the status item survives a
/// crowded menu bar; the menu header carries the full label via [`state_label`].
pub fn tray_title(peak: bool, badge: bool) -> String {
    let mut title = if peak {
        "🔴 2×".to_string()
    } else {
        "🟢 off".to_string()
    };
    if badge {
        title.push_str(" ⚠");
    }
    title
}

/// Full state label used in the menu header.
pub fn state_label(peak: bool) -> &'static str {
    if peak { "🔴 DS 2×" } else { "🟢 DS off" }
}

/// Header and price lines for one model at the current window.
///
/// `multiplier` is the provider's peak multiplier from the Overlays; it only
/// applies when `peak` is true, so off-peak prices stay flat.
pub fn price_lines(model: &ModelEntry, peak: bool, multiplier: f64) -> Vec<String> {
    let factor = if peak { multiplier } else { 1.0 };
    let window = if peak {
        format!("peak {}×", format_multiplier(multiplier))
    } else {
        "off-peak".to_string()
    };

    vec![
        format!("{} — {}", model.name, window),
        format!("Input: ${}/M", format_usd(model.price.input * factor)),
        format!("Output: ${}/M", format_usd(model.price.output * factor)),
        format!("Cache read: ${}/M", format_usd(model.cache_read * factor)),
    ]
}

/// Next Pricing Window transition in local time with UTC in parentheses.
pub fn next_transition_line(utc: DateTime<Utc>, local: DateTime<FixedOffset>) -> String {
    if utc == DateTime::<Utc>::MAX_UTC {
        return "Next transition: none within the calendar horizon".to_string();
    }

    format!(
        "Next transition: {} local ({} UTC)",
        local.format("%a %H:%M"),
        utc.format("%a %H:%M")
    )
}

/// Pick line plus one line per reason; a no-candidate result names the filter.
pub fn recommendation_lines(recommendation: &Recommendation) -> Vec<String> {
    match (&recommendation.pick, &recommendation.no_candidate) {
        (Some(pick), _) => {
            let mut lines = vec![format!("Pick: {}", pick.name)];
            lines.extend(
                recommendation
                    .reasons
                    .iter()
                    .map(|reason| format!("Reason: {}", reason.text)),
            );
            lines
        }
        (None, Some(no_candidate)) => vec![format!(
            "No candidate: {} — {}",
            no_candidate.filter, no_candidate.message
        )],
        (None, None) => vec!["No candidate: no recommendation available".to_string()],
    }
}

/// Up to three alternative picks, in engine order.
pub fn alternative_lines(recommendation: &Recommendation) -> Vec<String> {
    recommendation
        .alternatives
        .iter()
        .take(3)
        .map(|pick| format!("Alternative: {}", pick.name))
        .collect()
}

/// Active catalog source and the last successful fetch, when known.
pub fn source_line(source: ActiveSource, last_sync: Option<DateTime<Utc>>) -> String {
    let label = match source {
        ActiveSource::Remote => "remote snapshot",
        ActiveSource::LastValid => "last valid snapshot",
        ActiveSource::Bundled => "bundled snapshot",
    };
    let synced = match last_sync {
        Some(time) => time.format("%Y-%m-%d %H:%M UTC").to_string(),
        None => "never synced".to_string(),
    };

    format!("Source: {label} · last sync {synced}")
}

/// Menu warning lines for non-fatal Overlay validation findings.
///
/// The holiday-coverage gap is the user-visible case: peak math keeps working
/// from the weekday rules, but the menu names the missing year.
pub fn overlay_warning_lines(warnings: &[Warning]) -> Vec<String> {
    warnings
        .iter()
        .map(|warning| format!("Warning: {warning}"))
        .collect()
}

/// Render a multiplier without a trailing `.0` (`2.0` becomes `2`).
fn format_multiplier(multiplier: f64) -> String {
    if multiplier.fract() == 0.0 {
        format!("{multiplier:.0}")
    } else {
        format!("{multiplier}")
    }
}

/// Render USD per million tokens with at least two and at most four decimals.
fn format_usd(value: f64) -> String {
    let mut text = format!("{value:.4}");
    if let Some(dot) = text.find('.') {
        while text.len() > dot + 3 && text.ends_with('0') {
            text.pop();
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use cost_guard_core::catalog::Price;
    use cost_guard_core::recommend::{FailingFilter, NoCandidate, Reason};
    use cost_guard_core::recommend::{Pick, ReasonCode};

    fn deepseek() -> ModelEntry {
        ModelEntry {
            id: "deepseek-v4.1-flash".to_string(),
            provider_id: "opencode-go".to_string(),
            name: "DeepSeek V4.1 Flash".to_string(),
            price: Price {
                input: 0.15,
                output: 0.6,
            },
            cache_read: 0.003,
            context: 1_048_576,
            tool_call: true,
            attachment: true,
            reasoning: true,
            release_date: Some("2026-09-10".to_string()),
        }
    }

    fn pick(name: &str) -> Pick {
        Pick {
            model_id: name.to_lowercase().replace(' ', "-"),
            name: name.to_string(),
            score: 8.5,
            cost_index: 0.25,
        }
    }

    fn recommendation_with_pick() -> Recommendation {
        Recommendation {
            pick: Some(pick("DeepSeek V4.1 Flash")),
            alternatives: vec![pick("Claude Haiku 5.5"), pick("Kimi K2")],
            reasons: vec![
                Reason {
                    code: ReasonCode::OffPeakWindow,
                    text: "off-peak pricing is active".to_string(),
                },
                Reason {
                    code: ReasonCode::StrongFit,
                    text: "strong capability fit for simple-coding".to_string(),
                },
            ],
            no_candidate: None,
        }
    }

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid fixture timestamp")
            .with_timezone(&Utc)
    }

    fn local(utc: DateTime<Utc>, offset_hours: i32) -> DateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(offset_hours * 3_600).expect("valid offset");
        offset.from_utc_datetime(&utc.naive_utc()).fixed_offset()
    }

    #[test]
    fn tray_title_matches_the_spec_exactly() {
        assert_eq!(tray_title(false, false), "🟢 off");
        assert_eq!(tray_title(true, false), "🔴 2×");
        assert_eq!(tray_title(false, true), "🟢 off ⚠");
        assert_eq!(tray_title(true, true), "🔴 2× ⚠");
    }

    #[test]
    fn menu_state_label_keeps_the_model_name() {
        assert_eq!(state_label(false), "🟢 DS off");
        assert_eq!(state_label(true), "🔴 DS 2×");
    }

    #[test]
    fn off_peak_prices_are_flat() {
        assert_eq!(
            price_lines(&deepseek(), false, 2.0),
            vec![
                "DeepSeek V4.1 Flash — off-peak",
                "Input: $0.15/M",
                "Output: $0.60/M",
                "Cache read: $0.003/M",
            ]
        );
    }

    #[test]
    fn peak_prices_apply_the_multiplier() {
        assert_eq!(
            price_lines(&deepseek(), true, 2.0),
            vec![
                "DeepSeek V4.1 Flash — peak 2×",
                "Input: $0.30/M",
                "Output: $1.20/M",
                "Cache read: $0.006/M",
            ]
        );
    }

    #[test]
    fn fractional_multipliers_keep_their_decimals() {
        let lines = price_lines(&deepseek(), true, 1.5);
        assert_eq!(lines[0], "DeepSeek V4.1 Flash — peak 1.5×");
        assert_eq!(lines[1], "Input: $0.225/M");
    }

    #[test]
    fn next_transition_shows_local_time_with_utc() {
        let utc = instant("2026-10-26T01:00:00Z");

        assert_eq!(
            next_transition_line(utc, local(utc, 8)),
            "Next transition: Mon 09:00 local (Mon 01:00 UTC)"
        );
    }

    #[test]
    fn missing_transition_is_reported_without_fake_dates() {
        assert_eq!(
            next_transition_line(
                DateTime::<Utc>::MAX_UTC,
                local(instant("2026-10-26T01:00:00Z"), 0)
            ),
            "Next transition: none within the calendar horizon"
        );
    }

    #[test]
    fn recommendation_renders_pick_and_reasons() {
        assert_eq!(
            recommendation_lines(&recommendation_with_pick()),
            vec![
                "Pick: DeepSeek V4.1 Flash",
                "Reason: off-peak pricing is active",
                "Reason: strong capability fit for simple-coding",
            ]
        );
    }

    #[test]
    fn alternatives_render_at_most_three() {
        let mut recommendation = recommendation_with_pick();
        recommendation.alternatives = vec![
            pick("Claude Haiku 5.5"),
            pick("Kimi K2"),
            pick("GPT-5 mini"),
            pick("Ignored Fourth"),
        ];

        assert_eq!(
            alternative_lines(&recommendation),
            vec![
                "Alternative: Claude Haiku 5.5",
                "Alternative: Kimi K2",
                "Alternative: GPT-5 mini",
            ]
        );
    }

    #[test]
    fn no_candidate_names_the_failing_filter() {
        let recommendation = Recommendation {
            pick: None,
            alternatives: Vec::new(),
            reasons: Vec::new(),
            no_candidate: Some(NoCandidate {
                filter: FailingFilter::Ignored,
                message: "every model is listed in ignoredModels".to_string(),
            }),
        };

        assert_eq!(
            recommendation_lines(&recommendation),
            vec!["No candidate: ignored models — every model is listed in ignoredModels"]
        );
    }

    #[test]
    fn recommendation_without_pick_or_reason_still_renders_a_line() {
        let recommendation = Recommendation {
            pick: None,
            alternatives: Vec::new(),
            reasons: Vec::new(),
            no_candidate: None,
        };

        assert_eq!(
            recommendation_lines(&recommendation),
            vec!["No candidate: no recommendation available"]
        );
    }

    #[test]
    fn source_lines_name_the_active_source_and_last_sync() {
        let synced = instant("2026-10-23T22:38:03Z");

        assert_eq!(
            source_line(ActiveSource::Remote, Some(synced)),
            "Source: remote snapshot · last sync 2026-10-23 22:38 UTC"
        );
        assert_eq!(
            source_line(ActiveSource::LastValid, Some(synced)),
            "Source: last valid snapshot · last sync 2026-10-23 22:38 UTC"
        );
        assert_eq!(
            source_line(ActiveSource::Bundled, None),
            "Source: bundled snapshot · last sync never synced"
        );
    }

    #[test]
    fn overlay_warnings_render_as_menu_lines() {
        assert_eq!(
            overlay_warning_lines(&[
                Warning::UncoveredHolidayYear(2027),
                Warning::UnratedModel("model-z".to_string()),
            ]),
            vec![
                "Warning: holiday calendar does not cover 2027",
                "Warning: model `model-z` has no capability rating",
            ]
        );
    }
}
