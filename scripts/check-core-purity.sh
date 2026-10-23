#!/usr/bin/env bash
# Fail when cost-guard-core gains GUI or platform-specific dependencies.
#
# The core must stay portable: no Tauri, no Objective-C bindings, no Windows API
# crates. Run from anywhere in the repository.
#
# Self-test: set COST_GUARD_PURITY_FIXTURE to a file containing package lines in
# `cargo tree` format (one package per line). The script checks that file instead
# of running cargo, which lets tests prove the matcher fails on banned crates.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

if [[ -n "${COST_GUARD_PURITY_FIXTURE:-}" ]]; then
    if [[ ! -f "${COST_GUARD_PURITY_FIXTURE}" ]]; then
        echo "error: fixture not found: ${COST_GUARD_PURITY_FIXTURE}" >&2
        exit 2
    fi
    tree_output="$(<"${COST_GUARD_PURITY_FIXTURE}")"
    tree_label="${COST_GUARD_PURITY_FIXTURE}"
else
    tree_output="$(cd -- "${repo_root}" && cargo tree -p cost-guard-core --prefix none --no-dedupe --edges normal,build,dev)"
    tree_label="cargo tree -p cost-guard-core"
fi

# Match whole package-name tokens, including families such as objc2-foundation
# and tauri-plugin-*.
forbidden='(^|[[:space:]])(tauri|objc2|windows-sys)([a-z0-9_-]*)([[:space:]]|$)'

offenders="$(printf '%s\n' "${tree_output}" | grep -Ei "${forbidden}" || true)"

if [[ -n "${offenders}" ]]; then
    echo "error: cost-guard-core must stay free of tauri, objc2* and windows-sys dependencies." >&2
    echo "offending entries from ${tree_label}:" >&2
    printf '%s\n' "${offenders}" | sed 's/^/  /' >&2
    exit 1
fi

echo "core dependency purity OK (${tree_label})"
