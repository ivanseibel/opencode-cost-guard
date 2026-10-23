# Catalog from the OpenCode mirror with curated overlays; peak pricing applied locally

## Status

accepted

The Catalog is synced by fetching `https://models.opencode.ai/api.json` (OpenCode's mirror of models.dev — identical content, no authentication), validated against a schema, with the previous snapshot kept for change detection and a bundled snapshot as offline fallback. Upstream carries only flat, off-peak prices and no window metadata (verified in models.dev and in OpenCode's local cache), so everything time- and plan-specific lives in curated overlays shipped with the app and maintainable by contributors: DeepSeek peak windows (UTC weekday intervals), the 2× peak multiplier, Chinese holiday exclusions, OpenCode Go monthly limits, and per-profile capability scores. The v0.1 catalog scope is the `opencode-go` Roster only. We do not scrape the docs site, and v0.1 does not read OpenCode's local cache or database — the app works with zero credentials and no coupling to OpenCode internals (see ADR-0004).

## Considered Options

- **Direct fetch + curated overlays** (chosen): fresh roster/prices without scraping; small, review-friendly curation surface; no auth.
- **Scraping the OpenCode Go docs tables**: richer plan data, but fragile and near-duplicate of the mirror's content.
- **Reading OpenCode's local SQLite cache / `opencode api`**: freshest data, but couples to OpenCode internals and requires credentials the app otherwise never needs.

## Consequences

- The peak price is computed locally (2× off-peak); if DeepSeek's multiplier changes, only an overlay changes — but displayed prices can silently diverge from upstream until the overlay is reviewed. Overlay entries therefore carry `source` and `lastVerified` fields.
- Change detection requires snapshot storage (see ADR-0003).
