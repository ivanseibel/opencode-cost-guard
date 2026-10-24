# M2 App Shell (macOS) Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.**

**If the skill cannot be activated, STOP and tell the user - do not proceed without it.**

---

**Design**: `.specs/features/m2-app-shell/design.md`
**Status**: Draft

---

## Test Coverage Matrix

> Generated from the verified Tauri research and existing repo conventions. GUI behaviors are manual-smoke evidence; logic lives in pure modules with unit tests.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| App pure logic (`app_state`, `ui_model`, `events`, scheduler/badge decisions, settings mapping) | unit | All branches; 1:1 to spec ACs; all listed edge cases | `crates/app/src/**/*.rs` (`#[cfg(test)]`) | `cargo test -p cost-guard-app` |
| Tauri glue (tray, menu, window lifecycle, spawned timers) | none - build gate + smoke | Compiles and behaves in smoke checklist | `crates/app/src/*.rs` | build gate |
| Static settings UI | none - manual checklist | Form fields, validation display, persistence | `crates/app/ui/*` | build gate + smoke |
| Core domain (unchanged) | unit (existing) | Existing suite stays green | `crates/core/**` | `cargo test -p cost-guard-core` |
| Workspace / CI / docs / assets | none - build gate only | Build gate only | `Cargo.toml`, `.github/workflows/ci.yml`, `README.md`, `CONTRIBUTING.md`, `crates/app/icons/*` | build gate |

## Gate Check Commands

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | App unit-test tasks | `cargo test -p cost-guard-app` |
| Full | Integration-test tasks / end of phase | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |
| Build | Glue/assets/config tasks and phase completion | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo check -p cost-guard-app && bash scripts/check-core-purity.sh` |
| Bundle (T12/T13 only) | Packaging smoke | `cd crates/app && cargo tauri build --bundles app` |

---

## Execution Plan

Phases are ordered; tasks within a phase run in order.

### Phase 1: App skeleton and state

```
T1 → T2 → T3 → T4
```

### Phase 2: Tray, menu, timers, sync

```
T4 → T5 → T6 → T7 → T8
```

### Phase 3: Settings, CI, smoke

```
T8 → T9 → T10 → T11 → T12 → T13
```

---

## Task Breakdown

### T1: App crate scaffold

**What**: Add `cost-guard-app` to the workspace with Tauri config, build script, Info.plist (`LSUIElement`), default capability file, placeholder icon, empty `ui/` pages and a minimal `main.rs` (accessory policy, zero windows, app stays alive).
**Where**: `crates/app/Cargo.toml`
**Depends on**: None
**Reuses**: verified layout from design.md; `tauri` 2.12.5, `tauri-build` 2.7.4, plugins 2.7.0
**Requirement**: APP-01, APP-02, PLAT-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Workspace builds: `cargo check -p cost-guard-app`
- [x] `tauri.conf.json` has `app.windows: []`, `withGlobalTauri`, `frontendDist: "ui"`, reverse-DNS identifier, `bundle.targets: ["app"]`
- [x] `Info.plist` sets `LSUIElement=true`; runtime uses `ActivationPolicy::Accessory`
- [x] `capabilities/default.json` starts with `core:default` only for window `settings`; `notification:default` is added in T7 and `autostart:default` in T9 when the plugins land
- [x] Placeholder `icons/icon.png` exists; app launches with no window and no Dock tile
- [x] Gate passes: build gate command

**Tests**: none (scaffold/config, build gate)
**Gate**: build
**Commit**: `feat(app): scaffold tauri menu-bar crate`

---

### T2: App state loading

**What**: Implement `AppState` loading config, overlays and snapshots from the app data dir with bundled fallback, plus profile/badge mutations.
**Where**: `crates/app/src/app_state.rs`
**Depends on**: T1
**Reuses**: `Config::load`, `Overlays::bundled`, `SnapshotStore`, `parse_snapshot`
**Requirement**: APP-03, SYNCA-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Missing config yields defaults; missing snapshot falls back to bundled and reports the source
- [x] Invalid config keeps the app alive with defaults in memory and an error string
- [x] `acknowledge_changes` and `set_profile` persist through core `Config`
- [x] Unit tests cover defaults, bundled fallback, invalid config, profile persistence (temp dirs)
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(app): load app state with bundled fallback`

---

### T3: UI model strings

**What**: Implement exact tray/menu strings: tray title with badge, peak-aware price block, next transition (local + UTC), recommendation lines, alternatives, no-candidate reason, source/last-sync line.
**Where**: `crates/app/src/ui_model.rs`
**Depends on**: T2
**Reuses**: core `WindowState`, `ModelEntry`, `Recommendation`, `ActiveSource`
**Requirement**: TRAY-01, TRAY-02, MENU-01, MENU-03, MENU-04, MENU-06

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `tray_title` returns exactly `🟢 DS off` / `🔴 DS 2×` with ` ⚠` suffix when badge is set
- [x] Price lines use peak-aware effective prices (2× off-peak during DeepSeek peak)
- [x] Recommendation lines include pick + reasons; alternatives render up to 3; no-candidate shows failing filter
- [x] Unit tests assert exact strings for both windows, badge on/off, fallback source
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(app): add tray and menu ui model`

---

### T4: Transition tracker and notification dedupe

**What**: Implement `TransitionTracker` producing at most one transition per state change honoring notification toggles.
**Where**: `crates/app/src/events.rs`
**Depends on**: T3
**Reuses**: core `Notifications`
**Requirement**: NOTIF-01, NOTIF-02, NOTIF-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] First observation initializes without notifying
- [x] off→peak and peak→off each fire once; repeated ticks are silent
- [x] Disabled `peak_enter`/`peak_exit` suppress the respective transition
- [x] Unit tests cover all four cases
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(app): add peak transition tracker`

---

### T5: Tray and menu construction

**What**: Build the native tray (template icon, title, menu) and rebuild menu/title only on state change; wire the settings/profile/sync/acknowledge/quit item ids.
**Where**: `crates/app/src/tray.rs`
**Depends on**: T4
**Reuses**: `ui_model` strings; Tauri menu/tray APIs from design
**Requirement**: TRAY-03, TRAY-04, MENU-01, MENU-02, MENU-03, MENU-04, MENU-05

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Tray appears with title and menu; no duplicate tray item (no `app.trayIcon` in config)
- [ ] `capabilities/default.json` gains `notification:default`
- [ ] Title/menu update only when state or badge changes; no write inside `setup()`
- [ ] Menu contains state, transition, prices, five profile check items, recommendation + reasons, alternatives submenu, source line, Sync (disabled when in flight), Settings, Quit
- [ ] Fallback path switches to icon + tooltip and logs once if `set_title` fails
- [ ] Gate passes: build gate command

**Tests**: none (Tauri glue; strings covered in T3)
**Gate**: build
**Commit**: `feat(app): add native tray and menu`

---

### T6: Menu actions

**What**: Wire menu events to state mutations: profile switch persists, acknowledge clears badge, sync triggers the job, settings opens the window, quit exits.
**Where**: `crates/app/src/menu.rs`
**Depends on**: T5
**Reuses**: `AppState` from T2
**Requirement**: MENU-02, MENU-05, SYNCA-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Profile change persists and rebuilds the menu with the correct check mark
- [ ] Acknowledge removes ` ⚠` and hides the item
- [ ] Quit calls `app.exit(0)`; Settings shows the window
- [ ] Gate passes: build gate command

**Tests**: none (event glue; persistence covered in T2)
**Gate**: build
**Commit**: `feat(app): wire menu actions`

---

### T7: Timers and notifications

**What**: Add the 30 s tick and sync scheduler; on transitions update tray/menu and send one notification with the recommended model.
**Where**: `crates/app/src/timer.rs`
**Depends on**: T6
**Reuses**: `TransitionTracker`, `ui_model`, notification plugin
**Requirement**: NOTIF-01, NOTIF-02, SYNCA-01, APP-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Tick uses `MissedTickBehavior::Skip`; no title writes when unchanged
- [ ] Notification body names the new recommended model for the active profile (or the no-candidate reason)
- [ ] `should_sync(now, last_sync, interval)` is pure and unit-tested
- [ ] Sync failure keeps state and logs; no modal
- [ ] Gate passes: quick gate command

**Tests**: unit (scheduler decision)
**Gate**: quick
**Commit**: `feat(app): add timers and peak notifications`

---

### T8: Sync job and change badge

**What**: Run core sync on a blocking worker with single-flight, persist snapshots, apply outcome (source, last sync, badge on catalog change).
**Where**: `crates/app/src/sync_job.rs`
**Depends on**: T7
**Reuses**: `SyncEngine`, `UreqClient`, `SnapshotStore`, `detect_changes`
**Requirement**: SYNCA-01, SYNCA-02, SYNCA-03, SYNCA-04, TRAY-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Single-flight guard prevents concurrent syncs; manual item disabled while in flight
- [ ] `apply_sync_outcome` unit tests cover Updated/NotModified/Fallback and badge on/off
- [ ] Menu shows active source and last successful sync time
- [ ] Gate passes: quick gate command

**Tests**: unit (state application)
**Gate**: quick
**Commit**: `feat(app): add sync job and change badge`

---

### T9: Settings commands

**What**: Implement `settings_get` / `settings_set` with DTO mapping, core validation, field-tagged errors and autostart application.
**Where**: `crates/app/src/settings.rs`
**Depends on**: T8
**Reuses**: `Config`, autostart plugin
**Requirement**: SET-01, SET-02, PLAT-02, SEC-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] DTO round-trips all config fields; unknown fields preserved by core save
- [ ] `capabilities/default.json` gains `autostart:default`
- [ ] Invalid values return `{ field, message }` and do not touch the file
- [ ] Autostart toggle calls the plugin only outside dev builds; LaunchAgent app name is stable
- [ ] Unit tests cover mapping and error mapping
- [ ] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(app): add settings commands`

---

### T10: Static settings UI

**What**: Build the settings page (autostart, notification toggles, default profile, sync interval, ignored models) with `invoke`, inline validation and saved feedback.
**Where**: `crates/app/ui/settings.js`
**Depends on**: T9
**Reuses**: generated JSON schema? no; plain HTML/JS with `withGlobalTauri`
**Requirement**: SET-01, SET-02, APP-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] All five settings render and round-trip through `settings_set`
- [ ] Field errors render next to the input; success shows a saved state
- [ ] No Node/npm in the build; files load from `frontendDist`
- [ ] Manual check documented (open window, edit, save, reopen)
- [ ] Gate passes: build gate command

**Tests**: none (static UI; manual checklist)
**Gate**: build
**Commit**: `feat(app): add static settings page`

---

### T11: Settings window lifecycle

**What**: Create/show/hide the settings webview with close→hide, prevent app exit on last-window close, and correct capability wiring.
**Where**: `crates/app/src/main.rs`
**Depends on**: T10
**Reuses**: `WebviewWindowBuilder`, capability file from T1
**Requirement**: SET-03, PLAT-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Menu Settings shows the window; close hides it and the app keeps running
- [ ] Quit exits cleanly with the window hidden or open
- [ ] Bundled build has no Dock icon and no Cmd-Tab entry (`LSUIElement`); dev uses accessory policy
- [ ] Gate passes: build gate command

**Tests**: none (window lifecycle glue; smoke)
**Gate**: build
**Commit**: `feat(app): add settings window lifecycle`

---

### T12: CI and contributor docs

**What**: Update CI (Ubuntu core-only; macOS app check/build), install tauri-cli 2.12.4 prebuilt, ad-hoc sign the bundle, upload artifact; document local app commands in README/CONTRIBUTING.
**Where**: `.github/workflows/ci.yml`
**Depends on**: T11
**Reuses**: verified CI recipe from research
**Requirement**: APP-01, APP-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Ubuntu job runs core tests only; macOS job runs workspace tests + `cargo check -p cost-guard-app`
- [ ] Release-style job (workflow_dispatch or tag) installs tauri-cli, runs `cargo tauri build --bundles app`, ad-hoc signs, uploads zip
- [ ] README/CONTRIBUTING document `cargo tauri dev`, `cargo run -p cost-guard-app`, CLI install and bundling
- [ ] Gate passes: build gate command + `cd crates/app && cargo tauri build --bundles app`

**Tests**: none (CI/docs, build gate)
**Gate**: build
**Commit**: `ci: build and package the app shell`

---

### T13: Smoke verification and evidence

**What**: Run the bundled app and record evidence: tray title, menu contents, no Dock item, settings persistence, notifications, sync online/offline, badge acknowledge.
**Where**: `.specs/features/m2-app-shell/smoke.md`
**Depends on**: T12
**Reuses**: spike AX/screenshot technique; `cargo tauri build --bundles app`
**Requirement**: PLAT-01, PLAT-02, SEC-01, TRAY-01, TRAY-02, MENU-01, NOTIF-01, SYNCA-02

**Tools**:

- MCP: NONE
- Skill: NONE (screenshots/AX probing allowed)

**Done when**:

- [ ] `smoke.md` lists each check with pass/fail, command, and screenshot/probe evidence path
- [ ] Bundle launches from `target/release/bundle/macos/Cost Guard.app`; tray visible; activation policy accessory
- [ ] Peak state can be exercised with a debug fixture (fake clock) and produces exactly one notification
- [ ] Settings persist across restart; LaunchAgent created/removed when autostart toggled (outside dev)
- [ ] Gate passes: build gate command + bundle command

**Tests**: none (manual smoke with evidence)
**Gate**: build
**Commit**: `docs(app): add m2 smoke verification`

---

## Phase Execution Map

```
T1 → T2
T2 → T3
T3 → T4
T4 → T5
T5 → T6
T6 → T7
T7 → T8
T8 → T9
T9 → T10
T10 → T11
T11 → T12
T12 → T13
```

**Batch plan (orchestrator):** 13 tasks pack into 3 sequential batches, whole phases only:

- Batch A: Phase 1 (T1–T4) — 4 tasks
- Batch B: Phase 2 (T5–T8) — 4 tasks
- Batch C: Phase 3 (T9–T13) — 5 tasks

Verifier runs after Batch C. Batches are sequential because they share `crates/app` files and the git index; the only parallelizable work (static `ui/` assets) shares the index and is too small to justify a worktree split.

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Scaffold | 1 crate + config files | ✅ Granular |
| T2: App state | 1 module | ✅ Granular |
| T3: UI model | 1 module | ✅ Granular |
| T4: Transitions | 1 module | ✅ Granular |
| T5: Tray/menu build | 2 related modules (build only) | ✅ Granular |
| T6: Menu actions | 1 module | ✅ Granular |
| T7: Timers/notifications | 1 module | ✅ Granular |
| T8: Sync job | 1 module | ✅ Granular |
| T9: Settings commands | 1 module | ✅ Granular |
| T10: Settings UI | 1 file | ✅ Granular |
| T11: Window lifecycle | 1 file | ✅ Granular |
| T12: CI/docs | 1 workflow + docs | ✅ Granular |
| T13: Smoke | 1 evidence file | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | start | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |
| T6 | T5 | T5 → T6 | ✅ Match |
| T7 | T6 | T6 → T7 | ✅ Match |
| T8 | T7 | T7 → T8 | ✅ Match |
| T9 | T8 | T8 → T9 | ✅ Match |
| T10 | T9 | T9 → T10 | ✅ Match |
| T11 | T10 | T10 → T11 | ✅ Match |
| T12 | T11 | T11 → T12 | ✅ Match |
| T13 | T12 | T12 → T13 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1 | Workspace/assets/config | none | none | ✅ OK |
| T2 | App pure logic | unit | unit | ✅ OK |
| T3 | App pure logic | unit | unit | ✅ OK |
| T4 | App pure logic | unit | unit | ✅ OK |
| T5 | Tauri glue | none | none | ✅ OK |
| T6 | Tauri glue | none | none | ✅ OK |
| T7 | App pure logic + glue | unit | unit | ✅ OK |
| T8 | App pure logic + glue | unit | unit | ✅ OK |
| T9 | App pure logic | unit | unit | ✅ OK |
| T10 | Static UI | none | none | ✅ OK |
| T11 | Tauri glue | none | none | ✅ OK |
| T12 | CI/docs | none | none | ✅ OK |
| T13 | Smoke evidence | none | none | ✅ OK |
