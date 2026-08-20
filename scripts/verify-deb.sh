#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

if (($# > 1)); then
    echo "Usage: ./scripts/verify-deb.sh [path-to-package.deb]" >&2
    exit 1
fi

if (($# == 1)); then
    PACKAGE_PATH="$1"
else
    shopt -s nullglob
    packages=("${PROJECT_DIR}"/../airpods-hires-mic_*.deb)
    shopt -u nullglob
    if ((${#packages[@]} != 1)); then
        echo "Pass the package path explicitly when zero or multiple .deb files exist." >&2
        exit 1
    fi
    PACKAGE_PATH="${packages[0]}"
fi

[[ -f "${PACKAGE_PATH}" ]] || { echo "Package does not exist: ${PACKAGE_PATH}" >&2; exit 1; }

dpkg-deb --info "${PACKAGE_PATH}"
dpkg-deb --contents "${PACKAGE_PATH}"

if command -v lintian >/dev/null 2>&1; then
    lintian "${PACKAGE_PATH}"
else
    echo "lintian is not installed; skipped static package checks."
fi
