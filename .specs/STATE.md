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

### AD-006
- **Decision**: Peak pricing is applied per vendor (only models whose curated `vendor` has an active Pricing Window), and capability review must never assume a uniform price multiplier. Cost scoring uses a log-scaled min-max so zero-cost candidates do not collapse paid candidates to zero.
- **Reason**: core-v01 shipped a uniform multiplier that canceled in normalization (peak never changed a winner) and a `min/cost` ratio that zeroed every paid candidate whenever a free model was eligible.
- **Trade-off**: Recommendations now depend on curated `vendor` data; contributor models stay eligible and are flagged with a `DataTerms` reason instead of being excluded.
- **Scope**: Recommendation scoring, catalog/overlay data, acceptance matrix.
- **Date**: 2026-10-24
- **Status**: active

### AD-007
- **Decision**: The macOS shell lives in `crates/app` with `tauri.conf.json` at the crate root, static `ui/` assets (no Node), pinned Tauri 2.12.x, and Tauri CLI 2.12.4 installed prebuilt; Ubuntu CI runs core only while the macOS job checks/builds the app.
- **Reason**: CLI discovers the crate at depth ≤3 without a `src-tauri` directory; static assets keep the build dependency-free; keeping the Ubuntu job core-only avoids GTK/WebKit deps.
- **Trade-off**: Bundling requires the Tauri CLI; app CI coverage is macOS-only.
- **Scope**: `crates/app`, CI, packaging for M2/M3.
- **Date**: 2026-10-24
- **Status**: active

### AD-008
- **Decision**: v0.1.0 is distributed as an ad-hoc-signed, non-notarized macOS `.app` zip via GitHub Releases, with the Gatekeeper workaround documented; notarization is deferred until external demand justifies the cost.
- **Reason**: Personal-first tool; the primary user builds from source. Developer ID + notarization adds $99/year, secrets and CI complexity now.
- **Trade-off**: Gatekeeper warnings and manual bypass on first launch; no updater until signing is consistent.
- **Scope**: Release workflow, docs, future distribution (Homebrew, updater).
- **Date**: 2026-10-24
- **Status**: active
- See `docs/adr/0008-unsigned-github-releases.md`.

## Handoff

- **Feature**: **v0.1.0 released** (2026-10-24) — https://github.com/ivanseibel/opencode-cost-guard/releases/tag/v0.1.0
- **Phase / Task**: M0–M3 complete; release pipeline exercised end-to-end
- **Completed**: `main` + tag `v0.1.0` pushed; Release run 40099813189 success; zip + `SHA256SUMS.txt` downloaded and verified locally
- **In-progress** (file:line): none
- **Next step**: dogfood v0.1.0; plan v0.2 (pre-peak warning, catalog-change notifications, settings polish, favorites) or Phase 2 (cost awareness)
- **Blockers**: none. The build is unsigned/not notarized; Gatekeeper instructions live in `docs/releasing.md`.
- **Uncommitted files**: none
- **Branch**: main

**Open follow-ups**

- Calibrate `data/capabilities.toml` scores after dogfooding; contributor models stay eligible per AD-006.
- M2 optional: wire the startup-sync test to the real call site; reload settings on re-show; GUI-only smoke legs remain observation-only.
