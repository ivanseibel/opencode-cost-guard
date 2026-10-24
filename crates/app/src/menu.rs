//! Native tray menu construction and the strings it renders.
//!
//! [`present`] is a pure projection of [`AppState`] into [`Presentation`], and
//! [`build`] turns that into Tauri menu items. Rebuilding only happens when the
//! presentation changes, which keeps the open menu stable (TRAY-03, MENU-01..06).

use chrono::{DateTime, Local, Utc};
use cost_guard_core::config::TaskProfile;
use cost_guard_core::recommend::Engine;
use cost_guard_core::schedule::state_for;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

use crate::app_state::AppState;
use crate::ui_model;

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
    pub prices: Vec<String>,
    pub profile: TaskProfile,
    pub recommendation: Vec<String>,
    pub alternatives: Vec<String>,
    pub source: String,
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
        prices,
        profile: state.config.default_profile,
        recommendation: ui_model::recommendation_lines(&recommendation),
        alternatives: ui_model::alternative_lines(&recommendation),
        source: ui_model::source_line(state.source, state.last_sync_at),
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

/// Build the complete native menu for one presentation.
pub fn build(app: &AppHandle, presentation: &Presentation) -> tauri::Result<Menu<Wry>> {
    let state_item = label(app, "view:state", &presentation.state)?;
    let transition_item = label(app, "view:transition", &presentation.transition)?;
    let source_item = label(app, "view:source", &presentation.source)?;

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

    let profile_items = TaskProfile::ALL
        .into_iter()
        .map(|profile| {
            CheckMenuItem::with_id(
                app,
                format!("{PROFILE_PREFIX}{}", profile.as_str()),
                profile_label(profile),
                true,
                profile == presentation.profile,
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

    let acknowledge_item = if presentation.badge {
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
        !presentation.syncing,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, ID_SETTINGS, "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, ID_QUIT, "Quit Cost Guard", true, None::<&str>)?;

    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&state_item, &transition_item, &top_separator];
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
