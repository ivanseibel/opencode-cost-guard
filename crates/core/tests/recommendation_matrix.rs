//! REC-11 acceptance matrix: 5 Task Profiles x peak/off-peak on bundled data.
//!
//! Every expectation below was derived by hand from the scoring formula and
//! the curated Overlays, not from engine output. The derivation, in short:
//!
//! - `cost_index = 0.35*input + 0.65*output`. Effective costs multiply by the
//!   candidate vendor's active peak multiplier and stay flat for vendors with
//!   no Pricing Window. Only DeepSeek doubles at peak; every other vendor holds
//!   its off-peak price.
//! - `cost_score = 1 - (ln(1+c) - ln(1+c_min)) / (ln(1+c_max) - ln(1+c_min))`
//!   over the candidates that survive the hard filters. `beam-free` sets
//!   `c_min = 0` and scores 1.0 for the unfiltered profiles; paid candidates
//!   stay differentiated. Visual/UI filters it out (no vision), leaving Muse
//!   Spark 1.3 as the cheapest eligible model.
//! - `score = 0.55*capability + 0.30*cost + 0.10*allowance + 0.05*context`.
//!   Context saturates at 200K, so every winner earns the full 0.05.
//! - Reason codes follow the T18 policy: capability >= 0.85 is `StrongFit`,
//!   cost >= 0.90 is `CheapCost`, cache-read <= $0.01/Mtok is `CheapCache`,
//!   allowance >= 75% of the largest published limit is `AllowanceHeadroom`,
//!   and a full context score is `ContextFit`. Visual/UI and Agentic/Tools/MCP
//!   add `VisionRequired` and `ToolsRequired`; picks whose capability entry
//!   sets `data_terms` append `DataTerms`. The window code names the picked
//!   model's vendor, so flat vendors stay `OffPeakWindow` during DeepSeek peak.
//!
//! Resulting winners: simple-coding and agentic-tools -> DeepSeek V4.1 Flash
//! (top capability within the profile plus the max 80 USD allowance);
//! complex-coding and large-refactor -> DeepSeek V4.1 Flash off-peak, Muse
//! Spark 1.3 Contributor at peak (DeepSeek's cost score falls below Muse's
//! flat-price score); visual-ui -> Muse Spark 1.3 Contributor in both states
//! (cheapest vision model, flat Meta price).

use chrono::{DateTime, TimeZone, Utc};
use cost_guard_core::catalog::{CatalogSnapshot, parse_snapshot};
use cost_guard_core::config::{Config, TaskProfile};
use cost_guard_core::overlay::Overlays;
use cost_guard_core::recommend::{Engine, ReasonCode, Recommendation};
use cost_guard_core::schedule::state_at;

/// One matrix expectation, paired with the reasoning that produced it.
struct Row {
    profile: TaskProfile,
    peak: bool,
    winner: &'static str,
    codes: &'static [ReasonCode],
    rationale: &'static str,
}

const COMPLEX_PEAK: [ReasonCode; 7] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::StrongFit,
    ReasonCode::CheapCost,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::DataTerms,
];
const REFACTOR_OFF_PEAK: [ReasonCode; 4] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
];
const REFACTOR_PEAK: [ReasonCode; 6] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::CheapCost,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::DataTerms,
];
const SIMPLE_OFF_PEAK: [ReasonCode; 5] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::StrongFit,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
];
const SIMPLE_PEAK: [ReasonCode; 5] = [
    ReasonCode::PeakWindow,
    ReasonCode::StrongFit,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
];
const AGENTIC_OFF_PEAK: [ReasonCode; 6] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::StrongFit,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::ToolsRequired,
];
const AGENTIC_PEAK: [ReasonCode; 6] = [
    ReasonCode::PeakWindow,
    ReasonCode::StrongFit,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::ToolsRequired,
];
const VISUAL_OFF_PEAK: [ReasonCode; 7] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::CheapCost,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::VisionRequired,
    ReasonCode::DataTerms,
];
const VISUAL_PEAK: [ReasonCode; 7] = [
    ReasonCode::OffPeakWindow,
    ReasonCode::CheapCost,
    ReasonCode::CheapCache,
    ReasonCode::AllowanceHeadroom,
    ReasonCode::ContextFit,
    ReasonCode::VisionRequired,
    ReasonCode::DataTerms,
];

const ROWS: &[Row] = &[
    Row {
        profile: TaskProfile::SimpleCoding,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &SIMPLE_OFF_PEAK,
        rationale: "off-peak: 8.6 simple capability and the 80 USD max allowance beat every paid peer; free beam-free sets the cost floor",
    },
    Row {
        profile: TaskProfile::SimpleCoding,
        peak: true,
        winner: "deepseek-v4.1-flash",
        codes: &SIMPLE_PEAK,
        rationale: "peak: uniform 2x preserves normalized cost ratios, so the off-peak winner holds and only the window code changes",
    },
    Row {
        profile: TaskProfile::ComplexCoding,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &SIMPLE_OFF_PEAK,
        rationale: "off-peak: 8.9 complex capability plus a 0.866 log cost score and the 80 USD max allowance beat every paid peer; free beam-free sets c_min = 0",
    },
    Row {
        profile: TaskProfile::ComplexCoding,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &COMPLEX_PEAK,
        rationale: "peak: DeepSeek doubles to 0.885 and its cost score drops to 0.768, so flat Meta at 0.165 (0.944) plus 8.6 capability overtakes DeepSeek's 8.9",
    },
    Row {
        profile: TaskProfile::VisualUi,
        peak: false,
        winner: "muse-spark-1.3-contributor",
        codes: &VISUAL_OFF_PEAK,
        rationale: "off-peak: beam-free lacks vision, so the cheapest vision model (Muse 1.3) owns cost_score 1.0 and its 8.4 visual rating beats other cheap vision models",
    },
    Row {
        profile: TaskProfile::VisualUi,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &VISUAL_PEAK,
        rationale: "peak: Meta has no Pricing Window, so Muse 1.3 keeps its 0.165 flat cost and OffPeakWindow code; DeepSeek's 2x only removes it from the alternatives",
    },
    Row {
        profile: TaskProfile::AgenticTools,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &AGENTIC_OFF_PEAK,
        rationale: "off-peak: 9.2 agentic capability plus the 80 USD max allowance outweigh Kimi K3.1 Pro's 9.6 rating with a 15 USD allowance",
    },
    Row {
        profile: TaskProfile::AgenticTools,
        peak: true,
        winner: "deepseek-v4.1-flash",
        codes: &AGENTIC_PEAK,
        rationale: "peak: all tool-capable candidates are multiplied, so the normalized ranking and the agentic winner are unchanged",
    },
    Row {
        profile: TaskProfile::LargeRefactor,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &REFACTOR_OFF_PEAK,
        rationale: "off-peak: 8.4 refactor capability is below StrongFit but the 0.866 log cost score, 80 USD max allowance and full context carry the score",
    },
    Row {
        profile: TaskProfile::LargeRefactor,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &REFACTOR_PEAK,
        rationale: "peak: DeepSeek's cost score drops to 0.768 while flat Meta holds 0.944; Muse's 8.2 refactor rating and 60 USD allowance now outweigh the 0.2 capability gap",
    },
];

fn data() -> (CatalogSnapshot, Overlays) {
    let snapshot = parse_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/bundled/opencode-go.json"
    )))
    .expect("bundled snapshot parses");
    let overlays = Overlays::bundled().expect("bundled overlays load");
    (snapshot, overlays)
}

/// Fixed instants: Monday 00:30 UTC is off-peak, Monday 02:00 UTC is peak.
fn instant_for(peak: bool, overlays: &Overlays) -> DateTime<Utc> {
    let instant = if peak {
        Utc.with_ymd_and_hms(2026, 10, 26, 2, 0, 0)
            .single()
            .expect("valid peak instant")
    } else {
        Utc.with_ymd_and_hms(2026, 10, 26, 0, 30, 0)
            .single()
            .expect("valid off-peak instant")
    };

    let state = state_at(instant, overlays);
    assert_eq!(state.peak, peak, "fixture instant has the wrong state");
    assert_eq!(state.provider, "deepseek");
    instant
}

fn codes(recommendation: &Recommendation) -> Vec<ReasonCode> {
    recommendation
        .reasons
        .iter()
        .map(|reason| reason.code)
        .collect()
}

#[test]
fn matrix_covers_five_profiles_at_peak_and_off_peak() {
    assert_eq!(ROWS.len(), 10);

    for profile in TaskProfile::ALL {
        let rows = ROWS.iter().filter(|row| row.profile == profile).count();
        assert_eq!(rows, 2, "{profile} must have a peak and an off-peak row");
    }
}

#[test]
fn recommendation_matrix_matches_hand_derived_expectations() {
    let (snapshot, overlays) = data();
    let config = Config::default();
    let engine = Engine::new(&snapshot, &overlays, &config);

    for row in ROWS {
        let recommendation = engine.recommend(row.profile, instant_for(row.peak, &overlays));
        let pick = recommendation
            .pick
            .as_ref()
            .unwrap_or_else(|| panic!("{}", row.rationale));

        assert_eq!(pick.model_id, row.winner, "{}", row.rationale);
        assert_eq!(codes(&recommendation), row.codes, "{}", row.rationale);
        assert!(
            recommendation
                .reasons
                .iter()
                .all(|reason| !reason.text.is_empty()),
            "every reason needs human-readable text: {}",
            row.rationale
        );
    }
}

#[test]
fn peak_scales_only_deepseek_picks() {
    let (snapshot, overlays) = data();
    let config = Config::default();
    let engine = Engine::new(&snapshot, &overlays, &config);

    for profile in [TaskProfile::SimpleCoding, TaskProfile::AgenticTools] {
        let off_peak = engine.recommend(profile, instant_for(false, &overlays));
        let peak = engine.recommend(profile, instant_for(true, &overlays));

        let off_pick = off_peak.pick.expect("off-peak pick");
        let peak_pick = peak.pick.expect("peak pick");
        assert_eq!(off_pick.model_id, peak_pick.model_id);
        assert!(
            (peak_pick.cost_index - 2.0 * off_pick.cost_index).abs() < 1e-9,
            "{profile}: peak cost {} is not twice off-peak {}",
            peak_pick.cost_index,
            off_pick.cost_index
        );
    }

    let off_peak = engine.recommend(TaskProfile::VisualUi, instant_for(false, &overlays));
    let peak = engine.recommend(TaskProfile::VisualUi, instant_for(true, &overlays));
    let off_pick = off_peak.pick.expect("off-peak pick");
    let peak_pick = peak.pick.expect("peak pick");
    assert_eq!(off_pick.model_id, peak_pick.model_id);
    assert!(
        (peak_pick.cost_index - off_pick.cost_index).abs() < 1e-12,
        "flat vendor pick changed cost at peak: {} -> {}",
        off_pick.cost_index,
        peak_pick.cost_index
    );
}
