//! Shared runtime helpers for the Tauri glue.
//!
//! The managed [`AppState`] lives behind a mutex; these helpers keep the lock
//! handling in one place and recover from a poisoned mutex instead of
//! panicking in the event loop.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;

/// Type under which the app state is managed.
pub type SharedState = Mutex<AppState>;

/// Run `f` with shared access to the app state.
pub fn with_state<R>(app: &AppHandle, f: impl FnOnce(&AppState) -> R) -> R {
    let state = app.state::<SharedState>();
    let guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&guard)
}

/// Run `f` with mutable access to the app state.
pub fn with_state_mut<R>(app: &AppHandle, f: impl FnOnce(&mut AppState) -> R) -> R {
    let state = app.state::<SharedState>();
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}
