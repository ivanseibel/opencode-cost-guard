# OpenCode Cost Guard — Product Brief (refined)

Refined on 2026-10-23 from the original discovery brief, after a grilling session with fact-checking against live sources (OpenCode v2.0.29, models.dev / models.opencode.ai, DeepSeek docs, Tauri 2.12.5). Glossary: [CONTEXT.md](../CONTEXT.md). Decisions: [docs/adr/](./adr/). MVP scope and acceptance criteria: [mvp.md](./mvp.md). Delivery plan: [backlog.md](./backlog.md).

## 1. Product

A macOS menu-bar utility for OpenCode Go users. It answers three questions, fast:

1. **What does using this model cost right now?** — peak/off-peak state, current Plan Prices, next Pricing Window transition.
2. **Is there a better alternative for what I'm about to do?** — a Recommendation for the active Task Profile, with a short justification and top alternatives.
3. **Did anything change I should know about?** — a `⚠` badge when a Catalog Change is detected.

Personal-first, open source (MIT OR Apache-2.0), zero telemetry, zero credentials.

## 2. Problem

OpenCode Go prices, limits, capabilities and promotions change frequently, and DeepSeek — the primary model — has time-based pricing (peak = 2× off-peak, in specific UTC windows, with Chinese holiday exclusions). Practical consequences:

1. Sessions started in a Peak Window burn Allowance twice as fast, often unnoticed.
2. The best value model changes with time of day and task type.
3. Roster and limits drift weekly; nobody tracks it.
4. Manual model choice depends on memory and repeated doc/dashboard checks.
5. Cheapest-per-token ≠ cheapest-in-practice (cache price, tool-use reliability, failure retries).

Core question:

> **Which model should I use now, for this task, given cost and context?**

## 3. User

- Primary: the author — daily OpenCode Go user on macOS, alternates models, agentic coding with terminal/tools/MCP, wants max quality per Allowance dollar, maintains other AI subscriptions too.
- Secondary: any OpenCode Go user on macOS (open-source audience, contributors).
- Future: Linux users; target Pop!_OS + COSMIC (Phase 4).

## 4. Confirmed decisions

| # | Decision | Where |
|---|----------|-------|
| 1 | Stack: Tauri 2 (≥ 2.12.5, pin minor) with database- and platform-independent Rust core | [ADR-0001](./adr/0001-tauri-2-stack.md) |
| 2 | v0.1 = full Must list (see [mvp.md](./mvp.md)); Should items → v0.2 | [mvp.md](./mvp.md) |
| 3 | Catalog covers the full OpenCode Go Roster (~35–37 models) | [ADR-0002](./adr/0002-catalog-and-pricing-data-strategy.md) |
| 4 | Catalog source: fetch the OpenCode mirror + curated Overlays; bundled snapshot fallback | [ADR-0002](./adr/0002-catalog-and-pricing-data-strategy.md) |
| 5 | Capability data: curated per-profile scores in-repo, contributor-editable | [ADR-0002](./adr/0002-catalog-and-pricing-data-strategy.md) |
| 6 | Peak pricing computed locally (2× off-peak) + curated CN holiday calendar; makeup weekends are off-peak | [ADR-0002](./adr/0002-catalog-and-pricing-data-strategy.md) |
| 7 | Allowance in v0.1: static published Go monthly limits only; no consumption tracking | [mvp.md](./mvp.md) |
| 8 | Catalog Sync: every 6 h + at start + manual, with ETag; snapshot diff detects changes | [ADR-0002](./adr/0002-catalog-and-pricing-data-strategy.md) |
| 9 | Persistence: JSON files; SQLite deferred to Phase 2 | [ADR-0003](./adr/0003-persistence-json-now-sqlite-later.md) |
| 10 | Security boundary: no OpenCode internals/credentials in v0.1; integration is Phase 3 work | [ADR-0004](./adr/0004-no-opencode-internals-in-v01.md) |
| 11 | OSS: dual MIT/Apache-2.0, DCO, CI (fmt/clippy/test on macOS + Ubuntu), unsigned GitHub Releases, non-affiliation disclaimer | [backlog.md](./backlog.md) M0 |
| 12 | UI and docs language: English | — |
| 13 | Tray shows peak status only (`🟢 off` / `🔴 2×`) + `⚠`; full label in the menu, recommendation in menu/tooltip | [mvp.md](./mvp.md) |

## 5. Facts that corrected the original brief

- **macOS current is 27 "Golden Gate"** (26 Tahoe still supported). Tray left-click regression fixed in Tauri 2.12.0; use ≥ 2.12.5.
- **Tray title text on macOS is supported** by Tauri 2, but there is an open bug where *changing* the title can make the tray vanish (tauri#12060, open since Dec 2024). Mitigation: update title only on state transitions; verify early; fallback = colored icon + tooltip.
- **Upstream catalogs carry off-peak prices only, with zero peak/window metadata** (verified in models.dev and in OpenCode's local cache). Peak must be applied by us.
- **DeepSeek current pricing**: V4.1 Flash off-peak $0.15/$0.60/cache $0.003 per M tokens, peak exactly 2×; Peak Windows 01:00–04:00 and 06:00–10:00 UTC, Mon–Fri, excluding Chinese public holidays; makeup weekends are not peak. (Three price surfaces diverge — DeepSeek direct, Go docs, Console. We scope to the Go Plan Price.)
- **OpenCode Go**: $10/mo (Go Plus $40/mo); per-model monthly dollar limits with 5 h/weekly/monthly metering windows; token prices identical on both plans.
- **No public usage/allowance API.** Console dashboard only. Local usage data exists on disk, but v0.1 does not touch it (ADR-0004).
- **Programmatic sources that exist**: `GET https://opencode.ai/inference/go/v1/models` (anonymous, IDs only), `https://models.opencode.ai/api.json` (full catalog with prices; what OpenCode v2 itself consumes, ~10 min cache upstream), `https://models.dev/api.json` (same content).
- **OpenCode v2 runs a local authenticated HTTP API** (~120 routes: active model per session, sessions, SSE events, `/api/model` with costs). Phase 3 integration is far cheaper than the original brief assumed.
- **Model IDs differ per gateway** (`deepseek-flash` in the catalog vs `deepseek-v4.1-flash` on Go). The recommender keys on `providerID + modelID`.
- **Secret-leak hazards**: `opencode debug config` and `opencode auth export` print secrets in plaintext. Never call them (ADR-0004).

## 6. Scope

- **v0.1 (MVP)**: macOS menu bar — see [mvp.md](./mvp.md) for the full scope and acceptance criteria.
- **Out of v0.1**: pre-peak warning, change notifications, styled settings, favorites (→ v0.2); history, allowance tracking, usage estimate, compare view (→ Phase 2); OpenCode integration (→ Phase 3); Linux (→ Phase 4); smart routing (→ Phase 5). Deliberately excluded from all phases: automatic in-session model switching, request interception, hard blocking, cloud sync, Windows, ML-based recommendations.

## 7. Roadmap

1. **Phase 1 — v0.1 MVP (macOS)**: tray + peak monitor, menu, notifications, catalog sync + badge, Recommendation Engine v1, minimal settings.
2. **v0.2 (Should)**: pre-peak warning (X min), catalog-change notifications, settings polish, app icon.
3. **Phase 2 — Cost awareness**: manual allowance tracking, estimated burn, budget alerts, side-by-side compare (revisits ADR-0003 for SQLite).
4. **Phase 3 — OpenCode integration**: active-model and session detection via the local HTTP API, deep-link "open OpenCode with recommended model", usage import; read-only + consent-first (ADR-0004).
5. **Phase 4 — Linux**: SNI tray (icon-only UX; no title text or click events on Linux), Pop!_OS + COSMIC validation, packaging.
6. **Phase 5 — Smart routing**: model policies per Task Profile, cost/quality presets, multi-provider, fallback chains, success-history-based recommendations.

## 8. Architecture

```
opencode-cost-guard/
├── Cargo.toml              # workspace
├── crates/
│   ├── core/               # cost-guard-core: catalog, windows/schedule, recommendation,
│   │                       # config, snapshots — no Tauri/macOS deps (crate boundary enforces portability)
│   └── app/                # cost-guard-app: Tauri 2 shell — tray, menu, notifications, settings window
├── data/                   # curated Overlays (TOML): pricing windows, CN holidays, Go limits,
│   │                       # capability scores (source + lastVerified fields)
├── CONTEXT.md
└── docs/                   # brief, mvp, backlog, adr/, research snapshots
```

The core crate compiles and tests on Ubuntu in CI — the portability criterion is mechanical, not aspirational.

## 9. Risks

| Risk | Mitigation |
|------|------------|
| Tray vanishes when title updates (tauri#12060) | Update only on transitions; spike first; fallback icon+tooltip |
| Overlay drifts from upstream (peak 2×, limits) | `source`/`lastVerified` fields per overlay entry; review issue per upstream release |
| CN holiday table needs yearly refresh | `good first issue` template; validated data file |
| Roster drift between docs/endpoint/mirror | Mirror is the single truth; diff badge surfaces changes |
| OpenCode API is "Experimental" (Phase 3) | Keep integration behind a trait; tolerate version churn |
| COSMIC SNI untested | Phase 4 includes live testing before promising support |
| Recommendation perceived as authoritative | Explicit "recommended" wording + visible reasons + easy override |

## 10. Success criteria

See [mvp.md](./mvp.md) — 11 testable acceptance criteria, including the 2-week personal dogfooding gate.

## 11. Deferred questions (not blocking v0.1)

- Variant-level recommendations (DeepSeek `low|high|max`) — Phase 2/3.
- Multi-Plan support (Console pay-as-you-go) — Phase 2+.
- Favorites pinning, Homebrew tap, notarization — on demand.
