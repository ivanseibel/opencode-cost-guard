# M3 Local Release Dry-Run

**Date**: 2026-10-24T11:40:35Z
**Commit**: `e110477` (`ci: guard release configuration`), branch `main`
**Host**: macOS 26.6.2, arm64 (Apple silicon)
**Toolchain**: rustc 1.97.1, tauri-cli 2.12.4

This run proves the packaging chain from `.github/workflows/release.yml` locally: build the bundle, ad-hoc sign it, zip it, generate `SHA256SUMS.txt` and verify the checksum.

**No tag, push or GitHub Release was performed.** `git tag -l` returned no tags before and after the run, no `git push` was executed, and no `gh release create` was executed. Publishing stays approval-gated (see [docs/releasing.md](../../../docs/releasing.md)).

## Commands

```sh
cd crates/app && cargo tauri build --bundles app
cd ../..
codesign --force --deep -s - "target/release/bundle/macos/Cost Guard.app"
codesign --verify --verbose=2 "target/release/bundle/macos/Cost Guard.app"
ditto -c -k --keepParent "target/release/bundle/macos/Cost Guard.app" \
  "target/release/CostGuard-v0.1.0-macos-arm64.zip"
cd target/release
shasum -a 256 CostGuard-v0.1.0-macos-arm64.zip > SHA256SUMS.txt
shasum -a 256 -c SHA256SUMS.txt
```

## Build and signature

```text
Finished `release` profile [optimized] target(s) in 14.43s
Built application at: /Users/ivanseibel/dev/personal/opencode-cost-guard/target/release/cost-guard-app
Bundling Cost Guard.app (.../target/release/bundle/macos/Cost Guard.app)
Finished 1 bundle at: .../target/release/bundle/macos/Cost Guard.app (15.18 MiB)

target/release/bundle/macos/Cost Guard.app: replacing existing signature
target/release/bundle/macos/Cost Guard.app: valid on disk
target/release/bundle/macos/Cost Guard.app: satisfies its Designated Requirement

Identifier=dev.ivanseibel.costguard
Format=app bundle with Mach-O thin (arm64)
Signature=adhoc
TeamIdentifier=not set
Sealed Resources version=2 rules=13 files=1
```

`CFBundleShortVersionString` and `CFBundleVersion` are both `0.1.0`.

## Artifacts

| Artifact | Path (repo-relative) | Size |
| -------- | -------------------- | ---- |
| App bundle | `target/release/bundle/macos/Cost Guard.app` | 15,859,712 bytes (15.18 MiB) |
| App binary | `target/release/cost-guard-app` | 15,919,152 bytes |
| Release zip | `target/release/CostGuard-v0.1.0-macos-arm64.zip` | 4,849,409 bytes (4.6 MiB) |
| Checksum file | `target/release/SHA256SUMS.txt` | 99 bytes |

`target/` is ignored by Git, so the artifacts stay local and are not part of the commit.

## Checksum

```text
d78e92113b79bbf4938cc12fd2fdcd963995db106d1b7d1226e6865ab2860421  CostGuard-v0.1.0-macos-arm64.zip
```

Verification output:

```text
CostGuard-v0.1.0-macos-arm64.zip: OK
```

`SHA256SUMS.txt` stores the bare file name, so `shasum -a 256 -c SHA256SUMS.txt` must run from the directory that contains the zip. The release notes repeat that requirement.

## Guard and gates

- `bash scripts/check-release-config.sh` printed `release config OK (version 0.1.0)`.
- Build gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (155 passed), `cargo check -p cost-guard-app`, `bash scripts/check-core-purity.sh` all passed.

## Notes

- `cargo tauri build` rewrote `tauri-build = "=2.7.4"` to `tauri-build = { version = "=2.7.4", features = [] }` in `crates/app/Cargo.toml`. That is a CLI normalization unrelated to this feature; the change was reverted, and the working tree was clean before this evidence file was added.
- The zip includes the app bundle's extended attributes (`._Contents` entries), which `ditto` and Finder restore on extraction. The checksum covers the archive as produced.
