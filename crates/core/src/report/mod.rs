//! Renderable status report used by the demo example and, later, the app menu.
//!
//! [`build_report`] is a pure function of its inputs plus the bundled Overlays:
//! the caller passes the clock instant, the active Catalog snapshot and the
//! config. It never reads the clock itself, so equal inputs render equal output
//! (DEMO-01).
//!
//! SPEC_DEVIATION: the design sketch had `build_report(now, source)` load all
//! data itself. The example needs to select a synced snapshot when one exists
//! (DEMO-02), and the report unit tests need fixture data, so the snapshot and
//! config are parameters. Bundled Overlays are still loaded inside.

use std::fmt;

use chrono::{DateTime, Datelike, Local, SecondsFormat, Utc};

use crate::catalog::CatalogSnapshot;
use crate::config::{Config, TaskProfile};
use crate::overlay::{OverlayError, Overlays};
use crate::recommend::{Engine, Recommendation};
use crate::schedule::{WindowState, state_at};
use crate::sync::ActiveSource;

/// Everything the report renders.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusReport {
    pub state: WindowState,
    pub recommendations: Vec<(TaskProfile, Recommendation)>,
    pub source: ActiveSource,
    pub warnings: Vec<String>,
}

/// Report construction errors.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("failed to load bundled overlays: {0}")]
    Overlays(#[from] OverlayError),
}

/// Build the status report for `now` from the active snapshot and config.
pub fn build_report(
    now: DateTime<Utc>,
    source: ActiveSource,
    snapshot: &CatalogSnapshot,
    config: &Config,
) -> Result<StatusReport, ReportError> {
    let overlays = Overlays::bundled()?;
    let state = state_at(now, &overlays);
    let engine = Engine::new(snapshot, &overlays, config);

    let recommendations = TaskProfile::ALL
        .into_iter()
        .map(|profile| (profile, engine.recommend(profile, &state)))
        .collect();
    let warnings = overlays
        .validate(&snapshot.models, now.year())
        .into_iter()
        .map(|warning| warning.to_string())
        .collect();

    Ok(StatusReport {
        state,
        recommendations,
        source,
        warnings,
    })
}

fn source_label(source: ActiveSource) -> &'static str {
    match source {
        ActiveSource::Remote => "remote snapshot",
        ActiveSource::LastValid => "last valid snapshot",
        ActiveSource::Bundled => "bundled snapshot",
    }
}

impl fmt::Display for StatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "OpenCode Cost Guard status")?;
        writeln!(
            f,
            "State: {}",
            if self.state.peak { "peak" } else { "off-peak" }
        )?;
        writeln!(
            f,
            "Next transition UTC: {}",
            self.state
                .next_transition
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        )?;
        let local = self.state.next_transition.with_timezone(&Local);
        writeln!(
            f,
            "Next transition local: {}",
            local.format("%Y-%m-%d %H:%M:%S %:z")
        )?;
        writeln!(f, "Data source: {}", source_label(self.source))?;

        if self.warnings.is_empty() {
            writeln!(f, "Warnings: none")?;
        } else {
            writeln!(f, "Warnings:")?;
            for warning in &self.warnings {
                writeln!(f, "  - {warning}")?;
            }
        }

        for (profile, recommendation) in &self.recommendations {
            writeln!(f)?;
            match (&recommendation.pick, &recommendation.no_candidate) {
                (Some(pick), _) => {
                    writeln!(f, "{profile}: {}", pick.name)?;
                    for reason in &recommendation.reasons {
                        writeln!(f, "  - {}: {}", reason.code, reason.text)?;
                    }
                }
                (None, Some(no_candidate)) => {
                    writeln!(f, "{profile}: no recommendation ({})", no_candidate.filter)?;
                    writeln!(f, "  - {}", no_candidate.message)?;
                }
                (None, None) => writeln!(f, "{profile}: no recommendation")?,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogSnapshot, ModelEntry, Price, parse_snapshot};
    use chrono::{TimeZone, Utc};

    fn snapshot() -> CatalogSnapshot {
        parse_snapshot(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/bundled/opencode-go.json"
        )))
        .expect("bundled snapshot parses")
    }

    fn instant(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 26, hour, minute, 0)
            .single()
            .expect("valid instant")
    }

    fn build(now: DateTime<Utc>, source: ActiveSource) -> StatusReport {
        build_report(now, source, &snapshot(), &Config::default()).expect("report builds")
    }

    fn local_line(now: DateTime<Utc>) -> String {
        now.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S %:z")
            .to_string()
    }

    #[test]
    fn fixed_peak_clock_renders_state_transition_and_profiles() {
        let now = instant(2, 0);
        let report = build(now, ActiveSource::Bundled);

        assert!(report.state.peak);
        assert_eq!(
            report.state.next_transition,
            instant(4, 0),
            "peak ends at 04:00 UTC"
        );
        assert_eq!(report.recommendations.len(), 5);
        assert_eq!(
            report
                .recommendations
                .iter()
                .map(|(profile, _)| *profile)
                .collect::<Vec<_>>(),
            TaskProfile::ALL.to_vec()
        );
        assert!(
            report
                .recommendations
                .iter()
                .all(|(_, recommendation)| recommendation.pick.is_some()
                    && !recommendation.reasons.is_empty())
        );

        let rendered = report.to_string();
        assert!(rendered.contains("State: peak"), "{rendered}");
        assert!(
            rendered.contains("Next transition UTC: 2026-10-26T04:00:00Z"),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!(
                "Next transition local: {}",
                local_line(report.state.next_transition)
            )),
            "{rendered}"
        );
        assert!(
            rendered.contains("Data source: bundled snapshot"),
            "{rendered}"
        );
        assert!(rendered.contains("Warnings: none"), "{rendered}");

        for (profile, recommendation) in &report.recommendations {
            let pick = recommendation.pick.as_ref().expect("pick");
            assert!(
                rendered.contains(&format!("{profile}: {}", pick.name)),
                "{rendered}"
            );
            let first_code = recommendation.reasons[0].code.to_string();
            assert!(
                rendered.contains(&format!("  - {first_code}: ")),
                "{rendered}"
            );
        }
    }

    #[test]
    fn weekend_clock_renders_off_peak_and_the_monday_transition() {
        let now = Utc
            .with_ymd_and_hms(2026, 10, 24, 2, 0, 0)
            .single()
            .expect("valid instant");
        let report = build(now, ActiveSource::Bundled);

        assert!(!report.state.peak);
        assert_eq!(report.state.next_transition, instant(1, 0));

        let rendered = report.to_string();
        assert!(rendered.contains("State: off-peak"), "{rendered}");
        assert!(
            rendered.contains("Next transition UTC: 2026-10-26T01:00:00Z"),
            "{rendered}"
        );
        assert!(rendered.contains("Next transition local: "), "{rendered}");
    }

    #[test]
    fn every_active_source_has_a_distinct_label() {
        let now = instant(2, 0);

        for (source, label) in [
            (ActiveSource::Remote, "remote snapshot"),
            (ActiveSource::LastValid, "last valid snapshot"),
            (ActiveSource::Bundled, "bundled snapshot"),
        ] {
            let report = build(now, source);
            assert_eq!(report.source, source);
            assert!(
                report
                    .to_string()
                    .contains(&format!("Data source: {label}")),
                "{label}"
            );
        }
    }

    #[test]
    fn rendering_is_deterministic_for_equal_input() {
        let now = instant(2, 0);
        let first = build(now, ActiveSource::Bundled);
        let second = build(now, ActiveSource::Bundled);

        assert_eq!(first, second);
        assert_eq!(first.to_string(), second.to_string());
    }

    #[test]
    fn unrated_snapshot_models_surface_as_warnings() {
        let mut snapshot = snapshot();
        snapshot.models.push(ModelEntry {
            id: "unrated-model".to_string(),
            provider_id: "opencode-go".to_string(),
            name: "Unrated Model".to_string(),
            price: Price {
                input: 0.1,
                output: 0.2,
            },
            cache_read: 0.01,
            context: 200_000,
            tool_call: true,
            attachment: false,
            reasoning: false,
            release_date: None,
        });

        let report = build_report(
            instant(2, 0),
            ActiveSource::Bundled,
            &snapshot,
            &Config::default(),
        )
        .expect("report builds");

        assert_eq!(
            report.warnings,
            vec!["model `unrated-model` has no capability rating".to_string()]
        );
        assert!(
            report
                .to_string()
                .contains("Warnings:\n  - model `unrated-model` has no capability rating"),
            "{report}"
        );
    }
}
