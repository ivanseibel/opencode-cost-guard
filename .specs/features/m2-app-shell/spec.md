# M2 App Shell (macOS) Specification

## Problem Statement

The core crate is complete and verified, but nothing runs yet: no menu-bar app, no tray, no notifications, no settings. This feature adds `crates/app`, a Tauri 2 shell that exposes the core to the author as a persistent macOS menu-bar utility, and wires catalog sync, task profiles and recommendations into a native menu.

## Goals

- [ ] A bundled macOS app with no Dock icon runs from the menu bar and shows DeepSeek peak state at a glance
- [ ] Menu exposes prices, next transition, Task Profile switch, recommendation with reasons, sync and settings
- [ ] Peak enter/exit notifications fire once per transition (when enabled)
- [ ] Catalog sync runs at start, every 6 h and on demand; changes set a dismissible badge
- [ ] Settings window edits the core `Config` with validation; autostart toggles a LaunchAgent
- [ ] `cost-guard-core` stays free of Tauri dependencies (CI purity gate)

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Release workflow, notarization, Homebrew, updater | Backlog M3 |
| Pre-peak warning, catalog-change notifications, favorites, app icon | v0.2 |
| Linux build | Phase 4 |
| Reading OpenCode files/DB/CLI or credentials | ADR-0004 |
| Multi-plan support (Console), allowance tracking | Phase 2+ |
| React or any Node-based frontend build | Static HTML/JS keeps the app dependency-free |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Tauri version | `tauri` 2.12.x, pin minor; spike validates tray title first | tray-icon can change in minor releases; macOS 27 fixes landed in 2.12.0 | y |
| App layout | `crates/app` workspace member with its own `tauri.conf.json`; static `frontendDist` (HTML/JS, no Node); CI runs `cargo check -p cost-guard-app` on macOS only | Keeps the Ubuntu job core-only, no GTK deps | y |
| Tray title fallback | If runtime title updates vanish the tray (tauri#12060), use colored icon + tooltip and document it | Spike decides; behavior must degrade gracefully | y |
| App identity | bundle id `dev.ivanseibel.opencode-cost-guard`; data dir from Tauri path API (`snapshots/` + `config.json`) | Personal-first, standard macOS paths | y |
| Timer cadence | state tick every 30 s; sync on start, every `syncIntervalHours` (default 6) and on demand; single-flight | Cheap, no busy polling | y |
| Menu language | English | Matches docs and OSS audience | y |
| Notification policy | Only peak enter/exit; dedupe by transition; honor config toggles | v0.1 scope | y |
| Visual verification | Pure logic (labels, transitions, badge, scheduler) unit-tested; tray/menu/notification behavior verified by spike + manual checklist evidence | GUI automation is out of scope | y |
| Core changes | None required; if one becomes necessary it must be additive and covered by core tests | Protects the verified core | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: App shell and tray ⭐ MVP

**User Story**: As the author, I want a menu-bar app that shows DeepSeek peak state without opening anything.

**Acceptance Criteria**:

1. APP-01 WHEN the workspace builds on macOS THEN `crates/app` SHALL compile and `cargo tauri build` SHALL produce a `.app` bundle.
2. APP-02 The settings UI SHALL be served from a static `frontendDist` folder with no Node toolchain in the build.
3. APP-03 IF startup fails to load overlays or snapshots THEN the app SHALL stay running and show an error item in the menu instead of crashing.
4. TRAY-01 WHEN DeepSeek is off-peak THEN the tray title SHALL read `🟢 off`; WHEN peak THEN `🔴 2×`; the menu header SHALL show the full `🟢 DS off` / `🔴 DS 2×` label.
5. TRAY-02 WHILE a Catalog Change is unacknowledged THEN the tray title SHALL append ` ⚠`.
6. TRAY-03 The app SHALL update the tray title only when the state or badge value changes, never on every timer tick.
7. TRAY-04 IF runtime title updates hide the tray on the current macOS/Tauri combination THEN the app SHALL fall back to a colored icon with tooltip text and SHALL log the fallback once.

**Independent Test**: Launch the bundled app; menu bar shows state text; toggle a synthetic peak state in a debug build and watch the title change once.

---

### P1: Native menu ⭐ MVP

**User Story**: As the author, I want one click to see prices, switch profile and read the recommendation.

**Acceptance Criteria**:

1. MENU-01 WHEN the menu opens THEN it SHALL show the DeepSeek state, the next transition in local time and UTC, and input/output/cache-read prices for DeepSeek V4.1 Flash at the current window.
2. MENU-02 The menu SHALL show the five Task Profiles as check items and persist a change to `Config.defaultProfile`.
3. MENU-03 The menu SHALL show the current Recommendation as one pick line plus its reason lines, with up to three alternatives in a submenu.
4. MENU-04 The menu SHALL show the active catalog source (remote/last valid/bundled) and the last successful sync time.
5. MENU-05 The menu SHALL offer Sync Catalog (disabled while a sync is in flight), Settings and Quit.
6. MENU-06 WHEN the Recommendation has no candidate THEN the menu SHALL show the failing filter reason.

**Independent Test**: Open the menu in a debug run; switch profile; the recommendation line updates on the next menu open and the choice survives restart.

---

### P1: Notifications ⭐ MVP

**User Story**: As the author, I want a single notification when DeepSeek enters or leaves peak so I stop checking manually.

**Acceptance Criteria**:

1. NOTIF-01 WHEN the state transitions off-peak → peak AND peak notifications are enabled THEN the app SHALL send exactly one notification naming the new recommended model for the active profile.
2. NOTIF-02 WHEN the state transitions peak → off-peak AND peak notifications are enabled THEN the app SHALL send exactly one notification.
3. NOTIF-03 IF the same transition is evaluated again THEN the app SHALL NOT send a duplicate notification.

**Independent Test**: Debug build with a fake clock fixture transitions twice; count notifications (one per transition, none on repeated ticks).

---

### P1: Catalog sync and change badge ⭐ MVP

**User Story**: As the author, I want the catalog fresh without thinking about it and a visible flag when something changed.

**Acceptance Criteria**:

1. SYNCA-01 The app SHALL sync at startup, every configured interval and on manual trigger; overlapping syncs SHALL NOT run.
2. SYNCA-02 WHEN the new snapshot differs from the previous THEN the app SHALL set the unacknowledged badge until the user chooses "Acknowledge changes" in the menu.
3. SYNCA-03 WHEN sync falls back to last valid or bundled data THEN the menu SHALL show which source is active.
4. SYNCA-04 IF a sync fails THEN the app SHALL keep the previous state and log the failure without a modal dialog.

**Independent Test**: Stub HTTP responses in unit tests for scheduler/badge logic; manual run triggers Sync Catalog with the network on and off.

---

### P1: Settings and platform ⭐ MVP

**User Story**: As the author, I want to change autostart, notifications and the default profile from a small window.

**Acceptance Criteria**:

1. SET-01 WHEN Settings opens THEN the window SHALL show autostart, peak-enter/peak-exit toggles, default profile, sync interval and ignored models, prefilled from `Config`.
2. SET-02 IF input is invalid THEN saving SHALL show a field-level error and SHALL NOT persist the file.
3. SET-03 WHEN the Settings window closes THEN the app SHALL keep running; reopening SHALL show the persisted config.
4. PLAT-01 The bundled app SHALL show no Dock icon and no Cmd-Tab entry; dev runs SHALL use accessory activation policy.
5. PLAT-02 WHEN autostart is enabled THEN the app SHALL create the LaunchAgent entry; disabling SHALL remove it; default is off.
6. SEC-01 The app SHALL NOT read OpenCode files, databases, CLI output or credentials.

**Independent Test**: Change settings, restart, values persist; `LSUIElement` verified in the bundled Info.plist; LaunchAgent plist exists when enabled.

---

## Edge Cases

- IF the overlay holiday calendar misses the current year THEN the app SHALL still show peak state and surface the core warning in the menu.
- WHEN the settings window is open and the user quits from the menu THEN the process SHALL exit cleanly (window closes).
- IF no snapshot exists on first run THEN sync SHALL fall back to bundled data and the menu SHALL say so.
- WHEN a sync is already running and the user clicks Sync Catalog THEN the item SHALL be disabled, not queued twice.
- IF the tray icon cannot render the emoji title THEN the fallback icon SHALL still communicate peak via color.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| APP-01 | P1: Shell and tray | Tasks | Verified |
| APP-02 | P1: Shell and tray | Tasks | Verified |
| APP-03 | P1: Shell and tray | Tasks | Verified |
| TRAY-01 | P1: Shell and tray | Tasks | Verified |
| TRAY-02 | P1: Shell and tray | Tasks | Verified |
| TRAY-03 | P1: Shell and tray | Tasks | Verified |
| TRAY-04 | P1: Shell and tray | Tasks | Verified |
| MENU-01 | P1: Menu | Tasks | Verified |
| MENU-02 | P1: Menu | Tasks | Verified |
| MENU-03 | P1: Menu | Tasks | Verified |
| MENU-04 | P1: Menu | Tasks | Verified |
| MENU-05 | P1: Menu | Tasks | Verified |
| MENU-06 | P1: Menu | Tasks | Verified |
| NOTIF-01 | P1: Notifications | Tasks | Verified |
| NOTIF-02 | P1: Notifications | Tasks | Verified |
| NOTIF-03 | P1: Notifications | Tasks | Verified |
| SYNCA-01 | P1: Sync | Tasks | Verified |
| SYNCA-02 | P1: Sync | Tasks | Verified |
| SYNCA-03 | P1: Sync | Tasks | Verified |
| SYNCA-04 | P1: Sync | Tasks | Verified |
| SET-01 | P1: Settings | Tasks | Verified |
| SET-02 | P1: Settings | Tasks | Verified |
| SET-03 | P1: Settings | Tasks | Verified |
| PLAT-01 | P1: Settings | Tasks | Verified |
| PLAT-02 | P1: Settings | Tasks | Verified |
| SEC-01 | P1: Settings | Tasks | Verified |

**ID format:** `[CATEGORY]-[NUMBER]`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 26 total, 0 mapped to tasks (Tasks phase pending)

---

## Success Criteria

- [ ] Bundled app launches with no Dock icon, tray title correct, menu fully populated
- [ ] Peak transitions notify exactly once when enabled
- [ ] Sync works online/offline with correct source reporting and dismissible badge
- [ ] Settings persist and validate; autostart toggles the LaunchAgent
- [ ] `cargo test --workspace` (mac) + `cargo check -p cost-guard-app` green; core purity gate unchanged
- [ ] Independent Verifier PASS with per-AC evidence and killed sensor mutations
