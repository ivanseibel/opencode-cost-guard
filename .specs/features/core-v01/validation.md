# core-v01 Validation

## Validation: core-v01 — PASS ✅

**Date**: 2026-10-24
**Spec**: `.specs/features/core-v01/spec.md`
**Design**: `.specs/features/core-v01/design.md`
**Tasks**: `.specs/features/core-v01/tasks.md`
**Diff range**: `19cd8de..HEAD` (22 commits, T1–T22; HEAD `61d5d02`)
**Verifier**: independent sub-agent (author ≠ verifier)

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 Workspace scaffold | ✅ Done | Build gate green |
| T2 Dual licenses | ✅ Done | MIT + Apache-2.0 present |
| T3 Repo docs/templates | ✅ Done | README, CONTRIBUTING, issue + PR templates |
| T4 CI matrix | ✅ Done | macOS + Ubuntu, fmt/clippy/test |
| T5 Purity check | ✅ Done | Exit 0 on real tree, exit 1 on `tauri` fixture |
| T6 Pricing windows/holidays | ✅ Done | 2× multiplier, Mon–Fri, 2026 holidays + makeup weekends |
| T7 Bundled snapshot | ✅ Done | 29 `opencode-go` models |
| T8 Go limits | ✅ Done | 28 finite limits; `beam-free` documented unlimited |
| T9 Capability ratings | ✅ Done | 29/29 roster models rated |
| T10 Catalog types/parse | ✅ Done | Roster extraction, skipped entries, deprecated filter |
| T11 Overlay loading | ✅ Done | Metadata inheritance, malformed windows, warnings |
| T12 Schedule | ✅ Done | Boundaries, weekends, holidays, next transition, coverage gap |
| T13 Config | ✅ Done | Defaults, validation, atomic save, top-level round-trip |
| T14 Snapshot store | ✅ Done | Rotation, exactly two files, truncated-as-absent |
| T15 HTTP client | ✅ Done | Trait, request builder, 10 s timeout |
| T16 Sync fallback | ✅ Done | 200/304/error/invalid/empty/304-without-snapshot |
| T17 Change detection | ✅ Done | Added, removed, price fields, capability flags |
| T18 Scoring/reasons | ✅ Done | Peak multiplier, weights, tie-breaks, closed reason set |
| T19 Recommendation engine | ✅ Done | Filters, ignored/unrated, top-3 alternatives, no-candidate |
| T20 Acceptance matrix | ✅ Done | 10 rows, hand-derived winners + codes |
| T21 Report builder | ✅ Done | State, UTC/local transition, 5 profiles, source |
| T22 Example binary | ✅ Done | Runs offline; bundled source when no snapshot |

All task checkboxes in `tasks.md` are checked. T22 is a thin wrapper verified by running the example (below).

---

## Gate Check

| Gate | Command | Exit | Result |
| ---- | ------- | ---- | ------ |
| Format | `cargo fmt --check` | 0 | ✅ |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | ✅ |
| Tests | `cargo test --workspace` | 0 | ✅ 79 passed, 0 failed, 0 ignored |
| Purity | `bash scripts/check-core-purity.sh` | 0 | ✅ `core dependency purity OK` |
| Purity self-test | `COST_GUARD_PURITY_FIXTURE=<tauri fixture> bash scripts/check-core-purity.sh` | 1 (expected) | ✅ fails on `tauri` |
| Purity syntax | `bash -n scripts/check-core-purity.sh` | 0 | ✅ |
| Demo | `cargo run -q -p cost-guard-core --example status` | 0 | ✅ complete report offline |

**Test count**: 76 unit (`crates/core/src/**`) + 3 integration (`crates/core/tests/recommendation_matrix.rs`) = **79**.
**Before feature**: 0 (base commit `19cd8de` is docs/specs only). **Delta**: +79. **Skipped**: none. **Failures**: none.

**Example output check** (`cargo run -q -p cost-guard-core --example status`, repo root, no snapshot):
- `State: off-peak` — current schedule state ✅
- `Next transition UTC: 2026-10-26T01:00:00Z` — next transition in UTC ✅
- `Next transition local: 2026-10-26 01:00:00 +00:00` — local rendering ✅
- `Data source: bundled snapshot` — bundled fallback because no current snapshot ✅ (DEMO-02)
- Five profile sections: `simple-coding`, `complex-coding`, `visual-ui`, `agentic-tools`, `large-refactor`, each with a pick and reasons ✅
- `Warnings: none` — bundled roster + 2026 calendar fully covered ✅

---

## Spec-Anchored Acceptance Criteria

**Rules applied**: spec-defined outcome re-derived per AC; assertions target values/state, not call presence; every row cites `file:line`.

**Result key**: ✅ PASS = outcome exercised and matched; ⚠️ PASS = outcome matched but with a precision/automation limitation recorded in [Spec-Precision Notes](#spec-precision-notes); ❌ = uncovered or wrong outcome.

| ID | Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| -- | ------------------------- | -------------------- | ----------------------- | ------ |
| SCAF-01 | CI runs fmt, clippy, tests on macOS and Ubuntu | Both runners; `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` | `.github/workflows/ci.yml:20` — `matrix.os: [macos-latest, ubuntu-latest]`; `:29` fmt; `:32` clippy; `:35` test | ✅ PASS (inspection; local gate exit 0) |
| SCAF-02 | Repo contains dual licenses, README build+disclaimer, CONTRIBUTING DCO+Overlay process, templates | All listed artifacts present and substantive | `LICENSE-MIT:1`, `LICENSE-APACHE:1`; `README.md:39` build, `README.md:110` non-affiliation; `CONTRIBUTING.md:35` DCO, `CONTRIBUTING.md:65` Overlays; `.github/ISSUE_TEMPLATE/holiday-refresh.yml:1`; `.github/pull_request_template.md:1` | ✅ PASS (inspection) |
| SCAF-03 | Purity check fails on `tauri`, `objc2*`, `windows-sys` direct/transitive | Exit 0 clean tree; exit non-zero on banned crate | `scripts/check-core-purity.sh:30` — regex `(tauri\|objc2\|windows-sys)([a-z0-9_-]*)`; `:34-39` — print offenders + `exit 1` | ✅ PASS (real exit 0; `tauri` fixture exit 1) |
| SCHED-01 | Instant → peak iff Mon–Fri, `[01:00,04:00)` or `[06:00,10:00)`, not a CN holiday | Peak only on listed weekdays inside half-open windows, holidays off-peak | `crates/core/src/schedule/mod.rs:274` — `assert_eq!(state.peak, expected)` over `:262-270`; holiday `:309` — `assert!(!state.peak)` | ✅ PASS |
| SCHED-02 | Boundary instants: start peak, end off-peak | 01:00/06:00 peak; 04:00/10:00 off-peak | `crates/core/src/schedule/mod.rs:274` — table cases `(1,0,0,true)`, `(4,0,0,false)`, `(6,0,0,true)`, `(10,0,0,false)` at `:262-270` | ✅ PASS |
| SCHED-03 | Weekend or makeup-weekend date → off-peak | `peak == false` | `crates/core/src/schedule/mod.rs:300` — weekend `assert!(!state.peak)`; `:319` — makeup weekend `assert!(!state.peak)` | ✅ PASS |
| SCHED-04 | Result includes next transition in UTC | Exact next instant (window end, midnight, next peak, Monday) | `crates/core/src/schedule/mod.rs:285-292` — `assert_eq!(...next_transition, 04:00/10:00)`; also `:301`, `:311`, `:329`, `:338`, `:347` | ✅ PASS |
| SCHED-05 | Pure function of (instant, rules); no clock/mutable state | Equal inputs → equal output; historic instant resolves from rules | `crates/core/src/schedule/mod.rs:379` — `assert_eq!(first, second)`; `:383` — historic `assert!(historic.peak)`; code inspection: no `Utc::now` in `state_at` | ✅ PASS |
| SCHED-06 | Uncovered year → compute without holiday exclusions + flag gap | `coverage_gap == Some(year)`, peak computed from weekday/window rules | `crates/core/src/schedule/mod.rs:365` — `assert_eq!(state.coverage_gap, Some(2031))`; `:366` — `assert!(state.peak)` | ⚠️ PASS (instant-year case; cross-year horizon note 1) |
| DATA-01 | Overlay metadata required, inherited; empty override fails naming file/entry | `MissingMetadata`/`EmptyMetadataOverride` naming file+field / entry+field; inheritance works | `crates/core/src/overlay/mod.rs:754-755` — message contains `pricing-windows.toml` + `source`; `:768-770` — entry + `last_verified`; `:800-805` — per-entry override accepted; `:720-722` — inherited source equals file source | ✅ PASS |
| DATA-02 | Overlay set defines 2× windows, holidays+makeup, per-model limits, per-model capabilities | Multiplier 2.0; exact windows; 2026 holidays; 29 rated models; 28 finite limits + `beam-free` unlimited | `crates/core/src/overlay/mod.rs:702` — `assert_eq!(provider.multiplier, 2.0)`; `:703-715` — exact `[60,240)`, `[360,600)`; `:716` — year 2026; `:717-718` — limits ≥28, capabilities ≥29; `:743-744` — no warnings over bundled roster; `data/limits.toml:124` documents `beam-free` unlisted | ✅ PASS |
| DATA-03 | Malformed window (end ≤ start, outside 00:00–24:00) fails naming entry | `MalformedWindow` names entry + reason | `crates/core/src/overlay/mod.rs:779-780` — contains `deepseek` + `end must be after start`; `:786-787` — contains `00:00-24:00` | ✅ PASS |
| DATA-04 | Unrated catalog models → warning listing them | One `UnratedModel(id)` per unrated model | `crates/core/src/overlay/mod.rs:813` — `assert_eq!(warnings, vec![Warning::UnratedModel("model-b")])`; `:815-817` — exact message | ✅ PASS |
| DATA-05 | Holiday calendar misses current year → warning naming year | `UncoveredHolidayYear(year)` | `crates/core/src/overlay/mod.rs:825-826` — `assert_eq!(... , vec![Warning::UncoveredHolidayYear(2027)])`; `:828` — 2026 empty | ✅ PASS |
| SYNC-01 | Successful sync stores current, one previous, source/ETag/lastSyncedAt | `Updated{Remote}`; current fields recorded; previous rotation | `crates/core/src/sync/mod.rs:256-261` — `SyncStatus::Updated { source: Remote }`; `:264-269` — model id, source, ETag, RFC 3339 timestamp; `crates/core/src/sync/snapshot.rs:139-144` — current/previous exact + exactly two files | ✅ PASS |
| SYNC-02 | 304 → `NotModified`, snapshots unchanged | Status `NotModified`; bytes and current snapshot identical; ETag sent | `crates/core/src/sync/mod.rs:295-297` — `assert_eq!(outcome.status, NotModified)`, bytes unchanged, current equals seeded; `:299-301` — call carried `"old-etag"` | ✅ PASS |
| SYNC-03 | Fetch failure → last valid, then bundled; report active source | `Fallback{LastValid}` with reason when snapshot exists; `Fallback{Bundled}` otherwise | `crates/core/src/sync/mod.rs:318-321` — LastValid + reason contains `boom`; `:344-348` — 500 → LastValid; `:441-444` — offline + no snapshot → Bundled | ✅ PASS |
| SYNC-04 | Invalid schema → do not overwrite last valid | `Fallback{LastValid}`; current bytes unchanged | `crates/core/src/sync/mod.rs:371-379` — reason contains `invalid payload`; bytes equal `before`; current equals seeded | ✅ PASS |
| SYNC-05 | Extract exactly `opencode-go`; skip/report missing required fields; sync succeeds | Only roster provider; skipped list with missing names; `Updated{Remote}` + warning | `crates/core/src/catalog/mod.rs:228-242` — 1 model, fields, other provider excluded; `:274-290` — exact skipped id + missing list + reason; `crates/core/src/sync/mod.rs:466-482` — Updated + 1 warning, 1 stored model | ✅ PASS |
| SYNC-06 | Diff lists additions, removals, per-field price old→new, capability flag changes | Exact enum variants and old/new values | `crates/core/src/sync/changes.rs:171-176` — Added; `:184-189` — Removed; `:201-223` — Input/Output/CacheRead old→new; `:235-257` — ToolCall/Attachment/Reasoning old→new | ✅ PASS |
| SYNC-07 | 10-second timeout per request | Default and custom timeout reach agent config | `crates/core/src/sync/http.rs:155-158` — default `10s` + `agent.config().timeouts().global == Some(10s)`; `:163-166` — custom 3 s propagates | ✅ PASS |
| SYNC-08 | Atomic persistence; partial write not valid on load | Truncated/invalid file → absent; temp+rename; no temp left | `crates/core/src/sync/snapshot.rs:156-157` — both truncated/invalid → `None`; `:179-190` — no `.tmp` after save, stale temp not adopted; implementation `:52-61` temp+rename | ✅ PASS |
| SYNC-09 | Injectable HTTP client; tests 200/304/error/invalid without network | Trait usable via stub; scripted responses | `crates/core/src/sync/http.rs:170-179` — `&dyn HttpClient` stub returns exact status/body/etag; `crates/core/src/sync/mod.rs:178-188` — stub implements trait and records calls | ✅ PASS |
| SYNC-10 | 304 with no valid snapshot → treat as failure, fallback | `Fallback{Bundled}`, reason mentions 304, no current file created | `crates/core/src/sync/mod.rs:421-428` — `source == Bundled`, reason contains `304`, `current_bytes(...).is_none()` | ✅ PASS |
| REC-01 | One top pick, non-empty reasons, ≤3 alternatives from snapshot/schedule/overlays/config | Pick present; reasons non-empty; exactly 3 alternatives; no-candidate none | `crates/core/src/recommend/mod.rs:435-447` — `pick.model_id == "a"`, `!reasons.is_empty()`, `alternatives.len() == 3`, `no_candidate.is_none()` | ✅ PASS |
| REC-02 | Visual/UI only vision-capable candidates | Pick has vision; text-only absent from alternatives | `crates/core/src/recommend/mod.rs:464-471` — `pick.model_id == "vision"`; all alternatives `!= "text-only"` | ✅ PASS |
| REC-03 | Agentic/Tools only tool-capable candidates | Pick has tools; no-tools absent | `crates/core/src/recommend/mod.rs:488-495` — `pick.model_id == "tools"`; all alternatives `!= "no-tools"` | ✅ PASS |
| REC-04 | Peak → peak prices (off-peak × multiplier); else off-peak | Effective input/output ×2 at peak; 0 % change off-peak; peak winner cost = 2× off-peak | `crates/core/src/recommend/score.rs:337-343` — off-peak 0.15/0.6, peak 0.3/1.2; `crates/core/tests/recommendation_matrix.rs:263-269` — `peak_pick.cost_index == 2.0 * off_pick.cost_index` | ✅ PASS |
| REC-05 | No candidate passes filters → `NoRecommendation` naming filter | `pick == None`, no alternatives, `no_candidate.filter` is the failing filter, message names it | `crates/core/src/recommend/mod.rs:579-653` — EmptyRoster, Ignored, Unrated, Vision, Tools; `:635-636` message contains `image input`; `:653` contains `tool calls` | ✅ PASS |
| REC-06 | Config `ignoredModels` never in pick or alternatives | Ignored `opencode-go/a` absent; `b` wins, alternatives `[c]` | `crates/core/src/recommend/mod.rs:520-534` — `pick == "b"`, alternatives exactly `["c"]`, all `!= "a"` | ✅ PASS |
| REC-07 | Unrated models excluded from picks and alternatives | `rated` wins; only `also-rated` alternative; `unrated` absent | `crates/core/src/recommend/mod.rs:555-569` — pick `rated`, alternatives `["also-rated"]`, all `!= "unrated"` | ✅ PASS |
| REC-08 | Score tie → lower effective price; price tie → stable model ID | `cheap` before `pricey`; `a-model` before `b-model` | `crates/core/src/recommend/score.rs:496-502` — order `["cheap","pricey"]`; `:511-517` — order `["a-model","b-model"]` | ✅ PASS |
| REC-09 | Deterministic output incl. alternative ordering | Identical recommendation equality; rendering equality | `crates/core/src/recommend/mod.rs:705-709` — `assert_eq!(first, second)`, pick `a`; `crates/core/src/report/mod.rs:267-268` — report and string equal | ✅ PASS |
| REC-10 | Each reason pairs closed-set code with human-readable string | Exact code sequence; every text non-empty and not the code | `crates/core/src/recommend/mod.rs:667-685` — exact `[PeakWindow, StrongFit, CheapCost, CheapCache, AllowanceHeadroom, ContextFit]`; texts non-empty and `!= code.as_str()` | ✅ PASS |
| REC-11 | 5 profiles × peak/off-peak match fixture winners + codes | 10 exact rows; winners and full reason-code sequences | `crates/core/tests/recommendation_matrix.rs:238-247` — `pick.model_id == row.winner`, `codes == row.codes`, texts non-empty; `:216-221` — 10 rows, 2 per profile; independently re-derived all 10 rows from `design.md` + bundled data — exact match | ✅ PASS |
| CFG-01 | No config file → defaults | `autostart=false`, notifications on, `simple-coding`, 6 h, empty ignored; no file written | `crates/core/src/config/mod.rs:270-278` — every default asserted; `Config::default()` equal; file absent | ✅ PASS |
| CFG-02 | Invalid values → fail naming field; file unmodified | Error contains field name; bytes unchanged | `crates/core/src/config/mod.rs:295-302` — `expect_err`, message contains `syncIntervalHours`/`defaultProfile`/`ignoredModels`, before == after | ⚠️ PASS (semantic invalid values; type-mismatch precision note 2) |
| CFG-03 | Save atomic; unknown JSON fields round-trip | No temp left; camelCase fields; unknown top-level keys preserved across save/reload | `crates/core/src/config/mod.rs:312-323` — file exists, no `.tmp`, exact JSON fields; `:352-354` — `futureField`/`unknownFlag` survive save and reload | ⚠️ PASS (top-level only; nested precision note 3) |
| CFG-04 | Validate interval ≥1, profile among 5, ignored IDs as `provider/model` | 1 OK, 0 fails; all 5 profiles parse; malformed IDs rejected | `crates/core/src/config/mod.rs:362-366` — 1 OK, 0 fails naming field; `:375-380` — six invalid IDs rejected; `:382-386` — all profiles round-trip, unknown fails; invalid profile at `:287` | ✅ PASS |
| DEMO-01 | Example prints state, next transition UTC+local, 5 profile recommendations | Rendered values present and correct | `crates/core/src/report/mod.rs:165-218` — peak state, 04:00 UTC, local line, 5 profiles, reasons, source; manual run exit 0 shows all fields | ✅ PASS |
| DEMO-02 | No synced snapshot → bundled snapshot and says so | `ActiveSource::Bundled` + `Data source: bundled snapshot` | `crates/core/examples/status.rs:31-39` — `load_current()` else bundled; manual run from repo root printed `Data source: bundled snapshot` | ⚠️ PASS (manual, no automated assertion; note 4) |

**Status**: ✅ All 41 ACs matched; 0 uncovered. ⚠️ 4 rows carry precision/automation notes; 6 notes total (see below).

### Payload / Conjunction Rule

Every AC row asserts payload values or state, not merely that a call occurred:
- Snapshot tests assert model IDs, source URL, ETag, RFC 3339 timestamp, and exact file names — not just `save()`.
- Sync tests assert `SyncStatus` variants **and** active-source labels, reasons, stored bytes, and ETag propagation.
- Change detection asserts exact enum variants with `old`/`new` values for all three price fields and all three capability flags.
- Recommendation tests assert winner ID, exact alternative list/order, exact reason-code sequence, and non-empty human text.
- Config tests assert every default value, field names in errors, file bytes before/after, and unknown-key preservation.
- Overlay tests assert exact multiplier, exact window minutes, exact warning enums/messages, and metadata inheritance/override values.
- No AC is satisfied by a bare `assert!(result.is_ok())` or call-presence check.

---

## Discrimination Sensor

**Depth**: P0-full (6 behavior-level mutations; core pricing/sync/recommendation paths).
**Isolation**: `mktemp -d "$TMPDIR/cg-sensor-XXXXXX"` → `git worktree add <dir> HEAD --detach`; mutations applied only in the scratch; shared `CARGO_TARGET_DIR` under `$TMPDIR`; source reverted with `git checkout -- <file>` between mutations; scratch removed with `git worktree remove --force`. **No `git stash` used.**

| # | Mutation | File:line | Command | Result | Killed? | Restored? |
| - | -------- | --------- | ------- | ------ | ------- | --------- |
| 1 | Window end condition `< end` → `<= end` (end boundary becomes peak) | `crates/core/src/schedule/mod.rs:112` | `cargo test -p cost-guard-core boundary_instants` | exit 101; `boundary_instants_follow_half_open_rule` FAILED at `:274` | ✅ Killed | ✅ worktree reverted |
| 2 | 304 guard inverted: `304 if stored.is_some()` → `stored.is_none()` (wrong 304 source selection) | `crates/core/src/sync/mod.rs:81` | `cargo test -p cost-guard-core not_modified` | exit 101; 2 tests FAILED (`not_modified_without_snapshot...`, `not_modified_leaves...`) | ✅ Killed | ✅ worktree reverted |
| 3 | Vision hard filter inverted: `capability.vision` → `!capability.vision` | `crates/core/src/recommend/mod.rs:152` | `cargo test -p cost-guard-core visual_ui_only` | exit 101; `visual_ui_only_considers_vision_capable_models` FAILED at `:465` | ✅ Killed | ✅ worktree reverted |
| 4 | Peak multiplier removed: `factor = if peak { multiplier } else { 1.0 }` → `factor = 1.0` | `crates/core/src/recommend/score.rs:66` | `cargo test -p cost-guard-core peak_multiplier_doubles` | exit 101; `peak_multiplier_doubles_the_effective_price` FAILED at `:342` | ✅ Killed | ✅ worktree reverted |
| 5 | Capability diff class removed: deleted `push_capability_changes(...)` call | `crates/core/src/sync/changes.rs:72` | `cargo test -p cost-guard-core detects_capability_flag_changes` | exit 101; `detects_capability_flag_changes` FAILED at `:235` | ✅ Killed | ✅ worktree reverted |
| 6 | Unrated-model warning suppressed: `if !rated.contains(...)` → `if false && !rated.contains(...)` | `crates/core/src/overlay/mod.rs:222` | `cargo test -p cost-guard-core warns_about_unrated_models` | exit 101; `warns_about_unrated_models` FAILED at `:813` | ✅ Killed | ✅ worktree reverted |

**Result**: 6/6 mutants killed — PASS ✅. The tests discriminate all five required behavior classes plus DATA-04 warning behavior.

**Isolation verification**: real-tree `git status --porcelain` was empty before the sensor and empty after cleanup; `git worktree list` shows only the main worktree; scratch and shared target removed. No real source file was modified.

---

## Spec-Precision Notes

These are precision/automation limitations, not uncovered ACs. No AC’s tested outcome is wrong.

1. **SCHED-04/SCHED-06 — cross-year transition horizon (Minor/edge).** `state_at` flags a coverage gap only for the instant’s year (`crates/core/src/schedule/mod.rs:51`, `:80-97`). A probe at `2026-12-31T10:30Z` returned `next_transition = 2027-01-01T01:00Z` with `coverage_gap = None`, although the calendar covers only 2026. If 2027-01-01 is a holiday, the reported transition is wrong and unflagged. The spec’s Goal mentions year rollover; the AC text only covers the instant’s year. Recommend either flagging the next transition’s year or documenting that the calendar must cover the transition horizon.
2. **CFG-02 — type-mismatched values (Minor).** Semantic invalid values name the field and leave the file untouched (`crates/core/src/config/mod.rs:295-302`). A wrong JSON type (`{"syncIntervalHours": "6"}`) returns `ConfigError::Parse` with the file path and line/column, not the field name (observed via the example, exit 1). Safety contract holds; literal “naming the offending field” is weaker for type errors.
3. **CFG-03 — nested unknown fields (Minor).** Unknown **top-level** keys round-trip (`crates/core/src/config/mod.rs:102-103`, `:352-354`). Unknown keys nested inside `notifications` are dropped because `Notifications` has no flattened map (`:76-81`). Design says “unknown top-level keys”; spec says “unknown JSON fields”. Recommend amending the spec to “top-level” or flattening nested structs.
4. **DEMO-02 — manual-only evidence (Minor/process).** The example’s bundled fallback is verified by inspection (`crates/core/examples/status.rs:31-39`) and by running `cargo run … --example status` from the repo root (`Data source: bundled snapshot`), but no automated assertion exercises the example binary. `tasks.md` T22 explicitly scopes tests as “none (thin wrapper)”, so this is an accepted process precision gap; a smoke test would make the spec’s “automated test per AC” success criterion literal.
5. **SYNC-05 — required-field breadth (Minor/coverage).** The skip test asserts 5 of the 8 required fields (`name`, `cost.input`, `cost.cache_read`, `limit.context`, `tool_call`; `crates/core/src/catalog/mod.rs:274-290`). `id`, `cost.output`, and `attachment` are checked by `crates/core/src/catalog/mod.rs:113-130` but have no dedicated missing-field assertion. Behavior is code-verified, not test-verified for those three.
6. **SYNC-03 / DATA-01 — error-variant and file breadth (Minor/coverage).** Timeout and unreadable-body errors share the transport fallback arm (`crates/core/src/sync/mod.rs:74-77`) but only `HttpError::Transport` is scripted. Missing-metadata failure is exercised for `pricing-windows.toml` only; the same check is applied to all four Overlay files (`crates/core/src/overlay/mod.rs:388`, `:450`, `:505`, `:565`). No behavioral defect observed.

**Naming note (accepted, not a gap):** SYNC-01 says `lastSyncedAt`; the implementation and design schema use `fetched_at` (`crates/core/src/catalog/mod.rs:43`, asserted at `crates/core/src/sync/mod.rs:269`). Same value and purpose; design-level rename.

---

## Reported Deviations — Judgment

| Reported deviation | Judgment | Rationale |
| ------------------ | -------- | --------- |
| Overlay warnings exposed via `Overlays::validate(roster, year)` instead of `load_dir` | **ACCEPT** | `load_dir` has no roster or current year; DATA-04/DATA-05 need both. The deviation is documented in `crates/core/src/overlay/mod.rs:200-202`; warnings are tested (`:813`, `:825-826`) and used by the report (`crates/core/src/report/mod.rs:55-59`). |
| `build_report` takes snapshot + config (design sketch omitted them) | **ACCEPT** | Required to select a synced snapshot for DEMO-02 and to inject fixtures without hidden I/O. Documented at `crates/core/src/report/mod.rs:8-11`; determinism and five-profile rendering are tested (`:165-218`, `:267-268`). |
| `SyncEngine::bundled()` accessor | **ACCEPT** | Exposes the bundled fallback snapshot to callers selecting data after `Fallback{Bundled}`. No in-tree caller yet (small API surface; not harmful). Not a spec gap: `sync()` still reports the active source per SYNC-03. |
| `tempfile` dev-dependency landed in T13 | **ACCEPT** | Dev-only dependency; first used by config tests in T13, then snapshot tests. Task ordering detail, not a behavior or licensing change. |
| `Cargo.lock` ignored by git (library-only stage) | **ACCEPT** | Workspace has one library crate and no binary target yet; `.gitignore:5` documents that the app crate will decide lockfile policy later. No spec requirement mandates a committed lockfile. |

---

## Edge Cases

| Edge case | Result | Evidence |
| --------- | ------ | -------- |
| 00:59:59 / 01:00:00 / 03:59:59 / 04:00:00 / 05:59:59 / 06:00:00 / 09:59:59 / 10:00:00 UTC | ✅ | `crates/core/src/schedule/mod.rs:262-274` |
| Feb 29 on a peak weekday | ✅ | `crates/core/src/schedule/mod.rs:351-357` |
| CN public holiday on a weekday → whole date off-peak | ✅ | `crates/core/src/schedule/mod.rs:305-311` |
| Mirror 200 with zero `opencode-go` models → invalid → fallback | ✅ | `crates/core/src/sync/mod.rs:383-405` |
| Partially written/truncated snapshot ignored as invalid | ✅ | `crates/core/src/sync/snapshot.rs:148-157` |
| Equal score + equal price → order by model ID | ✅ | `crates/core/src/recommend/score.rs:506-517` |
| Year rollover | ⚠️ | Not tested; cross-year coverage flag precision note 1 |

---

## Code Quality

| Principle | Status | Notes |
| --------- | ------ | ----- |
| Minimum code | ✅ | No speculative features; one small unused accessor (`SyncEngine::bundled`) noted and accepted. |
| Surgical changes | ✅ | Feature commits are scoped T1–T22; no unrelated code edits. |
| No scope creep | ✅ | No Tauri/app shell, no SQLite, no network tests, consistent with out-of-scope table. |
| Matches patterns | ✅ | Module-per-concern, `thiserror`, typed errors, co-located unit tests, integration fixture. |
| Spec-anchored outcome check | ✅ | All 41 rows target values/state; 4 rows carry precision notes, none wrong. |
| Per-layer coverage expectation | ✅ | Domain logic 1:1 to ACs; REC-11 matrix covers 5×2; HTTP helper unit-tested; CI/docs/data by gate/inspection. |
| Every test maps to a requirement/edge/done-when | ✅ | 79 tests map to ACs, listed edge cases, or task done-when (snapshot rotation, atomic save, data coverage). |
| Documented project guidelines followed | ✅ | `CONTRIBUTING.md` (hermetic tests, purity, typed errors, DCO); no separate coding-principles file in repo. |

**Test quality spot-check**: the recommendation matrix and scoring tests assert exact winners, exact reason-code vectors, exact score arithmetic, and 2× peak cost — non-shallow. Sensor mutants confirm the assertions bite.

---

## Requirement Traceability

All 41 requirements are verified as of this report:

- Scaffold: SCAF-01, SCAF-02, SCAF-03 — ✅
- Schedule: SCHED-01..06 — ✅ (SCHED-06 instant-year case; cross-year precision note)
- Overlays: DATA-01..05 — ✅
- Sync: SYNC-01..10 — ✅
- Recommendation: REC-01..11 — ✅
- Config: CFG-01..04 — ✅ (CFG-02/03 precision notes)
- Demo: DEMO-01, DEMO-02 — ✅ (DEMO-02 manual)

`spec.md` statuses were not edited here because this verification was scoped to writing `validation.md` only (no code/test/spec modifications). Recommended new status for all 41: **Verified**.

---

## Summary

**Overall**: ✅ Ready.

**Spec-anchored check**: 41/41 ACs matched; 0 uncovered; 6 precision/automation notes.
**Sensor**: 6/6 mutations killed.
**Gate**: 4/4 gates passed (fmt, clippy, tests, purity), plus the example run.

**What works**:
- Pricing schedule is correct at all eight time boundaries, weekends, holiday weekdays, makeup weekends, leap day, and next-transition cases; a synthetic calendar keeps tests independent of seed drift.
- Sync survives 200, 304, network error, 500, invalid payload, empty roster, and 304-without-snapshot; reports active source and preserves the last valid snapshot.
- Snapshot persistence rotates exactly two files atomically, and truncated files are treated as absent.
- Recommendation engine applies vision/tools/ignored/unrated filters, scores deterministically, caps alternatives at three, and matches the hand-derived 5×2 matrix exactly (independently re-derived during verification).
- Config defaults, validation, atomic save, and top-level unknown-field round-trip work; invalid files are never rewritten.
- Demo runs fully offline and shows state, UTC/local transition, five profiles, reasons, source, and warnings.

**Issues found** (precision, no functional AC failure):
1. Cross-year transition can be computed from an uncovered calendar without a flag — recommend a follow-up (note 1).
2. Type-mismatched config errors name path/line/column, not the field — recommend `serde_path_to_error` or scoping CFG-02 to validation values (note 2).
3. Nested unknown config keys do not round-trip — align spec wording or flatten nested structs (note 3).
4. DEMO-02 is manual-only — optional smoke test (note 4).
5. SYNC-05 skip-field assertion breadth and SYNC-03/DATA-01 error/file breadth — strengthen if desired (notes 5–6).

**Next steps**: Feature can close as PASS. Route notes 1–3 to a small follow-up (or spec amendment) if literal AC coverage is required; none blocks the v0.1 core behavior.
