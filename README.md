# OpenCode Cost Guard

A macOS menu-bar utility for [OpenCode](https://opencode.ai) Go users. It answers three questions at a glance:

1. **What does using this model cost right now?** Peak or off-peak state, current plan prices, and the next pricing-window transition.
2. **Is there a better model for what I am about to do?** A deterministic recommendation for the active Task Profile, with reasons and alternatives.
3. **Did anything change?** A warning badge when the model catalog changes.

OpenCode Cost Guard is open source under MIT OR Apache-2.0. It uses no credentials, sends no telemetry, and keeps all data local.

## Status

Early development. The platform-independent core crate (`cost-guard-core`) and the Tauri menu-bar shell (`cost-guard-app`) are in place. Builds are unsigned pre-release artifacts, so there is no notarized installer yet.

## Why

DeepSeek, the primary OpenCode Go model, charges peak prices in fixed UTC windows on weekdays, with Chinese public holidays excluded. Peak is currently 2x the off-peak price, so sessions started in a peak window burn allowance twice as fast. The model roster and monthly limits also change regularly. Cost Guard keeps that state visible and recommends a model per task profile instead of relying on memory.

The core is pure Rust and has no Tauri, macOS, or Windows dependencies. CI runs the core suite on Ubuntu and the full workspace, including the app crate, on macOS.

## Repository layout

```
Cargo.toml                  Cargo workspace
crates/core/                cost-guard-core: catalog, schedule, sync, recommend, config, report
crates/app/                 cost-guard-app: Tauri menu-bar shell, tray, settings UI
data/                       curated TOML Overlays (created in the data milestone)
docs/                       product brief, MVP scope, backlog, ADRs
.specs/                     feature specs, designs, tasks
scripts/                    repository checks
```

## Build from source

### Prerequisites

- macOS or Linux for the core; macOS for the app shell.
- [Rust](https://rustup.rs) installed through `rustup`. The pinned toolchain in `rust-toolchain.toml` installs automatically on the first Cargo command.

### Build

```sh
git clone https://github.com/ivanseibel/opencode-cost-guard.git
cd opencode-cost-guard
cargo build --workspace
```

### App shell

The app needs [tauri-cli](https://tauri.app) 2.12.4. Install the official prebuilt binary on Apple silicon:

```sh
curl --fail --location --silent --show-error \
  --output "$TMPDIR/cargo-tauri.zip" \
  "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v2.12.4/cargo-tauri-aarch64-apple-darwin.zip"
unzip -o "$TMPDIR/cargo-tauri.zip" -d "$HOME/.cargo/bin"
chmod +x "$HOME/.cargo/bin/cargo-tauri"
cargo tauri --version
```

Run the shell with the Tauri dev loop (searches for `tauri.conf.json` under `crates/app`):

```sh
cd crates/app && cargo tauri dev
```

Or run the crate directly from the repository root. The static `ui/` assets are embedded, so no Node toolchain or dev server is required:

```sh
cargo run -p cost-guard-app
```

Build the macOS bundle with the CLI:

```sh
cd crates/app && cargo tauri build --bundles app
```

The bundle is written to `target/release/bundle/macos/Cost Guard.app`. CI ad-hoc signs release artifacts; local builds have no signing identity.

## Test

```sh
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-core-purity.sh
```

The test suite is hermetic: no test performs network I/O. Catalog sync is exercised through an injected HTTP client.

CI runs `cargo clippy -p cost-guard-core` and `cargo test -p cost-guard-core` on Ubuntu and the full workspace checks on macOS. The macOS job also runs `cargo check -p cost-guard-app`; a tag or manual `workflow_dispatch` additionally builds, ad-hoc signs, and uploads the `.app` zip.

## Run the demo

The core ships an offline example that prints the current pricing-window state, the next transition in UTC and local time, and a recommendation for each of the five Task Profiles:

```sh
cargo run -p cost-guard-core --example status
```

The example uses bundled data when no synced snapshot exists and says so in the output. It requires no network access.

## Installing an unsigned build

Release artifacts are ad-hoc signed but not notarized yet, so macOS Gatekeeper may block the first launch.

To open the app, right-click it in Finder and choose **Open**, then confirm. To clear the quarantine flag from Terminal instead:

```sh
xattr -dr com.apple.quarantine "/Applications/Cost Guard.app"
```

For a locally built bundle the path is `target/release/bundle/macos/Cost Guard.app`. Only clear quarantine on a build you obtained from this repository's releases or built yourself.

## Privacy and security

- No OpenCode credentials, local database, configuration files, or CLI output are read.
- No telemetry and no analytics.
- Catalog sync is an anonymous GET to the public mirror.
- Config and snapshots are JSON files stored locally.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Overlay data updates are the easiest entry point and are labeled `good first issue` in the issue tracker.

## Project documents

- [Product brief](docs/brief.md)
- [MVP scope and acceptance criteria](docs/mvp.md)
- [Backlog](docs/backlog.md)
- [Architecture decisions](docs/adr/)
- [Glossary](CONTEXT.md)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project, as defined in the Apache-2.0 license, shall be dual licensed as above, without additional terms or conditions.

## Disclaimer

OpenCode Cost Guard is an independent community project. It is **not affiliated with, endorsed by, or sponsored by OpenCode or SST**. OpenCode and SST names and marks belong to their respective owners.
