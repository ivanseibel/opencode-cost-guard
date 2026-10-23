# Persistence: JSON files in v0.1, SQLite deferred until history lands

## Status

accepted

v0.1 persists everything as human-readable JSON on disk: user config, the current Catalog snapshot and the previous snapshot (for change detection). Curated overlays ship as TOML data files in the repo (TOML for comments — `source` and `lastVerified` annotations matter). No database. v0.1 has no query needs: the app holds one catalog in memory and diffs two files. SQLite earns its complexity only when price history, allowance tracking or usage estimation arrive (Phase 2), and the JSON→SQLite migration then is a day of work, not a rewrite.

## Consequences

- "Something changed" detection compares exactly two snapshots; no long history until Phase 2.
