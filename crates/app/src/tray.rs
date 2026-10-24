//! Native tray construction and change-gated updates.
//!
//! The tray is created once in `setup`; later refreshes compare the new
//! [`Presentation`] with the last rendered one and skip every write when
//! nothing changed. Menu rebuilds follow the broader presentation delta, while
//! `set_title` only runs when the title string itself changes (TRAY-03). If
//! `set_title` fails, the tray falls back to a peak-colored icon plus tooltip
//! and logs the fallback once (TRAY-04).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
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

/// Last title handed to the tray; `None` until the tray is built.
#[derive(Debug, Default)]
pub struct LastTitle(Mutex<Option<String>>);

/// Whether `next` differs from the last rendered title (TRAY-03).
///
/// Presentation equality also covers `syncing`, `source`, `profile` and
/// `error`; those may rebuild the menu but must not rewrite the title.
pub fn title_changed(last: Option<&str>, next: &str) -> bool {
    last != Some(next)
}

/// Set after the first `set_title` failure; later refreshes use the fallback.
static TITLE_FALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Colored fallback icon chosen from the pricing state (TRAY-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackIcon {
    OffPeak,
    Peak,
}

impl FallbackIcon {
    /// PNG bytes for the icon.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::OffPeak => include_bytes!("../icons/tray-fallback-off.png"),
            Self::Peak => include_bytes!("../icons/tray-fallback-on.png"),
        }
    }

    /// Label used by the one-time fallback log.
    pub fn label(self) -> &'static str {
        match self {
            Self::OffPeak => "off-peak green",
            Self::Peak => "peak red",
        }
    }
}

/// Pick the fallback icon for the current pricing state.
///
/// The emoji title is the primary signal; when macOS hides it, the icon color
/// still communicates peak via green/red.
pub fn fallback_icon(peak: bool) -> FallbackIcon {
    if peak {
        FallbackIcon::Peak
    } else {
        FallbackIcon::OffPeak
    }
}

/// Build the tray icon, its title and its menu. Called once from `setup`.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let presentation = current_presentation(app);
    let menu = menu::build(app, &presentation)?;

    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::include_image!("icons/tray-template.png"))
        .icon_as_template(true)
        .menu(&menu)
        .title(presentation.title.clone())
        .tooltip(presentation.title.clone())
        .show_menu_on_left_click(false);
    builder.build(app)?;

    store_title(app, presentation.title.clone());
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

/// Re-assert the status item when macOS hid it (menu-bar overflow) or when a
/// title update removed it (tauri#12060). Cheap: runs on the 30 s tick.
pub fn ensure_visible(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let Ok(rect) = tray.rect() else {
        return;
    };
    if !needs_reassert(rect) {
        return;
    }
    match tray
        .set_visible(false)
        .and_then(|()| tray.set_visible(true))
    {
        Ok(()) => eprintln!("cost-guard: tray was hidden or zero-sized; re-asserted visibility"),
        Err(error) => eprintln!("cost-guard: failed to re-assert tray visibility: {error}"),
    }
}

/// Whether the tray rectangle proves the status item is missing or collapsed.
fn needs_reassert(rect: Option<tauri::Rect>) -> bool {
    match rect {
        None => true,
        Some(rect) => match rect.size {
            tauri::Size::Physical(size) => size.width == 0 || size.height == 0,
            tauri::Size::Logical(size) => size.width == 0.0 || size.height == 0.0,
        },
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
    if title_changed(load_title(app).as_deref(), &presentation.title) {
        apply_title(&tray, &presentation)?;
        store_title(app, presentation.title.clone());
    }

    store(app, presentation);
    Ok(())
}

fn current_presentation(app: &AppHandle) -> Presentation {
    let syncing = app.state::<SyncGuard>().is_running();
    runtime::with_state(app, |state| menu::present(state, syncing, clock::now()))
}

fn apply_title(tray: &TrayIcon<Wry>, presentation: &Presentation) -> tauri::Result<()> {
    if !TITLE_FALLBACK_ACTIVE.load(Ordering::SeqCst) {
        match tray.set_title(Some(&presentation.title)) {
            Ok(()) => return Ok(()),
            Err(error) => {
                TITLE_FALLBACK_ACTIVE.store(true, Ordering::SeqCst);
                eprintln!(
                    "cost-guard: tray title update failed ({error}); falling back to the {} icon and tooltip",
                    fallback_icon(presentation.peak).label()
                );
            }
        }
    }

    let icon = Image::from_bytes(fallback_icon(presentation.peak).bytes())?;
    tray.set_icon(Some(icon))?;
    tray.set_icon_as_template(false)?;
    tray.set_tooltip(Some(&presentation.title))
}

fn load(app: &AppHandle) -> Option<Presentation> {
    let state = app.state::<LastPresentation>();
    let guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clone()
}

fn load_title(app: &AppHandle) -> Option<String> {
    let state = app.state::<LastTitle>();
    let guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clone()
}

fn store_title(app: &AppHandle, title: String) {
    let state = app.state::<LastTitle>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(title);
}

fn store(app: &AppHandle, presentation: Presentation) {
    let state = app.state::<LastPresentation>();
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(presentation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_icon_follows_the_peak_state() {
        assert_eq!(fallback_icon(false), FallbackIcon::OffPeak);
        assert_eq!(fallback_icon(true), FallbackIcon::Peak);
        assert_ne!(fallback_icon(false).bytes(), fallback_icon(true).bytes());
    }

    #[test]
    fn both_fallback_icons_decode_as_png() {
        for icon in [FallbackIcon::OffPeak, FallbackIcon::Peak] {
            let decoded = Image::from_bytes(icon.bytes()).expect("fallback icon decodes");
            assert_eq!((decoded.width(), decoded.height()), (44, 44));
        }
    }

    #[test]
    fn title_changes_only_when_the_string_differs() {
        assert!(title_changed(None, "🟢 off"));
        assert!(!title_changed(Some("🟢 off"), "🟢 off"));
        assert!(title_changed(Some("🟢 off"), "🔴 2×"));
        assert!(!title_changed(Some("🟢 off ⚠"), "🟢 off ⚠"));
    }

    fn rect(width: u32, height: u32) -> tauri::Rect {
        tauri::Rect {
            position: tauri::Position::Physical(tauri::PhysicalPosition::new(0, 0)),
            size: tauri::Size::Physical(tauri::PhysicalSize::new(width, height)),
        }
    }

    #[test]
    fn missing_or_zero_sized_tray_rect_needs_reassert() {
        assert!(needs_reassert(None));
        assert!(needs_reassert(Some(rect(0, 22))));
        assert!(needs_reassert(Some(rect(20, 0))));
    }

    #[test]
    fn visible_tray_rect_is_left_alone() {
        assert!(!needs_reassert(Some(rect(80, 22))));
    }
}
