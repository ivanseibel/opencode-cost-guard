//! Settings commands: DTO mapping, validation errors and autostart.
//!
//! The webview never sees [`Config`] directly. [`SettingsDto`] is the frozen
//! camelCase contract used by `ui/settings.js`; [`FieldError`] names the input
//! that failed (SET-01, SET-02, PLAT-02).

use std::path::Path;

use cost_guard_core::config::{Config, ConfigError, Notifications, TaskProfile};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt as _;

use crate::runtime;
use crate::tray;

/// Autostart entry name; stable across bundles and launches (PLAT-02).
pub const AUTOSTART_APP_NAME: &str = "dev.ivanseibel.costguard";

/// Settings payload exchanged with the webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub autostart: bool,
    pub notifications: NotificationsDto,
    pub default_profile: TaskProfile,
    pub sync_interval_hours: u32,
    pub ignored_models: Vec<String>,
}

/// Notification toggles in the webview contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationsDto {
    pub peak_enter: bool,
    pub peak_exit: bool,
}

/// Field-tagged save error rendered next to the offending input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl From<&Config> for SettingsDto {
    fn from(config: &Config) -> Self {
        Self {
            autostart: config.autostart,
            notifications: NotificationsDto {
                peak_enter: config.notifications.peak_enter,
                peak_exit: config.notifications.peak_exit,
            },
            default_profile: config.default_profile,
            sync_interval_hours: config.sync_interval_hours,
            ignored_models: config.ignored_models.clone(),
        }
    }
}

impl SettingsDto {
    /// Build a full config, preserving unknown top-level keys (CFG-03).
    pub fn into_config(self, current: &Config) -> Config {
        Config {
            autostart: self.autostart,
            notifications: Notifications {
                peak_enter: self.notifications.peak_enter,
                peak_exit: self.notifications.peak_exit,
            },
            default_profile: self.default_profile,
            sync_interval_hours: self.sync_interval_hours,
            ignored_models: self.ignored_models,
            extra: current.extra.clone(),
        }
    }
}

/// Map a core config error onto the field the webview can highlight.
pub fn map_config_error(error: ConfigError) -> FieldError {
    match error {
        ConfigError::InvalidField { field, message } => FieldError {
            field: field.to_string(),
            message,
        },
        other => FieldError {
            field: "config".to_string(),
            message: other.to_string(),
        },
    }
}

/// Validate and persist settings without touching Tauri.
///
/// Invalid values return [`FieldError`] before any write, so the file on disk
/// is untouched (SET-02).
pub fn save_settings(
    data_dir: &Path,
    current: &Config,
    settings: SettingsDto,
) -> Result<Config, FieldError> {
    let config = settings.into_config(current);
    config.validate().map_err(map_config_error)?;
    config.save(data_dir).map_err(map_config_error)?;
    Ok(config)
}

/// Debug-only fixture: log the settings page render probe from JS.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn settings_probe(report: String) {
    eprintln!("cost-guard: settings render probe: {report}");
}

/// Read the current settings for the webview.
#[tauri::command]
pub fn settings_get(app: AppHandle) -> SettingsDto {
    let settings = runtime::with_state(&app, |state| SettingsDto::from(&state.config));
    eprintln!(
        "cost-guard: settings_get served profile={} intervalHours={} autostart={} ignoredModels={}",
        settings.default_profile,
        settings.sync_interval_hours,
        settings.autostart,
        settings.ignored_models.len()
    );
    settings
}

/// Validate, persist and apply settings, returning a field error on failure.
#[tauri::command]
pub fn settings_set(app: AppHandle, settings: SettingsDto) -> Result<(), FieldError> {
    let (data_dir, current) =
        runtime::with_state(&app, |state| (state.data_dir.clone(), state.config.clone()));
    let autostart_changed = settings.autostart != current.autostart;
    let config = save_settings(&data_dir, &current, settings)?;

    // Persist before applying so a LaunchAgent failure never loses the choice,
    // then mirror the saved file in memory and refresh the menu.
    let autostart_result = if autostart_changed {
        apply_autostart(&app, config.autostart)
    } else {
        Ok(())
    };
    runtime::with_state_mut(&app, |state| {
        state.config = config;
        state.error = None;
    });
    tray::refresh(&app);

    autostart_result
}

/// Enable or disable the LaunchAgent, never inside a dev build (PLAT-02).
fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), FieldError> {
    if tauri::is_dev() {
        return Ok(());
    }

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    result.map_err(|error| FieldError {
        field: "autostart".to_string(),
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    fn full_config() -> Config {
        Config {
            autostart: true,
            notifications: Notifications {
                peak_enter: false,
                peak_exit: true,
            },
            default_profile: TaskProfile::VisualUi,
            sync_interval_hours: 12,
            ignored_models: vec![
                "openai/gpt-5".to_string(),
                "opencode-go/deepseek-v4.1-flash".to_string(),
            ],
            extra: BTreeMap::from([("futureField".to_string(), json!({"nested": 42}))]),
        }
    }

    fn dto(interval: u32, ignored: &[&str]) -> SettingsDto {
        SettingsDto {
            autostart: false,
            notifications: NotificationsDto {
                peak_enter: true,
                peak_exit: false,
            },
            default_profile: TaskProfile::ComplexCoding,
            sync_interval_hours: interval,
            ignored_models: ignored.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn dto_round_trips_every_config_field() {
        let config = full_config();

        let settings = SettingsDto::from(&config);
        assert!(settings.autostart);
        assert!(!settings.notifications.peak_enter);
        assert!(settings.notifications.peak_exit);
        assert_eq!(settings.default_profile, TaskProfile::VisualUi);
        assert_eq!(settings.sync_interval_hours, 12);
        assert_eq!(settings.ignored_models, config.ignored_models);

        assert_eq!(settings.into_config(&config), config);
    }

    #[test]
    fn dto_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(SettingsDto::from(&full_config())).expect("serialize dto");

        assert_eq!(value["autostart"], json!(true));
        assert_eq!(value["notifications"]["peakEnter"], json!(false));
        assert_eq!(value["notifications"]["peakExit"], json!(true));
        assert_eq!(value["defaultProfile"], json!("visual-ui"));
        assert_eq!(value["syncIntervalHours"], json!(12));
        assert_eq!(
            value["ignoredModels"],
            json!(["openai/gpt-5", "opencode-go/deepseek-v4.1-flash"])
        );
    }

    #[test]
    fn unknown_fields_survive_a_settings_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = full_config();

        let saved = save_settings(dir.path(), &current, dto(6, &[])).expect("save settings");
        assert_eq!(saved.extra, current.extra);

        let reloaded = Config::load(dir.path()).expect("reload config");
        assert_eq!(
            reloaded.extra.get("futureField"),
            Some(&json!({"nested": 42}))
        );
        assert_eq!(reloaded.default_profile, TaskProfile::ComplexCoding);
        assert_eq!(reloaded.sync_interval_hours, 6);
        assert!(reloaded.notifications.peak_enter);
        assert!(!reloaded.notifications.peak_exit);
    }

    #[test]
    fn valid_settings_persist_and_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = Config::default();

        let saved =
            save_settings(dir.path(), &current, dto(9, &["openai/gpt-5"])).expect("save settings");

        let reloaded = Config::load(dir.path()).expect("reload config");
        assert_eq!(reloaded, saved);
        assert_eq!(reloaded.sync_interval_hours, 9);
        assert_eq!(reloaded.ignored_models, vec!["openai/gpt-5".to_string()]);
    }

    #[test]
    fn invalid_values_name_the_field_and_leave_the_file_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = Config::default();
        let cases: [(SettingsDto, &str); 2] = [
            (dto(0, &[]), "syncIntervalHours"),
            (dto(6, &["no-slash"]), "ignoredModels"),
        ];

        for (settings, field) in cases {
            let error = save_settings(dir.path(), &current, settings).expect_err("invalid save");

            assert_eq!(error.field, field);
            assert!(!error.message.is_empty());
            assert!(!dir.path().join("config.json").exists());
        }
    }

    #[test]
    fn config_level_errors_map_to_the_config_pseudo_field() {
        let parse_error = serde_json::from_str::<Value>("not json").expect_err("invalid json");
        let mapped = map_config_error(ConfigError::Parse {
            path: "/tmp/config.json".into(),
            source: parse_error,
        });

        assert_eq!(mapped.field, "config");
        assert!(mapped.message.contains("/tmp/config.json"), "{mapped:?}");
    }

    #[test]
    fn field_errors_keep_the_core_field_and_message() {
        let mapped = map_config_error(ConfigError::InvalidField {
            field: "syncIntervalHours",
            message: "must be at least 1".to_string(),
        });

        assert_eq!(
            mapped,
            FieldError {
                field: "syncIntervalHours".to_string(),
                message: "must be at least 1".to_string(),
            }
        );
    }
}
