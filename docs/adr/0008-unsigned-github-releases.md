# Unsigned GitHub Releases; notarization deferred

## Status

accepted

v0.1.0 is distributed as an ad-hoc-signed, non-notarized macOS `.app` zip attached to GitHub Releases. Release notes and the README state the unsigned status and the Gatekeeper workaround (`xattr -dr com.apple.quarantine`, or right-click → Open). We do not buy an Apple Developer ID or add notarization to CI now: it costs $99/year plus certificate secrets and a notarization step, while the primary user builds from source and the OSS audience is small. Revisit when external users or a Homebrew tap make trust friction a real cost.

## Consequences

- macOS Gatekeeper warns on first launch; docs must always include the workaround.
- The Tauri updater stays out of scope until signing is consistent.
- The release workflow stays simple: build, ad-hoc sign, zip, checksum, publish.
