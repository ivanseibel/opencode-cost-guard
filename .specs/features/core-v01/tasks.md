# Core v0.1 Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.** Do not search for skill files by filesystem path. The skill is the source of truth for the full flow (per-task cycle, sub-agent delegation, adequacy review, Verifier, discrimination sensor).

**If the skill cannot be activated, STOP and tell the user - do not proceed without it.**

---

**Design**: `.specs/features/core-v01/design.md`
**Status**: Draft

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec - confirm before Execute. Guidelines found: none in the repo (greenfield); the spec Success Criteria mandate an automated test per AC and zero network in tests, so strong defaults apply.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Domain logic (catalog parse, overlays, schedule, config, snapshots, sync, changes, scoring, engine, report) | unit | All branches; 1:1 to spec ACs; every listed edge case | `crates/core/src/**/*.rs` (`#[cfg(test)]`) | `cargo test -p cost-guard-core` |
| Acceptance / spec matrix | integration | REC-11 matrix: 5 profiles × peak/off-peak, expected winners and reason codes | `crates/core/tests/*.rs` | `cargo test -p cost-guard-core --tests` |
| HTTP request helper | unit | URL and `If-None-Match` presence/absence; timeout config | `crates/core/src/sync/http.rs` (`#[cfg(test)]`) | `cargo test -p cost-guard-core` |
| Curated data files | none - validated by loader tests | Metadata present; structurally valid; roster coverage | `data/**/*.toml`, `data/bundled/*.json` | build gate |
| Workspace / CI / docs / licenses / scripts | none - build gate only | Build gate only | `Cargo.toml`, `.github/workflows/ci.yml`, `README.md`, `CONTRIBUTING.md`, `LICENSE-*`, `scripts/check-core-purity.sh` | build gate |
| Example binary | none - report logic unit-tested | Compiles and runs offline | `crates/core/examples/status.rs` | build gate |

## Gate Check Commands

> Generated from codebase - confirm before Execute.

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | After tasks with unit tests only | `cargo test -p cost-guard-core` |
| Full | After tasks with integration tests | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |
| Build | After config/data/entity-only tasks and at phase completion | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` (`bash scripts/check-core-purity.sh` joins the build gate once T5 lands) |

---

## Execution Plan

Phases are ordered and run sequentially - each phase completes before the next begins, and tasks within a phase execute in order.

### Phase 1: Foundation

```
T1 → T2 → T3 → T4 → T5
```

### Phase 2: Data seeds

```
T6 → T7 → T8 → T9
```

### Phase 3: Catalog and Overlays

```
T10 → T11
```

### Phase 4: Schedule and Config

```
T12 → T13
```

### Phase 5: Sync

```
T14 → T15 → T16 → T17
```

### Phase 6: Recommendation and Report

```
T18 → T19
T19 → T20
T19 → T21
T21 → T22
```

---

## Task Breakdown

### T1: Workspace scaffold

**What**: Create the Cargo workspace (`crates/core` member), pin the toolchain, and initialize the core crate skeleton with empty module declarations.
**Where**: `Cargo.toml`
**Depends on**: None
**Reuses**: none
**Requirement**: SCAF-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `cargo check --workspace` succeeds
- [x] `rust-toolchain.toml` pins a current stable toolchain; edition 2024 set in `crates/core/Cargo.toml`
- [x] `.gitignore` ignores `target/`
- [x] Gate passes: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

**Tests**: none (workspace config layer, build gate only)
**Gate**: build
**Commit**: `build: scaffold cargo workspace and core crate`

---

### T2: Dual licenses

**What**: Add `LICENSE-MIT` and `LICENSE-APACHE` with the decided dual-license texts.
**Where**: `LICENSE-MIT`
**Depends on**: T1
**Reuses**: none
**Requirement**: SCAF-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Both license files exist with standard texts
- [x] Gate passes: build gate command from the Gate Check Commands table

**Tests**: none (legal text, build gate only)
**Gate**: build
**Commit**: `chore: add dual MIT/Apache-2.0 licenses`

---

### T3: Repo docs and templates

**What**: Write `README.md` (what/why, build-from-source, unsigned-build Gatekeeper note, non-affiliation disclaimer), `CONTRIBUTING.md` (dev setup, DCO sign-off, Overlay update procedure), and issue/PR templates.
**Where**: `README.md`
**Depends on**: T2
**Reuses**: `docs/brief.md`, `docs/mvp.md`, `docs/backlog.md`, `CONTEXT.md` as source material
**Requirement**: SCAF-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] README covers build, test, run-the-demo, and the non-affiliation disclaimer
- [x] CONTRIBUTING covers DCO sign-off and how to update each Overlay file
- [x] `.github/ISSUE_TEMPLATE/` (holiday-refresh template included) and PR template exist
- [x] Gate passes: build gate command

**Tests**: none (docs, build gate only)
**Gate**: build
**Commit**: `docs: add README, CONTRIBUTING and templates`

---

### T4: CI matrix

**What**: Add the GitHub Actions workflow running fmt, clippy, and tests on macOS and Ubuntu.
**Where**: `.github/workflows/ci.yml`
**Depends on**: T3
**Reuses**: none
**Requirement**: SCAF-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Workflow triggers on push and pull_request with `macos-latest` and `ubuntu-latest` matrix
- [x] Steps run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [x] `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` passes locally

**Tests**: none (CI config, build gate only)
**Gate**: build
**Commit**: `ci: add fmt, clippy and test matrix`

---

### T5: Core purity check

**What**: Add a script that fails when `cost-guard-core`'s dependency tree includes `tauri`, `objc2*` or `windows-sys`, with a self-test mode fed by a fixture list; wire it into CI.
**Where**: `scripts/check-core-purity.sh`
**Depends on**: T4
**Reuses**: `cargo tree` output
**Requirement**: SCAF-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Script exits 0 on the current tree and exits non-zero when fed a fixture list containing `tauri`
- [x] CI workflow gains a purity step using the script
- [x] `bash -n scripts/check-core-purity.sh` is clean
- [x] Gate passes: build gate command plus `bash scripts/check-core-purity.sh`

**Tests**: none (script layer, build gate only)
**Gate**: build
**Commit**: `ci: enforce core dependency purity`

---

### T6: Seed pricing windows and CN holidays

**What**: Author `pricing-windows.toml` (DeepSeek: multiplier 2.0, Mon–Fri, `[01:00,04:00)` and `[06:00,10:00)`, UTC) and `holidays-cn.toml` (2026 official holidays and makeup weekends), each with `source` and `last_verified`.
**Where**: `data/pricing-windows.toml`
**Depends on**: T1
**Reuses**: session research (DeepSeek pricing page); official CN 2026 holiday notice via web
**Requirement**: DATA-02

**Tools**:

- MCP: NONE
- Skill: NONE (web fetch allowed for the official notice)

**Done when**:

- [x] Both files parse as TOML (`python3 -c "import tomllib,..."`)
- [x] `source` holds a real, citable URL; `last_verified` is today's date
- [x] 2026 holiday and makeup-weekend lists match the official notice, source recorded
- [x] Gate passes: build gate command

**Tests**: none (curated data, validated by T11 loader tests)
**Gate**: build
**Commit**: `data: seed pricing windows and CN holidays`

---

### T7: Bundled roster snapshot

**What**: Fetch the live mirror, extract the `opencode-go` provider into `data/bundled/opencode-go.json` using the snapshot schema (schema, source, etag, fetched_at, models).
**Where**: `data/bundled/opencode-go.json`
**Depends on**: T6
**Reuses**: `https://models.opencode.ai/api.json`
**Requirement**: SYNC-05

**Tools**:

- MCP: NONE
- Skill: NONE (curl + jq or a one-off script allowed)

**Done when**:

- [x] Snapshot parses as JSON and matches the design schema
- [x] Contains only provider `opencode-go` models, deprecated entries excluded
- [x] `fetched_at` and `source` recorded; model count documented in the commit body
- [x] Gate passes: build gate command

**Tests**: none (data file, parsed by T10/T16 tests)
**Gate**: build
**Commit**: `data: add bundled opencode-go snapshot`

---

### T8: Seed Go monthly limits

**What**: Author `limits.toml` with the published per-model monthly USD limits for the Go Roster, using the ids from the bundled snapshot.
**Where**: `data/limits.toml`
**Depends on**: T7
**Reuses**: OpenCode Go docs table; `data/bundled/opencode-go.json`
**Requirement**: DATA-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Every snapshot model id appears with `monthly_usd`, or is explicitly listed as unlisted with a note
- [x] `source` and `last_verified` present and citable
- [x] File parses as TOML
- [x] Gate passes: build gate command

**Tests**: none (curated data, validated by T11)
**Gate**: build
**Commit**: `data: seed Go monthly limits`

---

### T9: Seed roster capability ratings

**What**: Author `capabilities.toml` with tier, flags, context and per-profile scores for every snapshot model id.
**Where**: `data/capabilities.toml`
**Depends on**: T8
**Reuses**: models.dev metadata, public model cards and benchmarks
**Requirement**: DATA-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Every snapshot model id has an entry with `tier`, `vision`, `tools`, `reasoning`, `context`
- [x] Profile scores (0–10) provided for the models expected to win any matrix row; others may inherit tier base
- [x] `notes` explains non-obvious ratings; `source` + `last_verified` present
- [x] Gate passes: build gate command

**Tests**: none (curated data, validated by T11)
**Gate**: build
**Commit**: `data: seed roster capability ratings`

---

### T10: Catalog types and mirror parsing

**What**: Implement catalog domain types and `parse_mirror` extracting the `opencode-go` Roster, filtering deprecated entries and reporting skipped ones.
**Where**: `crates/core/src/catalog/mod.rs`
**Depends on**: T1
**Reuses**: `serde`, `serde_json`
**Requirement**: SYNC-05

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `parse_mirror` returns roster models and a skipped list for entries missing required fields
- [x] Deprecated entries are excluded
- [x] Unit tests cover: valid fixture, missing fields, deprecated filtering, snapshot JSON round-trip
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add catalog types and roster parsing`

---

### T11: Overlay loading and validation

**What**: Implement Overlay structs and loaders for the four TOML files, metadata inheritance with per-entry overrides, and the warning set.
**Where**: `crates/core/src/overlay/mod.rs`
**Depends on**: T10
**Reuses**: `data/*.toml` seeds from T6–T9; `include_str!` bundling
**Requirement**: DATA-01, DATA-02, DATA-03, DATA-04, DATA-05

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Bundled overlays load; load_dir loads the repo `data/` directory
- [x] Unit tests cover: missing metadata names file/entry; malformed window fails; unrated model warning; uncovered-year warning; per-entry override accepted and validated
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add overlay loading and validation`

---

### T12: Pricing window schedule

**What**: Implement `state_at` with the peak rule, next-transition computation and coverage-gap flag.
**Where**: `crates/core/src/schedule/mod.rs`
**Depends on**: T11
**Reuses**: Overlay types from T11; `chrono`
**Requirement**: SCHED-01, SCHED-02, SCHED-03, SCHED-04, SCHED-05, SCHED-06

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests use a synthetic calendar fixture (not the seed data) and cover: 8 boundary instants, weekend, holiday weekday, makeup weekend, next transition from off-peak/inside window/after last window, leap day, determinism, coverage gap
- [x] No clock reads inside the function
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add pricing window schedule`

---

### T13: Config load and save

**What**: Implement `Config` with defaults, validation, atomic save, and unknown-field round-trip.
**Where**: `crates/core/src/config/mod.rs`
**Depends on**: T12
**Reuses**: `TaskProfile` enum location decided in design (catalog types module)
**Requirement**: CFG-01, CFG-02, CFG-03, CFG-04

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests cover: defaults when file missing; invalid value errors naming the field and file unmodified; atomic save; unknown-field round-trip; validation rules
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add config load and save`

---

### T14: Snapshot store

**What**: Implement `SnapshotStore` with current/previous rotation, atomic writes, and parse-failure-as-absent behavior.
**Where**: `crates/core/src/sync/snapshot.rs`
**Depends on**: T10
**Reuses**: `CatalogSnapshot` from T10; `tempfile` (dev)
**Requirement**: SYNC-01, SYNC-08

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests cover: save rotates current→previous; exactly two files retained; interrupted/truncated file treated as absent; atomic temp+rename behavior
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add snapshot store`

---

### T15: Injectable HTTP client

**What**: Define the `HttpClient` trait plus a `ureq` implementation with timeout and a pure request builder setting `If-None-Match` when an ETag is present.
**Where**: `crates/core/src/sync/http.rs`
**Depends on**: T14
**Reuses**: `ureq` + rustls
**Requirement**: SYNC-07, SYNC-09

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests cover: request builder sets/omits `If-None-Match`; timeout value propagates; a stub implementation can be used through the trait
- [x] No test performs network I/O
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add injectable HTTP client`

---

### T16: Catalog sync with fallback chain

**What**: Implement `SyncEngine::sync` orchestrating fetch, validation, roster extraction, snapshot persistence, ETag handling and the fallback chain with active-source reporting.
**Where**: `crates/core/src/sync/mod.rs`
**Depends on**: T15
**Reuses**: `parse_mirror` (T10), `SnapshotStore` (T14), `HttpClient` (T15)
**Requirement**: SYNC-01, SYNC-02, SYNC-03, SYNC-04, SYNC-05, SYNC-07, SYNC-09, SYNC-10

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests with stub clients cover: 200 valid → Updated(Remote); 304 → NotModified with unchanged files; network error → LastValid; invalid payload → LastValid untouched; empty roster → invalid; 304 without snapshot → Bundled; skipped-entry warnings surfaced
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add catalog sync with fallback chain`

---

### T17: Catalog change detection

**What**: Implement `detect_changes` producing additions, removals, per-field price changes and capability flag changes.
**Where**: `crates/core/src/sync/changes.rs`
**Depends on**: T16
**Reuses**: `CatalogSnapshot` (T10)
**Requirement**: SYNC-06

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Unit tests cover: added, removed, price change per field, capability change, identical snapshots → empty
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add catalog change detection`

---

### T18: Scoring and reason codes

**What**: Implement effective pricing (peak multiplier), `cost_index`, normalization, score composition, tie-breaks and the closed `ReasonCode` set.
**Where**: `crates/core/src/recommend/score.rs`
**Depends on**: T10
**Reuses**: Overlay limits and capabilities (T11); `WindowState` (T12)
**Requirement**: REC-04, REC-08

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Unit tests cover: peak doubles effective price; cost_index weights; normalization across candidates; lower cost wins ties; equal cost orders by model id; reason-code derivation inputs
- [ ] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add scoring and reason codes`

---

### T19: Recommendation engine

**What**: Implement profile filters, ignored/unrated exclusion, selection, top-3 alternatives, no-candidate reporting and reason assembly.
**Where**: `crates/core/src/recommend/mod.rs`
**Depends on**: T18
**Reuses**: `score` (T18), `Config`, `Overlays`, `CatalogSnapshot`
**Requirement**: REC-01, REC-02, REC-03, REC-05, REC-06, REC-07, REC-09, REC-10

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Unit tests cover: vision filter, tools filter, ignored models, unrated excluded, NoRecommendation names failing filter, alternatives capped at 3, reasons non-empty with codes from the closed set, determinism across runs
- [ ] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add recommendation engine`

---

### T20: Recommendation acceptance matrix

**What**: Add the integration test asserting expected winners and reason codes for 5 profiles × peak/off-peak against bundled data, with expectations derived by hand from the spec and curation, not from engine output.
**Where**: `crates/core/tests/recommendation_matrix.rs`
**Depends on**: T19
**Reuses**: bundled snapshot (T7), Overlays (T6–T9), `Engine` (T19)
**Requirement**: REC-11

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Matrix fixture lists expected winner + expected reason codes per row, each with a one-line rationale
- [ ] Test runs offline using bundled data only
- [ ] Gate passes: full gate command

**Tests**: integration
**Gate**: full
**Commit**: `test(core): add recommendation acceptance matrix`

---

### T21: Status report builder

**What**: Implement `build_report` and `Display` rendering state, next transition (UTC and local), 5 profile recommendations and active source.
**Where**: `crates/core/src/report/mod.rs`
**Depends on**: T19
**Reuses**: schedule (T12), engine (T19), config (T13)
**Requirement**: DEMO-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Unit tests with a fixed clock assert: state line, next transition in UTC and local, five profile sections with reasons, source label
- [ ] Rendering is deterministic for equal input
- [ ] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(core): add status report builder`

---

### T22: Status example binary

**What**: Add the example wiring `build_report` with bundled data and an optional snapshot store, printing the report offline.
**Where**: `crates/core/examples/status.rs`
**Depends on**: T21
**Reuses**: `report` (T21), bundled overlays/snapshot
**Requirement**: DEMO-01, DEMO-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `cargo run -p cost-guard-core --example status` prints a complete report with no network access
- [ ] Running from a directory without snapshots uses bundled data and says so
- [ ] Gate passes: build gate command

**Tests**: none (thin wrapper; report logic covered in T21)
**Gate**: build
**Commit**: `feat(core): add status example binary`

---

## Phase Execution Map

Visual representation of task ordering. Phases run in sequence, and tasks within a phase run in order:

```
T1 → T2
T2 → T3
T3 → T4
T4 → T5
T6 → T7
T7 → T8
T8 → T9
T10 → T11
T12 → T13
T14 → T15
T15 → T16
T16 → T17
T18 → T19
T19 → T20
T19 → T21
T21 → T22
T1 → T6
T1 → T10
T11 → T12
T10 → T14
T10 → T18
```

Execution is strictly sequential - there is no intra-phase parallelism. A single agent (or batch worker) works one task at a time, in order.

**Batch plan (orchestrator):** 22 tasks pack into 4 sequential workers, whole phases only:

- Batch A: Phase 1 (T1–T5) — 5 tasks
- Batch B: Phases 2–3 (T6–T11) — 6 tasks
- Batch C: Phases 4–5 (T12–T17) — 6 tasks
- Batch D: Phase 6 (T18–T22) — 5 tasks

After Batch D commits, a fresh Verifier runs (author ≠ verifier), then `validate_state.py` closes the feature.

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Workspace scaffold | 1 workspace manifest + crate skeleton | ✅ Granular |
| T2: Dual licenses | 2 license files, one concept | ✅ Granular |
| T3: Repo docs and templates | 1 docs bundle, cohesive | ✅ Granular |
| T4: CI matrix | 1 workflow file | ✅ Granular |
| T5: Core purity check | 1 script + CI wiring | ✅ Granular |
| T6: Pricing windows and holidays | 2 overlay files, one concern (time rules) | ✅ Granular |
| T7: Bundled snapshot | 1 data file | ✅ Granular |
| T8: Go limits | 1 data file | ✅ Granular |
| T9: Capability ratings | 1 data file | ✅ Granular |
| T10: Catalog types + parse | 1 module | ✅ Granular |
| T11: Overlay loading | 1 module | ✅ Granular |
| T12: Schedule | 1 module | ✅ Granular |
| T13: Config | 1 module | ✅ Granular |
| T14: Snapshot store | 1 module | ✅ Granular |
| T15: HTTP client | 1 module | ✅ Granular |
| T16: Sync orchestration | 1 module | ✅ Granular |
| T17: Change detection | 1 module | ✅ Granular |
| T18: Scoring | 1 module | ✅ Granular |
| T19: Recommendation engine | 1 module | ✅ Granular |
| T20: Acceptance matrix | 1 integration test | ✅ Granular |
| T21: Report builder | 1 module | ✅ Granular |
| T22: Example binary | 1 file | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | start of Phase 1 | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |
| T6 | T1 | T1 → T6 | ✅ Match |
| T7 | T6 | T6 → T7 | ✅ Match |
| T8 | T7 | T7 → T8 | ✅ Match |
| T9 | T8 | T8 → T9 | ✅ Match |
| T10 | T1 | T1 → T10 | ✅ Match |
| T11 | T10 | T10 → T11 | ✅ Match |
| T12 | T11 | T11 → T12 | ✅ Match |
| T13 | T12 | T12 → T13 | ✅ Match |
| T14 | T10 | T10 → T14 | ✅ Match |
| T15 | T14 | T14 → T15 | ✅ Match |
| T16 | T15 | T15 → T16 | ✅ Match |
| T17 | T16 | T16 → T17 | ✅ Match |
| T18 | T10 | T10 → T18 | ✅ Match |
| T19 | T18 | T18 → T19 | ✅ Match |
| T20 | T19 | T19 → T20 | ✅ Match |
| T21 | T19 | T19 → T21 | ✅ Match |
| T22 | T21 | T21 → T22 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1 | Workspace config | none | none | ✅ OK |
| T2 | License files | none | none | ✅ OK |
| T3 | Docs | none | none | ✅ OK |
| T4 | CI config | none | none | ✅ OK |
| T5 | Script | none | none | ✅ OK |
| T6 | Curated data | none | none | ✅ OK |
| T7 | Curated data | none | none | ✅ OK |
| T8 | Curated data | none | none | ✅ OK |
| T9 | Curated data | none | none | ✅ OK |
| T10 | Domain logic | unit | unit | ✅ OK |
| T11 | Domain logic | unit | unit | ✅ OK |
| T12 | Domain logic | unit | unit | ✅ OK |
| T13 | Domain logic | unit | unit | ✅ OK |
| T14 | Domain logic | unit | unit | ✅ OK |
| T15 | HTTP helper | unit | unit | ✅ OK |
| T16 | Domain logic | unit | unit | ✅ OK |
| T17 | Domain logic | unit | unit | ✅ OK |
| T18 | Domain logic | unit | unit | ✅ OK |
| T19 | Domain logic | unit | unit | ✅ OK |
| T20 | Acceptance matrix | integration | integration | ✅ OK |
| T21 | Domain logic | unit | unit | ✅ OK |
| T22 | Example binary | none | none | ✅ OK |
