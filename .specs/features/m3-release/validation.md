# m3-release Validation

**Date**: 2026-10-24
**Spec**: `.specs/features/m3-release/spec.md`
**Diff range**: `5934b55..HEAD` (`144eb94..16b6914`, five feature commits; 9 files, +482/−24)
**Verifier**: independent sub-agent (author ≠ verifier); read-only over the real tree, mutations confined to a disposable worktree
**Report**: PASS ✅

**Result**: PASS ✅ — 9/9 acceptance criteria matched, 7/7 spec-scoped guard mutations killed, 6/6 gates green.

No tag, push or GitHub Release exists: `git tag --list` is empty, `git ls-remote --tags origin` is empty, `gh api repos/ivanseibel/opencode-cost-guard/tags` returns `0`, `gh api repos/ivanseibel/opencode-cost-guard/releases` returns `0`, and `gh release list` is empty. HEAD is `16b6914` with a clean tree.

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 changelog + version alignment | ✅ Done | `CHANGELOG.md:5`, `Cargo.toml:6`, `crates/app/tauri.conf.json:4` all `0.1.0` |
| T2 release workflow | ✅ Done | `.github/workflows/release.yml:1-119` |
| T3 release docs | ✅ Done | `docs/releasing.md`, `README.md`, `CONTRIBUTING.md` |
| T4 release config guard | ✅ Done | `scripts/check-release-config.sh:1-122`, CI wiring `.github/workflows/ci.yml:55-56` |
| T5 local dry-run | ✅ Done | `.specs/features/m3-release/dry-run.md`, artifacts verified on disk |

All checkboxes in `tasks.md` are checked; no blocked or partial task.

---

## Spec-Anchored Acceptance Criteria

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| REL-01: `v*` tag → build macOS app, ad-hoc sign, zip, `SHA256SUMS.txt` | Job on `macos-latest` runs `cargo tauri build --bundles app`, `codesign --force --deep -s -`, `ditto` to `CostGuard-<tag>-macos-arm64.zip`, `shasum -a 256 … > SHA256SUMS.txt` | `.github/workflows/release.yml:5-6` (tag filter), `:18` (macos-latest), `:38` (build), `:42` (ad-hoc sign), `:51-54` (name + zip + checksum). YAML parses (PyYAML + Ruby); each `run:` block passes `bash -n`. Dry-run reproduces the same chain: `dry-run.md:15-23`. | ✅ PASS (static; tag publish intentionally not executed) |
| REL-02: tag build → GitHub Release for that tag with zip + checksums, notes include unsigned warning + install/verification | `gh release create "$GITHUB_REF_NAME" <zip> SHA256SUMS.txt --notes-file`, notes contain "not notarized", install steps, `shasum -a 256 -c`, `xattr`, changelog excerpt | `.github/workflows/release.yml:110-119` (`gh release create`, both assets attached), `:74` (unsigned/not-notarized warning), `:79-91` (install + checksum + xattr), `:107` (CHANGELOG excerpt). Notes generator executed in a temp dir: exit 0, output contained `not notarized`, `shasum -a 256 -c SHA256SUMS.txt`, `xattr -dr com.apple.quarantine`, `CostGuard-v0.1.0-macos-arm64.zip`, `## 0.1.0`. | ✅ PASS |
| REL-03: publish only for tag pushes; manual dispatch builds/uploads without release | Publish step gated on `refs/tags/v`; dispatch reaches build + `upload-artifact` only | `.github/workflows/release.yml:7` (`workflow_dispatch`), `:67` and `:111` (`if: startsWith(github.ref, 'refs/tags/v')` on notes + publish), `:57-64` (upload runs unconditionally). `docs/releasing.md:75` states dispatch never creates a release. Parsed workflow confirms both publish steps carry the tag-only condition. | ✅ PASS |
| REL-04: release job requests only `contents: write`, default token | Workflow `contents: read`; release job `contents: write`; `GH_TOKEN: ${{ github.token }}` | `.github/workflows/release.yml:9-10` (workflow read), `:19-20` (job write only), `:113` (`github.token`). No other permission keys in the job. | ✅ PASS |
| REL-05: `CHANGELOG.md` has `0.1.0` entry with core + app-shell highlights | `## 0.1.0` heading plus `cost-guard-core` and `cost-guard-app` sections | `CHANGELOG.md:5` (`## 0.1.0 - 2026-10-24`), `:11` (`**Core engine (cost-guard-core)**`), `:20` (`**App shell (cost-guard-app)**`). Guard asserts the heading: `scripts/check-release-config.sh:89-91`; sensor M2 killed. | ✅ PASS |
| REL-06: `docs/releasing.md`, `README.md`, `CONTRIBUTING.md` document build, checksum verification, `xattr -dr com.apple.quarantine`, unsigned status, approval-gated release | All five topics present across the three docs; release path is local dry-run → approval → tag → push | `docs/releasing.md:3` (unsigned/not notarized), `:13` (checksum), `:26` (xattr), `:33-52` (build/sign), `:56-71` (approval → tag → push); `README.md:104-116` (unsigned, xattr, `SHA256SUMS.txt`, `shasum -a 256 -c`, approval-gated guide); `CONTRIBUTING.md:62-66` (xattr), `:176-180` (unsigned, approval-gated, tag flow, links guide). | ✅ PASS ⚠️ (CONTRIBUTING references checksum verification via the guide instead of restating the command — see gap 3) |
| REL-07: recorded local dry-run of build → ad-hoc sign → zip → SHA-256, no tag/push/publish, with commands, paths, sizes, checksums | `CostGuard-v0.1.0-macos-arm64.zip` + `SHA256SUMS.txt`, recorded hash matches, explicit "no tag/push/release" note | `dry-run.md:10` (no tag/push/release), `:12-24` (commands), `:47-54` (paths/sizes), `:61` (hash), `:67` (`OK`). On disk: zip `4,849,409` bytes, `SHA256SUMS.txt` `99` bytes, `shasum -a 256 -c SHA256SUMS.txt` → `CostGuard-v0.1.0-macos-arm64.zip: OK`, hash `d78e92113b79bbf4938cc12fd2fdcd963995db106d1b7d1226e6865ab2860421`. Bundle verifies ad-hoc (`codesign --verify` exit 0, `Signature=adhoc`), plist versions `0.1.0`. | ✅ PASS ⚠️ (bundle size recorded as `du` allocation, not logical bytes — see gap 5) |
| REL-08: workspace version = `tauri.conf.json` version = release tag before publishing | `0.1.0` in both files; guard rejects a mismatched tag | `Cargo.toml:6` (`version = "0.1.0"`), `crates/app/tauri.conf.json:4` (`"version": "0.1.0"`), `scripts/check-release-config.sh:85-87` (file mismatch) and `:108-114` (tag check). `bash scripts/check-release-config.sh v0.1.0` → exit 0; `… v0.2.0` → exit 1 (`tag v0.2.0 does not match workspace version 0.1.0`). No tag exists yet. | ✅ PASS ⚠️ (workflow does not invoke the tag-mode guard — see gap 1) |
| REL-09: guard validates version consistency, workflow steps, changelog, Gatekeeper note; non-zero on missing invariant; CI runs it | Exit 0 on clean tree; exit 1 per removed invariant; CI step present | `scripts/check-release-config.sh:62-114` (all invariants, aggregated failures), `:116-122` (non-zero + report); `.github/workflows/ci.yml:55-56` (CI step). Clean run → exit 0, `release config OK (version 0.1.0)`. Sensor M1-M7 all exit 1. | ✅ PASS |

**Status**: ✅ 9/9 acceptance criteria matched the spec-defined outcome; 3 precision/automation gaps flagged (non-blocking, detailed below).

---

## Workflow Validation

| Check | Evidence | Result |
| ----- | -------- | ------ |
| YAML parses | `python3` (PyYAML) and `ruby -ryaml` both load `release.yml` (1 job: `release`) and `ci.yml` (2 jobs: `check`, `package`) | ✅ |
| Every `run:` block is valid shell | 7 extracted run scripts (`Install pinned toolchain`, tauri-cli, build, sign, package, notes, publish) all pass `bash -n` | ✅ |
| Release only on tag push | `release.yml:67`, `:111` — `if: startsWith(github.ref, 'refs/tags/v')` on both notes and publish; trigger `:5-6` | ✅ |
| `workflow_dispatch` builds without publishing | Trigger `:7`; dispatch reaches checkout/toolchain/build/sign/package/upload only; both tag-gated steps skip; `docs/releasing.md:75` documents this | ✅ |
| Permissions least privilege | Workflow `contents: read` (`:9-10`); single job overrides with `contents: write` only (`:19-20`); token `${{ github.token }}` (`:113`) | ✅ |
| Notes contain unsigned warning, install, checksum, xattr | Generated notes in temp workspace: `not notarized` ×2, `shasum -a 256 -c SHA256SUMS.txt`, `xattr -dr com.apple.quarantine`, download filename, `## 0.1.0` excerpt | ✅ |
| Artifact naming + checksum steps | `release.yml:51-54`: `zip="CostGuard-${TAG}-macos-arm64.zip"`, `ditto -c -k --keepParent`, `test -f`, `shasum -a 256 … > SHA256SUMS.txt`; matches dry-run `CostGuard-v0.1.0-macos-arm64.zip` | ✅ |
| Missing-artifact guard before publish | `release.yml:50` (`test -d "$app"`), `:53` (`test -f "$zip"`), `set -e` default GitHub shell → job fails before `gh release create` | ✅ (static only; not executed in CI) |
| CI also packages on tag/dispatch | `ci.yml:60` gate + `:78-93`; does not publish | ✅ |

---

## Discrimination Sensor

Isolated scratch: `git worktree add --detach <tmp>/m3-verify-scratch HEAD` (no `git stash`). Baseline guard in scratch: exit 0. Each mutation applied to pristine files, guard run, files restored, guard re-run (exit 0) before the next mutation. Worktree removed with `git worktree remove --force` afterwards. Real-tree `git status --porcelain` was empty before and after; no stash entries were created.

### Spec-scoped mutations (required)

| # | Mutation | File:line | Expected | Guard output | Killed? |
| - | -------- | --------- | -------- | ------------ | ------- |
| M1 | Tauri version `0.1.0` → `0.1.1` (workspace/tauri mismatch) | `crates/app/tauri.conf.json:4` | exit 1 | `version mismatch: Cargo.toml has 0.1.0, crates/app/tauri.conf.json has 0.1.1` | ✅ Killed |
| M2 | Changelog heading `## 0.1.0` → `## 0.0.9` | `CHANGELOG.md:5` | exit 1 | `changelog entry for 0.1.0: CHANGELOG.md does not contain '## 0.1.0'` | ✅ Killed |
| M3 | Delete `shasum -a 256 "$zip" > SHA256SUMS.txt` | `.github/workflows/release.yml:54` | exit 1 | `release workflow checksum step: … does not contain 'shasum …'` | ✅ Killed |
| M4 | `gh release create` → `gh release upload` (no release) | `.github/workflows/release.yml:115` | exit 1 | `release workflow release creation: … does not contain 'gh release create'` | ✅ Killed |
| M5 | Remove every `xattr -dr com.apple.quarantine` occurrence (install + troubleshooting) | `docs/releasing.md:26,83` | exit 1 | `documented Gatekeeper workaround: docs/releasing.md does not contain 'xattr -dr com.apple.quarantine'` | ✅ Killed |
| M6 | Job permission `contents: write` → `contents: read` | `.github/workflows/release.yml:20` | exit 1 | `release workflow job permissions: … does not contain 'contents: write'` | ✅ Killed |
| M7 | Tag filter `- "v*"` → `- "*"` (any tag publishes) | `.github/workflows/release.yml:6` | exit 1 | `release workflow v* tag filter: … does not contain '- "v*"'` | ✅ Killed |

### Supplementary hardening probes (outside REL-09's enumerated invariants)

| # | Probe | File:line | Guard | Reading |
| - | ----- | --------- | ----- | ------- |
| P1 | Delete both `if: startsWith(github.ref, 'refs/tags/v')` gates | `.github/workflows/release.yml:67,111` | exit 0 — survived | Guard does not assert REL-03's publish gate; current workflow is correct. Hardening gap (gap 2). |
| P2 | Delete `test -f "$zip"` artifact assertion | `.github/workflows/release.yml:53` | exit 0 — survived | Guard does not assert edge case 5; workflow has the assertion today. Hardening gap (gap 4). |
| P3 | Replace README `xattr -dr …` with `xattr -c …` | `README.md:111` | exit 0 — survived | Guard only reads `docs/releasing.md`; REL-06 is satisfied today. Hardening gap (gap 3). |
| P4 | Remove CONTRIBUTING's link to `docs/releasing.md` | `CONTRIBUTING.md:178` | exit 0 — survived | Same as P3. |

**Sensor depth**: P0-full (7 spec-scoped behavior mutations, above the ≥5 requirement).
**Result**: 7/7 required mutations killed — PASS ✅. 4/4 out-of-scope probes survived and are recorded as hardening recommendations, not AC failures.

---

## Edge Cases

- [x] Tag ≠ workspace version → `bash scripts/check-release-config.sh v0.2.0` exits 1 with the mismatch message (`scripts/check-release-config.sh:108-114`).
- [x] Missing changelog version entry → sensor M2 exit 1 (`scripts/check-release-config.sh:89-91`).
- [x] Workflow loses checksum or release-creation step → sensor M3/M4 exit 1 (`scripts/check-release-config.sh:95-96`).
- [x] Docs lose Gatekeeper workaround → sensor M5 exit 1 (`scripts/check-release-config.sh:100`).
- [x] Missing artifact after zip → `test -d "$app"` / `test -f "$zip"` under the default `set -e` shell (`release.yml:50,53`) stops the job before publish. Static only; guard does not protect this line (probe P2).

---

## Gate Check

Gate command (Build + Release, `tasks.md:27-28`):

| # | Command | Exit | Evidence |
| - | ------- | ---- | -------- |
| 1 | `cargo fmt --check` | 0 | no output |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | `No issues found` |
| 3 | `cargo test --workspace` | 0 | 155 passed, 0 failed, 0 ignored (64 core unit + 87 core integration/unit + 4 recommendation matrix; matches `dry-run.md:75`) |
| 4 | `cargo check -p cost-guard-app` | 0 | `Finished dev profile` |
| 5 | `bash scripts/check-core-purity.sh` | 0 | `core dependency purity OK` |
| 6 | `bash scripts/check-release-config.sh` | 0 | `release config OK (version 0.1.0)` |

- **Total**: 6/6 passed, 0 failed, 0 skipped.
- **Test count**: 155 after the feature; the feature adds no tests (docs/CI only), matching `tasks.md:14-20` (matrix requires none for all layers). No tests were deleted or weakened.
- **Real tree**: `git status --porcelain` empty before gates and after the sensor cleanup.

---

## Dry-Run Evidence

| Item | Recorded (`dry-run.md`) | Observed on disk | Match |
| ---- | ----------------------- | ---------------- | ----- |
| Zip size | 4,849,409 bytes (`:53`) | 4,849,409 bytes | ✅ |
| Checksum file size | 99 bytes (`:54`) | 99 bytes | ✅ |
| SHA-256 | `d78e9211…860421` (`:61`) | `d78e92113b79bbf4938cc12fd2fdcd963995db106d1b7d1226e6865ab2860421` (`target/release/SHA256SUMS.txt:1`) | ✅ |
| Verification | `CostGuard-…zip: OK` (`:67`) | `shasum -a 256 -c` exit 0, `OK` | ✅ |
| Timestamp | 2026-10-24T11:40:35Z (`:3`) | zip mtime 2026-10-24T11:40:51Z (UTC) — 16 s after run start | ✅ (start vs completion) |
| Binary size | 15,919,152 bytes (`:52`) | 15,919,152 bytes | ✅ |
| Bundle size | 15,859,712 bytes (`:51`) | `du -sk` = 15,488 KiB = 15,859,712 bytes; logical file bytes = 15,849,303 | ⚠️ measurement method (gap 5) |
| Signature | `Signature=adhoc`, `TeamIdentifier=not set` (`:34-42`) | `codesign -dv`: `Signature=adhoc`, `TeamIdentifier=not set`; `codesign --verify` exit 0 | ✅ |
| Zip integrity | not recorded | `unzip -t` → `No errors detected` | ✅ |
| No tag/push/release | `:10` | `git tag --list` empty; no remote tags; 0 GitHub releases | ✅ |

`SHA256SUMS.txt` stores the bare filename, so `shasum -a 256 -c` must run from `target/release`; the report and release notes both state this (`dry-run.md:70`, `release.yml:86`).

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code | ✅ 122-line guard, 119-line workflow; no unused flexibility |
| Surgical changes | ✅ 9 files, all release-scoped; no core/app source touched |
| No scope creep | ✅ No notarization, DMG, Homebrew, updater, Linux/Windows |
| Matches patterns | ✅ Guard mirrors `scripts/check-core-purity.sh` style; workflow mirrors `ci.yml` packaging |
| Spec-anchored outcome check | ✅ Assertions/commands target the spec-defined values and artifact names |
| Per-layer coverage expectation | ✅ Guard is the executable check; docs/workflow/dry-run layers require none (`tasks.md:14-20`) |
| Every test maps to a requirement | ✅ No new tests; all 155 existing tests are pre-feature |
| Documented guidelines followed | ✅ `CONTRIBUTING.md:44-73` (fmt/clippy/test/check/purity), Conventional Commits, DCO |

No `SPEC_DEVIATION` markers. No unrelated "improvements".

---

## Spec-Precision / Automation Gaps (non-blocking)

1. **REL-08 tag check is not wired into publishing (Medium).** `scripts/check-release-config.sh:108-114` can reject a mismatched tag, but `.github/workflows/release.yml` never invokes the guard and `ci.yml:56` runs it without the tag argument. The only enforcement before publish is the manual step in `docs/releasing.md:60`. The spec/tasks do not require workflow wiring, so this is precision/hardening, not an AC failure.
2. **REL-03 publish gate is unguarded (Low–Medium).** Probe P1 removed both `if: startsWith(github.ref, 'refs/tags/v')` gates (`release.yml:67,111`); the guard still exited 0. A future regression could publish on `workflow_dispatch` without CI noticing. Current tree behaves correctly.
3. **REL-06 coverage is partial in the guard (Low).** The guard checks only `docs/releasing.md:100-106`; removing the README (P3) or CONTRIBUTING (P4) Gatekeeper/checksum text does not fail. README and CONTRIBUTING also reference checksum verification via the guide rather than restating `shasum -a 256 -c`; T3's done-when accepts this, but a literal reading of REL-06 ("…and CONTRIBUTING.md SHALL document…checksum verification") is stricter.
4. **Edge case 5 is unguarded (Low).** Probe P2 removed `test -f "$zip"` (`release.yml:53`) with the guard still green. The workflow currently fails before publish as required; only regression protection is missing.
5. **Dry-run bundle size is a `du` allocation figure (Cosmetic).** `dry-run.md:51` records 15,859,712 bytes, which equals `du -sk` 15,488 KiB, while the bundle's logical file bytes total 15,849,303. The zip and SHA-256 are unaffected. The recorded timestamp is the run start (11:40:35Z), 16 s before the zip mtime (11:40:51Z).
6. **Spec bookkeeping is stale (Bookkeeping).** `spec.md:99-115` still lists every requirement as `Pending` and says "0 mapped to tasks". The task instructed the Verifier to write only `validation.md`, so `spec.md` was intentionally not edited; the traceability table should be updated by the orchestrator.

---

## Requirement Traceability Update

Not applied: the dispatch restricted writes to `validation.md`. Recommended statuses for `spec.md`: REL-01 … REL-09 → ✅ Verified.

---

## Summary

**Overall**: ✅ Ready to publish after explicit maintainer approval (publishing itself remains out of scope and was not performed).

**Spec-anchored check**: 9/9 ACs matched spec outcome; 6 precision/bookkeeping gaps flagged (none blocking).
**Sensor**: 7/7 spec-scoped mutations killed; 4 supplementary probes survived as hardening notes.
**Gate**: 6/6 passed.

**What works**: tag-driven build/sign/zip/checksum/release pipeline, tag-only publish with least-privilege token, dispatch build-only path, version/changelog/workflow/docs guard wired into CI, complete install/checksum/Gatekeeper docs, and a reproducible local dry-run whose artifacts and SHA-256 match the record.

**Issues found**: no blocking AC gap. Highest-value follow-up is invoking `bash scripts/check-release-config.sh "$GITHUB_REF_NAME"` (or a dedicated version check) in the release workflow before packaging, plus guarding the tag-only publish gate.

**Next steps**: optional hardening tasks for gaps 1-2; otherwise request explicit approval to create and push `v0.1.0`.
