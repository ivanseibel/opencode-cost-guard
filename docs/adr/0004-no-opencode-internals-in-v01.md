# v0.1 never touches OpenCode internals, credentials or data

## Status

accepted

The app answers pricing questions from public sources (catalog mirror + curated overlays) and its own config. It never reads OpenCode's SQLite database, auth files or service password; never shells out to `opencode auth export` or `opencode debug config` (both print secrets in plaintext — verified); and requires no credentials of any kind. Detecting the active model and sessions — which OpenCode's local HTTP API already supports — is deliberate Phase 3 work, to be designed as a read-only, consented surface rather than file scavenging.

## Considered Options

- **Zero coupling** (chosen): trivially auditable security posture; slower roadmap.
- **Read local cache/DB now**: fresher data and usage stats today, but couples to an undocumented internal format, needs the service password, and puts sensitive files in scope.

## Consequences

- Cost-awareness features (today's spend, per-model usage) cannot land before the integration work decides how to read usage safely (Phase 3); until then allowance-aware features rely on manual input and published limits.
