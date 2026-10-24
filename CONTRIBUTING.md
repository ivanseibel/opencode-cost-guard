# Contributing

Thanks for helping improve OpenCode Cost Guard. This project is a small, personal-first tool, so the contribution process stays lightweight: one clear change per pull request, tests and linters green, and a signed-off commit.

## Development setup

1. Install [Rust](https://rustup.rs) through `rustup`.
2. Clone the repository and enter it:

   ```sh
   git clone https://github.com/ivanseibel/opencode-cost-guard.git
   cd opencode-cost-guard
   ```

3. The pinned toolchain in `rust-toolchain.toml` installs automatically on the first Cargo command.

## App shell setup (macOS)

The Tauri shell in `crates/app` needs [tauri-cli](https://tauri.app) 2.12.4. Install the official prebuilt binary on Apple silicon:

```sh
curl --fail --location --silent --show-error \
  --output "$TMPDIR/cargo-tauri.zip" \
  "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v2.12.4/cargo-tauri-aarch64-apple-darwin.zip"
unzip -o "$TMPDIR/cargo-tauri.zip" -d "$HOME/.cargo/bin"
chmod +x "$HOME/.cargo/bin/cargo-tauri"
cargo tauri --version
```

Run the app with the Tauri dev loop:

```sh
cd crates/app && cargo tauri dev
```

Or run the crate directly from the repository root. The static `ui/` assets are embedded, so no Node toolchain is required:

```sh
cargo run -p cost-guard-app
```

## Before opening a pull request

Run the same checks as CI from the repository root:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p cost-guard-app
bash scripts/check-core-purity.sh
```

On Ubuntu CI only `cargo fmt --check`, `cargo clippy -p cost-guard-core --all-targets -- -D warnings`, `cargo test -p cost-guard-core`, and the purity script run; the workspace checks and the app check are macOS-only.

Build and smoke-test the macOS bundle before changing app wiring or packaging:

```sh
cd crates/app && cargo tauri build --bundles app
```

The bundle lands at `target/release/bundle/macos/Cost Guard.app`. It is unsigned locally; CI ad-hoc signs release artifacts. To bypass Gatekeeper for an unsigned build you trust:

```sh
xattr -dr com.apple.quarantine "target/release/bundle/macos/Cost Guard.app"
```

Requirements:

- No test may require network access. Use the injected `HttpClient` for sync tests.
- The `cost-guard-core` dependency tree must stay free of `tauri`, `objc2*`, and `windows-sys`.
- Public fallible functions return typed errors. Do not add `unwrap` on external data.
- Keep changes surgical. A pull request should address one issue.

## Developer Certificate of Origin (DCO)

Every commit must be signed off under the [Developer Certificate of Origin 1.1](https://developercertificate.org/). Sign-off is a declaration that you wrote the change or have the right to submit it under the project's license. No separate CLA is required.

Create the sign-off with `git commit -s`:

```sh
git commit -s -m "docs: fix typo in README"
```

The resulting trailer must match your real name and email:

```
Signed-off-by: Your Name <you@example.com>
```

If you forgot the sign-off on the latest commit:

```sh
git commit --amend -s --no-edit
```

For older commits, rebase and sign each one:

```sh
git rebase --signoff <base-branch>
```

Pull requests with unsigned commits cannot be merged.

## Updating Overlays

Overlays are the project-maintained data layer over the upstream catalog. They live in `data/` as TOML and are the supported way to correct pricing windows, holidays, monthly limits, and capability ratings without code changes.

### Rules that apply to every Overlay file

- Every file declares a non-empty `source` and `last_verified` (ISO date, `YYYY-MM-DD`). Entries inherit these values.
- An entry may override `source` or `last_verified`, but an override must also be non-empty.
- Missing required metadata fails loading and names the file or entry.
- Malformed pricing windows fail loading: the end must be after the start, and both must fall within `00:00` to `24:00`.
- Cite a primary source. Prefer the provider's pricing page or an official government notice over a blog post or screenshot.
- Keep all snapshot model IDs covered. Validation warns about models without a capability rating.

After editing, run:

```sh
cargo test -p cost-guard-core
```

### `data/pricing-windows.toml`

Defines peak windows and the peak multiplier per provider.

1. Check the provider's current pricing page.
2. Update `multiplier`, `days`, and `windows` for the provider entry. Windows use UTC and the half-open convention `[start, end)`.
3. Update `source` and `last_verified` at the file level, or on the entry if only one provider changed.
4. Run the tests. Boundary tests use a synthetic calendar, so changes to seed data do not weaken them.

### `data/holidays-cn.toml`

Defines the Chinese public holiday calendar, including makeup weekends.

1. Use the official State Council notice for the year.
2. Add or update one `[[year]]` entry:

   ```toml
   [[year]]
   year = 2027
   holidays = ["2027-01-01", "2027-02-06"]
   makeup_weekends = ["2027-02-07"]
   ```

3. `holidays` are full dates that are off-peak even on a weekday. `makeup_weekends` are working weekends; they remain off-peak because they fall on a weekend, and the list documents the official adjustment.
4. Record the notice URL in `source` and set `last_verified` to the date you checked it.
5. Run the tests, including the schedule coverage-gap test for years without an entry.

Yearly refreshes are tracked with the `holiday-refresh` issue template and labeled `good first issue`.

### `data/limits.toml`

Defines published per-model monthly USD limits for the Go plan.

1. Check the OpenCode Go documentation table.
2. Add an entry for every model in the bundled snapshot. If a model has no published limit, list it explicitly as unlisted with a note instead of omitting it.
3. Update `source` and `last_verified`.
4. Run the tests. Loader tests assert that every snapshot model is covered.

### `data/capabilities.toml`

Defines tiers, flags, context size, and per-profile capability scores.

1. Start from models.dev metadata and the model's public card for `vision`, `tools`, `reasoning`, and `context`.
2. Set `tier` to one of the agreed vocabulary values (`frontier`, `strong`, `standard`, `light`).
3. Profile scores range from 0 to 10. Provide scores for models expected to win a recommendation matrix row. Other models may inherit the tier base.
4. Use `notes` to explain non-obvious ratings, promotions, or data-use terms.
5. Update `source` and `last_verified`, then run the tests.

### Bundled catalog snapshot

`data/bundled/opencode-go.json` mirrors the `opencode-go` provider from `https://models.opencode.ai/api.json`. Maintainers refresh it from the live mirror and record `source`, `etag`, and `fetched_at`. Do not hand-edit model prices in the snapshot to work around a stale upstream value; use an Overlay entry with a citation instead.

## Releases

Release and install instructions live in [docs/releasing.md](docs/releasing.md). v0.1.0 ships as an ad-hoc-signed, non-notarized `.app` zip, so the guide documents checksum verification and the Gatekeeper workaround.

Publishing is approval-gated. Run the local packaging dry-run, get explicit approval, then create and push the `v*` tag. The tag push triggers `.github/workflows/release.yml`, which builds, ad-hoc signs, zips, writes `SHA256SUMS.txt` and publishes the GitHub Release. Do not create a release by hand. A manual `workflow_dispatch` run builds artifacts without publishing.

## Issues and pull requests

- Use the issue templates for bug reports and feature requests.
- Use the `holiday-refresh` template when a new year's official holiday notice is published.
- Fill in the pull request template, including test evidence and the related issue.
- Keep commit messages in [Conventional Commits](https://www.conventionalcommits.org/) form, for example `fix(core): handle 304 without a snapshot`.

## License

By contributing, you agree that your contribution is dual licensed under MIT OR Apache-2.0, matching the project license.
