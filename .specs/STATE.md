# STATE

## Decisions

### AD-001
- **Decision**: OpenCode Cost Guard is a Tauri 2 app with a platform-independent Rust core crate; the core must not depend on Tauri or macOS APIs.
- **Reason**: Cross-platform tray/plugin ecosystem plus a core reusable on Linux; core purity is CI-enforced.
- **Trade-off**: Heavier bundle and webview footprint than a pure-Rust tray.
- **Scope**: All crates and features.
- **Date**: 2026-10-23
- **Status**: active
- See `docs/adr/0001-tauri-2-stack.md`.

### AD-002
- **Decision**: Catalog comes from the OpenCode mirror (`models.opencode.ai/api.json`) with curated TOML Overlays for peak windows, holidays, limits and capability scores; peak prices are computed locally (off-peak × multiplier).
- **Reason**: Upstream carries no peak metadata; Overlays are reviewable and contributor-editable.
- **Trade-off**: Overlay values can drift from upstream until reviewed.
- **Scope**: Core catalog, sync, recommendation; future UI.
- **Date**: 2026-10-23
- **Status**: active
- See `docs/adr/0002-catalog-and-pricing-data-strategy.md`.

### AD-003
- **Decision**: Persistence is JSON files in v0.1 (config + current/previous snapshots); SQLite is deferred to Phase 2.
- **Reason**: No query needs yet; diffing two snapshots is enough; migration later is cheap.
- **Trade-off**: No history until Phase 2.
- **Scope**: Core storage.
- **Date**: 2026-10-23
- **Status**: active
- See `docs/adr/0003-persistence-json-now-sqlite-later.md`.

### AD-004
- **Decision**: v0.1 never reads OpenCode files, DB, CLI output or credentials; integration is Phase 3 and consent-first.
- **Reason**: Zero-credential, auditable security posture.
- **Trade-off**: No local usage/allowance data before Phase 3.
- **Scope**: All features until Phase 3.
- **Date**: 2026-10-23
- **Status**: active
- See `docs/adr/0004-no-opencode-internals-in-v01.md`.

### AD-005
- **Decision**: Core conventions: single crate `cost-guard-core`, edition 2024, stable toolchain pinned in `rust-toolchain.toml`, `chrono` for time, `ureq` + rustls behind an injected HTTP trait, `serde`/`toml`/`serde_json`, `thiserror`; unit tests co-located, integration fixtures under `tests/`.
- **Reason**: One crate for a small codebase keeps iteration cheap; injected HTTP keeps CI hermetic.
- **Trade-off**: Blocking HTTP needs a worker thread in the async Tauri app; pinned toolchain must be bumped deliberately.
- **Scope**: Core crate and all future crates following its conventions.
- **Date**: 2026-10-23
- **Status**: active

## Handoff

- **Feature**: core-v01 (`.specs/features/core-v01/`)
- **Phase / Task**: Design complete, Tasks pending approval
- **Completed**: T- (none yet)
- **In-progress**: none
- **Next step**: approve tasks, then dispatch batch 1 of Execute
- **Blockers**: none
- **Uncommitted files**: design.md, tasks.md (pending approval commit)
- **Branch**: main
