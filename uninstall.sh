#!/usr/bin/env bash
set -euo pipefail

APP_NAME="airpods-hires-mic"
SERVICE_NAME="${APP_NAME}.service"
BINARY="${HOME}/.local/bin/${APP_NAME}"
CONFIG_DIR="${HOME}/.config/${APP_NAME}"
ENV_FILE="${CONFIG_DIR}/environment"
SERVICE_FILE="${HOME}/.config/systemd/user/${SERVICE_NAME}"
FIFO="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/airpods-hires-mic.fifo"
LOCK="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/airpods-hires-mic.lock"

if [[ -f "${SERVICE_FILE}" && ! -L "${SERVICE_FILE}" ]]; then
    systemctl --user disable --now "${SERVICE_NAME}"
elif [[ -e "${SERVICE_FILE}" || -L "${SERVICE_FILE}" ]]; then
    echo "Refusing to remove unsafe service path: ${SERVICE_FILE}" >&2
    exit 1
fi

if pgrep -f '(^|/)airpods-hires-mic( |$)' >/dev/null 2>&1; then
    echo "${APP_NAME} is still running outside the user service. Stop it, then run uninstall.sh again." >&2
    exit 1
fi

if command -v pactl >/dev/null 2>&1 && pactl info >/dev/null 2>&1; then
    while read -r module_id module_name module_args; do
        if [[ "${module_name}" == "module-pipe-source" && "${module_args}" == *"source_name=AirPodsHiRes"* ]]; then
            pactl unload-module "${module_id}" || true
        fi
    done < <(pactl list short modules)
fi

if [[ -p "${FIFO}" ]]; then
    rm -f -- "${FIFO}"
elif [[ -e "${FIFO}" ]]; then
    echo "Refusing to remove non-FIFO path: ${FIFO}" >&2
fi

if [[ -f "${LOCK}" && ! -L "${LOCK}" ]]; then
    rm -f -- "${LOCK}"
elif [[ -e "${LOCK}" || -L "${LOCK}" ]]; then
    echo "Refusing to remove unsafe lock path: ${LOCK}" >&2
fi

rm -f -- "${BINARY}"

if [[ -f "${ENV_FILE}" && ! -L "${ENV_FILE}" ]]; then
    rm -f -- "${ENV_FILE}"
elif [[ -e "${ENV_FILE}" || -L "${ENV_FILE}" ]]; then
    echo "Refusing to remove unsafe environment path: ${ENV_FILE}" >&2
fi

rm -f -- "${SERVICE_FILE}"
rmdir -- "${CONFIG_DIR}" 2>/dev/null || true
systemctl --user daemon-reload
systemctl --user reset-failed "${SERVICE_NAME}" 2>/dev/null || true

echo "Removed ${APP_NAME} service, configuration, binary, virtual microphone module, and runtime files."
echo "System packages and Bluetooth/PipeWire/A2DP/AAC configuration were not changed."
