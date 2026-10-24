//! Native tray menu construction and the strings it renders.
//!
//! [`present`] is a pure projection of [`AppState`] into [`Presentation`], and
//! [`build`] turns that into Tauri menu items. Rebuilding only happens when the
//! presentation changes, which keeps the open menu stable (TRAY-03, MENU-01..06).

use chrono::{DateTime, Datelike, Local, Utc};
use cost_guard_core::config::TaskProfile;
use cost_guard_core::recommend::Engine;
use cost_guard_core::schedule::state_for;
use tauri::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::app_state::AppState;
use crate::ui_model;
use crate::{runtime, sync_job, tray};

/// Label of the settings webview; must match `capabilities/default.json`.
pub const SETTINGS_LABEL: &str = "settings";

/// Vendor whose Pricing Window drives the tray and price block.
pub const DEEPSEEK_VENDOR: &str = "deepseek";
/// Model whose prices the menu shows (MENU-01).
pub const PRICE_MODEL_ID: &str = "deepseek-v4.1-flash";
const PRICE_MODEL_NAME: &str = "DeepSeek V4.1 Flash";

/// Menu item id: dismiss the Catalog Change badge.
pub const ID_ACKNOWLEDGE: &str = "acknowledge";
/// Menu item id: run Catalog Sync.
pub const ID_SYNC: &str = "sync";
/// Menu item id: show the settings window.
pub const ID_SETTINGS: &str = "settings";
/// Menu item id: quit the app.
pub const ID_QUIT: &str = "quit";
/// Prefix of the five Task Profile check items.
pub const PROFILE_PREFIX: &str = "profile:";

/// Everything the tray title and menu render for one state snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    pub title: String,
    pub state: String,
    pub transition: String,
    pub peak: bool,
    pub prices: Vec<String>,
    pub profile: TaskProfile,
    pub recommendation: Vec<String>,
    pub alternatives: Vec<String>,
    pub source: String,
    pub warnings: Vec<String>,
    pub badge: bool,
    pub syncing: bool,
    pub error: Option<String>,
}

/// Project the current state into the exact strings shown to the user.
pub fn present(state: &AppState, syncing: bool, now: DateTime<Utc>) -> Presentation {
    let window = state_for(DEEPSEEK_VENDOR, now, &state.overlays);
    let peak = window.as_ref().is_some_and(|window| window.peak);
    let next = window
        .as_ref()
        .map_or(DateTime::<Utc>::MAX_UTC, |window| window.next_transition);
    let local = if next == DateTime::<Utc>::MAX_UTC {
        next.fixed_offset()
    } else {
        next.with_timezone(&Local).fixed_offset()
    };
    let multiplier = state
        .overlays
        .windows
        .providers
        .iter()
        .find(|provider| provider.provider == DEEPSEEK_VENDOR)
        .map_or(1.0, |provider| provider.multiplier);
    let prices = state
        .snapshot
        .models
        .iter()
        .find(|model| model.id == PRICE_MODEL_ID)
        .map_or_else(
            || vec![format!("{PRICE_MODEL_NAME} is not in the current catalog")],
            |model| ui_model::price_lines(model, peak, multiplier),
        );
    let recommendation = Engine::new(&state.snapshot, &state.overlays, &state.config)
        .recommend(state.config.default_profile, now);

    Presentation {
        title: ui_model::tray_title(peak, state.badge),
        state: ui_model::tray_title(peak, false),
        transition: ui_model::next_transition_line(next, local),
        peak,
        prices,
        profile: state.config.default_profile,
        recommendation: ui_model::recommendation_lines(&recommendation),
        alternatives: ui_model::alternative_lines(&recommendation),
        source: ui_model::source_line(state.source, state.last_sync_at),
        warnings: ui_model::overlay_warning_lines(
            &state.overlays.validate(&state.snapshot.models, now.year()),
        ),
        badge: state.badge,
        syncing,
        error: state.error.clone(),
    }
}

/// Human-readable label for a Task Profile check item.
pub fn profile_label(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::SimpleCoding => "Simple Coding",
        TaskProfile::ComplexCoding => "Complex Coding",
        TaskProfile::VisualUi => "Visual UI",
        TaskProfile::AgenticTools => "Agentic Tools",
        TaskProfile::LargeRefactor => "Large Refactor",
    }
}

/// Check and visibility state of the menu items built from a presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionState {
    /// One entry per Task Profile, in `TaskProfile::ALL` order (MENU-02).
    pub profile_checks: Vec<(TaskProfile, bool)>,
    /// Sync Catalog is enabled unless a sync is in flight (MENU-05).
    pub sync_enabled: bool,
    /// Acknowledge changes is visible only while the badge is set (SYNCA-02).
    pub acknowledge_visible: bool,
}

/// Project profile checks and action-item state from a presentation.
pub fn action_state(presentation: &Presentation) -> ActionState {
    ActionState {
        profile_checks: TaskProfile::ALL
            .into_iter()
            .map(|profile| (profile, profile == presentation.profile))
            .collect(),
        sync_enabled: !presentation.syncing,
        acknowledge_visible: presentation.badge,
    }
}

/// Build the complete native menu for one presentation.
pub fn build(app: &AppHandle, presentation: &Presentation) -> tauri::Result<Menu<Wry>> {
    #[cfg(debug_assertions)]
    eprintln!(
        "cost-guard: menu built state={} | transition={} | peak={} | prices={:?} | profile={} | recommendation={:?} | alternatives={:?} | source={} | warnings={:?} | syncing={} | badge={}",
        presentation.state,
        presentation.transition,
        presentation.peak,
        presentation.prices,
        presentation.profile,
        presentation.recommendation,
        presentation.alternatives,
        presentation.source,
        presentation.warnings,
        presentation.syncing,
        presentation.badge
    );

    let state_item = label(app, "view:state", &presentation.state)?;
    let transition_item = label(app, "view:transition", &presentation.transition)?;
    let source_item = label(app, "view:source", &presentation.source)?;
    let warning_items = presentation
        .warnings
        .iter()
        .enumerate()
        .map(|(index, text)| label(app, &format!("view:warning:{index}"), text))
        .collect::<tauri::Result<Vec<_>>>()?;

    let top_separator = PredefinedMenuItem::separator(app)?;
    let price_separator = PredefinedMenuItem::separator(app)?;
    let profile_separator = PredefinedMenuItem::separator(app)?;
    let recommendation_separator = PredefinedMenuItem::separator(app)?;
    let action_separator = PredefinedMenuItem::separator(app)?;

    let price_items = presentation
        .prices
        .iter()
        .enumerate()
        .map(|(index, text)| label(app, &format!("view:price:{index}"), text))
        .collect::<tauri::Result<Vec<_>>>()?;

    let actions = action_state(presentation);
    let profile_items = actions
        .profile_checks
        .iter()
        .map(|(profile, checked)| {
            CheckMenuItem::with_id(
                app,
                format!("{PROFILE_PREFIX}{}", profile.as_str()),
                profile_label(*profile),
                true,
                *checked,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    let recommendation_items = presentation
        .recommendation
        .iter()
        .enumerate()
        .map(|(index, text)| label(app, &format!("view:recommendation:{index}"), text))
        .collect::<tauri::Result<Vec<_>>>()?;

    let alternative_items = if presentation.alternatives.is_empty() {
        vec![label(app, "view:alternative:none", "No alternatives")?]
    } else {
        presentation
            .alternatives
            .iter()
            .enumerate()
            .map(|(index, text)| label(app, &format!("view:alternative:{index}"), text))
            .collect::<tauri::Result<Vec<_>>>()?
    };
    let alternative_refs: Vec<&dyn IsMenuItem<Wry>> = alternative_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<Wry>)
        .collect();
    let alternatives = Submenu::with_items(app, "Alternatives", true, &alternative_refs)?;

    let acknowledge_item = if actions.acknowledge_visible {
        Some(MenuItem::with_id(
            app,
            ID_ACKNOWLEDGE,
            "Acknowledge changes",
            true,
            None::<&str>,
        )?)
    } else {
        None
    };
    let error_item = presentation
        .error
        .as_ref()
        .map(|error| label(app, "view:error", &format!("Error: {error}")))
        .transpose()?;
    let sync_item = MenuItem::with_id(
        app,
        ID_SYNC,
        "Sync Catalog",
        actions.sync_enabled,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, ID_SETTINGS, "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, ID_QUIT, "Quit Cost Guard", true, None::<&str>)?;

    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&state_item, &transition_item];
    items.extend(
        warning_items
            .iter()
            .map(|item| item as &dyn IsMenuItem<Wry>),
    );
    items.push(&top_separator);
    items.extend(price_items.iter().map(|item| item as &dyn IsMenuItem<Wry>));
    items.push(&price_separator);
    items.extend(
        profile_items
            .iter()
            .map(|item| item as &dyn IsMenuItem<Wry>),
    );
    items.push(&profile_separator);
    items.extend(
        recommendation_items
            .iter()
            .map(|item| item as &dyn IsMenuItem<Wry>),
    );
    items.push(&alternatives);
    items.push(&recommendation_separator);
    items.push(&source_item);
    if let Some(item) = &acknowledge_item {
        items.push(item);
    }
    if let Some(item) = &error_item {
        items.push(item);
    }
    items.push(&sync_item);
    items.push(&settings_item);
    items.push(&action_separator);
    items.push(&quit_item);

    Menu::with_items(app, &items)
}

/// Disabled menu item used for read-only labels.
fn label(app: &AppHandle, id: &str, text: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, text, false, None::<&str>)
}

/// Handle a tray menu selection (MENU-02, MENU-05, SYNCA-02).
pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();

    if let Some(value) = id.strip_prefix(PROFILE_PREFIX) {
        switch_profile(app, value);
        return;
    }

    match id {
        ID_ACKNOWLEDGE => {
            runtime::with_state_mut(app, |state| {
                state.acknowledge_changes();
            });
            tray::refresh(app);
        }
        ID_SYNC => sync_job::spawn(app),
        ID_SETTINGS => show_settings(app),
        ID_QUIT => app.exit(0),
        _ => {}
    }
}

fn switch_profile(app: &AppHandle, value: &str) {
    let profile = match value.parse::<TaskProfile>() {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("cost-guard: ignoring unknown profile menu id `{value}`: {error}");
            return;
        }
    };

    if let Err(error) = runtime::with_state_mut(app, |state| state.set_profile(profile)) {
        eprintln!("cost-guard: failed to persist profile: {error}");
        runtime::with_state_mut(app, |state| state.error = Some(error.to_string()));
    }
    tray::refresh(app);
}

#[cfg(debug_assertions)]
const RENDER_PROBE: &str = r#"
setTimeout(() => {
  const ids = ["autostart", "peak-enter", "peak-exit", "default-profile", "sync-interval", "ignored-models"];
  const fields = ids.map((id) => {
    const el = document.getElementById(id);
    let value = el
      ? (el.type === "checkbox" ? String(el.checked) : el.value)
      : null;
    return [id, !!el, el ? el.offsetWidth : 0, el ? el.offsetHeight : 0, value];
  });
  const h1 = document.querySelector("h1");
  const report = {
    readyState: document.readyState,
    visibility: document.visibilityState,
    hasFocus: document.hasFocus(),
    viewport: [window.innerWidth, window.innerHeight, window.devicePixelRatio],
    h1: h1 ? h1.textContent : null,
    fields,
    text: document.body.innerText.replace(/\s+/g, " ").slice(0, 300),
  };
  window.__TAURI__.core.invoke("settings_probe", { report: JSON.stringify(report) })
    .catch((error) => console.error("probe failed", error));
}, 800);
"#;

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        if let Err(error) = window.show() {
            eprintln!("cost-guard: failed to show settings: {error}");
        }
        let _ = window.set_focus();
        return;
    }

    match WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
        .title("Cost Guard Settings")
        .inner_size(520.0, 420.0)
        .resizable(false)
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
            #[cfg(debug_assertions)]
            if std::env::var_os("COST_GUARD_PROBE_SETTINGS").is_some() {
                let _ = window.eval(RENDER_PROBE);
            }
        }
        Err(error) => eprintln!("cost-guard: failed to open settings: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn instant() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 26, 2, 0, 0)
            .single()
            .expect("valid instant")
    }

    #[test]
    fn bundled_overlays_add_no_warning_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = AppState::load(dir.path().to_path_buf());

        let presentation = present(&state, false, instant());

        assert!(presentation.warnings.is_empty(), "{presentation:?}");
    }

    #[test]
    fn holiday_coverage_gap_is_surfaced_in_the_presentation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = AppState::load(dir.path().to_path_buf());
        state
            .overlays
            .holidays
            .years
            .retain(|year| year.year != 2026);

        let presentation = present(&state, false, instant());

        assert_eq!(
            presentation.warnings,
            vec!["Warning: holiday calendar does not cover 2026"]
        );
        assert_eq!(
            presentation.state, "🔴 DS 2×",
            "peak math still works without the holiday calendar"
        );
    }

    #[test]
    fn action_state_exposes_five_profile_checks_with_the_active_one_checked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = AppState::load(dir.path().to_path_buf());
        state.config.default_profile = TaskProfile::LargeRefactor;

        let actions = action_state(&present(&state, false, instant()));

        assert_eq!(actions.profile_checks.len(), TaskProfile::ALL.len());
        let checked: Vec<TaskProfile> = actions
            .profile_checks
            .iter()
            .filter(|(_, is_checked)| *is_checked)
            .map(|(profile, _)| *profile)
            .collect();
        assert_eq!(checked, vec![TaskProfile::LargeRefactor]);
        assert_eq!(
            actions
                .profile_checks
                .iter()
                .map(|(profile, _)| profile_label(*profile))
                .collect::<Vec<_>>(),
            vec![
                "Simple Coding",
                "Complex Coding",
                "Visual UI",
                "Agentic Tools",
                "Large Refactor",
            ]
        );
    }

    #[test]
    fn sync_item_is_disabled_while_a_sync_is_in_flight() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = AppState::load(dir.path().to_path_buf());

        assert!(action_state(&present(&state, false, instant())).sync_enabled);
        assert!(!action_state(&present(&state, true, instant())).sync_enabled);
    }

    #[test]
    fn acknowledge_item_is_visible_only_with_the_badge() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = AppState::load(dir.path().to_path_buf());

        assert!(!action_state(&present(&state, false, instant())).acknowledge_visible);

        state.badge = true;
        assert!(action_state(&present(&state, false, instant())).acknowledge_visible);
    }
}
