//! User config with defaults, validation and forward-compatible round-trip.
//!
//! The file is `<dir>/config.json`. Unknown top-level keys are preserved on
//! save through [`Config::extra`] (CFG-03). Loads never write: an invalid value
//! returns a typed error naming the field and leaves the file untouched
//! (CFG-02).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Config file name inside the caller-provided directory.
pub const CONFIG_FILE: &str = "config.json";

/// The five Task Profiles the engine recommends for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskProfile {
    SimpleCoding,
    ComplexCoding,
    VisualUi,
    AgenticTools,
    LargeRefactor,
}

impl TaskProfile {
    /// Every Task Profile, in canonical order.
    pub const ALL: [TaskProfile; 5] = [
        TaskProfile::SimpleCoding,
        TaskProfile::ComplexCoding,
        TaskProfile::VisualUi,
        TaskProfile::AgenticTools,
        TaskProfile::LargeRefactor,
    ];

    /// Stable identifier used in config and reports.
    pub fn as_str(self) -> &'static str {
        match self {
            TaskProfile::SimpleCoding => "simple-coding",
            TaskProfile::ComplexCoding => "complex-coding",
            TaskProfile::VisualUi => "visual-ui",
            TaskProfile::AgenticTools => "agentic-tools",
            TaskProfile::LargeRefactor => "large-refactor",
        }
    }
}

impl std::fmt::Display for TaskProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an unknown Task Profile id.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown task profile `{0}`; expected one of simple-coding, complex-coding, visual-ui, agentic-tools, large-refactor"
)]
pub struct UnknownTaskProfile(pub String);

impl FromStr for TaskProfile {
    type Err = UnknownTaskProfile;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        TaskProfile::ALL
            .into_iter()
            .find(|profile| profile.as_str() == value)
            .ok_or_else(|| UnknownTaskProfile(value.to_string()))
    }
}

/// Notification toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Notifications {
    pub peak_enter: bool,
    pub peak_exit: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            peak_enter: true,
            peak_exit: true,
        }
    }
}

/// Validated user config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub autostart: bool,
    pub notifications: Notifications,
    pub default_profile: TaskProfile,
    pub sync_interval_hours: u32,
    pub ignored_models: Vec<String>,
    /// Unknown top-level keys, preserved across save (CFG-03).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: false,
            notifications: Notifications::default(),
            default_profile: TaskProfile::SimpleCoding,
            sync_interval_hours: 6,
            ignored_models: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

/// Config load/save errors. Every variant names the file or the field.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("invalid config field `{field}`: {message}")]
    InvalidField {
        field: &'static str,
        message: String,
    },
}

impl Config {
    /// Load `<dir>/config.json`, or defaults when the file does not exist.
    pub fn load(dir: &Path) -> Result<Config, ConfigError> {
        let path = dir.join(CONFIG_FILE);

        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Config::default());
            }
            Err(source) => return Err(ConfigError::Io { path, source }),
        };

        let raw: RawConfig = serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.clone(),
            source,
        })?;

        let default_profile =
            raw.default_profile
                .parse()
                .map_err(|error: UnknownTaskProfile| ConfigError::InvalidField {
                    field: "defaultProfile",
                    message: error.to_string(),
                })?;

        let config = Config {
            autostart: raw.autostart,
            notifications: raw.notifications,
            default_profile,
            sync_interval_hours: raw.sync_interval_hours,
            ignored_models: raw.ignored_models,
            extra: raw.extra,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate field ranges and formats (CFG-04).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.sync_interval_hours < 1 {
            return Err(ConfigError::InvalidField {
                field: "syncIntervalHours",
                message: "must be at least 1".to_string(),
            });
        }

        for model in &self.ignored_models {
            if !is_provider_model(model) {
                return Err(ConfigError::InvalidField {
                    field: "ignoredModels",
                    message: format!("`{model}` is not a provider/model id"),
                });
            }
        }

        Ok(())
    }

    /// Validate, then write `<dir>/config.json` atomically via temp + rename.
    pub fn save(&self, dir: &Path) -> Result<(), ConfigError> {
        self.validate()?;

        let path = dir.join(CONFIG_FILE);
        let temp = dir.join(format!("{CONFIG_FILE}.tmp"));
        let json = serde_json::to_string_pretty(self)?;

        std::fs::write(&temp, json).map_err(|source| ConfigError::Io {
            path: temp.clone(),
            source,
        })?;
        std::fs::rename(&temp, &path).map_err(|source| ConfigError::Io { path, source })?;

        Ok(())
    }
}

fn is_provider_model(value: &str) -> bool {
    let Some((provider, model)) = value.split_once('/') else {
        return false;
    };
    !provider.is_empty()
        && !model.is_empty()
        && !model.contains('/')
        && !value.chars().any(char::is_whitespace)
}

/// Deserialization mirror used to report `defaultProfile` errors by field name.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawConfig {
    autostart: bool,
    notifications: Notifications,
    default_profile: String,
    sync_interval_hours: u32,
    ignored_models: Vec<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            notifications: Notifications::default(),
            default_profile: TaskProfile::SimpleCoding.as_str().to_string(),
            sync_interval_hours: 6,
            ignored_models: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_config(dir: &Path, contents: &str) {
        std::fs::write(dir.join(CONFIG_FILE), contents).expect("write config fixture");
    }

    #[test]
    fn missing_file_loads_defaults_without_writing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = Config::load(dir.path()).expect("defaults load");

        assert!(!config.autostart);
        assert!(config.notifications.peak_enter);
        assert!(config.notifications.peak_exit);
        assert_eq!(config.default_profile, TaskProfile::SimpleCoding);
        assert_eq!(config.sync_interval_hours, 6);
        assert!(config.ignored_models.is_empty());
        assert!(config.extra.is_empty());
        assert_eq!(config, Config::default());
        assert!(!dir.path().join(CONFIG_FILE).exists());
    }

    #[test]
    fn invalid_values_name_the_field_and_leave_the_file_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");

        let cases = [
            (r#"{"syncIntervalHours": 0}"#, "syncIntervalHours"),
            (r#"{"defaultProfile": "turbo"}"#, "defaultProfile"),
            (r#"{"ignoredModels": ["no-slash"]}"#, "ignoredModels"),
        ];

        for (contents, field) in cases {
            write_config(dir.path(), contents);
            let before = std::fs::read(dir.path().join(CONFIG_FILE)).expect("read fixture");

            let error = Config::load(dir.path()).expect_err("invalid config fails");
            assert!(
                error.to_string().contains(field),
                "error does not name {field}: {error}"
            );

            let after = std::fs::read(dir.path().join(CONFIG_FILE)).expect("read fixture");
            assert_eq!(before, after, "load must not modify the config file");
        }
    }

    #[test]
    fn save_is_atomic_and_writes_camel_case_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        Config::default().save(dir.path()).expect("save defaults");

        let path = dir.path().join(CONFIG_FILE);
        assert!(path.is_file());
        assert!(!dir.path().join(format!("{CONFIG_FILE}.tmp")).exists());

        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read saved config"))
                .expect("saved config is JSON");
        assert_eq!(value["autostart"], json!(false));
        assert_eq!(value["notifications"]["peakEnter"], json!(true));
        assert_eq!(value["notifications"]["peakExit"], json!(true));
        assert_eq!(value["defaultProfile"], json!("simple-coding"));
        assert_eq!(value["syncIntervalHours"], json!(6));
        assert_eq!(value["ignoredModels"], json!([]));
    }

    #[test]
    fn unknown_fields_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_config(
            dir.path(),
            r#"{
                "autostart": true,
                "futureField": { "nested": 42 },
                "unknownFlag": true
            }"#,
        );

        let config = Config::load(dir.path()).expect("config with unknown fields loads");
        assert!(config.autostart);
        assert_eq!(
            config.extra.get("futureField"),
            Some(&json!({"nested": 42}))
        );
        assert_eq!(config.extra.get("unknownFlag"), Some(&json!(true)));

        config.save(dir.path()).expect("save round-trip");
        let saved: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(CONFIG_FILE)).expect("read saved config"),
        )
        .expect("saved config is JSON");

        assert_eq!(saved["futureField"], json!({"nested": 42}));
        assert_eq!(saved["unknownFlag"], json!(true));
        assert_eq!(Config::load(dir.path()).expect("reload"), config);
    }

    #[test]
    fn validation_covers_interval_profiles_and_ignored_ids() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.sync_interval_hours = 1;
        assert!(config.validate().is_ok());
        config.sync_interval_hours = 0;
        let error = config.validate().expect_err("zero interval fails");
        assert!(error.to_string().contains("syncIntervalHours"), "{error}");

        config.sync_interval_hours = 6;
        config.ignored_models = vec![
            "openai/gpt-5".to_string(),
            "opencode-go/deepseek-v4.1-flash".to_string(),
        ];
        assert!(config.validate().is_ok());

        for invalid in ["no-slash", "a/b/c", "/model", "provider/", "a /b", "a/ b"] {
            config.ignored_models = vec![invalid.to_string()];
            let error = config.validate().expect_err("invalid ignored id fails");
            assert!(error.to_string().contains("ignoredModels"), "{error}");
            assert!(error.to_string().contains(invalid), "{error}");
        }

        for profile in TaskProfile::ALL {
            let parsed: TaskProfile = profile.as_str().parse().expect("canonical id parses");
            assert_eq!(parsed, profile);
        }
        assert!("not-a-profile".parse::<TaskProfile>().is_err());
    }

    #[test]
    fn save_rejects_invalid_config_without_writing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = Config {
            sync_interval_hours: 0,
            ..Config::default()
        };

        let error = config.save(dir.path()).expect_err("invalid save fails");

        assert!(error.to_string().contains("syncIntervalHours"), "{error}");
        assert!(!dir.path().join(CONFIG_FILE).exists());
        assert!(!dir.path().join(format!("{CONFIG_FILE}.tmp")).exists());
    }
}
