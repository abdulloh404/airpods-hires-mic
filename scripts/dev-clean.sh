#!/usr/bin/env bash
set -euo pipefail

APP_NAME="airpods-hires-mic"
SERVICE_NAME="${APP_NAME}-dev.service"
CONFIG_DIR="${HOME}/.config/${APP_NAME}"
ENV_FILE="${CONFIG_DIR}/dev.environment"
USER_SYSTEMD_DIR="${HOME}/.config/systemd/user"
SERVICE_FILE="${USER_SYSTEMD_DIR}/${SERVICE_NAME}"

if systemctl --user cat "${SERVICE_NAME}" >/dev/null 2>&1; then
    systemctl --user disable --now "${SERVICE_NAME}" >/dev/null 2>&1 || true
fi

rm -f -- "${SERVICE_FILE}" "${ENV_FILE}"
systemctl --user daemon-reload
rmdir "${CONFIG_DIR}" 2>/dev/null || true
echo "Removed development service and configuration only. Production files were not changed."
