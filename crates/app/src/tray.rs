//! Native tray construction and change-gated updates.
//!
//! The tray is created once in `setup`; later refreshes compare the new
//! [`Presentation`] with the last rendered one and skip every write when
//! nothing changed (TRAY-03). If `set_title` fails, the tray falls back to the
//! icon plus tooltip and logs the fallback once (TRAY-04).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::clock;
use crate::menu::{self, Presentation};
use crate::runtime;
use crate::sync_job::SyncGuard;

/// Tray id used by every lookup and update.
pub const TRAY_ID: &str = "main";

/// Last rendered presentation; `None` until the tray is built.
#[derive(Debug, Default)]
pub struct LastPresentation(Mutex<Option<Presentation>>);

/// Set after the first `set_title` failure; later refreshes use the fallback.
static TITLE_FALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Build the tray icon, its title and its menu. Called once from `setup`.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let presentation = current_presentation(app);
    let menu = menu::build(app, &presentation)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .title(presentation.title.clone())
        .tooltip(presentation.title.clone())
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon).icon_as_template(true);
    }
    builder.build(app)?;

    store(app, presentation);
    Ok(())
}

/// Refresh the tray from the current state, skipping unchanged presentations.
pub fn refresh(app: &AppHandle) {
    if let Err(error) = try_refresh(app) {
        eprintln!("cost-guard: failed to refresh the tray: {error}");
    }
}

/// Ask the main thread to refresh the tray.
///
/// Safe from any thread: on the main thread Tauri runs the closure inline.
pub fn request_refresh(app: &AppHandle) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || refresh(&handle)) {
        eprintln!("cost-guard: failed to schedule a tray refresh: {error}");
    }
}

fn try_refresh(app: &AppHandle) -> tauri::Result<()> {
    let presentation = current_presentation(app);
    if load(app).as_ref() == Some(&presentation) {
        return Ok(());
    }

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = menu::build(app, &presentation)?;
    tray.set_menu(Some(menu))?;
    apply_title(app, &tray, &presentation.title)?;

    store(app, presentation);
    Ok(())
}

fn current_presentation(app: &AppHandle) -> Presentation {
    let syncing = app.state::<SyncGuard>().is_running();
    runtime::with_state(app, |state| menu::present(state, syncing, clock::now()))
}

fn apply_title(app: &AppHandle, tray: &TrayIcon<Wry>, title: &str) -> tauri::Result<()> {
    if !TITLE_FALLBACK_ACTIVE.load(Ordering::SeqCst) {
        match tray.set_title(Some(title)) {
            Ok(()) => return Ok(()),
            Err(error) => {
                TITLE_FALLBACK_ACTIVE.store(true, Ordering::SeqCst);
                eprintln!(
                    "cost-guard: tray title update failed ({error}); falling back to icon and tooltip"
                );
            }
        }
    }

    if let Some(icon) = app.default_window_icon().cloned() {
        tray.set_icon(Some(icon))?;
        tray.set_icon_as_template(false)?;
    }
    tray.set_tooltip(Some(title))
}

fn load(app: &AppHandle) -> Option<Presentation> {
    let state = app.state::<LastPresentation>();
    let guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clone()
}

fn store(app: &AppHandle, presentation: Presentation) {
    let state = app.state::<LastPresentation>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(presentation);
}
