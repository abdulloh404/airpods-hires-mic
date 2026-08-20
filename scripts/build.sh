#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
DESKTOP_DIR="${PROJECT_DIR}/apps/desktop"

[[ -d "${DESKTOP_DIR}/node_modules" ]] || {
    echo "Desktop dependencies are missing. Run: cd apps/desktop && npm install" >&2
    exit 1
}

cd "${PROJECT_DIR}"
cargo build --locked --release --package airpods-daemon --bins

cd "${DESKTOP_DIR}"
npm run tauri -- build --no-bundle
