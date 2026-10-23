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
use cost_guard_core::catalog::{CatalogSnapshot, ModelEntry, Price, parse_snapshot};
use cost_guard_core::config::{Config, TaskProfile};
use cost_guard_core::overlay::{
    CapabilitiesOverlay, HolidayYear, HolidaysOverlay, LimitsOverlay, ModelCapability, Overlays,
    PricingWindow, ProviderWindows, Tier, WindowsOverlay,
};
use cost_guard_core::recommend::{Engine, ReasonCode, Recommendation};
use cost_guard_core::schedule::state_at;
use std::collections::BTreeMap;

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
        rationale: "off-peak: 0.55*0.86 + 0.30*0.866 + 0.10*1.00 + 0.05*1.00 = 0.8828; runner Muse 1.3 scores 0.8372",
    },
    Row {
        profile: TaskProfile::SimpleCoding,
        peak: true,
        winner: "deepseek-v4.1-flash",
        codes: &SIMPLE_PEAK,
        rationale: "peak: DeepSeek 0.4425 -> 0.8850 drops cost 0.866 -> 0.768, score 0.8534; Muse stays flat at 0.8372",
    },
    Row {
        profile: TaskProfile::ComplexCoding,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &SIMPLE_OFF_PEAK,
        rationale: "off-peak: 0.55*0.89 + 0.30*0.866 + 0.10*1.00 + 0.05*1.00 = 0.8993; runner Muse 1.3 scores 0.8812",
    },
    Row {
        profile: TaskProfile::ComplexCoding,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &COMPLEX_PEAK,
        rationale: "peak: DeepSeek falls to 0.8699; Muse's flat 0.165 gives cost 0.944 and 0.55*0.86 + 0.30*0.944 + 0.10*0.75 + 0.05*1.00 = 0.8812",
    },
    Row {
        profile: TaskProfile::VisualUi,
        peak: false,
        winner: "muse-spark-1.3-contributor",
        codes: &VISUAL_OFF_PEAK,
        rationale: "off-peak: beam-free lacks vision, so Muse 1.3 sets c_min = 0.165 and cost 1.0; 0.55*0.84 + 0.30*1.00 + 0.10*0.75 + 0.05*1.00 = 0.8870, runner mimo-v2.5 0.8861",
    },
    Row {
        profile: TaskProfile::VisualUi,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &VISUAL_PEAK,
        rationale: "peak: Meta has no window, so the same 0.8870 stands; DeepSeek's 2x only drops it from the alternatives",
    },
    Row {
        profile: TaskProfile::AgenticTools,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &AGENTIC_OFF_PEAK,
        rationale: "off-peak: 0.55*0.92 + 0.30*0.866 + 0.10*1.00 + 0.05*1.00 = 0.9158; runner Muse 1.3 scores 0.8702",
    },
    Row {
        profile: TaskProfile::AgenticTools,
        peak: true,
        winner: "deepseek-v4.1-flash",
        codes: &AGENTIC_PEAK,
        rationale: "peak: cost 0.866 -> 0.768 gives 0.55*0.92 + 0.30*0.768 + 0.10*1.00 + 0.05*1.00 = 0.8864; Muse stays 0.8702",
    },
    Row {
        profile: TaskProfile::LargeRefactor,
        peak: false,
        winner: "deepseek-v4.1-flash",
        codes: &REFACTOR_OFF_PEAK,
        rationale: "off-peak: 0.55*0.84 + 0.30*0.866 + 0.10*1.00 + 0.05*1.00 = 0.8718; runner Muse 1.3 scores 0.8592",
    },
    Row {
        profile: TaskProfile::LargeRefactor,
        peak: true,
        winner: "muse-spark-1.3-contributor",
        codes: &REFACTOR_PEAK,
        rationale: "peak: Muse's flat cost 0.944 gives 0.55*0.82 + 0.30*0.944 + 0.10*0.75 + 0.05*1.00 = 0.8592 vs DeepSeek 0.8424 after its 2x",
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

fn synthetic_capability(id: &str, vendor: &str, simple: f64) -> ModelCapability {
    ModelCapability {
        id: id.to_string(),
        vendor: vendor.to_string(),
        tier: Tier::Frontier,
        vision: true,
        tools: true,
        reasoning: true,
        context: 1_000_000,
        data_terms: false,
        profiles: BTreeMap::from([("simple".to_string(), simple)]),
        notes: None,
        source: "https://example.com/synthetic".to_string(),
        last_verified: "2026-10-23".to_string(),
    }
}

fn synthetic_model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        provider_id: "opencode-go".to_string(),
        name: id.to_string(),
        price: Price {
            input: 1.0,
            output: 1.0,
        },
        cache_read: 0.01,
        context: 1_000_000,
        tool_call: true,
        attachment: true,
        reasoning: true,
        release_date: None,
    }
}

fn synthetic_snapshot() -> CatalogSnapshot {
    CatalogSnapshot {
        schema: 1,
        source: "https://example.com/synthetic".to_string(),
        etag: None,
        fetched_at: "2026-10-23T00:00:00Z".to_string(),
        models: vec![synthetic_model("vendor-a"), synthetic_model("rival-b")],
    }
}

fn synthetic_overlays() -> Overlays {
    let source = || "https://example.com/synthetic".to_string();
    let last_verified = || "2026-10-23".to_string();

    Overlays {
        windows: WindowsOverlay {
            schema: 1,
            source: source(),
            last_verified: last_verified(),
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
                source: source(),
                last_verified: last_verified(),
            }],
        },
        holidays: HolidaysOverlay {
            schema: 1,
            source: source(),
            last_verified: last_verified(),
            years: vec![HolidayYear {
                year: 2026,
                holidays: Vec::new(),
                makeup_weekends: Vec::new(),
                source: source(),
                last_verified: last_verified(),
            }],
        },
        limits: LimitsOverlay {
            schema: 1,
            source: source(),
            last_verified: last_verified(),
            plan: None,
            models: Vec::new(),
        },
        capabilities: CapabilitiesOverlay {
            schema: 1,
            source: source(),
            last_verified: last_verified(),
            models: vec![
                synthetic_capability("vendor-a", "deepseek", 9.0),
                synthetic_capability("rival-b", "anthropic", 7.0),
            ],
        },
    }
}

/// MATRIX-02: a rival wins at peak once only the vendor's price doubles.
///
/// Both models cost 1.0 off-peak, so the log range is degenerate and both
/// score cost 1.0; vendor-a wins on 0.90 vs 0.70 capability. At peak only
/// vendor-a doubles (2.0), so rival-b takes cost 1.0 and scores 0.785 while
/// vendor-a's capability edge cannot cover the 0.30 cost loss.
#[test]
fn synthetic_peak_multiplier_flips_the_winner() {
    let snapshot = synthetic_snapshot();
    let overlays = synthetic_overlays();
    let config = Config::default();
    let engine = Engine::new(&snapshot, &overlays, &config);

    let off_peak = engine.recommend(TaskProfile::SimpleCoding, instant_for(false, &overlays));
    let peak = engine.recommend(TaskProfile::SimpleCoding, instant_for(true, &overlays));

    let off_pick = off_peak.pick.as_ref().expect("off-peak pick");
    assert_eq!(off_pick.model_id, "vendor-a");
    assert!((off_pick.cost_index - 1.0).abs() < 1e-12);

    let peak_pick = peak.pick.as_ref().expect("peak pick");
    assert_eq!(peak_pick.model_id, "rival-b");
    assert!((peak_pick.cost_index - 1.0).abs() < 1e-12);

    let repriced = peak
        .alternatives
        .iter()
        .find(|pick| pick.model_id == "vendor-a")
        .expect("vendor-a alternative");
    assert!(
        (repriced.cost_index - 2.0).abs() < 1e-12,
        "only vendor-a should double at peak, got {}",
        repriced.cost_index
    );
}
