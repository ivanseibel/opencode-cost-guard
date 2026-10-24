#!/usr/bin/env bash
# Guard the release configuration before a tag is created.
#
# Checks that the workspace and Tauri versions agree, the changelog entry
# exists, the release workflow keeps its required steps and the docs keep the
# Gatekeeper workaround. Run from anywhere in the repository; the script exits
# non-zero and lists every failure it finds.
#
# Usage:
#   bash scripts/check-release-config.sh            # invariants only
#   bash scripts/check-release-config.sh v0.1.0     # also require tag == version
#
# Self-check: temporarily remove a guarded value (for example the
# `gh release create` step or the `xattr` command) and re-run. The script must
# exit non-zero. Restore the file afterwards.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

if [[ $# -gt 1 ]]; then
    echo "usage: bash scripts/check-release-config.sh [tag]" >&2
    exit 2
fi

failures=()

fail() {
    failures+=("$1")
}

# Report a missing required literal in a file. Missing files are reported by
# require_file, so this stays quiet when the file is absent.
require_contains() {
    local path="$1"
    local literal="$2"
    local description="$3"

    [[ -f "${path}" ]] || return 0
    grep -Fq -- "${literal}" "${path}" \
        || fail "${description}: ${path#"${repo_root}"/} does not contain '${literal}'"
}

require_file() {
    local path="$1"
    [[ -f "${path}" ]] || fail "missing file: ${path#"${repo_root}"/}"
}

cargo_toml="${repo_root}/Cargo.toml"
tauri_conf="${repo_root}/crates/app/tauri.conf.json"
changelog="${repo_root}/CHANGELOG.md"
release_workflow="${repo_root}/.github/workflows/release.yml"
release_docs="${repo_root}/docs/releasing.md"

require_file "${cargo_toml}"
require_file "${tauri_conf}"
require_file "${changelog}"
require_file "${release_workflow}"
require_file "${release_docs}"

workspace_version=""
if [[ -f "${cargo_toml}" ]]; then
    workspace_version="$(awk '
        /^\[workspace\.package\]/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }
    ' "${cargo_toml}")"
    [[ -n "${workspace_version}" ]] \
        || fail "workspace version not found in Cargo.toml [workspace.package]"
fi

tauri_version=""
if [[ -f "${tauri_conf}" ]]; then
    tauri_version="$(python3 -c '
import json
import sys

print(json.load(open(sys.argv[1]))["version"])
' "${tauri_conf}" 2>/dev/null || true)"
    [[ -n "${tauri_version}" ]] \
        || fail "version not found in crates/app/tauri.conf.json"
fi

if [[ -n "${workspace_version}" && -n "${tauri_version}" && "${workspace_version}" != "${tauri_version}" ]]; then
    fail "version mismatch: Cargo.toml has ${workspace_version}, crates/app/tauri.conf.json has ${tauri_version}"
fi

if [[ -n "${workspace_version}" ]]; then
    require_contains "${changelog}" "## ${workspace_version}" "changelog entry for ${workspace_version}"
fi

require_contains "${release_workflow}" "tags:" "release workflow tag trigger"
require_contains "${release_workflow}" '- "v*"' "release workflow v* tag filter"
require_contains "${release_workflow}" 'shasum -a 256 "$zip" > SHA256SUMS.txt' "release workflow checksum step"
require_contains "${release_workflow}" "gh release create" "release workflow release creation"
require_contains "${release_workflow}" "contents: write" "release workflow job permissions"
require_contains "${release_workflow}" "codesign --force --deep -s -" "release workflow ad-hoc signing"

require_contains "${release_docs}" "xattr -dr com.apple.quarantine" "documented Gatekeeper workaround"
if [[ -f "${release_docs}" ]]; then
    grep -Eiq "unsigned" "${release_docs}" \
        || fail "release docs must state that the build is unsigned"
    grep -Eiq "not notarized" "${release_docs}" \
        || fail "release docs must state that the build is not notarized"
fi

if [[ $# -eq 1 ]]; then
    tag="$1"
    expected="v${workspace_version}"
    if [[ "${tag}" != "${expected}" ]]; then
        fail "tag ${tag} does not match workspace version ${workspace_version} (expected ${expected})"
    fi
fi

if (( ${#failures[@]} > 0 )); then
    echo "release config check failed:" >&2
    printf '  - %s\n' "${failures[@]}" >&2
    exit 1
fi

echo "release config OK (version ${workspace_version})"
