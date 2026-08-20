#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="airpods-hires-mic-dev.service"

if systemctl --user cat "${SERVICE_NAME}" >/dev/null 2>&1; then
    systemctl --user stop "${SERVICE_NAME}"
    echo "Stopped ${SERVICE_NAME}."
else
    echo "${SERVICE_NAME} is not installed."
fi
