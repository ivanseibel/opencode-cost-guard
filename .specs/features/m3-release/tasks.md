# M3 Release Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.**

---

**Design**: skipped (Medium; decisions live in `spec.md` Assumptions + `docs/adr/0008-unsigned-github-releases.md`)
**Status**: Draft

---

## Test Coverage Matrix

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Release guard script | none - executed by the gate | All invariants; non-zero on each missing item | `scripts/check-release-config.sh` | `bash scripts/check-release-config.sh` |
| Workflow / docs / changelog | none - guard script covers invariants | Guard exits 0 | `.github/workflows/release.yml`, `docs/releasing.md`, `README.md`, `CHANGELOG.md` | build gate |
| Local dry-run evidence | none - recorded evidence | Commands, paths, sizes, checksums | `.specs/features/m3-release/dry-run.md` | build gate |

## Gate Check Commands

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | Script-only iterations | `bash scripts/check-release-config.sh` |
| Build | Every task | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo check -p cost-guard-app && bash scripts/check-core-purity.sh` |
| Release | T4+ and CI | `bash scripts/check-release-config.sh` |

---

## Execution Plan

Single phase, sequential.

### Phase 1: Release pipeline

```
T1 → T2 → T3 → T4 → T5
```

---

## Task Breakdown

### T1: Changelog and version alignment

**What**: Add `CHANGELOG.md` with a `0.1.0` entry summarizing the core engine and the M2 app shell; confirm workspace and tauri versions are `0.1.0`.
**Where**: `CHANGELOG.md`
**Depends on**: None
**Reuses**: `docs/mvp.md`, `docs/backlog.md`, `.specs/features/*`
**Requirement**: REL-05, REL-08

**Done when**:

- [x] `CHANGELOG.md` has `## 0.1.0` with core + app-shell highlights
- [x] Workspace version and `crates/app/tauri.conf.json` version are both `0.1.0`
- [x] Gate passes: build gate command (guard script arrives in T4; until then run the rest)

**Tests**: none (docs/versions; guard covers in T4)
**Gate**: build
**Commit**: `docs: add 0.1.0 changelog`

---

### T2: Release workflow

**What**: Add `.github/workflows/release.yml`: tag push builds/signs/zips/checksums and creates a GitHub Release; manual dispatch builds artifacts only.
**Where**: `.github/workflows/release.yml`
**Depends on**: T1
**Reuses**: packaging recipe from `.github/workflows/ci.yml`
**Requirement**: REL-01, REL-02, REL-03, REL-04

**Done when**:

- [x] Tag trigger `v*`; dispatch runs build-only without release creation
- [x] Steps: install tauri-cli 2.12.4, build bundle, ad-hoc codesign, zip `CostGuard-${tag}-macos-arm64.zip`, `shasum -a 256` → `SHA256SUMS.txt`, `gh release create` with notes file
- [x] Notes include the unsigned warning, install steps and checksum verification
- [x] Job permissions `contents: write` only
- [x] YAML parses (`python3`/`ruby` YAML)
- [x] Gate passes: build gate command

**Tests**: none (workflow; guard covers invariants in T4)
**Gate**: build
**Commit**: `ci: add tag-driven release workflow`

---

### T3: Release documentation

**What**: Add `docs/releasing.md` and update `README.md`/`CONTRIBUTING.md` with install, checksum verification, Gatekeeper workaround, unsigned status and the approval-gated release procedure (tag is created only after explicit approval).
**Where**: `docs/releasing.md`
**Depends on**: T2
**Reuses**: `docs/adr/0008-unsigned-github-releases.md`
**Requirement**: REL-06

**Done when**:

- [ ] Docs include `xattr -dr com.apple.quarantine` and state the build is unsigned/not notarized
- [ ] Checksum verification instructions use `SHA256SUMS.txt`
- [ ] Release procedure states: local dry-run → approval → tag → push tag → workflow publishes
- [ ] README links install/release docs; CONTRIBUTING links releasing guide
- [ ] Gate passes: build gate command

**Tests**: none (docs; guard covers required strings)
**Gate**: build
**Commit**: `docs: add release and install guide`

---

### T4: Release config guard

**What**: Add `scripts/check-release-config.sh` asserting version consistency, changelog entry, workflow invariants (tag trigger, checksum, `gh release create`, `contents: write`, codesign) and documented Gatekeeper note; wire it into CI.
**Where**: `scripts/check-release-config.sh`
**Depends on**: T3
**Reuses**: existing purity-script style
**Requirement**: REL-09

**Done when**:

- [ ] Script exits 0 on the current tree and non-zero when any invariant is removed (test by temporary mutation, then restore)
- [ ] CI runs it (both OS jobs or the macOS job)
- [ ] Gate passes: build + release gate commands
- [ ] Short usage comment at the top of the script

**Tests**: none - script is the gate; self-check documented
**Gate**: build
**Commit**: `ci: guard release configuration`

---

### T5: Local release dry-run

**What**: Execute the packaging chain locally (build bundle, ad-hoc sign, zip, SHA-256) without tagging/pushing/publishing, and record commands, artifact paths, sizes and checksums.
**Where**: `.specs/features/m3-release/dry-run.md`
**Depends on**: T4
**Reuses**: `cargo tauri build --bundles app`
**Requirement**: REL-07

**Done when**:

- [ ] Bundle rebuilt, ad-hoc signed, zipped as `CostGuard-v0.1.0-macos-arm64.zip`
- [ ] `SHA256SUMS.txt` generated and verified with `shasum -a 256 -c`
- [ ] `dry-run.md` records commands, paths, sizes, checksums and timestamp
- [ ] Explicit note: no tag, push or GitHub Release performed
- [ ] Gate passes: build + release gate commands

**Tests**: none (recorded evidence)
**Gate**: build
**Commit**: `docs: record m3 release dry-run`

---

## Phase Execution Map

```
T1 → T2 → T3 → T4 → T5
```

**Batch plan:** 5 tasks in a single batch (one OpenCode worker). Verifier runs after T5. Publishing (tag/push/release) is a separate approval-gated action and is NOT part of this batch.

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Changelog | 1 file | ✅ Granular |
| T2: Release workflow | 1 workflow | ✅ Granular |
| T3: Docs | 1 guide + 2 edits | ✅ Granular |
| T4: Guard script | 1 script + CI line | ✅ Granular |
| T5: Dry-run | 1 evidence file | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | start | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer | Matrix Requires | Task Says | Status |
| ---- | ---------- | --------------- | --------- | ------ |
| T1 | Docs/versions | none | none | ✅ OK |
| T2 | Workflow | none | none | ✅ OK |
| T3 | Docs | none | none | ✅ OK |
| T4 | Guard script | none | none | ✅ OK |
| T5 | Evidence | none | none | ✅ OK |
