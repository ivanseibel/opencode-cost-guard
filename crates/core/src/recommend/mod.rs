//! Deterministic, explainable model recommendations per Task Profile.
//!
//! The engine applies hard filters first (REC-02, REC-03), then excludes
//! models listed in config `ignoredModels` (REC-06) and models without a
//! capability rating (REC-07), scores the survivors with [`score`], and returns
//! one pick plus up to three alternatives. No input order or clock affects the
//! result (REC-09). When the pool is emptied, the result names the filter that
//! removed the last candidate (REC-05).

pub mod score;

use chrono::{DateTime, Utc};

use crate::catalog::{CatalogSnapshot, ModelEntry};
use crate::config::{Config, TaskProfile};
use crate::overlay::{ModelCapability, Overlays};
use crate::schedule::state_for;

pub use score::ReasonCode;

use score::{CandidateInput, ScoredCandidate, reason_codes, score_candidates};

/// One recommended model.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub model_id: String,
    pub name: String,
    pub score: f64,
    /// Effective (peak-aware) blended price used for tie-breaking.
    pub cost_index: f64,
}

/// A reason paired with a code from the closed set (REC-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub code: ReasonCode,
    pub text: String,
}

/// The hard filter that left no candidate (REC-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailingFilter {
    EmptyRoster,
    Ignored,
    Unrated,
    Vision,
    Tools,
}

impl FailingFilter {
    /// Stable identifier used in reports.
    pub fn as_str(self) -> &'static str {
        match self {
            FailingFilter::EmptyRoster => "empty roster",
            FailingFilter::Ignored => "ignored models",
            FailingFilter::Unrated => "capability rating",
            FailingFilter::Vision => "vision support",
            FailingFilter::Tools => "tool-call support",
        }
    }

    /// Human-readable explanation naming the filter.
    pub fn message(self) -> &'static str {
        match self {
            FailingFilter::EmptyRoster => "the catalog snapshot contains no models",
            FailingFilter::Ignored => "every model is listed in ignoredModels",
            FailingFilter::Unrated => "no model has a capability rating",
            FailingFilter::Vision => "no rated model supports image input",
            FailingFilter::Tools => "no rated model supports tool calls",
        }
    }
}

impl std::fmt::Display for FailingFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why no recommendation was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoCandidate {
    pub filter: FailingFilter,
    pub message: String,
}

/// Result of one recommendation request (REC-01).
#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    pub pick: Option<Pick>,
    pub alternatives: Vec<Pick>,
    pub reasons: Vec<Reason>,
    pub no_candidate: Option<NoCandidate>,
}

impl Recommendation {
    fn no_candidate(filter: FailingFilter) -> Self {
        Self {
            pick: None,
            alternatives: Vec::new(),
            reasons: Vec::new(),
            no_candidate: Some(NoCandidate {
                filter,
                message: filter.message().to_string(),
            }),
        }
    }
}

/// Recommendation Engine over one snapshot, Overlay set and config.
pub struct Engine<'a> {
    snapshot: &'a CatalogSnapshot,
    overlays: &'a Overlays,
    config: &'a Config,
}

impl<'a> Engine<'a> {
    /// Build an engine borrowing the data it recommends from.
    pub fn new(snapshot: &'a CatalogSnapshot, overlays: &'a Overlays, config: &'a Config) -> Self {
        Self {
            snapshot,
            overlays,
            config,
        }
    }

    /// Recommend for `profile` at `now` (REC-01..10).
    ///
    /// Each candidate's multiplier comes from its own vendor's Pricing Window,
    /// so a DeepSeek peak does not reprice flat vendors (PEAK-01).
    pub fn recommend(&self, profile: TaskProfile, now: DateTime<Utc>) -> Recommendation {
        if self.snapshot.models.is_empty() {
            return Recommendation::no_candidate(FailingFilter::EmptyRoster);
        }

        let mut considered: Vec<&ModelEntry> = self
            .snapshot
            .models
            .iter()
            .filter(|model| !self.is_ignored(model))
            .collect();
        if considered.is_empty() {
            return Recommendation::no_candidate(FailingFilter::Ignored);
        }

        let rated: Vec<(&ModelEntry, &ModelCapability)> = considered
            .drain(..)
            .filter_map(|model| self.capability(&model.id).map(|cap| (model, cap)))
            .collect();
        if rated.is_empty() {
            return Recommendation::no_candidate(FailingFilter::Unrated);
        }

        let eligible: Vec<(&ModelEntry, &ModelCapability)> = match profile {
            TaskProfile::VisualUi => rated
                .into_iter()
                .filter(|(_, capability)| capability.vision)
                .collect(),
            TaskProfile::AgenticTools => rated
                .into_iter()
                .filter(|(_, capability)| capability.tools)
                .collect(),
            TaskProfile::SimpleCoding | TaskProfile::ComplexCoding | TaskProfile::LargeRefactor => {
                rated
            }
        };
        if eligible.is_empty() {
            return Recommendation::no_candidate(match profile {
                TaskProfile::VisualUi => FailingFilter::Vision,
                TaskProfile::AgenticTools => FailingFilter::Tools,
                TaskProfile::SimpleCoding
                | TaskProfile::ComplexCoding
                | TaskProfile::LargeRefactor => FailingFilter::Unrated,
            });
        }

        let inputs: Vec<CandidateInput<'_>> = eligible
            .iter()
            .map(|(model, capability)| CandidateInput {
                model_id: &model.id,
                name: &model.name,
                price: &model.price,
                multiplier: self.vendor_multiplier(&capability.vendor, now),
                cache_read: model.cache_read,
                context: model.context,
                capability,
                monthly_limit: self.monthly_limit(&model.id),
            })
            .collect();

        let scored = score_candidates(&inputs, profile);
        self.assemble(scored, profile, now)
    }

    /// Active peak multiplier for one vendor, or `1.0` when it has no window
    /// or is off-peak.
    fn vendor_multiplier(&self, vendor: &str, now: DateTime<Utc>) -> f64 {
        let Some(state) = state_for(vendor, now, self.overlays) else {
            return 1.0;
        };
        if !state.peak {
            return 1.0;
        }
        self.overlays
            .windows
            .providers
            .iter()
            .find(|provider| provider.provider == vendor)
            .map_or(1.0, |provider| provider.multiplier)
    }

    /// Turn ordered scores into the public recommendation shape.
    fn assemble(
        &self,
        scored: Vec<ScoredCandidate>,
        profile: TaskProfile,
        now: DateTime<Utc>,
    ) -> Recommendation {
        let Some((first, rest)) = scored.split_first() else {
            return Recommendation::no_candidate(FailingFilter::Unrated);
        };

        let peak = state_for(&first.vendor, now, self.overlays).is_some_and(|state| state.peak);
        let mut reasons: Vec<Reason> = reason_codes(first, peak, profile)
            .into_iter()
            .map(|code| Reason {
                code,
                text: reason_text(code, profile),
            })
            .collect();
        if first.data_terms {
            reasons.push(Reason {
                code: ReasonCode::DataTerms,
                text: reason_text(ReasonCode::DataTerms, profile),
            });
        }

        Recommendation {
            pick: Some(Pick::from(first)),
            alternatives: rest.iter().take(3).map(Pick::from).collect(),
            reasons,
            no_candidate: None,
        }
    }

    fn is_ignored(&self, model: &ModelEntry) -> bool {
        self.config.ignored_models.iter().any(|entry| {
            entry == &model.id
                || entry
                    .split_once('/')
                    .is_some_and(|(provider, id)| provider == model.provider_id && id == model.id)
        })
    }

    fn capability(&self, model_id: &str) -> Option<&ModelCapability> {
        self.overlays
            .capabilities
            .models
            .iter()
            .find(|capability| capability.id == model_id)
    }

    fn monthly_limit(&self, model_id: &str) -> Option<f64> {
        self.overlays
            .limits
            .models
            .iter()
            .find(|limit| limit.id == model_id)
            .map(|limit| limit.monthly_usd)
    }
}

impl From<&ScoredCandidate> for Pick {
    fn from(candidate: &ScoredCandidate) -> Self {
        Self {
            model_id: candidate.model_id.clone(),
            name: candidate.name.clone(),
            score: candidate.score,
            cost_index: candidate.cost_index,
        }
    }
}

fn reason_text(code: ReasonCode, profile: TaskProfile) -> String {
    match code {
        ReasonCode::OffPeakWindow => "off-peak pricing is active".to_string(),
        ReasonCode::PeakWindow => {
            "peak pricing is active; prices include the overlay multiplier".to_string()
        }
        ReasonCode::StrongFit => format!("strong capability fit for {profile}"),
        ReasonCode::CheapCost => "among the cheapest effective token prices".to_string(),
        ReasonCode::CheapCache => "cheap cache reads keep multi-turn loops economical".to_string(),
        ReasonCode::AllowanceHeadroom => "large monthly allowance headroom".to_string(),
        ReasonCode::ContextFit => "context window meets the 200K target".to_string(),
        ReasonCode::ToolsRequired => "tool-call support is required for this profile".to_string(),
        ReasonCode::VisionRequired => "image input is required for this profile".to_string(),
        ReasonCode::DataTerms => "prompts may be used to train the provider's models".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogSnapshot, ModelEntry, Price};
    use crate::config::Config;
    use crate::overlay::{
        CapabilitiesOverlay, HolidayYear, HolidaysOverlay, LimitsOverlay, ModelLimit,
        PricingWindow, ProviderWindows, Tier, WindowsOverlay,
    };
    use std::collections::BTreeMap;
    fn instant(peak: bool) -> DateTime<Utc> {
        use chrono::TimeZone;

        let (hour, minute) = if peak { (2, 0) } else { (0, 30) };
        Utc.with_ymd_and_hms(2026, 10, 26, hour, minute, 0)
            .single()
            .expect("valid instant")
    }

    fn model(id: &str, vision: bool, tools: bool) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            provider_id: "opencode-go".to_string(),
            name: id.to_string(),
            price: Price {
                input: 0.0,
                output: 0.0,
            },
            cache_read: 0.001,
            context: 1_000_000,
            tool_call: tools,
            attachment: vision,
            reasoning: true,
            release_date: None,
        }
    }

    fn capability(
        id: &str,
        tier: Tier,
        vision: bool,
        tools: bool,
        profile: &str,
        score: f64,
    ) -> ModelCapability {
        ModelCapability {
            id: id.to_string(),
            vendor: "deepseek".to_string(),
            tier,
            vision,
            tools,
            reasoning: true,
            context: 1_000_000,
            data_terms: false,
            profiles: BTreeMap::from([(profile.to_string(), score)]),
            notes: None,
            source: "test".to_string(),
            last_verified: "2026-10-23".to_string(),
        }
    }

    fn overlays(capabilities: Vec<ModelCapability>) -> Overlays {
        let metadata = || {
            (
                "https://example.com/test".to_string(),
                "2026-10-23".to_string(),
            )
        };
        let limits = capabilities
            .iter()
            .map(|capability| ModelLimit {
                id: capability.id.clone(),
                monthly_usd: 10.0,
                source: metadata().0,
                last_verified: metadata().1,
            })
            .collect();

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
                    windows: vec![PricingWindow {
                        start_minutes: 60,
                        end_minutes: 240,
                    }],
                    source: metadata().0,
                    last_verified: metadata().1,
                }],
            },
            holidays: HolidaysOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                years: vec![HolidayYear {
                    year: 2026,
                    holidays: Vec::new(),
                    makeup_weekends: Vec::new(),
                    source: metadata().0,
                    last_verified: metadata().1,
                }],
            },
            limits: LimitsOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                plan: Some("go".to_string()),
                models: limits,
            },
            capabilities: CapabilitiesOverlay {
                schema: 1,
                source: metadata().0,
                last_verified: metadata().1,
                models: capabilities,
            },
        }
    }

    fn snapshot(models: Vec<ModelEntry>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema: 1,
            source: "https://example.com/test".to_string(),
            etag: None,
            fetched_at: "2026-10-23T00:00:00Z".to_string(),
            models,
        }
    }

    fn config() -> Config {
        Config::default()
    }

    fn priced_model(id: &str, input: f64, output: f64) -> ModelEntry {
        ModelEntry {
            price: Price { input, output },
            ..model(id, true, true)
        }
    }

    fn single_model_recommendation(data_terms: bool) -> Recommendation {
        let models = vec![model("a", true, true)];
        let mut capability = capability("a", Tier::Strong, true, true, "simple", 9.0);
        capability.data_terms = data_terms;
        let snapshot = snapshot(models);
        let overlays = overlays(vec![capability]);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        engine.recommend(TaskProfile::SimpleCoding, instant(false))
    }

    #[test]
    fn returns_a_pick_reasons_and_at_most_three_alternatives() {
        let models = vec![
            model("a", true, true),
            model("b", true, true),
            model("c", true, true),
            model("d", true, true),
            model("e", true, true),
        ];
        let capabilities = vec![
            capability("a", Tier::Strong, true, true, "simple", 9.0),
            capability("b", Tier::Strong, true, true, "simple", 8.0),
            capability("c", Tier::Strong, true, true, "simple", 7.0),
            capability("d", Tier::Strong, true, true, "simple", 6.0),
            capability("e", Tier::Strong, true, true, "simple", 5.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::SimpleCoding, instant(false));

        let pick = recommendation.pick.expect("a pick");
        assert_eq!(pick.model_id, "a");
        assert!(!recommendation.reasons.is_empty());
        assert_eq!(recommendation.alternatives.len(), 3);
        assert_eq!(
            recommendation
                .alternatives
                .iter()
                .map(|pick| pick.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "c", "d"]
        );
        assert!(recommendation.no_candidate.is_none());
    }

    #[test]
    fn visual_ui_only_considers_vision_capable_models() {
        let models = vec![model("text-only", false, true), model("vision", true, true)];
        let capabilities = vec![
            capability("text-only", Tier::Frontier, false, true, "visual", 9.5),
            capability("vision", Tier::Strong, true, true, "visual", 7.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::VisualUi, instant(false));

        let pick = recommendation.pick.expect("a pick");
        assert_eq!(pick.model_id, "vision");
        assert!(
            recommendation
                .alternatives
                .iter()
                .all(|pick| pick.model_id != "text-only")
        );
    }

    #[test]
    fn agentic_tools_only_considers_tool_capable_models() {
        let models = vec![model("no-tools", true, false), model("tools", true, true)];
        let capabilities = vec![
            capability("no-tools", Tier::Frontier, true, false, "agentic", 9.5),
            capability("tools", Tier::Strong, true, true, "agentic", 7.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::AgenticTools, instant(false));

        let pick = recommendation.pick.expect("a pick");
        assert_eq!(pick.model_id, "tools");
        assert!(
            recommendation
                .alternatives
                .iter()
                .all(|pick| pick.model_id != "no-tools")
        );
    }

    #[test]
    fn ignored_models_never_appear_in_picks_or_alternatives() {
        let models = vec![
            model("a", true, true),
            model("b", true, true),
            model("c", true, true),
        ];
        let capabilities = vec![
            capability("a", Tier::Frontier, true, true, "simple", 9.5),
            capability("b", Tier::Strong, true, true, "simple", 8.0),
            capability("c", Tier::Standard, true, true, "simple", 7.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = Config {
            ignored_models: vec!["opencode-go/a".to_string()],
            ..Config::default()
        };
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::SimpleCoding, instant(false));

        assert_eq!(recommendation.pick.expect("a pick").model_id, "b");
        assert_eq!(
            recommendation
                .alternatives
                .iter()
                .map(|pick| pick.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["c"]
        );
        assert!(
            recommendation
                .alternatives
                .iter()
                .all(|pick| pick.model_id != "a")
        );
    }

    #[test]
    fn unrated_models_are_excluded() {
        let models = vec![
            model("rated", true, true),
            model("also-rated", true, true),
            model("unrated", true, true),
        ];
        let capabilities = vec![
            capability("rated", Tier::Standard, true, true, "simple", 6.0),
            capability("also-rated", Tier::Standard, true, true, "simple", 5.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::SimpleCoding, instant(false));

        assert_eq!(recommendation.pick.expect("a pick").model_id, "rated");
        assert_eq!(
            recommendation
                .alternatives
                .iter()
                .map(|pick| pick.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["also-rated"]
        );
        assert!(
            recommendation
                .alternatives
                .iter()
                .all(|pick| pick.model_id != "unrated")
        );
    }

    #[test]
    fn no_candidate_names_the_failing_filter() {
        let config = config();

        let empty = snapshot(Vec::new());
        let overlays_empty = overlays(Vec::new());
        let engine = Engine::new(&empty, &overlays_empty, &config);
        assert_eq!(
            engine
                .recommend(TaskProfile::SimpleCoding, instant(false))
                .no_candidate
                .expect("no candidate")
                .filter,
            FailingFilter::EmptyRoster
        );

        let ignored_snapshot = snapshot(vec![model("a", true, true)]);
        let ignored_overlays = overlays(vec![capability(
            "a",
            Tier::Strong,
            true,
            true,
            "simple",
            8.0,
        )]);
        let ignored_config = Config {
            ignored_models: vec!["opencode-go/a".to_string()],
            ..Config::default()
        };
        let engine = Engine::new(&ignored_snapshot, &ignored_overlays, &ignored_config);
        assert_eq!(
            engine
                .recommend(TaskProfile::SimpleCoding, instant(false))
                .no_candidate
                .expect("no candidate")
                .filter,
            FailingFilter::Ignored
        );

        let unrated_snapshot = snapshot(vec![model("a", true, true)]);
        let unrated_overlays = overlays(Vec::new());
        let engine = Engine::new(&unrated_snapshot, &unrated_overlays, &config);
        assert_eq!(
            engine
                .recommend(TaskProfile::SimpleCoding, instant(false))
                .no_candidate
                .expect("no candidate")
                .filter,
            FailingFilter::Unrated
        );

        let text_only = snapshot(vec![model("text-only", false, true)]);
        let text_only_overlays = overlays(vec![capability(
            "text-only",
            Tier::Strong,
            false,
            true,
            "visual",
            8.0,
        )]);
        let engine = Engine::new(&text_only, &text_only_overlays, &config);
        let visual = engine.recommend(TaskProfile::VisualUi, instant(false));
        let no_visual = visual.no_candidate.expect("no candidate");
        assert_eq!(no_visual.filter, FailingFilter::Vision);
        assert!(no_visual.message.contains("image input"), "{no_visual:?}");
        assert!(visual.pick.is_none());
        assert!(visual.alternatives.is_empty());

        let no_tools = snapshot(vec![model("no-tools", true, false)]);
        let no_tools_overlays = overlays(vec![capability(
            "no-tools",
            Tier::Strong,
            true,
            false,
            "agentic",
            8.0,
        )]);
        let engine = Engine::new(&no_tools, &no_tools_overlays, &config);
        let agentic = engine.recommend(TaskProfile::AgenticTools, instant(false));
        let no_agentic = agentic.no_candidate.expect("no candidate");
        assert_eq!(no_agentic.filter, FailingFilter::Tools);
        assert!(no_agentic.message.contains("tool calls"), "{no_agentic:?}");
    }

    #[test]
    fn reasons_pair_closed_set_codes_with_human_readable_text() {
        let models = vec![model("a", true, true)];
        let capabilities = vec![capability("a", Tier::Strong, true, true, "simple", 9.0)];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let recommendation = engine.recommend(TaskProfile::SimpleCoding, instant(true));

        assert_eq!(
            recommendation
                .reasons
                .iter()
                .map(|reason| reason.code)
                .collect::<Vec<_>>(),
            vec![
                ReasonCode::PeakWindow,
                ReasonCode::StrongFit,
                ReasonCode::CheapCost,
                ReasonCode::CheapCache,
                ReasonCode::AllowanceHeadroom,
                ReasonCode::ContextFit,
            ]
        );
        for reason in &recommendation.reasons {
            assert!(!reason.text.is_empty());
            assert_ne!(reason.text, reason.code.as_str());
        }
    }

    #[test]
    fn non_deepseek_models_keep_flat_prices_during_deepseek_peak() {
        let models = vec![
            priced_model("deepseek-model", 1.0, 1.0),
            priced_model("vendor-model", 1.0, 1.0),
        ];
        let mut other_vendor = capability("vendor-model", Tier::Strong, true, true, "simple", 8.0);
        other_vendor.vendor = "anthropic".to_string();
        let capabilities = vec![
            capability("deepseek-model", Tier::Strong, true, true, "simple", 8.0),
            other_vendor,
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let off_peak = engine.recommend(TaskProfile::SimpleCoding, instant(false));
        let peak = engine.recommend(TaskProfile::SimpleCoding, instant(true));

        let off_pick = off_peak.pick.as_ref().expect("off-peak pick");
        let peak_pick = peak.pick.as_ref().expect("peak pick");
        assert_eq!(off_pick.model_id, "deepseek-model");
        assert_eq!(peak_pick.model_id, "vendor-model");
        assert!((peak_pick.cost_index - 1.0).abs() < 1e-12);

        let flat = off_peak
            .alternatives
            .iter()
            .find(|pick| pick.model_id == "vendor-model")
            .expect("flat vendor alternative");
        let doubled = peak
            .alternatives
            .iter()
            .find(|pick| pick.model_id == "deepseek-model")
            .expect("deepseek alternative");
        assert!((flat.cost_index - 1.0).abs() < 1e-12);
        assert!((doubled.cost_index - 2.0).abs() < 1e-12);

        assert!(
            peak.reasons
                .iter()
                .any(|reason| reason.code == ReasonCode::OffPeakWindow)
        );
        assert!(
            peak.reasons
                .iter()
                .all(|reason| reason.code != ReasonCode::PeakWindow)
        );
    }

    #[test]
    fn data_terms_reason_appears_only_when_the_pick_has_data_terms() {
        let flagged = single_model_recommendation(true);
        let reason = flagged
            .reasons
            .iter()
            .find(|reason| reason.code == ReasonCode::DataTerms)
            .expect("DataTerms reason");
        assert!(!reason.text.is_empty());
        assert_ne!(reason.text, reason.code.as_str());
        assert_eq!(
            flagged.reasons.last().map(|reason| reason.code),
            Some(ReasonCode::DataTerms)
        );

        let clean = single_model_recommendation(false);
        assert!(
            clean
                .reasons
                .iter()
                .all(|reason| reason.code != ReasonCode::DataTerms)
        );
    }

    #[test]
    fn identical_inputs_produce_identical_recommendations() {
        let models = vec![
            model("b", true, true),
            model("a", true, true),
            model("c", true, true),
        ];
        let capabilities = vec![
            capability("a", Tier::Strong, true, true, "simple", 8.0),
            capability("b", Tier::Strong, true, true, "simple", 8.0),
            capability("c", Tier::Strong, true, true, "simple", 8.0),
        ];
        let snapshot = snapshot(models);
        let overlays = overlays(capabilities);
        let config = config();
        let engine = Engine::new(&snapshot, &overlays, &config);

        let first = engine.recommend(TaskProfile::SimpleCoding, instant(false));
        let second = engine.recommend(TaskProfile::SimpleCoding, instant(false));

        assert_eq!(first, second);
        assert_eq!(first.pick.expect("a pick").model_id, "a");
    }
}
