//! Cost Guard app modules.
//!
//! The binary in `main.rs` owns the Tauri runtime. Logic that can be tested
//! without a webview lives in this library target so unit tests reach it
//! directly.

pub mod app_state;
pub mod events;
pub mod menu;
pub mod runtime;
pub mod settings;
pub mod sync_job;
pub mod timer;
pub mod tray;
pub mod ui_model;
