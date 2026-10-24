# M2 App Shell Validation

**Date**: 2026-10-24
**Spec**: `.specs/features/m2-app-shell/spec.md`
**Design**: `.specs/features/m2-app-shell/design.md`
**Tasks**: `.specs/features/m2-app-shell/tasks.md`
**Smoke**: `.specs/features/m2-app-shell/smoke.md`
**Diff range**: feature `311df62..HEAD`; fix loop `488aedd..cd452d3` (7 commits)
**Verifier**: independent sub-agent (author ≠ verifier)

## Fix loop iteration 2

This report replaces the iteration-1 FAIL. It re-derives all 26 requirements at
`HEAD = cd452d3` and re-runs the gate and the discrimination sensor. The seven
fix commits were reviewed as diffs, not as claims:

| Prior gap | Fix commit | Iteration-2 evidence | Status |
| --------- | ---------- | -------------------- | ------ |
| SYNCA-01 startup sync conditional | `bd50d5a` | `crates/app/src/main.rs:42` spawns one sync unconditionally in `setup`; `timer.rs:185-204` asserts Startup ignores a fresh snapshot; `evidence/sync-startup.log:5-6` shows `sync started` with a 1-minute-old snapshot | ✅ Fixed |
| Holiday-coverage warning never surfaced | `8a7bcbf` | `crates/app/src/menu.rs:99-101` projects `Overlays::validate`; `menu.rs:402-422` asserts the exact line and that peak math still works | ✅ Fixed |
| Fake-clock parser untested (sensor survivor) | `834a968` | `crates/app/src/clock.rs:64-123` tests valid, offset, invalid, empty and elapsed; sensor M1 is now killed | ✅ Fixed |
| APP-03 panics on bundled-data failure | `705cea3` | `app_state.rs:60-83` never returns `Result`; `app_state.rs:314-339` asserts degraded state + error item + warning | ✅ Fixed |
| TRAY-04 fallback had one green icon | `4dda257` | `tray.rs:44-79` selects two PNGs; decoded center pixels are green `(52,199,89)` and red `(255,59,48)`; `tray.rs:203-216` asserts selection + PNG decode | ✅ Fixed |
| TRAY-03 title rewritten on non-title changes | `1d44dc9` | `tray.rs:37-39` compares the title string; `tray.rs:129-132` gates `set_title`; test `tray.rs:218-224` | ✅ Fixed |
| MENU-02/MENU-05 seam assertions missing | `cd452d3` | `menu.rs:119-140` `action_state`; tests `menu.rs:424-474` assert five checks, `sync_enabled == !syncing`, Acknowledge visibility | ✅ Fixed |

**Result**: **PASS ✅**

**Spec-anchored check**: 26/26 ACs matched the spec-defined outcome; 0 blocking gaps.
Five observation limitations are documented below (native menu clicks, painted
settings window, autostart UI toggle, real `set_title` failure, close/quit/reopen),
each with compensating evidence and an explicit non-blocking decision.

**Sensor**: 6 mutations injected, 6 killed, 0 survived.
**Gates**: 5/5 passed; release bundle rebuilt and hash-confirmed at
`target/release/bundle/macos/Cost Guard.app` (15.18 MiB).

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1: App crate scaffold | ✅ Done | Workspace member, `frontendDist: "ui"`, `app.windows: []`, `LSUIElement`, capabilities |
| T2: App state loading | ✅ Done | Defaults, bundled fallback, invalid config, degraded bundled data, profile persistence |
| T3: UI model strings | ✅ Done | Exact tray/menu strings, both windows, badge, prices, sources, warnings |
| T4: Transition tracker | ✅ Done | First observation silent; one event per transition; toggles tested |
| T5: Tray and menu construction | ✅ Done | Construction + `title_changed` + peak-aware fallback + presentation seam tests |
| T6: Menu actions | ✅ Done | IDs/wiring present; profile persistence asserted; clicks blocked (documented) |
| T7: Timers and notifications | ✅ Done | `MissedTickBehavior::Skip`; pure scheduler tests; fake clock now tested |
| T8: Sync job and change badge | ✅ Done | Single-flight, Updated/NotModified/Fallback, badge on change only |
| T9: Settings commands | ✅ Done | DTO round-trip, field-tagged errors, autostart application |
| T10: Static settings UI | ✅ Done | All six controls round-trip; DOM probe proves prefill in the packaged webview |
| T11: Settings window lifecycle | ✅ Done | Close→hide, exit guard, accessory policy; clicks unobserved (documented) |
| T12: CI and contributor docs | ✅ Done | Ubuntu core-only; macOS workspace check; package job; README/CONTRIBUTING |
| T13: Smoke verification | ✅ Done | Honest PASS/PARTIAL/BLOCKED split; no fabricated GUI result |

---

## Spec-Anchored Acceptance Criteria

| ID | Spec-defined outcome | `file:line` + assertion | Result |
| -- | -------------------- | ----------------------- | ------ |
| APP-01 | `crates/app` compiles; `cargo tauri build` produces a `.app` | `crates/app/Cargo.toml:2` package `cost-guard-app`; `crates/app/tauri.conf.json:15` `"targets": ["app"]`; `cargo check -p cost-guard-app` exit 0; rebuilt bundle exists (15.18 MiB, arm64, ad-hoc) | ✅ PASS |
| APP-02 | Static `frontendDist`, no Node toolchain | `tauri.conf.json:7` `"frontendDist": "ui"`; `ui/index.html:1` static markup; no `package.json`/`node_modules` under `crates/app`; `.github/workflows/ci.yml` has no Node setup | ✅ PASS |
| APP-03 | Bad overlays/snapshots don't crash; error item shown | `app_state.rs:60-83` `load` returns `Self` and degrades; test `app_state.rs:314-339`: `error.contains("bundled overlays")` (325), `presentation.error == Some(error)` (334), degraded prices/recommendation/warning (329-338); `main.rs:32` has no `?`; invalid-config path `app_state.rs:377-390` | ✅ PASS |
| TRAY-01 | `🟢 DS off` / `🔴 DS 2×` | `ui_model.rs:201-206` asserts all four combinations exactly; live off-peak in `evidence/tray-probe.txt:1-2` + `tray-title.png`; peak presentation in `fake-clock-peak-enter.log:9` | ✅ PASS |
| TRAY-02 | Append ` ⚠` while unacknowledged | `ui_model.rs:204-205` asserts `"🟢 DS off ⚠"` and `"🔴 DS 2× ⚠"`; badge set only on non-empty `detect_changes` `app_state.rs:145-149`; ack clears once `app_state.rs:179-183`; menu item visible only with badge `menu.rs:131-140,222-232` + test `menu.rs:465-474` | ✅ PASS |
| TRAY-03 | Title updated only on state/badge change | `tray.rs:37-39` `title_changed(last, next)` is `last != Some(next)`; `tray.rs:118-136` skips unchanged presentations and gates `set_title` at 129; test `tray.rs:218-224` asserts no write for equal titles | ✅ PASS |
| TRAY-04 | Fall back to colored icon + tooltip; log once | `tray.rs:44-79` maps peak→`tray-fallback-on.png` (center RGBA 255,59,48 = red) and off-peak→`tray-fallback-off.png` (52,199,89 = green); `tray.rs:143-161` catches `set_title` error, sets the static flag once (42,144-155), logs once, sets icon + tooltip; tests `tray.rs:203-216` assert selection and 32×32 PNG decode | ✅ PASS (trigger unobserved; documented) |
| MENU-01 | State, next transition local+UTC, V4.1 Flash prices at current window | `menu.rs:59-105` composes all parts; `ui_model.rs:209-232` asserts off-peak and peak input/output/cache-read; `ui_model.rs:242-260` asserts `"Mon 09:00 local (Mon 01:00 UTC)"`; `evidence/menu-built.log:1` shows the composition | ✅ PASS |
| MENU-02 | Five profile check items; persist `defaultProfile` | `menu.rs:131-140` `action_state.profile_checks`; `menu.rs:184-197` builds one `CheckMenuItem` per entry; test `menu.rs:424-454` asserts count == `TaskProfile::ALL.len()`, checked == `[LargeRefactor]`, all five labels; persistence `app_state.rs:392-404` asserts reloaded `default_profile` | ✅ PASS |
| MENU-03 | One pick line + reasons + ≤3 alternatives in a submenu | `ui_model.rs:263-272` asserts pick + reason lines; `ui_model.rs:275-292` asserts 4 alternatives truncate to 3; `menu.rs:199-220` builds label items + `Alternatives` submenu | ✅ PASS |
| MENU-04 | Active source + last successful sync | `ui_model.rs:328-343` asserts remote/last-valid/bundled labels and `%Y-%m-%d %H:%M UTC`; `menu.rs:98` uses `source_line`; offline/online logs corroborate sources | ✅ PASS |
| MENU-05 | Sync Catalog (disabled in flight), Settings, Quit | `menu.rs:238-246` creates the three items, Sync enabled = `actions.sync_enabled`; test `menu.rs:456-463` asserts `true` then `false`; handlers `menu.rs:299-311` | ✅ PASS |
| MENU-06 | No-candidate shows failing filter reason | `ui_model.rs:295-310` asserts exact `"No candidate: ignored models — every model is listed in ignoredModels"`; `menu.rs:96` renders the lines | ✅ PASS |
| NOTIF-01 | Exactly one notification naming the new model for the active profile | `events.rs:71-81` asserts `observe(false)=None` then `observe(true)=Some(EnteredPeak)` and repeated ticks `None`; `timer.rs:112-126` names pick + profile label; one `show()` call `timer.rs:128-137`; `fake-clock-peak-enter.log:6-7` body `Now recommending DeepSeek V4.1 Flash for Simple Coding.` | ✅ PASS |
| NOTIF-02 | Exactly one notification on peak→off | `events.rs:84-94` asserts one `ExitedPeak` and silent repeats; `fake-clock-peak-exit.log:6-7` shows one detection and one send | ✅ PASS |
| NOTIF-03 | Repeated evaluation sends no duplicate | `events.rs:79-80,92-93` assert repeated `observe` returns `None`; toggle cases `events.rs:97-149`; smoke logs show no repeat after the transition | ✅ PASS |
| SYNCA-01 | Sync at startup, every interval, manual; no overlap | Startup: `main.rs:42` unconditional `sync_job::spawn` + `evidence/sync-startup.log:5-6`; interval: `timer.rs:54-64,86-91` + boundary tests `timer.rs:154-183`; manual: `menu.rs:306`; single-flight: `sync_job.rs:172-184` asserts second `try_start()` is refused | ✅ PASS (test seam not the wired path; documented) |
| SYNCA-02 | Badge on changed snapshot until Acknowledge | `app_state.rs:129-150` sets badge only when `detect_changes` is non-empty; tests `app_state.rs:430-463` (changed → badge, identical → off); Acknowledge item only with badge `menu.rs:131-140` + test `menu.rs:465-474`; ack clears `app_state.rs:179-183` | ✅ PASS |
| SYNCA-03 | Menu shows remote/last valid/bundled source | `app_state.rs:157-163` applies fallback source; `ui_model.rs:328-343` asserts all three labels; `sync_job.rs:252-292` asserts Bundled/LastValid outcomes; offline logs | ✅ PASS |
| SYNCA-04 | Keep previous state, log failure, no modal | `sync_job.rs:78-90` logs every outcome/warning; `sync_job.rs:252-292` asserts fallback state kept; no dialog API in `crates/app`; offline logs show the app stayed alive | ✅ PASS |
| SET-01 | Show autostart, both toggles, profile, interval, ignored models, prefilled from `Config` | `settings.rs:46-59` maps every field; test `settings.rs:209-221` round-trips all; `ui/index.html:31-109` has all six controls + error slots; `settings.js:172-188` applies the DTO; `evidence/settings-render-probe.log:6` reports all six present with non-zero layout and persisted values (`false`, `true`, `true`, `simple-coding`, `6`, empty) | ✅ PASS (paint unobserved; documented) |
| SET-02 | Invalid input → field-level error, nothing persisted | `settings.rs:96-105` validates before `Config::save`; test `settings.rs:271-287` asserts field `syncIntervalHours`/`ignoredModels`, non-empty message, and no `config.json`; UI inline rendering `settings.js:330-358` | ✅ PASS |
| SET-03 | Close keeps app running; reopen shows persisted config | `main.rs:17-25` `prevent_close` + `hide`; exit guard `main.rs:55-62` only prevents `code: None`; re-show `menu.rs:355-362`; `settings.js:380` loads persisted values once. Save→close→reopen and restart paths show persisted values; an unsaved edit (or a menu profile change while hidden) can leave the hidden form stale because re-show does not re-invoke `settings_get` | ✅ PASS (close/quit unobserved; stale-form edge documented, non-blocking) |
| PLAT-01 | No Dock icon/Cmd-Tab; accessory in dev | `crates/app/Info.plist:5-6` `LSUIElement=true`; `main.rs:27-28` `ActivationPolicy::Accessory`; rebuilt bundle Info.plist `LSUIElement => true`; `evidence/bundle-info.txt` Launch Services `type="UIElement"` | ✅ PASS |
| PLAT-02 | Enable creates LaunchAgent, disable removes, default off | `settings.rs:17-18` stable name; `settings.rs:153-169` enable/disable and `tauri::is_dev()` gate; `main.rs:66-71` plugin with `MacosLauncher::LaunchAgent`; default `autostart: false` `crates/core/src/config/mod.rs:106-109`; `evidence/autostart-launchagent.log:1-22` creates/removes the plist with `RunAtLoad=true` | ✅ PASS (UI toggle unobserved; documented) |
| SEC-01 | No OpenCode files/DBs/CLI/credentials | `app_state.rs:21-24` embeds the catalog; reads are `Config`/`SnapshotStore` under the app data dir only (`main.rs:30-32`); grep over `crates/app/src` + `ui` finds no OpenCode path, DB, CLI or credential access; `bash scripts/check-core-purity.sh` exit 0 (`core dependency purity OK`) | ✅ PASS |

**Status**: ✅ All 26 ACs met; 0 blocking gaps. Five observation limitations and
two evidence-quality notes are recorded below; none changes a spec outcome.

### Multi-part AC check

| AC | Parts | All parts asserted on values? |
| -- | ----- | ----------------------------- |
| TRAY-01 | off-peak title, peak title | ✅ exact strings |
| TRAY-02 | badge off/on for both windows | ✅ exact strings |
| MENU-01 | state + local + UTC + input/output/cache prices, both windows | ✅ each part exact; composition in `menu-built.log` |
| MENU-02 | five check items + persisted `defaultProfile` | ✅ count/checked/labels asserted; persistence asserted |
| MENU-03 | pick + reasons + ≤3 alternatives | ✅ exact, including 4→3 truncation |
| MENU-04 | source label + last-sync time, all three sources | ✅ all three exact |
| MENU-05 | Sync disabled in flight + Settings + Quit | ✅ `sync_enabled` asserted; Settings/Quit IDs + handlers inspected |
| NOTIF-01 | exactly one + model + active profile | ✅ tracker + smoke body |
| SET-01 | six controls + values from `Config` | ✅ DOM probe values + DTO round-trip |

---

## Edge Cases

| Edge case (spec) | Status | Evidence |
| ---------------- | ------ | -------- |
| Holiday calendar misses current year → still show peak + surface warning | ✅ Handled | `menu.rs:402-422` asserts `warnings == ["Warning: holiday calendar does not cover 2026"]` and `state == "🔴 DS 2×"`; `ui_model.rs:346-357` asserts warning line formatting; `bundled_overlays_add_no_warning_lines` `menu.rs:392-400` |
| Settings open + Quit → clean exit | ✅ Code-path verified, unobserved | `main.rs:17-25` only hides Settings; `main.rs:55-62` prevents exit only for `code: None`; `menu.rs:308` calls `app.exit(0)`; no GUI input to click Quit (documented) |
| No snapshot on first run → bundled data, menu says so | ✅ Handled | `app_state.rs:86-92` selects `bundled`/`ActiveSource::Bundled`; test `app_state.rs:298-311`; `ui_model.rs:339-342` asserts `"Source: bundled snapshot · last sync never synced"` |
| Sync in flight + click Sync Catalog → disabled, not queued | ✅ Handled | `menu.rs:131-140,238-244`; test `menu.rs:456-463`; single-flight `sync_job.rs:172-184` |
| Tray cannot render emoji → fallback icon communicates peak via color | ✅ Selection tested, trigger unobserved | `tray.rs:44-79` + tests `tray.rs:203-216`; PNG centers decode to green/red; the actual tauri#12060 failure did not reproduce (spike in `design.md:6`, `smoke.md:247-259`) |

---

## GUI Evidence Judgment

The host was locked (`CGSSessionScreenIsLocked=1`, `evidence/session-lock.txt`) and
the session had no usable Accessibility grant (`evidence/gui-permission-blocked.txt`,
`evidence/gui-driver-locked.txt`). Native menu clicks, a painted settings
screenshot, and a real `set_title` failure could not be observed. Judgment per
affected AC:

| AC | What is unobservable | Compensating seam / artifact | Decision |
| -- | -------------------- | ---------------------------- | -------- |
| MENU-02 | Five native check items, click persistence | `action_state` pure projection `menu.rs:131-140`; assertions on count, checked state and labels `menu.rs:424-454`; persistence reload `app_state.rs:392-404` | Non-blocking: the item list/check state is derived from the asserted projection and consumed verbatim by `build` (`menu.rs:184-197`); the click path is a three-line handler |
| MENU-05 | Sync item disabled state, native menu open | `sync_enabled` asserted for both flags `menu.rs:456-463`; `build` passes it to `MenuItem::with_id` `menu.rs:238-244` | Non-blocking: seam → constructor is direct and compile-checked |
| TRAY-02 / SYNCA-02 | Live badge and Acknowledge click | Badge state/ack logic tested `app_state.rs:430-463`; Acknowledge visibility seam + test `menu.rs:465-474` | Non-blocking: badge string and state are independently asserted; click is a direct handler |
| SET-01 | Painted form | DOM probe from the real packaged webview `evidence/settings-render-probe.log:6`: all six controls present, non-zero layout, persisted values; `settings.js:172-188` maps them | Non-blocking: the AC says "show", and the probe proves the rendered DOM + values; only pixels are missing because WebKit does not paint a hidden (locked) page |
| PLAT-02 | Toggling autostart in the app | Plugin wiring `settings.rs:153-169`, `main.rs:66-71`; LaunchAgent mechanism exercised with the plugin's own `auto-launch` implementation, same app name and bundle executable (`autostart-launchagent.log`) | Non-blocking: mechanism + production wiring both evidenced; the missing leg is a UI click, not the LaunchAgent behavior |
| TRAY-04 | Real fallback trigger | Icon selection asserted `tray.rs:203-216`; PNG colors verified; fallback branch code-inspected (`tray.rs:143-161`) | Non-blocking: the triggering condition did not reproduce on this Tauri/macOS pair; the fallback is defensive and its color contract is now tested |
| SET-03 | Close, reopen, quit-with-window-open | Code paths `main.rs:17-25,55-62`, `menu.rs:355-362`; persisted prefill probe; restart path | Non-blocking for the AC, with a Minor note: hidden-window re-show does not reload, so unsaved edits can reappear |

No GUI result is fabricated: `smoke.md` marks these PARTIAL/BLOCKED and names the
compensating evidence. The new seams do not make the native rendering prove
itself, but they do anchor the state that rendering consumes. That is the
documented limitation the spec's "Visual verification" assumption
(`spec.md:40`) allows.

---

## Discrimination Sensor

Scratch worktree: `$TMPDIR/m2v2.6nslVc/wt`, detached at `cd452d3`
(`git worktree add ... HEAD --detach`). Mutations applied one at a time, each
reverted with `git checkout -- .`, then the worktree was removed with
`git worktree remove --force` and the scratch directory deleted. No `git stash`.

Real-tree `git status --porcelain` before the sensor: `?? .specs/features/m2-app-shell/validation.md`.
After cleanup: byte-identical (`diff` clean). Isolation confirmed.

| # | Mutation | File:line | Command | Killed? |
| - | -------- | --------- | ------- | ------- |
| 1 | Valid RFC 3339 anchors parse to `None` (mandatory fake-clock parser fault) | `crates/app/src/clock.rs:56` | `cargo test -p cost-guard-app clock::` | ✅ Killed (exit 101; `valid_anchor_parses_to_utc`, `offset_anchor_is_normalized_to_utc` failed) |
| 2 | Startup sync returns `false` regardless of snapshot age | `crates/app/src/timer.rs:61` | `cargo test -p cost-guard-app timer::tests::startup_syncs_even_with_a_fresh_snapshot` | ✅ Killed (exit 101 at `timer.rs:194`) |
| 3 | Degraded-data branch drops the recorded error | `crates/app/src/app_state.rs:76-82` | `cargo test -p cost-guard-app app_state::tests::bundled_failure_degrades_instead_of_aborting_startup` | ✅ Killed (exit 101 at `app_state.rs:320`, error string missing) |
| 4 | `title_changed` always returns `true` | `crates/app/src/tray.rs:38` | `cargo test -p cost-guard-app tray::tests::title_changes_only_when_the_string_differs` | ✅ Killed (exit 101 at `tray.rs:222`) |
| 5 | `fallback_icon` always returns the off-peak icon | `crates/app/src/tray.rs:73-79` | `cargo test -p cost-guard-app tray::tests::fallback_icon_follows_the_peak_state` | ✅ Killed (exit 101 at `tray.rs:203`) |
| 6 | Presentation warnings forced to empty | `crates/app/src/menu.rs:99-101` | `cargo test -p cost-guard-app menu::tests::holiday_coverage_gap_is_surfaced_in_the_presentation` | ✅ Killed (exit 101 at `menu.rs:412`) |

**Sensor depth**: lightweight, 6 targeted behavior-level faults (above the ≥5
minimum), covering all previously surviving/gapped behaviors.

**Result**: 6/6 killed — PASS ✅. The iteration-1 survivor
(`clock.rs`) is now killed at `clock.rs:56` by the new tests.

---

## Gate Check

| Command | Exit | Result |
| ------- | ---- | ------ |
| `cargo fmt --check` | 0 | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | ✅ |
| `cargo test --workspace` | 0 | ✅ 155 tests: 64 app + 87 core unit + 4 core integration; 0 failed, 0 ignored/skipped |
| `cargo check -p cost-guard-app` | 0 | ✅ |
| `bash scripts/check-core-purity.sh` | 0 | ✅ `core dependency purity OK` |
| `cd crates/app && cargo tauri build --bundles app` | 0 | ✅ `Cost Guard.app` 15.18 MiB; bundle executable SHA-256 equals `target/release/cost-guard-app` |

- **Test count before feature**: 91 (core unchanged: `git diff 311df62..HEAD -- crates/core` is empty)
- **Test count after feature**: 155
- **Delta**: +64 app tests (+17 in the iteration-2 fix loop)
- **Skipped tests**: none
- **Failures**: none

**Bundle**: `target/release/bundle/macos/Cost Guard.app` rebuilt by the verifier
at HEAD. `Contents/Info.plist` has `CFBundleIdentifier=dev.ivanseibel.costguard`
and `LSUIElement=true`; executable is arm64, ad-hoc signed, and byte-identical to
the freshly built release binary. This supersedes the iteration-1 mtime caveat:
the bundle content is confirmed current at `cd452d3`.

---

## Code Quality

| Check | Status |
| ----- | ------ |
| No features beyond what was asked | ✅ (debug fixtures exist only for T13 evidence and are `cfg(debug_assertions)`) |
| No abstractions for single-use code | ✅ except `timer::sync_requested`/`SyncTrigger::Startup`, which is a test seam not called by `main.rs` (note below) |
| No unnecessary flexibility | ✅ |
| Only touched files required | ✅ (`crates/core` unchanged; CI/docs/assets only) |
| Didn't "improve" unrelated code | ✅ |
| Matches existing patterns/style | ✅ |
| Would a senior engineer approve? | ✅ |
| Tests map to ACs and are non-shallow | ✅ spot-checked `clock`, `app_state`, `menu`, `tray`, `settings`; exact-value assertions |
| Spec-anchored outcome check | ✅ 26/26 |
| Per-layer Coverage Expectation | ✅ pure modules 1:1; Tauri glue per the approved matrix (build + smoke) |
| Every test maps to a spec AC/edge case | ✅ |
| Documented project guidelines followed | ✅ `CONTRIBUTING.md` app-shell section; core purity gate |

**Quality note**: `timer::sync_requested(SyncTrigger::Startup, ...)` is exercised
only by `timer.rs:185-204`; production startup bypasses it and calls
`sync_job::spawn` directly at `main.rs:42`. The behavior is still unconditional
and is observed in `sync-startup.log`, but the unit test does not cover the wired
call site. Recommendation: have `main.rs` consult the helper (or add a glue-level
assertion) so the test and the production path cannot drift. Non-blocking.

---

## Spec-Precision Gaps and Documented Limitations

1. **Native menu clicks unobserved** (MENU-02, MENU-05, TRAY-02, SYNCA-02).
   Compensated by pure `action_state` assertions and direct constructor use.
   **Blocking? No** — the state consumed by the menu is asserted; the click
   handlers are direct calls. `smoke.md:89-110` documents the block.
2. **Settings window never painted** (SET-01). Compensated by a DOM probe from
   the packaged webview showing all six controls, layout and persisted values
   (`settings-render-probe.log:6`). **Blocking? No** — the AC's observable state
   exists and is verified; only pixels are unavailable under the locked session.
3. **Autostart UI toggle unobserved** (PLAT-02). Compensated by production wiring
   plus the plugin's own LaunchAgent mechanism log. **Blocking? No** — the
   LaunchAgent create/remove behavior and default-off config are verified.
4. **Real `set_title` failure not reproducible** (TRAY-04). Compensated by
   icon-selection tests, decoded green/red pixels, and inspected one-time log
   flag. **Blocking? No** — the triggering condition is absent on this
   Tauri/macOS pair (spike in `design.md:6`); the fallback contract is now
   partially tested.
5. **Close/Quit/Reopen unobserved** (SET-03). Compensated by lifecycle code and
   persistence evidence. **Blocking? No**, but Minor: `show_settings`
   (`menu.rs:355-362`) does not re-invoke `settings_get`, so an unsaved edit or a
   menu profile change made while the window is hidden can leave the form stale
   on reopen. The save→close→reopen and restart paths show persisted values.
6. **Startup test seam not wired** (SYNCA-01). See Quality note. **Blocking? No**
   — `main.rs:42` is unconditional and `sync-startup.log` observes the result.

**Evidence-quality notes** (not AC gaps):

- `evidence/menu-built.log:1` was committed before `8a7bcbf`; it lacks the
  `peak=`/`warnings=` fields now emitted by `menu.rs:146`. The smoke text
  (`smoke.md:95-97`) describes the current log format, so that one claim is
  ahead of its artifact. The warning behavior itself is test-asserted
  (`menu.rs:402-422`) and no-warning-on-bundled-data is asserted at
  `menu.rs:392-400`.
- `fake-clock-peak-exit.log:7` lacks the ` | {body}` suffix emitted by current
  `timer.rs:134`, and both fake-clock logs predate the `peak=`/`warnings=`
  fields. They remain valid for the one-notification-per-transition claim, which
  is what NOTIF-01..03 require; they are not evidence of the newer log format.

---

## Requirement Traceability Update (proposed; `spec.md` not modified by the verifier)

| Requirement | Previous Status | Proposed Status |
| ----------- | --------------- | --------------- |
| APP-01 | Pending | ✅ Verified |
| APP-02 | Pending | ✅ Verified |
| APP-03 | Pending | ✅ Verified |
| TRAY-01 | Pending | ✅ Verified |
| TRAY-02 | Pending | ✅ Verified |
| TRAY-03 | Pending | ✅ Verified |
| TRAY-04 | Pending | ✅ Verified (trigger documented) |
| MENU-01 | Pending | ✅ Verified |
| MENU-02 | Pending | ✅ Verified |
| MENU-03 | Pending | ✅ Verified |
| MENU-04 | Pending | ✅ Verified |
| MENU-05 | Pending | ✅ Verified |
| MENU-06 | Pending | ✅ Verified |
| NOTIF-01 | Pending | ✅ Verified |
| NOTIF-02 | Pending | ✅ Verified |
| NOTIF-03 | Pending | ✅ Verified |
| SYNCA-01 | Pending | ✅ Verified |
| SYNCA-02 | Pending | ✅ Verified |
| SYNCA-03 | Pending | ✅ Verified |
| SYNCA-04 | Pending | ✅ Verified |
| SET-01 | Pending | ✅ Verified (paint documented) |
| SET-02 | Pending | ✅ Verified |
| SET-03 | Pending | ✅ Verified (stale-form edge documented) |
| PLAT-01 | Pending | ✅ Verified |
| PLAT-02 | Pending | ✅ Verified (UI toggle documented) |
| SEC-01 | Pending | ✅ Verified |

---

## Summary

**Overall**: ✅ Ready.

**Spec-anchored check**: 26/26 ACs matched the spec outcome; 0 blocking gaps;
5 documented observation limitations and 2 evidence-quality notes.
**Sensor**: 6/6 mutations killed (including the mandatory fake-clock parser).
**Gate**: 5/5 commands passed; 155 workspace tests; bundle rebuilt and verified.

**What works**: startup sync regardless of snapshot age; degraded startup with a
menu error instead of a panic; holiday-coverage warnings surfaced (and peak math
still correct); strict title dedupe; peak-colored fallback icons; five profile
check items; Sync disabled in flight; Acknowledge visibility; exact tray/menu
strings; one notification per transition; badge set/ack; source reporting;
settings DTO validation/persistence/prefill; LSUIElement + accessory policy;
core purity.

**Residual risk**: the only behaviors without direct observation are the native
GUI legs (menu clicks, painted settings window, autostart toggle, real
`set_title` failure, close/quit/reopen). Each has a code-level scaffold plus a
targeted seam or an out-of-process mechanism log, and `smoke.md` labels them
honestly. Two minor follow-ups are non-blocking: wire the startup-sync test to
the real call site, and reload `settings_get` when the hidden window is re-shown.

**Next steps**: ship M2; optionally schedule the two minor follow-ups for the
next app-shell touch. No fix→re-verify iteration is required.

---

## Post-validation amendment (2026-10-24)

The tray item disappeared at runtime while the process stayed alive (no crash): suspected menu-bar overflow after Snipaste added an item, or tauri#12060. The tray title is now `🟢 off` / `🔴 2×` (the menu header keeps the full `🟢 DS off` / `🔴 DS 2×`), and a 30 s watchdog re-asserts the status item when `TrayIcon::rect()` is missing, zero-sized or off-screen. The re-assert predicate is unit-tested. Historical ACs above remain as recorded.
