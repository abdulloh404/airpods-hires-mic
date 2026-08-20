#!/usr/bin/env bash
set -euo pipefail

APP_NAME="airpods-hires-mic"
SERVICE_NAME="${APP_NAME}-dev.service"
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
CONFIG_DIR="${HOME}/.config/${APP_NAME}"
ENV_FILE="${CONFIG_DIR}/dev.environment"
USER_SYSTEMD_DIR="${HOME}/.config/systemd/user"
SERVICE_FILE="${USER_SYSTEMD_DIR}/${SERVICE_NAME}"
SERVICE_TEMPLATE="${PROJECT_DIR}/packaging/${APP_NAME}-dev.service"
DAEMON_BIN="${PROJECT_DIR}/target/debug/${APP_NAME}"
DEVICE=""
FORCE=0

usage() {
    echo "Usage: ./scripts/dev-setup.sh --device <AIRPODS_MAC> [--force]"
}

if ((EUID == 0)); then
    echo "Do not run the development setup with sudo." >&2
    exit 1
fi

while (($#)); do
    case "$1" in
        --device)
            [[ $# -ge 2 ]] || { echo "--device requires a Bluetooth MAC address." >&2; exit 1; }
            DEVICE="$2"
            shift 2
            ;;
        --force)
            FORCE=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ ! "${DEVICE}" =~ ^([[:xdigit:]]{2}:){5}[[:xdigit:]]{2}$ ]]; then
    echo "A valid AirPods MAC is required." >&2
    usage >&2
    exit 1
fi
DEVICE="${DEVICE^^}"

for path in "${ENV_FILE}" "${SERVICE_FILE}"; do
    if [[ -L "${path}" || (-e "${path}" && ! -f "${path}") ]]; then
        echo "Refusing to overwrite unsafe path: ${path}" >&2
        exit 1
    fi
done

if [[ -e "${ENV_FILE}" && ${FORCE} -ne 1 ]]; then
    echo "${ENV_FILE} already exists; use --force to replace only the development configuration." >&2
    exit 1
fi

[[ -f "${SERVICE_TEMPLATE}" ]] || { echo "Development service template is missing." >&2; exit 1; }
[[ "${DAEMON_BIN}" != *$'\n'* && "${DAEMON_BIN}" != *" "* ]] || {
    echo "The checkout path cannot contain spaces for the development service." >&2
    exit 1
}

install -d -m 0755 "${CONFIG_DIR}" "${USER_SYSTEMD_DIR}"
temp_file="$(mktemp "${CONFIG_DIR}/.dev.environment.XXXXXX")"
trap 'rm -f "${temp_file}"' EXIT

printf 'AIRPODS_DEVICE=%s\nAIRPODS_MIC_GAIN_DB=18\nAIRPODS_MIC_LIMITER_DBFS=-3\nAIRPODS_HIRES_MIC_DEV_BIN=%s\n' \
    "${DEVICE}" "${DAEMON_BIN}" > "${temp_file}"
chmod 0644 "${temp_file}"
mv -f "${temp_file}" "${ENV_FILE}"
trap - EXIT
install -m 0644 "${SERVICE_TEMPLATE}" "${SERVICE_FILE}"

systemctl --user daemon-reload
echo "Created ${SERVICE_NAME}; it is not enabled and will not start at login."
echo "Start development with: ./scripts/dev.sh"
