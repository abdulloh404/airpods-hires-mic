#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

for command in cargo dh dpkg-buildpackage npm; do
    command -v "${command}" >/dev/null 2>&1 || {
        echo "Required command is unavailable: ${command}" >&2
        exit 1
    }
done

require_version() {
    local label="$1"
    local version_output="$2"
    local required_major="$3"
    local required_minor="$4"

    if [[ ! "${version_output}" =~ ([0-9]+)\.([0-9]+) ]]; then
        echo "Cannot determine ${label} version from: ${version_output}" >&2
        exit 1
    fi
    local major="${BASH_REMATCH[1]}"
    local minor="${BASH_REMATCH[2]}"
    if ((major < required_major || (major == required_major && minor < required_minor))); then
        echo "${label} ${required_major}.${required_minor}+ is required; found ${version_output}." >&2
        exit 1
    fi
}

require_version "Cargo" "$(cargo --version)" 1 85
require_version "Rust" "$(rustc --version)" 1 85
require_version "Node.js" "$(node --version)" 22 12

cd "${PROJECT_DIR}"
dpkg-buildpackage -us -uc -b
