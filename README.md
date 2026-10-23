# OpenCode Cost Guard

A macOS menu-bar utility for [OpenCode](https://opencode.ai) Go users. It answers three questions at a glance:

1. **What does using this model cost right now?** Peak or off-peak state, current plan prices, and the next pricing-window transition.
2. **Is there a better model for what I am about to do?** A deterministic recommendation for the active Task Profile, with reasons and alternatives.
3. **Did anything change?** A warning badge when the model catalog changes.

OpenCode Cost Guard is open source under MIT OR Apache-2.0. It uses no credentials, sends no telemetry, and keeps all data local.

## Status

Early development. The repository scaffold and the platform-independent core crate (`cost-guard-core`) are built first. The Tauri menu-bar shell follows in a later milestone, so there is no installable app yet.

## Why

DeepSeek, the primary OpenCode Go model, charges peak prices in fixed UTC windows on weekdays, with Chinese public holidays excluded. Peak is currently 2x the off-peak price, so sessions started in a peak window burn allowance twice as fast. The model roster and monthly limits also change regularly. Cost Guard keeps that state visible and recommends a model per task profile instead of relying on memory.

The core is pure Rust and has no Tauri, macOS, or Windows dependencies. CI proves it builds and tests on both macOS and Ubuntu.

## Repository layout

```
Cargo.toml                  Cargo workspace
crates/core/                cost-guard-core: catalog, schedule, sync, recommend, config, report
data/                       curated TOML Overlays (created in the data milestone)
docs/                       product brief, MVP scope, backlog, ADRs
.specs/                     feature specs, designs, tasks
scripts/                    repository checks
```

## Build from source

### Prerequisites

- macOS or Linux.
- [Rust](https://rustup.rs) installed through `rustup`. The pinned toolchain in `rust-toolchain.toml` installs automatically on the first Cargo command.

### Build

```sh
git clone https://github.com/ivanseibel/opencode-cost-guard.git
cd opencode-cost-guard
cargo build --workspace
```

## Test

```sh
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-core-purity.sh
```

The test suite is hermetic: no test performs network I/O. Catalog sync is exercised through an injected HTTP client.

## Run the demo

The core ships an offline example that prints the current pricing-window state, the next transition in UTC and local time, and a recommendation for each of the five Task Profiles:

```sh
cargo run -p cost-guard-core --example status
```

The example uses bundled data when no synced snapshot exists and says so in the output. It requires no network access.

## Installing an unsigned build

Release builds are not code-signed or notarized yet, so macOS Gatekeeper may block the first launch.

To open the app, right-click it in Finder and choose **Open**, then confirm. To clear the quarantine flag from Terminal instead:

```sh
xattr -dr com.apple.quarantine "/Applications/OpenCode Cost Guard.app"
```

Only do this for a build you obtained from this repository's releases.

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
