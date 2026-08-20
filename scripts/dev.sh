#!/usr/bin/env bash
set -euo pipefail

APP_NAME="airpods-hires-mic"
SERVICE_NAME="${APP_NAME}-dev.service"
PRODUCTION_SERVICE_NAME="${APP_NAME}.service"
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
DAEMON_MANIFEST="${PROJECT_DIR}/crates/airpods-daemon/Cargo.toml"
DESKTOP_DIR="${PROJECT_DIR}/apps/desktop"
ENV_FILE="${HOME}/.config/${APP_NAME}/dev.environment"

[[ -f "${DAEMON_MANIFEST}" ]] || { echo "Development daemon manifest is missing: ${DAEMON_MANIFEST}" >&2; exit 1; }
[[ -f "${DESKTOP_DIR}/package.json" ]] || { echo "Desktop application is missing: ${DESKTOP_DIR}" >&2; exit 1; }
[[ -f "${ENV_FILE}" ]] || { echo "Run ./scripts/dev-setup.sh --device <AIRPODS_MAC> first." >&2; exit 1; }
if systemctl --user is-active --quiet "${PRODUCTION_SERVICE_NAME}"; then
    echo "${PRODUCTION_SERVICE_NAME} is active. Stop it before starting the development service." >&2
    exit 1
fi

CARGO_TARGET_DIR="${PROJECT_DIR}/target" cargo build --locked --manifest-path "${DAEMON_MANIFEST}" --bin "${APP_NAME}" --bin airpods-battery
systemctl --user restart "${SERVICE_NAME}"

cd "${DESKTOP_DIR}"
npm run tauri -- dev
