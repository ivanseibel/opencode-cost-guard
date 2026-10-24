# Releasing OpenCode Cost Guard

This guide covers installing a release build and publishing one. v0.1.0 ships as an ad-hoc-signed macOS `.app` zip attached to a GitHub Release. The binary is **unsigned and not notarized**; Gatekeeper requires a manual bypass on first launch. The decision and its consequences are recorded in [ADR-0008](adr/0008-unsigned-github-releases.md).

## Install from a GitHub Release

Requirements: macOS 10.13 or newer on Apple silicon. Intel builds are not produced for v0.1.0.

1. Download both `CostGuard-v0.1.0-macos-arm64.zip` and `SHA256SUMS.txt` from the [releases page](https://github.com/ivanseibel/opencode-cost-guard/releases) into the same directory.
2. Verify the archive before opening it:

   ```sh
   shasum -a 256 -c SHA256SUMS.txt
   ```

   The output must end with `OK`. A mismatch means the download is corrupt or was tampered with; delete it and download again.
3. Unzip the archive and move `Cost Guard.app` to `/Applications`:

   ```sh
   ditto -x -k CostGuard-v0.1.0-macos-arm64.zip .
   mv "Cost Guard.app" /Applications/
   ```
4. Launch the app. Gatekeeper blocks the first launch because the build is not notarized. Right-click the app in Finder and choose **Open**, then confirm, or clear the quarantine flag from Terminal:

   ```sh
   xattr -dr com.apple.quarantine "/Applications/Cost Guard.app"
   ```

Only clear quarantine on a build you downloaded from this repository's releases or built yourself, and only after the checksum matches. Clearing the flag on an arbitrary unsigned app bypasses a protection you still need.

## Build from source

The app requires [tauri-cli](https://tauri.app) 2.12.4. Install the official prebuilt binary on Apple silicon:

```sh
curl --fail --location --silent --show-error \
  --output "$TMPDIR/cargo-tauri.zip" \
  "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v2.12.4/cargo-tauri-aarch64-apple-darwin.zip"
unzip -o "$TMPDIR/cargo-tauri.zip" -d "$HOME/.cargo/bin"
chmod +x "$HOME/.cargo/bin/cargo-tauri"
cargo tauri --version
```

Build and ad-hoc sign the bundle:

```sh
cd crates/app && cargo tauri build --bundles app
cd ../..
codesign --force --deep -s - "target/release/bundle/macos/Cost Guard.app"
```

The bundle is written to `target/release/bundle/macos/Cost Guard.app`. Local builds are unsigned by default; the `codesign` command applies the same ad-hoc signature the release workflow uses.

## Release procedure

Publishing is approval-gated. A tag is created only after an explicit go-ahead, and only after the packaging chain is proven locally. Do not run `gh release create` by hand.

1. **Local dry-run.** Run the full packaging chain without Git or GitHub operations: build the bundle, ad-hoc sign it, zip it as `CostGuard-v0.1.0-macos-arm64.zip`, generate `SHA256SUMS.txt`, and verify the checksum with `shasum -a 256 -c`. Record commands, paths, sizes and checksums in `.specs/features/m3-release/dry-run.md`.
2. **Explicit approval.** Ask the maintainer for approval to publish v0.1.0. No tag exists before this step.
3. **Check version alignment.** Confirm `[workspace.package].version` in `Cargo.toml`, `version` in `crates/app/tauri.conf.json` and the intended tag all read `0.1.0`. The release workflow publishes the tag it is given; it does not rewrite versions.
4. **Create the tag locally.**

   ```sh
   git tag -a v0.1.0 -m "OpenCode Cost Guard v0.1.0"
   ```

5. **Push the tag.** This is the publish trigger:

   ```sh
   git push origin v0.1.0
   ```

6. **Let the workflow publish.** `.github/workflows/release.yml` runs on `push` of a `v*` tag: it installs tauri-cli 2.12.4, builds the app, ad-hoc signs it, zips it, writes `SHA256SUMS.txt`, generates release notes and creates the GitHub Release for the tag with both files attached. Watch it with `gh run watch`, then confirm the release assets and download the zip once to re-verify the checksum.

A manual `workflow_dispatch` run builds and uploads the same artifacts to the workflow run, but never creates a release. Use it to test the pipeline without publishing.

### Release notes

The workflow generates the release body: the unsigned warning, install steps, checksum verification, a link back to this guide, and the matching `CHANGELOG.md` entry. Update `CHANGELOG.md` before tagging; the notes read from it.

## Troubleshooting

- **"Cost Guard is damaged and can't be opened."** The quarantine flag is set. Verify the checksum, then apply `xattr -dr com.apple.quarantine "/Applications/Cost Guard.app"`.
- **`shasum: no properly formatted checksum lines found`.** Run the check from the directory that contains the zip and `SHA256SUMS.txt`, and pass the file name, not a path inside another directory.
- **Gatekeeper still blocks after the `xattr` command.** Confirm the command points at the installed app, not the copy in `Downloads`. Re-run it, then open the app again.
- **Workflow fails before release creation.** The packaging step asserts that the bundle and zip exist; a missing artifact stops the job before publishing. Check the build log.
