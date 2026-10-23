# Backlog

Prioritized delivery plan. Milestones are sequential; issues inside a milestone can be parallelized. Vocabulary: [CONTEXT.md](../CONTEXT.md).

## M0 — Repo scaffold

- [ ] `git init`, workspace layout (`crates/core`, `crates/app`, `data/`, `docs/`), `.gitignore`, `rust-toolchain.toml`
- [ ] Licenses: MIT + Apache-2.0 dual; README (what/why, build from source, Gatekeeper note for unsigned builds, non-affiliation disclaimer)
- [ ] CONTRIBUTING.md (dev setup, DCO, how to update Overlays), issue/PR templates
- [ ] CI: fmt + clippy (`-D warnings`) + test, matrix `macos-latest` + `ubuntu-latest` (core crate proves portability)
- [ ] Labels incl. `good first issue` (Overlay data updates are the natural entry point for contributors)

## M1 — Core (`cost-guard-core`)

- [ ] Domain types: Catalog, Model Entry, Provider, Roster, Plan Price (serde)
- [ ] Schedule: Pricing Window state machine — DeepSeek windows 01:00–04:00 / 06:00–10:00 UTC Mon–Fri, CN holiday exclusion, makeup-weekend rule; fixed-clock tests (boundaries, weekends, holidays, year rollover)
- [ ] Overlays loader: pricing windows, CN holidays, Go limits, capability scores — validation + `source`/`lastVerified` required; fixture tests
- [ ] Catalog Sync: fetch mirror (ETag), schema validation, fallback chain (remote → last valid → bundled); HTTP client injected for tests
- [ ] Snapshot store: current + previous snapshot (JSON); Catalog Change diff
- [ ] Recommendation Engine v1: per-profile hard filters + weighted score + human-readable reasons; table-driven tests (5 profiles × peak/off-peak)
- [ ] Config: load/save/validate (JSON), defaults

## M2 — App shell (`cost-guard-app`)

- [ ] Spike: tray title update behavior (tauri#12060) — confirm transitions don't vanish the tray; decide fallback (icon+tooltip)
- [ ] Tauri setup (≥ 2.12.5, pinned minor): tray title states, `⚠` badge logic
- [ ] Dynamic native menu rendering from state (state + next transition + prices + profile + recommendation + actions)
- [ ] Notifications on Peak enter/exit (tauri-plugin-notification)
- [ ] Autostart toggle (tauri-plugin-autostart, LaunchAgent)
- [ ] Minimal settings window (HTML/TS) + config wiring
- [ ] `LSUIElement` for bundled builds; verify no Dock icon + correct dev vs bundled behavior
- [ ] Sync scheduler (6 h + manual), ETag, badge clearing

## M3 — v0.1.0 release

- [ ] Dogfooding: 2 weeks daily use; log incidents; verify acceptance criteria (including idle footprint measurement)
- [ ] README polish + screenshots; document unsigned-binary install workaround
- [ ] Release workflow: tag → artifacts + checksums on GitHub Releases
- [ ] Retro: revisit ADRs if reality contradicted them

## v0.2 — Should

- [ ] Pre-peak warning: configurable "N minutes before Peak" notification
- [ ] Catalog-change notifications (reuses M1 diff)
- [ ] Styled settings window; favorites
- [ ] App icon

## Phase 2 — Cost awareness

- [ ] SQLite migration when history lands (revisit ADR-0003)
- [ ] Price/allowance history; manual allowance tracking; estimated burn; budget alerts
- [ ] Side-by-side model comparison (Plan Prices + peak projection)

## Phase 3 — OpenCode integration

- [ ] Design doc: read-only + consent-first integration via OpenCode's local HTTP API (ADR-0004 gates this)
- [ ] Active-model / active-session detection; SSE events
- [ ] Deep-link / command "open OpenCode with recommended model"
- [ ] Usage import (local stats) feeding allowance awareness
- [ ] Variant-aware recommendations (`low|high|max`)

## Phase 4 — Linux

- [ ] SNI tray via libappindicator; icon-only UX (no title text, no click events on Linux)
- [ ] Live validation on Pop!_OS + COSMIC (SNI in `cosmic-applet-status-area`)
- [ ] Packaging: .deb / AppImage; notifications verification

## Phase 5 — Smart routing

- [ ] Per-profile model policies; cost/quality presets
- [ ] Multi-provider / multi-subscription fallback chains
- [ ] Success-history-informed recommendations
