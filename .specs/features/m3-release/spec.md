# M3 Release Specification

## Problem Statement

M2 ships a verified `.app`, but releases are still manual: no tag-driven pipeline, no checksums, no release notes, no documented install path for unsigned builds. This feature makes v0.1.0 publishable from a tag, with an explicit unsigned-distribution policy and a local dry-run that proves the packaging steps without touching GitHub.

## Goals

- [ ] Pushing a `v*` tag builds, signs (ad-hoc), zips and checksums the macOS app, then publishes a GitHub Release with notes and both artifacts
- [ ] Manual dispatch builds artifacts without creating a release
- [ ] Version consistency and release-config invariants are enforced by a script that runs in CI
- [ ] Docs explain install, checksum verification and the Gatekeeper workaround; publishing stays approval-gated
- [ ] A local dry-run proves build → sign → zip → checksums with recorded evidence

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Notarization / Developer ID signing | ADR-0008: unsigned releases for v0.1; revisit on external demand |
| Homebrew tap, updater, DMG | Backlog v0.2+/Phase 2 |
| Linux/Windows artifacts | Phase 4 / out of platform scope |
| Auto-publish from branch pushes or PRs | Releases are approval-gated |
| App icon redesign | v0.2 |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Version | `0.1.0` already set in workspace and tauri config; tag `v0.1.0` created locally only after explicit approval | Single source of truth; no bump needed for M3 | y |
| Distribution policy | Ad-hoc-signed, unsigned-for-Gatekeeper `.app` zip; notarization deferred | ADR-0008 | y |
| Artifact naming | `CostGuard-<tag>-macos-arm64.zip` + `SHA256SUMS.txt` | Clear, versioned, verifiable | y |
| Trigger policy | Release job on tag push; manual dispatch builds artifact only, never publishes | Prevents accidental releases | y |
| Release notes | Generated file with unsigned warning, install steps, checksum verification, plus the CHANGELOG entry | One source, explicit friction | y |
| Permissions | `contents: write` only in the release job | Least privilege | y |
| Platform | `macos-latest` (arm64 runner) | Matches the verified local target | y |

**Open questions:** none - all resolved or logged above.

---

## User Stories

### P1: Tag-driven release ⭐ MVP

**User Story**: As the maintainer, I want a tag push to produce a complete GitHub Release so shipping v0.1.0 is one approved action.

**Acceptance Criteria**:

1. REL-01 WHEN a `v*` tag is pushed THEN the workflow SHALL build the app on macOS, ad-hoc sign the bundle, zip it and generate `SHA256SUMS.txt`.
2. REL-02 WHEN the tag build succeeds THEN the workflow SHALL create a GitHub Release for that tag with the zip and checksum file attached, including the unsigned-build warning and install/verification instructions.
3. REL-03 The workflow SHALL publish only for tag pushes; manual dispatch SHALL build and upload artifacts without creating a release.
4. REL-04 The release job SHALL request only `contents: write` permission and rely on the default token.
5. REL-08 The workspace version, the app's `tauri.conf.json` version and the release tag SHALL be equal before publishing.

**Independent Test**: Local dry-run reproduces every command; workflow YAML parses; version guard script exits 0.

---

### P1: Release guard and docs ⭐ MVP

**User Story**: As a maintainer, I want a guard script and docs so a broken release config fails in CI, not at publish time.

**Acceptance Criteria**:

1. REL-05 `CHANGELOG.md` SHALL contain a `0.1.0` entry listing core and app-shell highlights.
2. REL-06 `docs/releasing.md`, `README.md` and `CONTRIBUTING.md` SHALL document build, checksum verification, the Gatekeeper workaround (`xattr -dr com.apple.quarantine`), unsigned status and the approval-gated release process.
3. REL-09 `scripts/check-release-config.sh` SHALL validate version consistency, the required release-workflow steps, changelog presence and the documented Gatekeeper note, exiting non-zero on any missing invariant; CI SHALL run it.

**Independent Test**: `bash scripts/check-release-config.sh` exits 0; deleting a required workflow step makes it fail.

---

### P1: Local dry-run ⭐ MVP

**User Story**: As the maintainer, I want proof the packaging works before pushing a tag.

**Acceptance Criteria**:

1. REL-07 A recorded local dry-run SHALL reproduce build → ad-hoc sign → zip → SHA-256 checksums without creating a tag, pushing, or publishing, and SHALL record commands, artifact paths, sizes and checksums.

**Independent Test**: Follow `dry-run.md`; artifacts and checksums match.

---

## Edge Cases

- IF the tag does not match the workspace version THEN `check-release-config.sh` SHALL fail.
- IF `CHANGELOG.md` lacks the version entry THEN the guard SHALL fail.
- IF the release workflow loses the checksum or release-creation step THEN the guard SHALL fail.
- IF the docs lose the Gatekeeper workaround THEN the guard SHALL fail.
- IF the artifact is missing after the zip step THEN the workflow SHALL fail before release creation.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| REL-01 | P1: Release | Design | Pending |
| REL-02 | P1: Release | Design | Pending |
| REL-03 | P1: Release | Design | Pending |
| REL-04 | P1: Release | Design | Pending |
| REL-05 | P1: Guard/docs | Design | Pending |
| REL-06 | P1: Guard/docs | Design | Pending |
| REL-07 | P1: Dry-run | Design | Pending |
| REL-08 | P1: Release | Design | Pending |
| REL-09 | P1: Guard/docs | Design | Pending |

**ID format:** `[CATEGORY]-[NUMBER]`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 9 total, 0 mapped to tasks (Tasks phase pending)

---

## Success Criteria

- [ ] `bash scripts/check-release-config.sh` and the full build gate are green
- [ ] Local dry-run produces the zip + checksums with recorded evidence
- [ ] Verifier PASS with per-AC evidence and killed sensor mutations
- [ ] No tag, push or release happens without explicit approval
