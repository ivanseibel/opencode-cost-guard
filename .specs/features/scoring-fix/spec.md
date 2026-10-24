# Scoring Fix (core-v01.1) Specification

## Problem Statement

The verified core-v01 implementation applies DeepSeek's peak multiplier to **every** candidate and normalizes cost as `min/cost`. Two consequences break the product promise: (1) a uniform multiplier cancels in normalization, so peak never changes a recommendation; (2) a zero-cost free model sets the floor, driving every paid model's `cost_score` to 0 in four of five profiles, so price contributes nothing. This feature supersedes the core-v01 interpretation of REC-04/REC-08 and restores peak-aware, cost-aware recommendations.

## Goals

- [ ] Peak pricing applies only to models of the vendor whose Pricing Window is active
- [ ] Cost normalization keeps paid candidates differentiated even when a free model is eligible
- [ ] Recommendations expose a `DataTerms` reason when the pick may train on prompts
- [ ] Acceptance matrix re-derived by hand; a synthetic test proves peak can flip a winner
- [ ] All gates green; independent Verifier PASS

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Tauri app shell (M2) | Separate feature; this is core logic only |
| Excluding free/contributor models by default | Product decision keeps them eligible with a visible reason |
| Allowance tracking, history, new data sources | Later phases |
| UI/menu changes | M2 |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Vendor identity source | Curated `vendor` field in `data/capabilities.toml` | The catalog only knows the gateway (`opencode-go`); vendor must be explicit and contributor-editable | y |
| Peak scope | Only vendors with a Pricing Window in the Overlay; all others stay flat | DeepSeek is the only time-priced vendor in the roster | y |
| Cost normalization | Log-scaled min-max: `1 - (ln(1+c) - ln(1+cmin)) / (ln(1+cmax) - ln(1+cmin))`; equal costs → all 1.0 | Non-collapsing with zero-cost candidates; compresses extreme ranges | y |
| Free/contributor policy | Eligible, flagged with a `DataTerms` reason when the pick has data-terms | Matches the recorded product assumption (visible flag, not silent exclusion) | y |
| Matrix expectations | Re-derived by hand under the new mechanics | Engine output must never define its own expectations | y |

**Open questions:** none - all resolved or logged above (required before the spec is confirmed).

---

## User Stories

### P1: Per-vendor peak pricing ⭐ MVP

**User Story**: As the menu app, I need peak pricing applied only to the vendor that actually has a Pricing Window so recommendations respond to DeepSeek peak.

**Acceptance Criteria**:

1. VEND-01 WHEN capability Overlays load THEN every entry SHALL declare a non-empty `vendor`; IF it is missing THEN loading SHALL fail naming the entry.
2. PEAK-01 WHILE a vendor's Peak Window is active the system SHALL apply that vendor's multiplier only to models of that vendor; models of other vendors SHALL keep their off-peak (flat) prices.
3. PEAK-02 WHEN the schedule returns a WindowState THEN the result SHALL name the provider (vendor) it describes.

**Independent Test**: Unit test with a DeepSeek-window calendar and two synthetic vendors: at peak only the DeepSeek candidate's effective price doubles.

---

### P1: Non-collapsing cost normalization ⭐ MVP

**User Story**: As the menu app, I need price to actually discriminate between paid models even when a free model exists.

**Acceptance Criteria**:

1. COST-01 WHEN candidates are scored THEN `cost_score` SHALL be `1 - (ln(1+c) - ln(1+c_min)) / (ln(1+c_max) - ln(1+c_min))` over eligible candidates, where `c` is the effective cost index.
2. COST-02 IF a zero-cost candidate is eligible THEN it SHALL score exactly 1.0 and paid candidates SHALL keep differentiated scores strictly between 0.0 and 1.0.
3. COST-03 IF all eligible candidates share the same cost index THEN every `cost_score` SHALL be 1.0.

**Independent Test**: Unit test with costs 0.0, 3.0, 3.5: scores are 1.0, then two distinct values in (0,1).

---

### P1: Terms visibility ⭐ MVP

**User Story**: As the author, I want contributor/free terms surfaced so a recommendation never silently pushes data-training models.

**Acceptance Criteria**:

1. TERMS-01 WHEN a model's capability entry has `data_terms = true` THEN the overlay SHALL expose the flag to the engine.
2. TERMS-02 WHEN the top pick has `data_terms = true` THEN the Recommendation SHALL include a `DataTerms` reason code with non-empty human-readable text.

**Independent Test**: Engine test with a data-terms candidate as winner asserts the `DataTerms` code and text.

---

### P1: Acceptance matrix and flip proof ⭐ MVP

**User Story**: As the author, I need the matrix to reflect the corrected mechanics and prove peak can change the winner.

**Acceptance Criteria**:

1. MATRIX-01 WHEN the 5-profile × peak/off-peak matrix runs THEN every row SHALL assert a winner and reason codes derived by hand from the scoring formula and curation.
2. MATRIX-02 WHEN a synthetic pair is scored where a vendor's peak multiplier makes its model more expensive than a rival THEN the engine SHALL pick the rival at peak and the vendor's model off-peak.

**Independent Test**: Full gate runs the re-derived matrix plus the synthetic flip test offline.

---

## Edge Cases

- IF a vendor has no Pricing Window THEN its models SHALL be scored at flat prices during peak.
- IF the capabilities Overlay contains an entry with an empty `vendor` THEN loading SHALL fail naming it.
- IF `data_terms` is absent THEN it SHALL default to `false`.
- WHEN only one candidate is eligible THEN its `cost_score` SHALL be 1.0.
- WHEN `ln(1+c_max) == ln(1+c_min)` THEN all scores SHALL be 1.0 (no division by zero).

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| VEND-01 | P1: Per-vendor peak | Tasks | Verified |
| PEAK-01 | P1: Per-vendor peak | Tasks | Verified |
| PEAK-02 | P1: Per-vendor peak | Tasks | Verified |
| COST-01 | P1: Cost normalization | Tasks | Verified |
| COST-02 | P1: Cost normalization | Tasks | Verified |
| COST-03 | P1: Cost normalization | Tasks | Verified |
| TERMS-01 | P1: Terms visibility | Tasks | Verified |
| TERMS-02 | P1: Terms visibility | Tasks | Verified |
| MATRIX-01 | P1: Matrix | Tasks | Verified |
| MATRIX-02 | P1: Matrix | Tasks | Verified |

**ID format:** `[CATEGORY]-[NUMBER]`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 10 total, 10 mapped to tasks, 0 unmapped

---

## Success Criteria

- [ ] All gates green (fmt, clippy `-D warnings`, workspace tests, core purity)
- [ ] Matrix shows the corrected mechanics; peak/off-peak winners may differ per row
- [ ] Synthetic flip test passes offline
- [ ] Verifier PASS with per-AC evidence, sensor mutations killed
