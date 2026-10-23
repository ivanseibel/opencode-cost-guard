# macOS-first menu bar app on Tauri 2 (Rust core + webview)

## Status

accepted

OpenCode Cost Guard is built as a Tauri 2 app with a platform-independent Rust core: native tray/menu on macOS, official plugins for notifications, autostart and updates, and a webview opened only for settings and (future) dashboard. Chosen over a pure-Rust tray (tray-icon + egui) for contributor reach and future UI richness, and over Swift/SwiftUI despite the better native feel, because a native shell would block the planned Linux port and shrink the contributor pool. The core must not depend on Tauri or macOS APIs so the tray shell can be replaced without touching domain logic.

## Considered Options

- **Tauri 2** (chosen): cross-platform tray + plugin ecosystem + webview for settings; heavier than pure Rust.
- **Pure Rust** (`tray-icon` + `muda` + egui): lightest, no webview; weaker UI and contributor story.
- **Swift/SwiftUI**: best macOS UX; kills the Linux path and the open-source contributor pool.
