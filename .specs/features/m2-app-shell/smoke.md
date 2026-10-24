# M2 App Shell — Smoke Verification

**Date**: 2026-10-24
**Machine**: macOS 26.6.2 (25G83), arm64, Rust 1.97.1, tauri-cli 2.12.4
**Bundle**: `target/release/bundle/macos/Cost Guard.app` (14.94 MiB, ad-hoc signed)
**Evidence directory**: `.specs/features/m2-app-shell/evidence/`

Status values: **PASS** (observed), **PARTIAL** (core behavior observed through a
probe/log; the GUI leg was blocked), **BLOCKED** (needs GUI input this host cannot
synthesize).

## Environment limitation

GUI **input** is unavailable to this session: Orca Computer Use has no
Accessibility grant and synthesized `CGEvent` clicks are ignored (the pointer
never moves). Full-display `screencapture` returns only the wallpaper, while
per-window `screencapture -l` works. Exact probes and errors:
`evidence/gui-permission-blocked.txt`.

Consequences: opening the native tray menu, clicking Settings, editing fields,
closing the window and clicking Quit could not be driven. Checks that depend on
those clicks are marked PARTIAL/BLOCKED and list the compensating evidence
(runtime logs, window probes, unit tests).

## Checks

| # | Check | Requirement | Status | Evidence |
| - | ----- | ----------- | ------ | -------- |
| 1 | Release bundle builds and launches | APP-01 | PASS | `bundle-info.txt` |
| 2 | No Dock icon / Cmd-Tab entry | PLAT-01 | PASS | `bundle-info.txt` |
| 3 | Tray visible with correct title | TRAY-01 | PASS | `tray-title.png`, `tray-probe.txt` |
| 4 | Badge suffix ` ⚠` | TRAY-02 | PARTIAL | unit tests; live badge blocked (menu) |
| 5 | Menu contents | MENU-01..06 | PARTIAL | `menu-built.log`; native open blocked |
| 6 | Peak notifications, one per transition | NOTIF-01..03 | PASS | `fake-clock-peak-enter.log`, `fake-clock-peak-exit.log` |
| 7 | Settings window + persistence | SET-01..03 | PARTIAL | `settings-probe.txt`, `settings-window.png`, `settings-window-run.log` |
| 8 | Autostart LaunchAgent | PLAT-02 | PARTIAL | `autostart-launchagent.log`; UI toggle blocked |
| 9 | Sync online/offline + source | SYNCA-01..04 | PASS | `sync-online.log`, `sync-online-release.log`, `sync-offline-*.log` |
| 10 | Badge acknowledge | SYNCA-02 | BLOCKED | menu click blocked; `app_state`/`sync_job` unit tests |
| 11 | Close hides, Quit exits | SET-03 | BLOCKED | code path only; no input |
| 12 | No OpenCode internals / credentials | SEC-01 | PASS | code inspection + core purity gate |

---

### 1. Release bundle builds and launches — PASS

```sh
cd crates/app
cargo tauri build --bundles app
open -n "target/release/bundle/macos/Cost Guard.app"
```

Built at `target/release/bundle/macos/Cost Guard.app` (14.94 MiB, ad-hoc
signed). Launch Services reports `CFBundleIdentifier="dev.ivanseibel.costguard"`
and `ApplicationType="UIElement"`; the process stayed alive. Full probe output:
`evidence/bundle-info.txt`.

### 2. No Dock icon / Cmd-Tab entry — PASS

`Info.plist` in the bundle contains `LSUIElement => true` and Launch Services
classifies the running app as `UIElement` (`evidence/bundle-info.txt`), which is
the macOS mechanism that removes both the Dock tile and the Cmd-Tab entry. A
visual Dock inspection was not possible because full-display capture is blocked;
the plist/Launch Services evidence is authoritative.

### 3. Tray visible with correct title — PASS

Window probe while the bundled app ran found the 101×33 pt status item at
`x=759, y=0`; per-window capture shows the green dot and `DS off`
(`evidence/tray-title.png`, probe in `evidence/tray-probe.txt`). This matches
TRAY-01 exactly (`🟢 DS off`).

### 4. Badge suffix — PARTIAL

The title builder appends ` ⚠` and unit tests assert both badge states
(`ui_model::tests::tray_title_matches_the_spec_exactly`). Producing a live badge
requires a catalog change plus opening the menu, which needs GUI input; blocked
by the environment above.

### 5. Menu contents — PARTIAL

Opening the native menu requires a tray click, which this host cannot
synthesize. The debug build logs the exact presentation that `menu::build`
renders on every rebuild (`evidence/menu-built.log`), including:

- state `🟢 DS off`, next transition in local time + UTC;
- DeepSeek V4.1 Flash prices (input/output/cache read), off-peak and 2× peak;
- active profile `visual-ui` with the five check items built from
  `TaskProfile::ALL`;
- pick line + reason lines, three alternatives;
- `Source: remote snapshot · last sync 2026-10-24 03:34 UTC`;
- `syncing` flag (Sync Catalog disabled while true) and `badge` flag
  (Acknowledge item only when true).

The exact strings are also asserted by the T3 unit tests
(`ui_model::tests`). Native rendering itself was not observed.

### 6. Peak notifications — PASS

Debug fake clock (compiled under `cfg(debug_assertions)`; release ignores it),
run for 75 s so at least three 30 s ticks occur:

```sh
COST_GUARD_FAKE_NOW=2026-10-26T00:59:45Z ./target/debug/cost-guard-app   # 00:59:45 -> 01:00+
COST_GUARD_FAKE_NOW=2026-10-26T03:59:45Z ./target/debug/cost-guard-app   # 03:59:45 -> 04:00+
```

Enter run: `detected=1`, `enter=1`, `exit=0`, body
`Now recommending DeepSeek V4.1 Flash for Simple Coding.`
(`evidence/fake-clock-peak-enter.log`). Exit run: `detected=1`, `exit=1`,
`enter=0` (`evidence/fake-clock-peak-exit.log`). Repeated ticks after each
transition stayed silent, proving dedupe (NOTIF-03). The first observation on
startup did not notify.

### 7. Settings window and persistence — PARTIAL

Window probe (`COST_GUARD_OPEN_SETTINGS=1`, debug-only fixture) created
`Cost Guard Settings` at exactly 520×420 pt; the webview loaded and invoked the
backend: `settings_get served profile=visual-ui intervalHours=1 autostart=false
ignoredModels=1`, read from a `config.json` written before the launch
(`evidence/settings-probe.txt`, `evidence/settings-window-run.log`,
`evidence/settings-window.png`). That proves the settings page is served from
`frontendDist`, the capability allows the invoke, and persisted values survive a
restart.

Saving/validation is covered by the same code path used by the command:

```sh
cargo test -p cost-guard-app settings::
```

(`dto_round_trips_every_config_field`, `dto_serializes_with_camel_case_keys`,
`unknown_fields_survive_a_settings_save`, `valid_settings_persist_and_reload`,
`invalid_values_name_the_field_and_leave_the_file_untouched`,
`config_level_errors_map_to_the_config_pseudo_field`,
`field_errors_keep_the_core_field_and_message`).

Editing fields and closing the window (close→hide) need GUI input and are
BLOCKED.

### 8. Autostart LaunchAgent — PARTIAL

Production wiring: `tauri-plugin-autostart` 2.7.0 with
`app_name("dev.ivanseibel.costguard")` and `MacosLauncher::LaunchAgent`;
`apply_autostart` returns early inside `tauri::is_dev()` builds.

The LaunchAgent mechanism was exercised with `auto-launch` 0.6.0 (the plugin's
own implementation) using the same app name and the release bundle executable:
`enable()` created `~/Library/LaunchAgents/dev.ivanseibel.costguard.plist` with
`Label=dev.ivanseibel.costguard`, the bundle executable in `ProgramArguments`,
and `RunAtLoad=true`; `is_enabled()` became `true`; `disable()` removed the file
and `is_enabled()` returned to `false`
(`evidence/autostart-launchagent.log`). The plist was removed after the check.

Toggling it through the Settings UI is BLOCKED by the GUI limitation above.

### 9. Sync online/offline and source reporting — PASS

Online (release bundle, stale snapshot to force a due sync):

```
cost-guard: sync started
cost-guard: sync updated (Remote)
```

`evidence/sync-online-release.log` (release build) and
`evidence/sync-online.log` (debug build); `catalog.current.json` was rewritten
with a new `fetched_at` and ETag.

Offline, using `sandbox-exec -p '(version 1)(allow default)(deny network*)'`:

```
cost-guard: sync fell back to LastValid: fetch failed: ... nodename nor servname provided ...
cost-guard: sync fell back to Bundled:   fetch failed: ... nodename nor servname provided ...
```

`evidence/sync-offline-last-valid.log` (snapshot present) and
`evidence/sync-offline-bundled.log` (snapshots removed). No modal appeared; the
app stayed alive and the active source is part of the menu presentation
(`menu-built.log`). The source line strings are unit-tested in `ui_model`.

### 10. Badge acknowledge — BLOCKED

Requires opening the menu and clicking Acknowledge changes. The badge logic is
covered by `app_state` and `sync_job` unit tests (badge turns on when
`detect_changes` is non-empty, stays off for identical catalogs, and
`acknowledge_changes` clears it).

### 11. Close hides / Quit exits — BLOCKED

Code path: `main.rs` prevents `CloseRequested` for the `settings` window and
hides it; `menu::on_menu_event` calls `app.exit(0)` for Quit;
`RunEvent::ExitRequested` is only prevented when the code is `None`. Clicking
either control needs GUI input, which is blocked; no runtime observation.

### 12. No OpenCode internals or credentials — PASS

The app reads only its own `config.json` and catalog snapshots; there is no code
path for OpenCode files, databases, CLI output or credentials. The core purity
gate stays green (`bash scripts/check-core-purity.sh`).

## Build and test gate

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p cost-guard-app
bash scripts/check-core-purity.sh
cd crates/app && cargo tauri build --bundles app
```

All green: 138 workspace tests (47 app unit tests, 87 core unit tests, 4 core
integration tests), clippy with `-D warnings`, and the release bundle above.

## Notes and cleanup

- The debug-only fixtures are `COST_GUARD_FAKE_NOW` (anchored RFC 3339 instant,
  advances with real elapsed time) and `COST_GUARD_OPEN_SETTINGS=1` (opens the
  settings window without a tray click); both are compiled out of release builds.
- The app data directory was backed up before the smoke run and restored
  afterwards; no config file or LaunchAgent was left behind.
