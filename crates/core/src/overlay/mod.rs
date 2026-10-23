//! Curated Overlay loading and validation.
//!
//! Every Overlay file declares `source` and `last_verified`; entries inherit
//! both values and may override them with their own non-empty values
//! (DATA-01). Window definitions are validated at load time (DATA-03).
//! Roster coverage (DATA-04) and holiday coverage (DATA-05) are warnings
//! produced by [`Overlays::validate`], which takes the active roster and the
//! current year.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::catalog::ModelEntry;

const WINDOWS_FILE: &str = "pricing-windows.toml";
const HOLIDAYS_FILE: &str = "holidays-cn.toml";
const LIMITS_FILE: &str = "limits.toml";
const CAPABILITIES_FILE: &str = "capabilities.toml";

/// All four curated Overlay files, validated and resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlays {
    pub windows: WindowsOverlay,
    pub holidays: HolidaysOverlay,
    pub limits: LimitsOverlay,
    pub capabilities: CapabilitiesOverlay,
}

/// `pricing-windows.toml`: peak windows and multipliers per provider.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowsOverlay {
    pub schema: u32,
    pub source: String,
    pub last_verified: String,
    pub providers: Vec<ProviderWindows>,
}

/// One provider's peak rule.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderWindows {
    pub provider: String,
    pub multiplier: f64,
    pub days: Vec<String>,
    pub windows: Vec<PricingWindow>,
    pub source: String,
    pub last_verified: String,
}

/// A half-open pricing window `[start, end)` in minutes from midnight UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricingWindow {
    pub start_minutes: u32,
    pub end_minutes: u32,
}

/// `holidays-cn.toml`: holiday dates and makeup weekends per year.
#[derive(Debug, Clone, PartialEq)]
pub struct HolidaysOverlay {
    pub schema: u32,
    pub source: String,
    pub last_verified: String,
    pub years: Vec<HolidayYear>,
}

/// One covered calendar year.
#[derive(Debug, Clone, PartialEq)]
pub struct HolidayYear {
    pub year: i32,
    pub holidays: Vec<String>,
    pub makeup_weekends: Vec<String>,
    pub source: String,
    pub last_verified: String,
}

/// `limits.toml`: published monthly USD limits per model.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitsOverlay {
    pub schema: u32,
    pub source: String,
    pub last_verified: String,
    pub plan: Option<String>,
    pub models: Vec<ModelLimit>,
}

/// One model's monthly allowance.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelLimit {
    pub id: String,
    pub monthly_usd: f64,
    pub source: String,
    pub last_verified: String,
}

/// `capabilities.toml`: tier, flags and profile scores per model.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilitiesOverlay {
    pub schema: u32,
    pub source: String,
    pub last_verified: String,
    pub models: Vec<ModelCapability>,
}

/// One model's curated capability rating.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelCapability {
    pub id: String,
    pub tier: Tier,
    pub vision: bool,
    pub tools: bool,
    pub reasoning: bool,
    pub context: u64,
    pub profiles: std::collections::BTreeMap<String, f64>,
    pub notes: Option<String>,
    pub source: String,
    pub last_verified: String,
}

/// Capability tier vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Frontier,
    Strong,
    Standard,
    Light,
}

/// Non-fatal validation findings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Warning {
    #[error("model `{0}` has no capability rating")]
    UnratedModel(String),
    #[error("holiday calendar does not cover {0}")]
    UncoveredHolidayYear(i32),
}

/// Overlay loading errors. Every variant names the file or the entry.
#[derive(Debug, thiserror::Error)]
pub enum OverlayError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {file}: {source}")]
    Toml {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{file}: unsupported schema {schema}")]
    UnsupportedSchema { file: String, schema: u32 },
    #[error("{file}: missing required metadata `{field}`")]
    MissingMetadata { file: String, field: &'static str },
    #[error("{file}: entry `{entry}` has empty `{field}` override")]
    EmptyMetadataOverride {
        file: String,
        entry: String,
        field: &'static str,
    },
    #[error("{file}: entry `{entry}` has malformed window [{window}]: {reason}")]
    MalformedWindow {
        file: String,
        entry: String,
        window: String,
        reason: &'static str,
    },
}

impl Overlays {
    /// Load the four overlays embedded at build time from `data/`.
    pub fn bundled() -> Result<Overlays, OverlayError> {
        Self::from_sources(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/pricing-windows.toml"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/holidays-cn.toml"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/limits.toml"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/capabilities.toml"
            )),
        )
    }

    /// Load the four overlays from a directory.
    ///
    // SPEC_DEVIATION: the design sketch returned warnings from `load_dir`.
    // DATA-04/DATA-05 need the active roster and the current year, which the
    // loader does not receive; `validate` exposes them instead.
    pub fn load_dir(dir: &Path) -> Result<Overlays, OverlayError> {
        let windows = read_file(dir, WINDOWS_FILE)?;
        let holidays = read_file(dir, HOLIDAYS_FILE)?;
        let limits = read_file(dir, LIMITS_FILE)?;
        let capabilities = read_file(dir, CAPABILITIES_FILE)?;
        Self::from_sources(&windows, &holidays, &limits, &capabilities)
    }

    /// Validate curated data against the active roster and calendar year.
    pub fn validate(&self, roster: &[ModelEntry], current_year: i32) -> Vec<Warning> {
        let mut warnings = Vec::new();

        let rated: BTreeSet<&str> = self
            .capabilities
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect();
        for model in roster {
            if !rated.contains(model.id.as_str()) {
                warnings.push(Warning::UnratedModel(model.id.clone()));
            }
        }

        if !self
            .holidays
            .years
            .iter()
            .any(|year| year.year == current_year)
        {
            warnings.push(Warning::UncoveredHolidayYear(current_year));
        }

        warnings
    }

    fn from_sources(
        windows: &str,
        holidays: &str,
        limits: &str,
        capabilities: &str,
    ) -> Result<Overlays, OverlayError> {
        let windows: RawWindows = parse_toml(WINDOWS_FILE, windows)?;
        let holidays: RawHolidays = parse_toml(HOLIDAYS_FILE, holidays)?;
        let limits: RawLimits = parse_toml(LIMITS_FILE, limits)?;
        let capabilities: RawCapabilities = parse_toml(CAPABILITIES_FILE, capabilities)?;

        Ok(Overlays {
            windows: windows.resolve()?,
            holidays: holidays.resolve()?,
            limits: limits.resolve()?,
            capabilities: capabilities.resolve()?,
        })
    }
}

fn read_file(dir: &Path, name: &str) -> Result<String, OverlayError> {
    let path = dir.join(name);
    std::fs::read_to_string(&path).map_err(|source| OverlayError::Io { path, source })
}

fn parse_toml<T: DeserializeOwned>(file: &str, text: &str) -> Result<T, OverlayError> {
    toml::from_str(text).map_err(|source| OverlayError::Toml {
        file: file.to_string(),
        source,
    })
}

fn check_schema(file: &str, schema: u32) -> Result<(), OverlayError> {
    if schema != 1 {
        return Err(OverlayError::UnsupportedSchema {
            file: file.to_string(),
            schema,
        });
    }
    Ok(())
}

fn check_file_metadata(file: &str, source: &str, last_verified: &str) -> Result<(), OverlayError> {
    if source.trim().is_empty() {
        return Err(OverlayError::MissingMetadata {
            file: file.to_string(),
            field: "source",
        });
    }
    if last_verified.trim().is_empty() {
        return Err(OverlayError::MissingMetadata {
            file: file.to_string(),
            field: "last_verified",
        });
    }
    Ok(())
}

fn resolve_metadata(
    file: &str,
    entry: &str,
    file_source: &str,
    file_last_verified: &str,
    source: Option<String>,
    last_verified: Option<String>,
) -> Result<(String, String), OverlayError> {
    let resolve = |value: Option<String>, file_value: &str, field: &'static str| match value {
        Some(value) if value.trim().is_empty() => Err(OverlayError::EmptyMetadataOverride {
            file: file.to_string(),
            entry: entry.to_string(),
            field,
        }),
        Some(value) => Ok(value.trim().to_string()),
        None => Ok(file_value.trim().to_string()),
    };

    Ok((
        resolve(source, file_source, "source")?,
        resolve(last_verified, file_last_verified, "last_verified")?,
    ))
}

fn parse_clock(value: &str) -> Option<u32> {
    let (hour, minute) = value.trim().split_once(':')?;
    if hour.is_empty() || hour.len() > 2 || minute.len() != 2 {
        return None;
    }
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    if minute > 59 {
        return None;
    }
    let minutes = hour * 60 + minute;
    (minutes <= 24 * 60).then_some(minutes)
}

fn parse_window(file: &str, entry: &str, raw: &[String]) -> Result<PricingWindow, OverlayError> {
    let malformed = |reason: &'static str| OverlayError::MalformedWindow {
        file: file.to_string(),
        entry: entry.to_string(),
        window: raw.join(", "),
        reason,
    };

    if raw.len() != 2 {
        return Err(malformed(
            "window must have exactly a start and an end time",
        ));
    }
    let start = parse_clock(&raw[0])
        .ok_or_else(|| malformed("start time is not HH:MM within 00:00-24:00"))?;
    let end = parse_clock(&raw[1])
        .ok_or_else(|| malformed("end time is not HH:MM within 00:00-24:00"))?;
    if end <= start {
        return Err(malformed("end must be after start"));
    }

    Ok(PricingWindow {
        start_minutes: start,
        end_minutes: end,
    })
}

#[derive(Debug, Deserialize)]
struct RawWindows {
    schema: u32,
    source: String,
    last_verified: String,
    #[serde(default)]
    provider: Vec<RawProviderWindows>,
}

#[derive(Debug, Deserialize)]
struct RawProviderWindows {
    provider: String,
    multiplier: f64,
    #[serde(default)]
    days: Vec<String>,
    #[serde(default)]
    windows: Vec<Vec<String>>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    last_verified: Option<String>,
}

impl RawWindows {
    fn resolve(self) -> Result<WindowsOverlay, OverlayError> {
        check_schema(WINDOWS_FILE, self.schema)?;
        check_file_metadata(WINDOWS_FILE, &self.source, &self.last_verified)?;

        let mut providers = Vec::with_capacity(self.provider.len());
        for entry in self.provider {
            let (source, last_verified) = resolve_metadata(
                WINDOWS_FILE,
                &entry.provider,
                &self.source,
                &self.last_verified,
                entry.source,
                entry.last_verified,
            )?;

            let mut windows = Vec::with_capacity(entry.windows.len());
            for raw in &entry.windows {
                windows.push(parse_window(WINDOWS_FILE, &entry.provider, raw)?);
            }

            providers.push(ProviderWindows {
                provider: entry.provider,
                multiplier: entry.multiplier,
                days: entry.days,
                windows,
                source,
                last_verified,
            });
        }

        Ok(WindowsOverlay {
            schema: self.schema,
            source: self.source.trim().to_string(),
            last_verified: self.last_verified.trim().to_string(),
            providers,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawHolidays {
    schema: u32,
    source: String,
    last_verified: String,
    #[serde(default)]
    year: Vec<RawHolidayYear>,
}

#[derive(Debug, Deserialize)]
struct RawHolidayYear {
    year: i32,
    #[serde(default)]
    holidays: Vec<String>,
    #[serde(default)]
    makeup_weekends: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    last_verified: Option<String>,
}

impl RawHolidays {
    fn resolve(self) -> Result<HolidaysOverlay, OverlayError> {
        check_schema(HOLIDAYS_FILE, self.schema)?;
        check_file_metadata(HOLIDAYS_FILE, &self.source, &self.last_verified)?;

        let mut years = Vec::with_capacity(self.year.len());
        for entry in self.year {
            let label = entry.year.to_string();
            let (source, last_verified) = resolve_metadata(
                HOLIDAYS_FILE,
                &label,
                &self.source,
                &self.last_verified,
                entry.source,
                entry.last_verified,
            )?;
            years.push(HolidayYear {
                year: entry.year,
                holidays: entry.holidays,
                makeup_weekends: entry.makeup_weekends,
                source,
                last_verified,
            });
        }

        Ok(HolidaysOverlay {
            schema: self.schema,
            source: self.source.trim().to_string(),
            last_verified: self.last_verified.trim().to_string(),
            years,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawLimits {
    schema: u32,
    source: String,
    last_verified: String,
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    model: Vec<RawLimit>,
}

#[derive(Debug, Deserialize)]
struct RawLimit {
    id: String,
    monthly_usd: f64,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    last_verified: Option<String>,
}

impl RawLimits {
    fn resolve(self) -> Result<LimitsOverlay, OverlayError> {
        check_schema(LIMITS_FILE, self.schema)?;
        check_file_metadata(LIMITS_FILE, &self.source, &self.last_verified)?;

        let mut models = Vec::with_capacity(self.model.len());
        for entry in self.model {
            let (source, last_verified) = resolve_metadata(
                LIMITS_FILE,
                &entry.id,
                &self.source,
                &self.last_verified,
                entry.source,
                entry.last_verified,
            )?;
            models.push(ModelLimit {
                id: entry.id,
                monthly_usd: entry.monthly_usd,
                source,
                last_verified,
            });
        }

        Ok(LimitsOverlay {
            schema: self.schema,
            source: self.source.trim().to_string(),
            last_verified: self.last_verified.trim().to_string(),
            plan: self.plan,
            models,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawCapabilities {
    schema: u32,
    source: String,
    last_verified: String,
    #[serde(default)]
    model: Vec<RawCapability>,
}

#[derive(Debug, Deserialize)]
struct RawCapability {
    id: String,
    tier: Tier,
    vision: bool,
    tools: bool,
    reasoning: bool,
    context: u64,
    #[serde(default)]
    profiles: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    last_verified: Option<String>,
}

impl RawCapabilities {
    fn resolve(self) -> Result<CapabilitiesOverlay, OverlayError> {
        check_schema(CAPABILITIES_FILE, self.schema)?;
        check_file_metadata(CAPABILITIES_FILE, &self.source, &self.last_verified)?;

        let mut models = Vec::with_capacity(self.model.len());
        for entry in self.model {
            let (source, last_verified) = resolve_metadata(
                CAPABILITIES_FILE,
                &entry.id,
                &self.source,
                &self.last_verified,
                entry.source,
                entry.last_verified,
            )?;
            models.push(ModelCapability {
                id: entry.id,
                tier: entry.tier,
                vision: entry.vision,
                tools: entry.tools,
                reasoning: entry.reasoning,
                context: entry.context,
                profiles: entry.profiles,
                notes: entry.notes,
                source,
                last_verified,
            });
        }

        Ok(CapabilitiesOverlay {
            schema: self.schema,
            source: self.source.trim().to_string(),
            last_verified: self.last_verified.trim().to_string(),
            models,
        })
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Tier::Frontier => "frontier",
            Tier::Strong => "strong",
            Tier::Standard => "standard",
            Tier::Light => "light",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Price, parse_snapshot};

    const WINDOWS: &str = r#"
schema = 1
source = "https://example.com/windows"
last_verified = "2026-10-23"

[[provider]]
provider = "deepseek"
multiplier = 2.0
days = ["Mon", "Tue", "Wed", "Thu", "Fri"]
windows = [["01:00", "04:00"], ["06:00", "10:00"]]
"#;

    const HOLIDAYS: &str = r#"
schema = 1
source = "https://example.com/holidays"
last_verified = "2026-10-23"

[[year]]
year = 2026
holidays = ["2026-01-01"]
makeup_weekends = ["2026-01-04"]
"#;

    const LIMITS: &str = r#"
schema = 1
source = "https://example.com/limits"
last_verified = "2026-10-23"
plan = "go"

[[model]]
id = "model-a"
monthly_usd = 60.0
"#;

    const CAPABILITIES: &str = r#"
schema = 1
source = "https://example.com/capabilities"
last_verified = "2026-10-23"

[[model]]
id = "model-a"
tier = "strong"
vision = true
tools = true
reasoning = true
context = 128000
profiles = { simple = 8.0, complex = 7.0 }
"#;

    fn load(
        windows: &str,
        holidays: &str,
        limits: &str,
        capabilities: &str,
    ) -> Result<Overlays, OverlayError> {
        Overlays::from_sources(windows, holidays, limits, capabilities)
    }

    fn default_overlays() -> Overlays {
        load(WINDOWS, HOLIDAYS, LIMITS, CAPABILITIES).expect("fixture loads")
    }

    fn model(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            provider_id: "opencode-go".to_string(),
            name: id.to_string(),
            price: Price {
                input: 0.1,
                output: 0.2,
            },
            cache_read: 0.01,
            context: 128_000,
            tool_call: true,
            attachment: false,
            reasoning: true,
            release_date: None,
        }
    }

    #[test]
    fn bundled_overlays_load() {
        let overlays = Overlays::bundled().expect("bundled overlays load");

        let provider = &overlays.windows.providers[0];
        assert_eq!(provider.provider, "deepseek");
        assert_eq!(provider.multiplier, 2.0);
        assert_eq!(
            provider.windows,
            vec![
                PricingWindow {
                    start_minutes: 60,
                    end_minutes: 240,
                },
                PricingWindow {
                    start_minutes: 360,
                    end_minutes: 600,
                },
            ]
        );
        assert_eq!(overlays.holidays.years[0].year, 2026);
        assert!(overlays.limits.models.len() >= 28);
        assert!(overlays.capabilities.models.len() >= 29);
        assert_eq!(
            overlays.capabilities.models[0].source,
            overlays.capabilities.source
        );
    }

    #[test]
    fn load_dir_loads_repo_data() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let overlays = Overlays::load_dir(&dir).expect("repo data loads");
        let bundled = Overlays::bundled().expect("bundled overlays load");

        assert_eq!(overlays, bundled);
    }

    #[test]
    fn bundled_data_covers_roster_and_current_year() {
        let overlays = Overlays::bundled().expect("bundled overlays load");
        let snapshot = parse_snapshot(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/bundled/opencode-go.json"
        )))
        .expect("bundled snapshot parses");

        let warnings = overlays.validate(&snapshot.models, 2026);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn missing_file_metadata_names_file() {
        let windows = WINDOWS.replace("source = \"https://example.com/windows\"\n", "");
        let error =
            load(&windows, HOLIDAYS, LIMITS, CAPABILITIES).expect_err("missing source fails");

        let message = error.to_string();
        assert!(message.contains("pricing-windows.toml"), "{message}");
        assert!(message.contains("source"), "{message}");
    }

    #[test]
    fn empty_entry_metadata_override_names_entry() {
        let windows = WINDOWS.replace(
            "windows = [[\"01:00\", \"04:00\"], [\"06:00\", \"10:00\"]]",
            "windows = [[\"01:00\", \"04:00\"], [\"06:00\", \"10:00\"]]\nlast_verified = \"  \"",
        );
        let error =
            load(&windows, HOLIDAYS, LIMITS, CAPABILITIES).expect_err("empty override fails");

        let message = error.to_string();
        assert!(message.contains("pricing-windows.toml"), "{message}");
        assert!(message.contains("deepseek"), "{message}");
        assert!(message.contains("last_verified"), "{message}");
    }

    #[test]
    fn malformed_windows_fail_naming_entry() {
        let reversed = WINDOWS.replace("[\"01:00\", \"04:00\"]", "[\"04:00\", \"01:00\"]");
        let error =
            load(&reversed, HOLIDAYS, LIMITS, CAPABILITIES).expect_err("end before start fails");
        let message = error.to_string();
        assert!(message.contains("deepseek"), "{message}");
        assert!(message.contains("end must be after start"), "{message}");

        let outside = WINDOWS.replace("[\"01:00\", \"04:00\"]", "[\"01:00\", \"24:30\"]");
        let error =
            load(&outside, HOLIDAYS, LIMITS, CAPABILITIES).expect_err("time outside the day fails");
        let message = error.to_string();
        assert!(message.contains("deepseek"), "{message}");
        assert!(message.contains("00:00-24:00"), "{message}");
    }

    #[test]
    fn per_entry_metadata_override_is_accepted() {
        let capabilities = CAPABILITIES.replace(
            "context = 128000",
            "context = 128000\nsource = \"https://example.com/model-a\"\nlast_verified = \"2026-10-22\"",
        );
        let overlays =
            load(WINDOWS, HOLIDAYS, LIMITS, &capabilities).expect("per-entry override loads");

        let entry = &overlays.capabilities.models[0];
        assert_eq!(entry.source, "https://example.com/model-a");
        assert_eq!(entry.last_verified, "2026-10-22");
        assert_eq!(
            overlays.capabilities.source,
            "https://example.com/capabilities"
        );
    }

    #[test]
    fn warns_about_unrated_models() {
        let overlays = default_overlays();
        let warnings = overlays.validate(&[model("model-a"), model("model-b")], 2026);

        assert_eq!(warnings, vec![Warning::UnratedModel("model-b".to_string())]);
        assert_eq!(
            warnings[0].to_string(),
            "model `model-b` has no capability rating"
        );
    }

    #[test]
    fn warns_about_uncovered_holiday_year() {
        let overlays = default_overlays();

        assert_eq!(
            overlays.validate(&[model("model-a")], 2027),
            vec![Warning::UncoveredHolidayYear(2027)]
        );
        assert!(overlays.validate(&[model("model-a")], 2026).is_empty());
    }
}
