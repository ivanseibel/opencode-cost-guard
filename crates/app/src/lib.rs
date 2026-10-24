//! Cost Guard app modules.
//!
//! The binary in `main.rs` owns the Tauri runtime. Logic that can be tested
//! without a webview lives in this library target so unit tests reach it
//! directly.

pub mod app_state;
