# Changelog

All notable changes to OpenCode Cost Guard are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 - 2026-10-24

First public pre-release: a menu-bar app for macOS backed by a portable Rust core. The build is ad-hoc signed but not notarized, so Gatekeeper requires the documented workaround on first launch.

### Added

**Core engine (`cost-guard-core`)**

- Pricing windows: peak and off-peak state derived from curated UTC windows, weekday rules and the Chinese holiday calendar, including makeup weekends, with the next transition shown in UTC and local time.
- Catalog: bundled snapshot plus curated TOML overlays for pricing windows, holidays, monthly limits and capability scores, with schema validation and `source`/`last_verified` metadata.
- Catalog sync: ETag-aware fetches of the public mirror, schema validation and a fallback chain (remote, last valid snapshot, bundled snapshot), plus change detection for the warning badge.
- Recommendation engine: hard filters per Task Profile, weighted scoring over peak-adjusted plan price, capability score, allowance headroom and context fit, with a top pick, a one-line reason and top alternatives; ties break toward the cheaper model.
- JSON configuration and snapshots, typed errors and a deterministic offline demo (`status` example).
- The core crate stays free of Tauri, Objective-C and Windows dependencies; CI enforces the boundary.

**App shell (`cost-guard-app`)**

- Menu-bar tray with peak state at a glance (`🟢 DS off` / `🔴 DS 2×`) and a `⚠` badge for unacknowledged catalog changes.
- Menu with DeepSeek state and next transition, peak-aware plan prices, Task Profile switcher, recommendation with reasons and alternatives, manual sync, Settings and Quit.
- Peak enter and exit notifications, each toggleable and delivered at most once per transition.
- Settings window for autostart, notification toggles, default Task Profile, sync interval and ignored models, with validation against the core `Config`.
- Bundled app runs as an accessory (`LSUIElement`, no Dock icon or Cmd-Tab entry) with LaunchAgent autostart.

### Fixed

- Peak pricing now applies per vendor, so only models of the vendor with an active Pricing Window are multiplied; a uniform multiplier previously canceled out during normalization and never changed a recommendation.
- Cost scoring uses a log-scaled min-max, so a free model no longer drives every paid candidate's cost score to zero. Peak can now flip a recommendation.
- Recommendation reasons expose a `DataTerms` note when a pick may use prompts for training.
- Tray title updates deduplicate, fallback icons stay peak-aware, bundled-data failures no longer stop the app, and sync runs at startup.

### Notes

- macOS 10.13 or newer on Apple silicon. Intel builds are not produced for v0.1.0.
- Install and checksum verification: [docs/releasing.md](docs/releasing.md).
- Unsigned and notarization policy: [ADR-0008](docs/adr/0008-unsigned-github-releases.md).
