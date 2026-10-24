# Scoring Fix (core-v01.1) Validation

**Date**: 2026-10-24
**Spec**: `.specs/features/scoring-fix/spec.md`
**Tasks**: `.specs/features/scoring-fix/tasks.md`
**Diff range**: `a73f125..92e5722` (6 commits)
**Verifier**: independent sub-agent (author ≠ verifier)

Commits:

| Commit | Subject |
| ------ | ------- |
| `3beee9b` | feat(overlay): add vendor and data-terms to capabilities |
| `e59c5f6` | chore(data): mark contributor models with data terms |
| `a37023b` | feat(schedule): expose per-vendor window state |
| `71b5a3e` | feat(recommend): add per-candidate multipliers and log cost scale |
| `37788ad` | feat(recommend): wire vendor lookup and data-terms reason |
| `92e5722` | test(recommend): re-derive acceptance matrix for peak flips |

Working tree at validation time: clean (`git status --porcelain` empty, before and after the sensor run). No code or test was modified by the Verifier; the only file added is this report.

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 Overlay schema | ✅ Done | `vendor` required non-empty, `data_terms` defaults false, error names entry |
| T2 Seed vendors/terms | ✅ Done | 29/29 entries have a vendor; exactly 3 contributor models set `data_terms = true` (verified by parsing the TOML) |
| T3 Per-vendor state | ✅ Done | `WindowState.provider`, `state_for`, `state_at` wrapper retained |
| T4 Scoring | ✅ Done | Per-candidate multiplier, log min-max, `DataTerms` code |
| T5 Engine wiring | ✅ Done | Vendor lookup per candidate; `DataTerms` reason appended for the pick |
| T6 Matrix + flip | ✅ Done | 10 rows re-derived, synthetic flip added; see process note below |

---

## Spec-Anchored Acceptance Criteria

Formula used for re-derivation: `cost_index = 0.35*input + 0.65*output`; `cost_score = 1 - (ln(1+c) - ln(1+c_min)) / (ln(1+c_max) - ln(1+c_min))`; `score = 0.55*capability + 0.30*cost + 0.10*allowance + 0.05*context`.

| Criterion | Spec-defined outcome | `file:line` + assertion | Result |
| --------- | -------------------- | ----------------------- | ------ |
| VEND-01 missing/empty `vendor` fails naming entry | Load error naming `capabilities.toml` + entry id + `vendor` | `crates/core/src/overlay/mod.rs:813` (`capabilities_require_a_non_empty_vendor`), assertions at `:817-819` and `:823-826` - `message.contains("capabilities.toml")`, `("model-a")`, `("vendor")` for both missing and whitespace-only vendor | ✅ PASS |
| PEAK-01 peak multiplier applies only to that vendor's models | DeepSeek effective price doubles at peak; other vendors stay flat; ranking may flip | `crates/core/src/recommend/score.rs:383` - `:416-420` asserts flat effective input `1.0`, peak effective input `2.0`, cost indices `1.0`/`2.0`, `flat.cost_score > peak.cost_score`. `crates/core/src/recommend/mod.rs:737` - `:760` peak pick cost_index `1.0`, `:772-773` flat vendor `1.0` vs doubled DeepSeek `2.0` | ✅ PASS |
| PEAK-02 `WindowState` names its provider | `state.provider == "deepseek"`; unknown vendor → `None` | `crates/core/src/schedule/mod.rs:291` (`state_names_the_provider_it_describes`): `:296` `assert_eq!(state.provider, "deepseek")`; `:299-302` `state_for("anthropic", ...) == None`. Also `:306` (`each_provider_has_its_own_peak_state`) and matrix fixture `:219-222` | ✅ PASS |
| COST-01 log-scaled min-max formula | Exact formula value for every non-max candidate | `crates/core/src/recommend/score.rs:155-180` (implementation); `:424` test asserts `b.cost_score ≈ 1 - (ln3-ln2)/(ln5-ln2)` at `:477-478` (1e-12); `:540` asserts `mid ≈ 1 - ln4/ln4.5` at `:541`; `:480`/`:545` assert the range maximum at `f64::EPSILON` - see precision note | ✅ PASS (⚠️ precision note) |
| COST-02 free candidate 1.0, paid strictly in (0,1), differentiated | Free exactly `1.0`; two paid values distinct and `0 < score < 1` | `crates/core/src/recommend/score.rs:484` (`zero_cost_candidate_scores_one_and_paid_candidates_stay_differentiated`): `:539` `assert_eq!(free.cost_score, 1.0)`; `:540-542` mid equals formula and `> 0.0 && < 1.0`; `:545-546` max equals `f64::EPSILON` and `> 0.0 && < 1.0`; `:547` `assert_ne!(mid.cost_score, pricey.cost_score)` | ✅ PASS |
| COST-03 equal costs → all 1.0, no division by zero | Every equal-cost candidate scores exactly 1.0 | `crates/core/src/recommend/score.rs:551` (`equal_costs_score_full_cost`) `:582` `all(|c| c.cost_score == 1.0)`; single-candidate edge at `:586` `:606` `assert_eq!(scored[0].cost_score, 1.0)` | ✅ PASS |
| TERMS-01 overlay exposes `data_terms` | Field parsed, defaults false, true when present | `crates/core/src/overlay/mod.rs:118-119` (model field), `:604` (propagation); test `:830` (`data_terms_defaults_to_false_and_parses_when_present`): `:834` `assert!(!entry.data_terms)`, `:839` `assert!(...data_terms)` | ✅ PASS |
| TERMS-02 pick with data terms carries `DataTerms` + non-empty text | Code and human-readable text present; absent otherwise | `crates/core/src/recommend/mod.rs:231-236` (append logic); test `:788` (`data_terms_reason_appears_only_when_the_pick_has_data_terms`): `:791-796` code found and text non-empty/not the code string; `:797-800` last reason is `DataTerms`; `:802-808` absent when false | ✅ PASS |
| MATRIX-01 10 rows assert hand-derived winner + codes | Exact winner and exact ordered code vector per row | `crates/core/tests/recommendation_matrix.rs:244` (`recommendation_matrix_matches_hand_derived_expectations`): `:256` `assert_eq!(pick.model_id, row.winner)`, `:257` `assert_eq!(codes(&recommendation), row.codes)`, `:258-265` every reason text non-empty. Row count/shape at `:234-241` | ✅ PASS |
| MATRIX-02 synthetic flip | Rival wins at peak; vendor model wins off-peak; only the vendor model reprices | `crates/core/tests/recommendation_matrix.rs:410` (`synthetic_peak_multiplier_flips_the_winner`): `:420` off-peak pick `vendor-a`; `:424` peak pick `rival-b`; `:421`/`:425` rival cost_index `1.0`; `:432-436` `vendor-a` alternative cost_index `2.0` at peak | ✅ PASS |

**Status**: ✅ All 10 ACs matched spec outcomes. 0 gaps. 1 precision note (COST-01/COST-02 boundary, below).

### Precision note — COST-01 vs COST-02 at `c_max`

COST-01's formula yields exactly `0.0` for the maximum-cost candidate; COST-02 requires paid candidates to score strictly inside `(0, 1)`. The two ACs are mathematically inconsistent at the boundary. The implementation (`score.rs:168-180`) resolves this in favor of COST-02 by flooring any positive-cost result at `f64::EPSILON` (2.22e-16), documented in the module comment (`score.rs:19-22`) and asserted explicitly (`score.rs:480`, `:545`).

Judgment: this **satisfies both ACs at representable precision** and is not a behavioral gap. Final-score impact is `0.30 × 2.22e-16 ≈ 6.7e-17`, far below any ranking threshold; ordering is unchanged. It is a **spec-precision gap only** in the narrow sense that COST-01 is not bit-exact at `c_max`. Recommended follow-up (spec text, not code): state that the range maximum scores `0.0` unless a zero-cost candidate is eligible, in which case paid scores are floored at `f64::EPSILON`. The floor is currently applied whenever `cost_index > 0`, i.e. also when no free candidate is eligible - a superset of COST-02's condition, still at epsilon scale.

---

## Matrix Integrity Check

T4/T5 did touch `crates/core/tests/recommendation_matrix.rs` before T6 (`71b5a3e`, `37788ad`), which is a process smell: T5's commit changed the complex/refactor peak winners to Muse at the same time it wired the engine. T6 (`92e5722`) then rewrote every rationale with explicit arithmetic. To avoid trusting either, the Verifier re-derived **all 10 rows** from `data/capabilities.toml`, `data/bundled/opencode-go.json` (prices), `data/limits.toml`, and the formula - independent of the engine and of the rationale strings.

Constants: DeepSeek V4.1 Flash `c = 0.35*0.15 + 0.65*0.60 = 0.4425`; Muse Spark 1.3 `c = 0.165`; max eligible `c = 14.4` (Gemini 4 Argon); `beam-free` sets `c_min = 0` for unfiltered profiles (visual filters it out for missing vision).

| # | Row (profile / state) | Re-derived winner | Re-derived score | Asserted winner | Match |
| - | --------------------- | ----------------- | ---------------- | --------------- | ----- |
| 1 | simple / off-peak | deepseek-v4.1-flash | 0.882803 (runner Muse 1.3 0.837244) | deepseek-v4.1-flash | ✅ |
| 2 | simple / peak | deepseek-v4.1-flash | 0.853449 (cost 0.768163; Muse 0.837244) | deepseek-v4.1-flash | ✅ |
| 3 | complex / off-peak | deepseek-v4.1-flash | 0.899303 (runner Muse 1.3 0.881244) | deepseek-v4.1-flash | ✅ |
| 4 | complex / peak (**flip**) | muse-spark-1.3-contributor | 0.881244 vs DeepSeek 0.869949 | muse-spark-1.3-contributor | ✅ |
| 5 | visual / off-peak | muse-spark-1.3-contributor | 0.887000 (runner mimo-v2.5 0.886096) | muse-spark-1.3-contributor | ✅ |
| 6 | visual / peak | muse-spark-1.3-contributor | 0.887000 (Meta flat; DeepSeek 2x) | muse-spark-1.3-contributor | ✅ |
| 7 | agentic / off-peak | deepseek-v4.1-flash | 0.915803 (runner Muse 1.3 0.870244) | deepseek-v4.1-flash | ✅ |
| 8 | agentic / peak | deepseek-v4.1-flash | 0.886449 (cost 0.768163) | deepseek-v4.1-flash | ✅ |
| 9 | refactor / off-peak | deepseek-v4.1-flash | 0.871803 (runner Muse 1.3 0.859244) | deepseek-v4.1-flash | ✅ |
| 10 | refactor / peak (**flip**) | muse-spark-1.3-contributor | 0.859244 vs DeepSeek 0.842449 | muse-spark-1.3-contributor | ✅ |

Winner and rationale arithmetic agree with the committed strings (`expected totals` above match e.g. `0.8828`, `0.8534`, `0.8812`, `0.8870`, `0.9158`, `0.8864`, `0.8718`, `0.8592`). Reason-code vectors were re-derived from the threshold policy and the winners' actual capability/cost/allowance/context/cache values; every one of the 10 vectors matched, including `PeakWindow` only for DeepSeek picks at peak and `OffPeakWindow` for flat-vendor picks during DeepSeek peak.

**Synthetic flip (`MATRIX-02`), independently derived**: both models cost `1.0` → log range degenerate → both `cost_score = 1.0`; vendor-a capability `0.90` beats rival `0.70` with equal allowance `0.5` and context `1.0` → `0.895` vs `0.785`. At peak only vendor-a (vendor `deepseek`, the sole provider in `synthetic_overlays`) doubles to `2.0`; rival (`anthropic`, no window) stays `1.0`. Then rival cost `1.0` and vendor-a cost `0.0` (floored to eps) → `0.785` vs `0.595`. The test's assertions (`rival-b` pick, `vendor-a` at cost_index `2.0`, rival at `1.0`) match this derivation. The flip is caused by the per-vendor window lookup, not by a uniform reprice.

**Matrix integrity**: ✅ 10/10 rows hand-derivable and matching; expectations are not merely engine output. Process note recorded, not a correctness gap.

---

## Payload / Conjunction Rule

| Check | Evidence | Result |
| ----- | -------- | ------ |
| Reasons carry codes from the closed set | `ReasonCode` enum `score.rs:231-242`; `DataTerms` has stable id `data-terms` (`score.rs:741-744`) | ✅ |
| Every reason has non-empty human text | Matrix asserts per row `recommendation_matrix.rs:258-265`; engine asserts `!text.is_empty()` and `text != code.as_str()` (`recommend/mod.rs:730-733`, `:795-796`) | ✅ |
| Exact conjunction (no missing/extra codes) | `assert_eq!(codes(&recommendation), row.codes)` on the full ordered vector (`recommendation_matrix.rs:257`) | ✅ |
| Effective prices asserted on values | `score.rs:416-419` (effective input `1.0`/`2.0`, cost index `1.0`/`2.0`); `recommend/mod.rs:760`, `:772-773` | ✅ |
| Cost scores asserted on values | Formula values at `score.rs:477-478`, `:540-541` (1e-12); boundary values `:480`, `:545`; ordering `:420`, `:547` | ✅ |
| Cost indices asserted on values in matrix | `recommendation_matrix.rs:283-287` (peak = 2× off-peak for DeepSeek picks), `:296` (flat pick unchanged), `:421`, `:425`, `:433` | ✅ |

---

## Discrimination Sensor

Isolated scratch worktree: `git worktree add $(mktemp -d "$TMPDIR/cg-sensor-XXXXXX") HEAD --detach`. Baseline `git status --porcelain` of the real tree: empty. One mutation at a time, `git checkout -- .` between mutants, scratch removed with `git worktree remove --force` at the end. `git stash` never used. Real tree porcelain after cleanup: empty (matches baseline byte-for-byte).

| # | Mutation | File:line (HEAD) | Test run | Result |
| - | -------- | ---------------- | -------- | ------ |
| a | Apply the first provider's multiplier to every candidate again (global peak) | `crates/core/src/recommend/mod.rs:197-210` (`vendor_multiplier`) | `cargo test -p cost-guard-core --test recommendation_matrix` | ✅ Killed - 3 failed: `synthetic_peak_multiplier_flips_the_winner` (`left "vendor-a", right "rival-b"`), `recommendation_matrix_matches_hand_derived_expectations`, `peak_scales_only_deepseek_picks` |
| b | Revert cost normalization to `min/cost` (`index == 0 ? 1.0 : min/index`) | `crates/core/src/recommend/score.rs:155-180` | `cargo test -p cost-guard-core --lib zero_cost_candidate_scores_one_and_paid_candidates_stay_differentiated` | ✅ Killed - 0 passed, 1 failed (assertion at `score.rs:530`) |
| c | Drop the vendor lookup: `vendor_multiplier` always returns `1.0` (all flat) | `crates/core/src/recommend/mod.rs:197-210` | `cargo test -p cost-guard-core --test recommendation_matrix` | ✅ Killed - 3 failed: flip test (`left "vendor-a", right "rival-b"`), matrix rows, `peak_scales_only_deepseek_picks` |
| d | Omit the `DataTerms` reason append | `crates/core/src/recommend/mod.rs:231-236` | `cargo test -p cost-guard-core --lib data_terms_reason_appears_only_when_the_pick_has_data_terms` | ✅ Killed - 0 passed, 1 failed (assertion at `recommend/mod.rs:789`) |

All four logs contain zero `error[E` / "could not compile" lines - the failures are genuine assertion failures, not build breaks.

**Sensor depth**: 4 behavior-level mutations (P1 default tier; feature touches ranking, a critical recommendation path).
**Result**: 4/4 killed - PASS ✅

---

## Gate Check

Gate command from `tasks.md` (Build): `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash scripts/check-core-purity.sh`, plus the explicit matrix run.

| Gate | Command | Exit | Result |
| ---- | ------- | ---- | ------ |
| Format | `cargo fmt --check` | 0 | ✅ |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | ✅ no warnings |
| Tests | `cargo test --workspace` | 0 | ✅ 87 unit + 4 integration, 0 failed, 0 ignored |
| Core purity | `bash scripts/check-core-purity.sh` | 0 | ✅ "core dependency purity OK" |
| Matrix only | `cargo test -p cost-guard-core --test recommendation_matrix` | 0 | ✅ 4/4 passed |

- **Test count before feature** (`a73f125`): 79 (76 unit + 3 integration).
- **Test count after feature** (`92e5722`): 91 (87 unit + 4 integration).
- **Delta**: +12 (+11 unit, +1 integration). The integration delta is net: the superseded `peak_multiplier_doubles_the_winning_effective_cost` (uniform-multiplier behavior) was replaced by `peak_scales_only_deepseek_picks` + `synthetic_peak_multiplier_flips_the_winner`; no assertion was weakened, and matrix rows now assert exact ordered code vectors.
- **Skipped/ignored**: none.

**Gate**: 5 passed, 0 failed.

---

## Edge Cases

- [x] Vendor with no Pricing Window stays flat at peak - `recommend/mod.rs:737-785`, `recommendation_matrix.rs:270-301`.
- [x] Empty/missing `vendor` fails naming the entry - `overlay/mod.rs:813-827`.
- [x] `data_terms` absent defaults to `false` - `overlay/mod.rs:830-834`.
- [x] Single eligible candidate scores `cost_score = 1.0` - `score.rs:586-607`.
- [x] Degenerate log range (`ln(1+c_max) == ln(1+c_min)`) → all 1.0, no division by zero - `score.rs:168`, `:551-583`.
- [x] Zero-cost candidate scores exactly 1.0 while paid candidates stay differentiated - `score.rs:484-548`.

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code / no speculative abstraction | ✅ Scoring stays in `score.rs`; engine owns vendor resolution only |
| Surgical changes | ✅ 7 files in the diff surface; `report/mod.rs` is a one-line signature adaptation required by `Engine::recommend(profile, now)` |
| No scope creep / unrelated "improvements" | ✅ No changes outside overlay, schedule, score, engine, matrix, curated data |
| Matches existing patterns/style | ✅ Co-located unit tests, integration fixture, `thiserror` errors, documented formulas |
| Spec-anchored outcome check | ✅ All 10 ACs assert spec-defined values |
| Per-layer coverage (domain 1:1 ACs) | ✅ Every AC has a direct unit or integration assertion |
| Every test maps to a requirement / edge case | ✅ New tests map to VEND-01, PEAK-01/02, COST-01/02/03, TERMS-01/02, MATRIX-01/02; no unclaimed test |
| Documented guidelines | ✅ None beyond repo conventions; strong defaults applied (co-located tests, no Tauri/macOS in core, purity script) |

---

## Requirement Traceability (recommendation only - spec not edited)

| Requirement | Current | Recommended |
| ----------- | ------- | ----------- |
| VEND-01 | In Tasks | ✅ Verified |
| PEAK-01 | In Tasks | ✅ Verified |
| PEAK-02 | In Tasks | ✅ Verified |
| COST-01 | In Tasks | ✅ Verified (with precision note) |
| COST-02 | In Tasks | ✅ Verified |
| COST-03 | In Tasks | ✅ Verified |
| TERMS-01 | In Tasks | ✅ Verified |
| TERMS-02 | In Tasks | ✅ Verified |
| MATRIX-01 | In Tasks | ✅ Verified |
| MATRIX-02 | In Tasks | ✅ Verified |

---

## Summary

**Overall**: ✅ Ready (PASS)

**Spec-anchored check**: 10/10 ACs matched spec outcome; 0 gaps; 1 precision note (COST-01 boundary vs COST-02, resolved by a documented one-ULP epsilon floor).
**Matrix integrity**: 10/10 rows independently re-derived from formula + curated data + bundled prices; all match. Both flips (complex, refactor at peak) reproduced; synthetic flip reproduces for the specified per-vendor reason.
**Sensor**: 4/4 mutations killed in an isolated scratch worktree; real tree porcelain unchanged (empty) before/after.
**Gate**: 5 passed, 0 failed; 91 tests (87 unit + 4 integration), 0 ignored.

**What works**: per-vendor peak repricing, non-collapsing log cost scores with an explicit free/paid edge policy, `DataTerms` visibility, exact-code reason payloads, and a hand-derivable acceptance matrix with a peak-flip proof.

**Issues found**: none blocking. One documentation-level precision note: COST-01 and COST-02 conflict at `c_max`; the epsilon floor is correct in behavior, so the spec should state the boundary rule explicitly in a future edit.

**Next steps**: mark the 10 requirements Verified in `spec.md`; optionally tighten the COST-01 boundary wording; proceed to the next feature.

**Lesson candidate** (not recorded - Verifier was scoped to this report only): when two ACs collide at a boundary (exact-value formula vs strict inequality), the spec must name the tie-break; implementation-side epsilon floors are acceptable only when documented and assertion-pinned.

---

## Post-validation amendment (2026-10-24)

The user chose to loosen COST-02 instead of documenting the epsilon floor. Paid candidates now score in `[0.0, 1.0]`, with the highest effective cost landing at exactly `0.0`; the `f64::EPSILON` floor was removed from `crates/core/src/recommend/score.rs`, and the spec wording plus boundary tests were updated. Full gate re-run green. The historical verdict above remains as recorded.
