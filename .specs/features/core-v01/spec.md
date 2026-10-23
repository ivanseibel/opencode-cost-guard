# Core v0.1 Specification

## Problem Statement

OpenCode Cost Guard has a refined product brief, a v0.1 MVP definition and four ADRs, but no code. The app needs a platform-independent core before any UI work: correct pricing-window logic, a validated catalog with curated overlays, change detection, and a deterministic recommendation engine. This feature delivers the repository scaffold (backlog M0) and the pure-Rust core crate (backlog M1). The Tauri shell (M2) and release (M3) are separate features.

## Goals

- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` pass on macOS and Ubuntu CI runners
- [ ] Pricing Window state is correct at every boundary, proven by fixed-clock tests (windows, weekends, holidays, makeup weekends, year rollover)
- [ ] Catalog Sync survives every failure mode through the fallback chain (remote → last valid → bundled), with injected-client tests covering 200/304/error/invalid payloads
- [ ] Recommendation Engine produces the fixture's expected winner and reason codes for 5 profiles × peak/off-peak using bundled data only

## Out of Scope

| Feature     | Reason         |
| ----------- | -------------- |
| Tauri app shell (tray, menu, notifications, settings window) | Backlog M2; core must build without it |
| Release workflow, packaging, Homebrew tap | Backlog M3 |
| Catalog-change notifications, pre-peak warning | Product v0.2 scope |
| Reading OpenCode local files, DB, CLI or credentials | ADR-0004 boundary |
| SQLite, price history | ADR-0003; JSON snapshots only |
| Live-network tests against the mirror | CI must stay hermetic; sync tests use injected stubs; one manual smoke check is documented instead |

---

## Assumptions & Open Questions

Every ambiguity is resolved or recorded here - nothing is left silently unclear.

| Assumption / decision | Chosen default  | Rationale | Confirmed? |
| --------------------- | --------------- | --------- | ---------- |
| Package name, edition, toolchain | `cost-guard-core`, edition 2024, stable pinned in `rust-toolchain.toml` | Rust convention; MSRV is not a goal for a personal-first tool | y |
| Overlay file format and layout | TOML, one file per concern (`pricing-windows`, `holidays-cn`, `limits`, `capabilities`) under `data/` | Comments carry `source`/`lastVerified`; contributor-editable (ADR-0002) | y |
| HTTP client crate | Chosen at Design; accessed behind an injectable trait; tests never hit the network | Keeps CI hermetic; Design picks async vs blocking | y |
| Initial capability seeds | Best-effort tiers + flags from public model cards; author reviews before any release | Full benchmark rigor is Phase 5; data stays contributor-editable | y |
| Snapshot retention | Exactly current + previous; older pruned | ADR-0003; change detection needs only two | y |
| Catalog Change severity | Any change sets the badge; severity classification deferred to v0.2 | v0.1 answers "something changed", not "how much" | y |
| CN holiday data | Curated yearly table including makeup weekends; 2026 seeded from official announcements, source recorded | ADR-0002; yearly refresh is a `good first issue` | y |
| Sync scheduling | Cadence lives in M2; core exposes `sync()` and stores `lastSyncedAt` | Keeps timers out of the core | y |
| `crates/app` placeholder | Not created in this feature; M2 creates it | Avoids empty-crate ceremony | y |

**Open questions:** none - all resolved or logged above (required before the spec is confirmed).

---

## User Stories

### P1: Repository scaffold ⭐ MVP

**User Story**: As a contributor, I want a workspace that builds, lints and tests identically on macOS and Ubuntu so that the core stays portable by construction.

**Why P1**: Nothing else can be built or verified without it, and portability is a v0.1 acceptance criterion.

**Acceptance Criteria** (each line is one EARS pattern):

1. SCAF-01 WHEN CI runs THEN it SHALL execute `cargo fmt --check`, `cargo clippy --workspace -- -D warnings` and `cargo test --workspace` on both a macOS runner and an Ubuntu runner.
2. SCAF-02 The repository SHALL contain `LICENSE-MIT`, `LICENSE-APACHE` (dual license), `README.md` (build instructions and non-affiliation disclaimer), `CONTRIBUTING.md` (DCO sign-off and Overlay update process), and issue/PR templates.
3. SCAF-03 WHEN the CI purity check runs THEN it SHALL fail if `cost-guard-core` depends, directly or transitively, on `tauri`, `objc2` or `windows-sys` crates.

**Independent Test**: Open a PR touching nothing; CI matrix runs and the purity check reports core dependencies clean.

---

### P1: Pricing Window schedule ⭐ MVP

**User Story**: As the app, I need the exact DeepSeek peak/off-peak state for any instant so the menu bar status and recommendations never lie.

**Why P1**: This is the product's core real-time signal.

**Acceptance Criteria**:

1. SCHED-01 WHEN state is requested for an instant THEN the schedule SHALL return peak if and only if the UTC instant falls on a Monday–Friday date, inside [01:00, 04:00) or [06:00, 10:00), and the date is not listed in the curated Chinese-holiday calendar.
2. SCHED-02 WHEN the instant equals a window boundary THEN the schedule SHALL return peak for start instants (01:00, 06:00) and off-peak for end instants (04:00, 10:00).
3. SCHED-03 WHEN the instant falls on a weekend or on a makeup-weekend date THEN the schedule SHALL return off-peak.
4. SCHED-04 WHEN state is returned THEN the result SHALL include the next transition instant in UTC.
5. SCHED-05 The schedule SHALL be a pure function of (instant, curated rules): no clock reads and no mutable state, so equal input yields equal output.
6. SCHED-06 IF the instant's year is not covered by the curated holiday calendar THEN the schedule SHALL compute state without holiday exclusions and SHALL flag the coverage gap in its result.

**Independent Test**: Fixed-clock tests assert state and next transition at all boundary instants, a holiday date, and a makeup-weekend date.

---

### P1: Curated overlays ⭐ MVP

**User Story**: As a maintainer, I want validated, annotated Overlay files so model data can be corrected and extended by contributors without code changes.

**Why P1**: Overlays are the only source of peak windows, holidays, limits and capability data (ADR-0002).

**Acceptance Criteria**:

1. DATA-01 WHEN Overlays load THEN every Overlay file SHALL declare non-empty `source` and `lastVerified`, inherited by its entries; per-entry overrides SHALL also be non-empty; IF a required value is missing THEN loading SHALL fail naming the file or entry.
2. DATA-02 The Overlay set SHALL define peak windows and the peak multiplier (2×) for DeepSeek, the CN holiday calendar including makeup weekends, monthly Go limits per model, and per-model capability ratings (tier plus vision/tools/reasoning/context flags) for every Roster model.
3. DATA-03 IF a window definition is malformed (end not after start, or outside 00:00–24:00) THEN loading SHALL fail naming the entry.
4. DATA-04 WHEN models in the catalog lack a capability rating THEN validation SHALL emit a warning listing them.
5. DATA-05 WHEN the holiday calendar does not cover the current year THEN validation SHALL emit a warning naming the uncovered year.

**Independent Test**: Fixture Overlays load clean; a mutated file missing `source` fails with the entry name in the error.

---

### P1: Catalog Sync and change detection ⭐ MVP

**User Story**: As the app, I need a fresh validated Catalog with a bulletproof fallback chain so pricing survives network failures and schema drift.

**Why P1**: Everything downstream (menu prices, recommendations, badge) reads the Catalog.

**Acceptance Criteria**:

1. SYNC-01 WHEN a sync completes successfully THEN the system SHALL store the payload as the current snapshot, keep exactly one previous snapshot, and record source URL, ETag and `lastSyncedAt`.
2. SYNC-02 WHEN the remote responds 304 THEN sync SHALL report `NotModified` and leave snapshots unchanged.
3. SYNC-03 IF a remote fetch fails (network error, non-2xx, timeout, unreadable body) THEN sync SHALL fall back to the last valid snapshot, then to the bundled snapshot, and SHALL report which source is active.
4. SYNC-04 IF the fetched payload fails schema validation THEN the system SHALL NOT overwrite the last valid snapshot.
5. SYNC-05 WHEN the mirror payload is parsed THEN the system SHALL extract exactly the `opencode-go` provider models as the Roster; entries missing required fields (`id`, `name`, `cost.input`, `cost.output`, `cost.cache_read`, `limit.context`, `tool_call`, `attachment`) SHALL be skipped and reported while the sync still succeeds.
6. SYNC-06 WHEN two snapshots are available THEN change detection SHALL list additions, removals, per-field price changes (old → new) and capability flag changes.
7. SYNC-07 The fetch SHALL enforce a 10-second timeout per request.
8. SYNC-08 Snapshot persistence SHALL be atomic: IF a write is interrupted THEN a partially written snapshot SHALL NOT be treated as valid on the next load.
9. SYNC-09 The fetch path SHALL be accessed through an injectable HTTP client abstraction so tests exercise 200, 304, error and invalid-payload responses without network access.
10. SYNC-10 IF a 304 response arrives while no valid snapshot exists THEN sync SHALL treat it as a failure and follow the fallback chain.

**Independent Test**: Stub client scripts the response sequence; each AC maps to one scripted scenario asserting snapshot state and reported source.

---

### P1: Recommendation Engine ⭐ MVP

**User Story**: As the app, I need a deterministic, explainable model recommendation per Task Profile so the menu can show a pick with a reason.

**Why P1**: It is one of the three product questions.

**Acceptance Criteria**:

1. REC-01 WHEN a recommendation is requested for a Task Profile THEN the system SHALL return one top pick, a non-empty reason list, and up to 3 alternatives, computed only from the current snapshot, the schedule state, the Overlays and the config.
2. REC-02 WHERE the Task Profile is Visual/UI the system SHALL only consider candidates with image input support.
3. REC-03 WHERE the Task Profile is Agentic/Tools/MCP the system SHALL only consider candidates with tool-call support.
4. REC-04 WHEN the schedule reports peak THEN scoring SHALL use peak prices (off-peak price × overlay multiplier); otherwise off-peak prices.
5. REC-05 IF no candidate passes the profile filters THEN the result SHALL be `NoRecommendation` and SHALL name the failing filter.
6. REC-06 Models listed in config `ignoredModels` SHALL never appear in a recommendation or its alternatives.
7. REC-07 Models without a capability rating SHALL be excluded from picks and alternatives.
8. REC-08 WHEN two candidates tie on score THEN the lower effective price SHALL win; IF prices also tie THEN ordering SHALL be stable by model ID.
9. REC-09 The engine SHALL be deterministic: identical inputs produce identical output, including alternative ordering.
10. REC-10 WHEN a recommendation is produced THEN each reason SHALL pair a code from the closed set defined in Design with a human-readable string.
11. REC-11 WHEN the 5-profile × peak/off-peak matrix runs against the curated test fixture THEN the engine SHALL produce the fixture's expected winner and expected reason codes present.

**Independent Test**: Table-driven fixture test runs all 10 combinations offline and prints picks and reasons.

---

### P1: Config ⭐ MVP

**User Story**: As the app, I need validated user config with safe defaults so first run works without a file and bad files never corrupt.

**Why P1**: Recommendations filter on config; M2 wires the settings window to it.

**Acceptance Criteria**:

1. CFG-01 WHEN no config file exists THEN load SHALL succeed with defaults: `autostart=false`, notifications enabled, `defaultProfile="simple-coding"`, `syncIntervalHours=6`, `ignoredModels=[]`.
2. CFG-02 IF the config file contains invalid values THEN load SHALL fail naming the offending field and SHALL leave the file unmodified.
3. CFG-03 WHEN config is saved THEN the write SHALL be atomic and unknown JSON fields SHALL round-trip preserved.
4. CFG-04 The system SHALL validate `syncIntervalHours ≥ 1`, `defaultProfile` among the 5 Task Profiles, and `ignoredModels` entries as `provider/model` IDs.

**Independent Test**: Round-trip test: load defaults, add an unknown field, save, reload, field intact.

---

### P2: Offline demo example

**User Story**: As the author, I want a runnable example that prints the current state and recommendations so the core is demoable before the app shell exists.

**Why P2**: Gives the feature an independent vertical slice; useful for dogfooding the engine.

**Acceptance Criteria**:

1. DEMO-01 WHEN the example runs offline THEN it SHALL print the schedule state, next transition in UTC and local time, and a recommendation for each of the 5 Task Profiles.
2. DEMO-02 IF no synced snapshot exists THEN the example SHALL run from the bundled snapshot and say so.

**Independent Test**: `cargo run -p cost-guard-core --example status` prints a complete report with no network available.

---

## Edge Cases

- WHEN the instant is 00:59:59, 01:00:00, 03:59:59, 04:00:00, 05:59:59, 06:00:00, 09:59:59 or 10:00:00 UTC THEN the state SHALL match the inclusive-start, exclusive-end rule.
- WHEN Feb 29 falls on a peak weekday THEN the schedule SHALL apply the normal weekday rule.
- WHEN a Chinese Public Holiday falls on a weekday THEN the whole date SHALL be off-peak regardless of time.
- IF the mirror returns 200 with zero `opencode-go` models THEN sync SHALL treat the payload as invalid and follow the fallback chain.
- IF a snapshot file on disk is partially written or truncated THEN loading SHALL ignore it as invalid rather than error fatally.
- WHEN two candidates have equal score and equal price THEN ordering SHALL be decided by model ID (stable).

---

## Requirement Traceability

Each requirement gets a unique ID for tracking across design, tasks, and validation.

| Requirement ID | Story       | Phase  | Status  |
| -------------- | ----------- | ------ | ------- |
| SCAF-01        | P1: Scaffold | Tasks | In Tasks |
| SCAF-02        | P1: Scaffold | Tasks | In Tasks |
| SCAF-03        | P1: Scaffold | Tasks | In Tasks |
| SCHED-01       | P1: Schedule | Tasks | In Tasks |
| SCHED-02       | P1: Schedule | Tasks | In Tasks |
| SCHED-03       | P1: Schedule | Tasks | In Tasks |
| SCHED-04       | P1: Schedule | Tasks | In Tasks |
| SCHED-05       | P1: Schedule | Tasks | In Tasks |
| SCHED-06       | P1: Schedule | Tasks | In Tasks |
| DATA-01        | P1: Overlays | Tasks | In Tasks |
| DATA-02        | P1: Overlays | Tasks | In Tasks |
| DATA-03        | P1: Overlays | Tasks | In Tasks |
| DATA-04        | P1: Overlays | Tasks | In Tasks |
| DATA-05        | P1: Overlays | Tasks | In Tasks |
| SYNC-01        | P1: Sync    | Tasks | In Tasks |
| SYNC-02        | P1: Sync    | Tasks | In Tasks |
| SYNC-03        | P1: Sync    | Tasks | In Tasks |
| SYNC-04        | P1: Sync    | Tasks | In Tasks |
| SYNC-05        | P1: Sync    | Tasks | In Tasks |
| SYNC-06        | P1: Sync    | Tasks | In Tasks |
| SYNC-07        | P1: Sync    | Tasks | In Tasks |
| SYNC-08        | P1: Sync    | Tasks | In Tasks |
| SYNC-09        | P1: Sync    | Tasks | In Tasks |
| SYNC-10        | P1: Sync    | Tasks | In Tasks |
| REC-01         | P1: Recommend | Tasks | In Tasks |
| REC-02         | P1: Recommend | Tasks | In Tasks |
| REC-03         | P1: Recommend | Tasks | In Tasks |
| REC-04         | P1: Recommend | Tasks | In Tasks |
| REC-05         | P1: Recommend | Tasks | In Tasks |
| REC-06         | P1: Recommend | Tasks | In Tasks |
| REC-07         | P1: Recommend | Tasks | In Tasks |
| REC-08         | P1: Recommend | Tasks | In Tasks |
| REC-09         | P1: Recommend | Tasks | In Tasks |
| REC-10         | P1: Recommend | Tasks | In Tasks |
| REC-11         | P1: Recommend | Tasks | In Tasks |
| CFG-01         | P1: Config  | Tasks | In Tasks |
| CFG-02         | P1: Config  | Tasks | In Tasks |
| CFG-03         | P1: Config  | Tasks | In Tasks |
| CFG-04         | P1: Config  | Tasks | In Tasks |
| DEMO-01        | P2: Demo    | Tasks | In Tasks |
| DEMO-02        | P2: Demo    | Tasks | In Tasks |

**ID format:** `[CATEGORY]-[NUMBER]`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 41 total, 41 mapped to tasks, 0 unmapped

---

## Success Criteria

How we know the feature is successful:

- [ ] CI green (fmt + clippy + tests) on macOS and Ubuntu
- [ ] Every acceptance criterion has at least one automated test, confirmed by the Verifier with per-AC evidence
- [ ] The demo example prints correct state and recommendations offline from bundled data
- [ ] No test in the suite requires network access
