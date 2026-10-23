# Core v0.1 Context

**Gathered:** 2026-10-23
**Spec:** `.specs/features/core-v01/spec.md`
**Status:** Ready for design

---

## Feature Boundary

Repository scaffold (backlog M0) plus the platform-independent core crate (backlog M1): workspace, CI, curated Overlays, pricing-window schedule, Catalog Sync with snapshots and change detection, Recommendation Engine v1, config. No UI, no Tauri, no network tests.

---

## Implementation Decisions

Decisions were captured during the design grilling session on 2026-10-23 and are recorded in the repo docs. This file aggregates the ones that bind this feature.

### Stack and layout

- Tauri 2 shell is a later feature; the core crate must build with zero Tauri/macOS dependencies (ADR-0001, SCAF-03).
- Workspace with `crates/core`; `crates/app` is created by the M2 feature, not here.
- Edition 2024, stable toolchain pinned; dual MIT/Apache-2.0 license; DCO sign-off; CI matrix macOS + Ubuntu.

### Data strategy

- Catalog source: fetch `https://models.opencode.ai/api.json`; curated TOML Overlays in `data/`; fallback chain remote → last valid → bundled snapshot (ADR-0002, SYNC-03).
- Upstream has off-peak prices only. Peak = off-peak × multiplier (2×) from Overlays; Peak Windows are 01:00–04:00 and 06:00–10:00 UTC, Mon–Fri, excluding Chinese public holidays; makeup weekends are off-peak (SCHED-01, REC-04).
- Catalog scope: `opencode-go` roster only (SYNC-05).
- Every overlay entry carries `source` and `lastVerified` (DATA-01).
- Change detection: diff current vs previous snapshot; any change sets the badge in v0.1 (ADR-0002, SYNC-06).

### Persistence and boundaries

- JSON snapshots (current + previous) and JSON config; SQLite deferred to Phase 2 (ADR-0003).
- No OpenCode internals, no credentials, no CLI reads; sync is anonymous (ADR-0004).
- Sync scheduling (6 h cadence, manual trigger) is M2 work; core exposes `sync()` and stores `lastSyncedAt`.

### Recommendation

- Deterministic and explainable; hard filters per profile, then weighted score (Q4/Q10 session decisions, REC-01..REC-11).
- Capability ratings are curated seeds (best-effort tiers + flags), refined during dogfooding.
- Config `ignoredModels` always wins; unrated models are never recommended.

---

## Specific References

- `docs/mvp.md` — v0.1 acceptance criteria the whole product must satisfy.
- `docs/backlog.md` — M0/M1 issue sketches this spec formalizes.
- `docs/adr/0001`–`0004` — accepted decisions.
- Real curation seeds from session research: DeepSeek V4.1 Flash off-peak $0.15/$0.60/cache $0.003; Go limits published per model; roster ≈ 35–37 models.

---

## Deferred Ideas

- Pre-peak warning, change notifications, favorites — v0.2.
- Allowance burn estimates, today's spend — Phase 2/3.
- Variant-aware recommendations (`low|high|max`) — Phase 3.
- App crate placeholder, release workflow — M2/M3 features.

---

## Agent's Discretion

Design phase decides: HTTP client crate and trait shape, exact TOML schemas, error type design, capability tier vocabulary, scoring weights and reason-code set, fixture design for the REC-11 matrix, and the purity-check implementation (script vs cargo-deny).
