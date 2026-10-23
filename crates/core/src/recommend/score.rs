//! Ranking math for the Recommendation Engine and the closed reason-code set.
//!
//! The weights come from `.specs/features/core-v01/design.md`; the per-vendor
//! peak rule and log cost scale come from `.specs/features/scoring-fix/spec.md`
//! (AD-006):
//!
//! ```text
//! cost_index   = 0.35 * effective_input + 0.65 * effective_output
//! effective    = off_peak * candidate_multiplier
//! candidate_multiplier = vendor's active peak multiplier, else 1.0
//! cost_score   = 1 - (ln(1+c) - ln(1+c_min)) / (ln(1+c_max) - ln(1+c_min))
//! score        = 0.55*capability + 0.30*cost + 0.10*allowance + 0.05*context
//! ```
//!
//! Edge policies keep the formula total:
//!
//! - Equal cost indices, one eligible candidate, or a degenerate log range all
//!   score `cost_score = 1.0` instead of dividing by zero.
//! - A zero-cost candidate scores exactly `1.0`. Paid candidates stay strictly
//!   inside `(0, 1)` and differentiated; COST-01's formula floors at exactly
//!   0.0 for the range maximum, so positive scores are clamped at one
//!   `f64::EPSILON` (numerically indistinguishable, ranking unchanged).
//! - An unlisted monthly allowance scores the neutral `0.5`.
//!
//! Ordering is deterministic: score descending, then lower effective
//! `cost_index`, then model id (REC-08). Reason thresholds are feature-local
//! policy, exposed so reports and tests can explain a pick.

use crate::catalog::Price;
use crate::config::TaskProfile;
use crate::overlay::{ModelCapability, Tier};

/// Weight of the input price in the blended cost index.
pub const INPUT_COST_WEIGHT: f64 = 0.35;
/// Weight of the output price in the blended cost index.
pub const OUTPUT_COST_WEIGHT: f64 = 0.65;

/// Weight of the capability component in the final score.
pub const CAPABILITY_WEIGHT: f64 = 0.55;
/// Weight of the cost component in the final score.
pub const COST_WEIGHT: f64 = 0.30;
/// Weight of the allowance component in the final score.
pub const ALLOWANCE_WEIGHT: f64 = 0.10;
/// Weight of the context component in the final score.
pub const CONTEXT_WEIGHT: f64 = 0.05;

/// Context length that saturates the context score.
pub const CONTEXT_TARGET: u64 = 200_000;
/// Allowance score used when a model has no published monthly amount.
pub const UNLISTED_ALLOWANCE_SCORE: f64 = 0.5;

/// Capability score that earns [`ReasonCode::StrongFit`].
pub const STRONG_FIT_THRESHOLD: f64 = 0.85;
/// Cost score that earns [`ReasonCode::CheapCost`].
pub const CHEAP_COST_THRESHOLD: f64 = 0.90;
/// Cache-read price (USD per Mtok) at or below which `CheapCache` applies.
pub const CHEAP_CACHE_THRESHOLD: f64 = 0.01;
/// Allowance score that earns [`ReasonCode::AllowanceHeadroom`].
pub const ALLOWANCE_HEADROOM_THRESHOLD: f64 = 0.75;

/// Price after applying a candidate's active multiplier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectivePrice {
    pub input: f64,
    pub output: f64,
}

/// Apply `multiplier` to `price`; callers pass `1.0` for flat vendors.
pub fn effective_price(price: &Price, multiplier: f64) -> EffectivePrice {
    EffectivePrice {
        input: price.input * multiplier,
        output: price.output * multiplier,
    }
}

/// Blend effective input and output prices into one comparable index.
pub fn cost_index(price: &EffectivePrice) -> f64 {
    INPUT_COST_WEIGHT * price.input + OUTPUT_COST_WEIGHT * price.output
}

/// Capability fallback for models without a profile score.
pub fn tier_base(tier: Tier) -> f64 {
    match tier {
        Tier::Frontier => 0.95,
        Tier::Strong => 0.80,
        Tier::Standard => 0.60,
        Tier::Light => 0.40,
    }
}

/// Capability Overlay profile key for a Task Profile.
pub fn profile_key(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::SimpleCoding => "simple",
        TaskProfile::ComplexCoding => "complex",
        TaskProfile::VisualUi => "visual",
        TaskProfile::AgenticTools => "agentic",
        TaskProfile::LargeRefactor => "refactor",
    }
}

/// Profile score from the Overlay, or the tier base when absent.
pub fn capability_score(capability: &ModelCapability, profile: TaskProfile) -> f64 {
    capability
        .profiles
        .get(profile_key(profile))
        .map_or_else(|| tier_base(capability.tier), |score| score / 10.0)
}

/// Inputs needed to score one eligible candidate.
#[derive(Debug, Clone, Copy)]
pub struct CandidateInput<'a> {
    pub model_id: &'a str,
    pub name: &'a str,
    pub price: &'a Price,
    /// Multiplier resolved for this candidate's vendor; `1.0` when flat.
    pub multiplier: f64,
    pub cache_read: f64,
    pub context: u64,
    pub capability: &'a ModelCapability,
    pub monthly_limit: Option<f64>,
}

/// A candidate with every score component materialized.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCandidate {
    pub model_id: String,
    pub name: String,
    pub vendor: String,
    pub effective_price: EffectivePrice,
    pub cache_read: f64,
    pub context: u64,
    pub data_terms: bool,
    pub cost_index: f64,
    pub capability_score: f64,
    pub cost_score: f64,
    pub allowance_score: f64,
    pub context_score: f64,
    pub score: f64,
}

/// Score and order the eligible candidates for `profile` (REC-04, REC-08).
pub fn score_candidates(
    candidates: &[CandidateInput<'_>],
    profile: TaskProfile,
) -> Vec<ScoredCandidate> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let max_allowance = candidates
        .iter()
        .filter_map(|candidate| candidate.monthly_limit)
        .fold(0.0_f64, f64::max);
    let (min_log_cost, max_log_cost) = candidates
        .iter()
        .map(|candidate| log_cost_index(candidate))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), log| {
            (min.min(log), max.max(log))
        });
    let log_range = max_log_cost - min_log_cost;

    let mut scored: Vec<ScoredCandidate> = candidates
        .iter()
        .map(|candidate| {
            let effective = effective_price(candidate.price, candidate.multiplier);
            let index = cost_index(&effective);
            let cost_score = if log_range <= f64::EPSILON {
                1.0
            } else {
                let normalized = 1.0 - (log_cost_index(candidate) - min_log_cost) / log_range;
                // COST-01 lands the range maximum at exactly 0.0. COST-02 keeps
                // paid candidates strictly inside (0, 1), so floor the result
                // at one epsilon: numerically indistinguishable, order intact.
                if index > 0.0 {
                    normalized.max(f64::EPSILON)
                } else {
                    normalized
                }
            };
            let allowance_score = match candidate.monthly_limit {
                Some(limit) if max_allowance > 0.0 => limit / max_allowance,
                _ => UNLISTED_ALLOWANCE_SCORE,
            };
            let context_score = (candidate.context as f64 / CONTEXT_TARGET as f64).min(1.0);
            let capability = capability_score(candidate.capability, profile);

            ScoredCandidate {
                model_id: candidate.model_id.to_string(),
                name: candidate.name.to_string(),
                vendor: candidate.capability.vendor.clone(),
                effective_price: effective,
                cache_read: candidate.cache_read,
                context: candidate.context,
                data_terms: candidate.capability.data_terms,
                cost_index: index,
                capability_score: capability,
                cost_score,
                allowance_score,
                context_score,
                score: CAPABILITY_WEIGHT * capability
                    + COST_WEIGHT * cost_score
                    + ALLOWANCE_WEIGHT * allowance_score
                    + CONTEXT_WEIGHT * context_score,
            }
        })
        .collect();

    sort_candidates(&mut scored);
    scored
}

/// `ln(1 + c)`, the scale used by the min-max cost normalization (COST-01).
fn log_cost_index(candidate: &CandidateInput<'_>) -> f64 {
    let index = cost_index(&effective_price(candidate.price, candidate.multiplier));
    (1.0 + index).ln()
}

/// Sort in place: score desc, lower cost index, then model id (REC-08).
fn sort_candidates(candidates: &mut [ScoredCandidate]) {
    candidates.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.cost_index.total_cmp(&b.cost_index))
            .then_with(|| a.model_id.cmp(&b.model_id))
    });
}

/// The closed reason-code set (REC-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasonCode {
    OffPeakWindow,
    PeakWindow,
    StrongFit,
    CheapCost,
    CheapCache,
    AllowanceHeadroom,
    ContextFit,
    ToolsRequired,
    VisionRequired,
    DataTerms,
}

impl ReasonCode {
    /// Stable identifier used in reports.
    pub fn as_str(self) -> &'static str {
        match self {
            ReasonCode::OffPeakWindow => "off-peak-window",
            ReasonCode::PeakWindow => "peak-window",
            ReasonCode::StrongFit => "strong-fit",
            ReasonCode::CheapCost => "cheap-cost",
            ReasonCode::CheapCache => "cheap-cache",
            ReasonCode::AllowanceHeadroom => "allowance-headroom",
            ReasonCode::ContextFit => "context-fit",
            ReasonCode::ToolsRequired => "tools-required",
            ReasonCode::VisionRequired => "vision-required",
            ReasonCode::DataTerms => "data-terms",
        }
    }
}

impl std::fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Codes explaining a pick, in fixed order (REC-10).
///
/// `peak` describes the picked model's vendor window. `DataTerms` is appended
/// by the engine because it depends on the pick's capability entry.
pub fn reason_codes(
    candidate: &ScoredCandidate,
    peak: bool,
    profile: TaskProfile,
) -> Vec<ReasonCode> {
    let mut codes = Vec::new();

    codes.push(if peak {
        ReasonCode::PeakWindow
    } else {
        ReasonCode::OffPeakWindow
    });
    if candidate.capability_score >= STRONG_FIT_THRESHOLD {
        codes.push(ReasonCode::StrongFit);
    }
    if candidate.cost_score >= CHEAP_COST_THRESHOLD {
        codes.push(ReasonCode::CheapCost);
    }
    if candidate.cache_read <= CHEAP_CACHE_THRESHOLD {
        codes.push(ReasonCode::CheapCache);
    }
    if candidate.allowance_score >= ALLOWANCE_HEADROOM_THRESHOLD {
        codes.push(ReasonCode::AllowanceHeadroom);
    }
    if candidate.context_score >= 1.0 {
        codes.push(ReasonCode::ContextFit);
    }
    if profile == TaskProfile::AgenticTools {
        codes.push(ReasonCode::ToolsRequired);
    }
    if profile == TaskProfile::VisualUi {
        codes.push(ReasonCode::VisionRequired);
    }

    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn capability(id: &str, tier: Tier, profiles: &[(&str, f64)]) -> ModelCapability {
        ModelCapability {
            id: id.to_string(),
            vendor: "deepseek".to_string(),
            tier,
            vision: true,
            tools: true,
            reasoning: true,
            context: CONTEXT_TARGET,
            data_terms: false,
            profiles: profiles
                .iter()
                .map(|(key, score)| ((*key).to_string(), *score))
                .collect::<BTreeMap<_, _>>(),
            notes: None,
            source: "test".to_string(),
            last_verified: "2026-10-23".to_string(),
        }
    }

    fn scored(id: &str, score: f64, cost_index: f64) -> ScoredCandidate {
        ScoredCandidate {
            model_id: id.to_string(),
            name: id.to_string(),
            vendor: "deepseek".to_string(),
            effective_price: EffectivePrice {
                input: cost_index,
                output: cost_index,
            },
            cache_read: 0.1,
            context: CONTEXT_TARGET,
            data_terms: false,
            cost_index,
            capability_score: 0.5,
            cost_score: 0.5,
            allowance_score: 0.5,
            context_score: 0.5,
            score,
        }
    }

    #[test]
    fn candidate_multiplier_scales_the_effective_price() {
        let price = Price {
            input: 0.15,
            output: 0.6,
        };

        let flat = effective_price(&price, 1.0);
        assert_eq!(flat.input, 0.15);
        assert_eq!(flat.output, 0.6);

        let peak = effective_price(&price, 2.0);
        assert!((peak.input - 0.3).abs() < 1e-12, "{peak:?}");
        assert!((peak.output - 1.2).abs() < 1e-12, "{peak:?}");
    }

    #[test]
    fn cost_index_weights_input_at_35_and_output_at_65_percent() {
        let index = cost_index(&EffectivePrice {
            input: 2.0,
            output: 4.0,
        });

        assert!((index - (0.35 * 2.0 + 0.65 * 4.0)).abs() < 1e-12);
        assert!((index - 3.3).abs() < 1e-12, "{index}");
    }

    #[test]
    fn each_candidate_uses_its_own_multiplier() {
        let price = Price {
            input: 1.0,
            output: 1.0,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [
            CandidateInput {
                model_id: "flat",
                name: "Flat",
                price: &price,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "peak",
                name: "Peak",
                price: &price,
                multiplier: 2.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
        ];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);
        let flat = scored.iter().find(|c| c.model_id == "flat").expect("flat");
        let peak = scored.iter().find(|c| c.model_id == "peak").expect("peak");

        assert!((flat.effective_price.input - 1.0).abs() < 1e-12);
        assert!((peak.effective_price.input - 2.0).abs() < 1e-12);
        assert!((flat.cost_index - 1.0).abs() < 1e-12);
        assert!((peak.cost_index - 2.0).abs() < 1e-12);
        assert!(flat.cost_score > peak.cost_score);
    }

    #[test]
    fn log_normalization_spreads_paid_candidates_between_zero_and_one() {
        let cheap = Price {
            input: 1.0,
            output: 1.0,
        };
        let mid = Price {
            input: 2.0,
            output: 2.0,
        };
        let pricey = Price {
            input: 4.0,
            output: 4.0,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [
            CandidateInput {
                model_id: "a",
                name: "A",
                price: &cheap,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "b",
                name: "B",
                price: &mid,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "c",
                name: "C",
                price: &pricey,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
        ];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);
        let a = scored.iter().find(|c| c.model_id == "a").expect("a");
        let b = scored.iter().find(|c| c.model_id == "b").expect("b");
        let c = scored.iter().find(|c| c.model_id == "c").expect("c");

        assert_eq!(a.cost_score, 1.0);
        let expected_b = 1.0 - (3.0_f64.ln() - 2.0_f64.ln()) / (5.0_f64.ln() - 2.0_f64.ln());
        assert!((b.cost_score - expected_b).abs() < 1e-12, "{b:?}");
        assert!(b.cost_score > 0.0 && b.cost_score < 1.0);
        assert_eq!(c.cost_score, f64::EPSILON);
    }

    #[test]
    fn zero_cost_candidate_scores_one_and_paid_candidates_stay_differentiated() {
        let free = Price {
            input: 0.0,
            output: 0.0,
        };
        let mid = Price {
            input: 3.0,
            output: 3.0,
        };
        let pricey = Price {
            input: 3.5,
            output: 3.5,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [
            CandidateInput {
                model_id: "free",
                name: "Free",
                price: &free,
                multiplier: 1.0,
                cache_read: 0.0,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: None,
            },
            CandidateInput {
                model_id: "mid",
                name: "Mid",
                price: &mid,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "pricey",
                name: "Pricey",
                price: &pricey,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
        ];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);
        let free = scored.iter().find(|c| c.model_id == "free").expect("free");
        let mid = scored.iter().find(|c| c.model_id == "mid").expect("mid");
        let pricey = scored
            .iter()
            .find(|c| c.model_id == "pricey")
            .expect("pricey");

        assert_eq!(free.cost_score, 1.0);
        let expected_mid = 1.0 - 4.0_f64.ln() / 4.5_f64.ln();
        assert!((mid.cost_score - expected_mid).abs() < 1e-12, "{mid:?}");
        assert!(mid.cost_score > 0.0 && mid.cost_score < 1.0);
        // COST-01 lands the range maximum at 0.0; COST-02 keeps every paid
        // candidate strictly above 0.0, so the boundary is floored at epsilon.
        assert_eq!(pricey.cost_score, f64::EPSILON);
        assert!(pricey.cost_score > 0.0 && pricey.cost_score < 1.0);
        assert_ne!(mid.cost_score, pricey.cost_score);
    }

    #[test]
    fn equal_costs_score_full_cost() {
        let price = Price {
            input: 3.5,
            output: 3.5,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [
            CandidateInput {
                model_id: "a",
                name: "A",
                price: &price,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "b",
                name: "B",
                price: &price,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
        ];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);

        assert!(scored.iter().all(|candidate| candidate.cost_score == 1.0));
    }

    #[test]
    fn single_candidate_scores_full_cost() {
        let price = Price {
            input: 3.5,
            output: 3.5,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [CandidateInput {
            model_id: "only",
            name: "Only",
            price: &price,
            multiplier: 1.0,
            cache_read: 0.001,
            context: CONTEXT_TARGET,
            capability: &capability,
            monthly_limit: Some(10.0),
        }];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);

        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].cost_score, 1.0);
    }

    #[test]
    fn score_composes_capability_cost_allowance_and_context() {
        let cheap = Price {
            input: 1.0,
            output: 1.0,
        };
        let pricey = Price {
            input: 2.0,
            output: 2.0,
        };
        let capability = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let candidates = [
            CandidateInput {
                model_id: "a",
                name: "A",
                price: &cheap,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET,
                capability: &capability,
                monthly_limit: Some(10.0),
            },
            CandidateInput {
                model_id: "b",
                name: "B",
                price: &pricey,
                multiplier: 1.0,
                cache_read: 0.001,
                context: CONTEXT_TARGET / 2,
                capability: &capability,
                monthly_limit: Some(5.0),
            },
        ];

        let scored = score_candidates(&candidates, TaskProfile::SimpleCoding);
        let a = scored.iter().find(|c| c.model_id == "a").expect("a");
        let b = scored.iter().find(|c| c.model_id == "b").expect("b");

        assert!((a.score - (0.55 * 0.8 + 0.30 * 1.0 + 0.10 * 1.0 + 0.05 * 1.0)).abs() < 1e-12);
        assert!(
            (b.score - (0.55 * 0.8 + 0.30 * f64::EPSILON + 0.10 * 0.5 + 0.05 * 0.5)).abs() < 1e-12
        );
    }

    #[test]
    fn equal_scores_break_ties_toward_the_lower_cost_index() {
        let mut candidates = vec![scored("pricey", 0.5, 2.0), scored("cheap", 0.5, 1.0)];

        sort_candidates(&mut candidates);

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["cheap", "pricey"]
        );
    }

    #[test]
    fn equal_scores_and_prices_order_by_model_id() {
        let mut candidates = vec![scored("b-model", 0.5, 1.0), scored("a-model", 0.5, 1.0)];

        sort_candidates(&mut candidates);

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a-model", "b-model"]
        );
    }

    #[test]
    fn reason_codes_derive_from_scoring_inputs() {
        let candidate = scored("a", 0.5, 1.0);
        let candidate = ScoredCandidate {
            capability_score: 0.90,
            cost_score: 0.95,
            cache_read: 0.005,
            allowance_score: 0.80,
            context_score: 1.0,
            ..candidate
        };

        assert_eq!(
            reason_codes(&candidate, false, TaskProfile::SimpleCoding),
            vec![
                ReasonCode::OffPeakWindow,
                ReasonCode::StrongFit,
                ReasonCode::CheapCost,
                ReasonCode::CheapCache,
                ReasonCode::AllowanceHeadroom,
                ReasonCode::ContextFit,
            ]
        );
        assert_eq!(
            reason_codes(&candidate, true, TaskProfile::SimpleCoding)[0],
            ReasonCode::PeakWindow
        );
    }

    #[test]
    fn weak_and_expensive_candidates_earn_only_the_window_code() {
        let candidate = ScoredCandidate {
            capability_score: 0.5,
            cost_score: 0.2,
            cache_read: 0.5,
            allowance_score: 0.2,
            context_score: 0.5,
            ..scored("a", 0.5, 1.0)
        };

        assert_eq!(
            reason_codes(&candidate, false, TaskProfile::SimpleCoding),
            vec![ReasonCode::OffPeakWindow]
        );
    }

    #[test]
    fn profile_requirements_add_their_reason_code() {
        let candidate = scored("a", 0.5, 1.0);

        let visual = reason_codes(&candidate, false, TaskProfile::VisualUi);
        assert_eq!(visual.last(), Some(&ReasonCode::VisionRequired));

        let agentic = reason_codes(&candidate, false, TaskProfile::AgenticTools);
        assert_eq!(agentic.last(), Some(&ReasonCode::ToolsRequired));
    }

    #[test]
    fn data_terms_code_has_a_stable_identifier() {
        assert_eq!(ReasonCode::DataTerms.as_str(), "data-terms");
        assert_eq!(ReasonCode::DataTerms.to_string(), "data-terms");
    }

    #[test]
    fn profile_scores_fall_back_to_the_tier_base() {
        let rated = capability("a", Tier::Strong, &[("simple", 8.0)]);
        let unrated = capability("b", Tier::Frontier, &[]);

        assert!((capability_score(&rated, TaskProfile::SimpleCoding) - 0.8).abs() < 1e-12);
        assert!((capability_score(&unrated, TaskProfile::SimpleCoding) - 0.95).abs() < 1e-12);
        assert!((tier_base(Tier::Standard) - 0.60).abs() < 1e-12);
        assert_eq!(profile_key(TaskProfile::LargeRefactor), "refactor");
    }
}
