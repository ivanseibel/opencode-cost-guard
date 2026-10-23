# Scoring Fix (core-v01.1) Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.**

**If the skill cannot be activated, STOP and tell the user - do not proceed without it.**

---

**Design**: skipped (Medium scope; design decisions live in `spec.md` Assumptions)
**Status**: In Progress

---

## Test Coverage Matrix

> Generated from the existing repo conventions (unit tests co-located, integration fixtures under `crates/core/tests/`). No new guidelines.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Domain logic (overlay, schedule, scoring, engine) | unit | All branches; 1:1 to spec ACs; every listed edge case | `crates/core/src/**/*.rs` (`#[cfg(test)]`) | `cargo test -p cost-guard-core` |
| Acceptance matrix + flip proof | integration | 10 matrix rows + synthetic flip | `crates/core/tests/*.rs` | `cargo test -p cost-guard-core --tests` |
| Curated data (`data/capabilities.toml`) | none - validated by loader tests | vendor + data_terms coverage for all 29 entries | `data/capabilities.toml` | build gate |

## Gate Check Commands

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | Unit-test tasks | `cargo test -p cost-guard-core` |
| Full | Integration-test tasks | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |
| Build | Data-only tasks and phase completion | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash scripts/check-core-purity.sh` |

---

## Execution Plan

Single phase, sequential.

### Phase 1: Scoring fix

```
T1 → T2
T1 → T3
T3 → T4
T4 → T5
T5 → T6
```

---

## Task Breakdown

### T1: Vendor and data-terms in capabilities overlay

**What**: Extend the capabilities overlay schema with `vendor` (required, non-empty) and `data_terms` (bool, default false), with validation and tests.
**Where**: `crates/core/src/overlay/mod.rs`
**Depends on**: None
**Reuses**: existing overlay structs and validation style
**Requirement**: VEND-01, TERMS-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Loading fails naming the entry when `vendor` is missing or empty
- [x] `data_terms` defaults to false when absent
- [x] Existing overlay fixtures updated; unit tests added for both rules
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(overlay): add vendor and data-terms to capabilities`

---

### T2: Seed vendors and data terms

**What**: Add `vendor` to all 29 capability entries and `data_terms = true` to the contributor models whose prompts may train models (`gemini-4-argon-contributor`, `muse-spark-1.2-contributor`, `muse-spark-1.3-contributor`).
**Where**: `data/capabilities.toml`
**Depends on**: T1
**Reuses**: capability ids from `data/bundled/opencode-go.json`
**Requirement**: VEND-01, TERMS-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Every entry has a non-empty `vendor` (deepseek, anthropic, google, openai, xai, zhipu, tencent, moonshot, meituan, xiaomi, minimax, meta, alibaba, stepfun, reflection)
- [x] `data_terms = true` only on the three contributor models above
- [x] File parses and bundled overlays load; bundled validation emits no warnings
- [x] Gate passes: build gate command

**Tests**: none (curated data, validated by overlay tests)
**Gate**: build
**Commit**: `chore(data): add vendors and data terms to capabilities`

---

### T3: Per-vendor window state

**What**: Add `provider` to `WindowState` and a `state_for(provider, now, overlays)` accessor; keep `state_at` as the DeepSeek convenience wrapper.
**Where**: `crates/core/src/schedule/mod.rs`
**Depends on**: T1
**Reuses**: existing window/holiday logic
**Requirement**: PEAK-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `WindowState` names the provider it describes
- [x] `state_for` returns None for vendors without a window
- [x] Existing schedule tests updated; new tests cover provider naming and unknown vendor
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(schedule): expose per-vendor window state`

---

### T4: Per-candidate multiplier and log cost scale

**What**: Change scoring so each candidate carries its own multiplier (1.0 for vendors without an active window) and replace `min/cost` with the log-scaled min-max formula; add the `DataTerms` reason code.
**Where**: `crates/core/src/recommend/score.rs`
**Depends on**: T3
**Reuses**: existing scoring module, effectiveness helper
**Requirement**: PEAK-01, COST-01, COST-02, COST-03, TERMS-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Effective price uses the candidate's own multiplier
- [x] Zero-cost candidate scores 1.0; paid candidates keep distinct scores in (0,1)
- [x] Equal costs (or single candidate, or degenerate log range) score 1.0 without division by zero
- [x] `DataTerms` code exists in the closed reason set
- [x] Unit tests per AC
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(recommend): apply per-vendor peak and log cost scale`

---

### T5: Engine wiring for vendors and terms

**What**: Resolve each candidate's vendor window in the engine, pass per-candidate multipliers, and append the `DataTerms` reason when the pick has data terms.
**Where**: `crates/core/src/recommend/mod.rs`
**Depends on**: T4
**Reuses**: `state_for` (T3), candidate capability vendor (T1)
**Requirement**: PEAK-01, TERMS-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Engine no longer applies a single global multiplier
- [x] Non-DeepSeek models keep flat prices during DeepSeek peak
- [x] `DataTerms` reason text rendered when the pick has data terms and absent otherwise
- [x] Unit tests cover both behaviors
- [x] Gate passes: quick gate command

**Tests**: unit
**Gate**: quick
**Commit**: `feat(recommend): wire vendor lookup and data-terms reason`

---

### T6: Re-derive acceptance matrix and flip proof

**What**: Re-derive the 10 matrix rows by hand under the new mechanics, update the doc comment/rationales, and add the synthetic peak-flip test.
**Where**: `crates/core/tests/recommendation_matrix.rs`
**Depends on**: T5
**Reuses**: bundled snapshot, overlays, engine
**Requirement**: MATRIX-01, MATRIX-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Each row documents the hand-derivation (one line per row)
- [ ] Expectations were derived from the formula, not from engine output
- [ ] Synthetic flip test: vendor model wins off-peak, rival wins at peak
- [ ] Matrix runs offline
- [ ] Gate passes: full gate command

**Tests**: integration
**Gate**: full
**Commit**: `test(recommend): re-derive acceptance matrix for peak flips`

---

## Phase Execution Map

```
T1 → T2
T1 → T3
T3 → T4
T4 → T5
T5 → T6
```

**Batch plan:** 6 tasks in a single batch, executed by one worker (≤ ~7 budget). Verifier runs after T6.

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Overlay schema | 1 module | ✅ Granular |
| T2: Seed data | 1 data file | ✅ Granular |
| T3: Schedule accessor | 1 module | ✅ Granular |
| T4: Scoring | 1 module | ✅ Granular |
| T5: Engine wiring | 1 module | ✅ Granular |
| T6: Matrix test | 1 integration test | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | start of phase | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T1 | T1 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |
| T6 | T5 | T5 → T6 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1 | Domain logic | unit | unit | ✅ OK |
| T2 | Curated data | none | none | ✅ OK |
| T3 | Domain logic | unit | unit | ✅ OK |
| T4 | Domain logic | unit | unit | ✅ OK |
| T5 | Domain logic | unit | unit | ✅ OK |
| T6 | Acceptance matrix | integration | integration | ✅ OK |
