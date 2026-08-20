#!/usr/bin/env bash
set -euo pipefail

APP_NAME="airpods-hires-mic"
SERVICE_NAME="${APP_NAME}.service"
INSTALL_DIR="${HOME}/.local/bin"
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="${HOME}/.config/${APP_NAME}"
ENV_FILE="${CONFIG_DIR}/environment"
USER_SYSTEMD_DIR="${HOME}/.config/systemd/user"
SERVICE_FILE="${USER_SYSTEMD_DIR}/${SERVICE_NAME}"
DEVICE=""
MIC_GAIN_DB="18"
MIC_LIMITER_DBFS="-3"

if ((EUID == 0)); then
    echo "Do not run this installer with sudo. Run ./install.sh as the desktop user." >&2
    exit 1
fi

usage() {
    echo "Usage: ./install.sh --device <AIRPODS_MAC>"
}

while (($#)); do
    case "$1" in
        --device)
            if (($# < 2)); then
                echo "--device requires a Bluetooth MAC address." >&2
                usage >&2
                exit 1
            fi
            DEVICE="$2"
            shift 2
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

if [[ -r "${ENV_FILE}" ]]; then
    while IFS='=' read -r key value; do
        case "${key}" in
            AIRPODS_DEVICE)
                if [[ -z "${DEVICE}" ]]; then
                    DEVICE="${value}"
                fi
                ;;
            AIRPODS_MIC_GAIN_DB)
                if [[ -n "${value}" ]]; then
                    MIC_GAIN_DB="${value}"
                fi
                ;;
            AIRPODS_MIC_LIMITER_DBFS)
                if [[ -n "${value}" ]]; then
                    MIC_LIMITER_DBFS="${value}"
                fi
                ;;
        esac
    done < "${ENV_FILE}"
fi

if [[ ! "${DEVICE}" =~ ^([[:xdigit:]]{2}:){5}[[:xdigit:]]{2}$ ]]; then
    echo "A valid AirPods MAC is required, for example:" >&2
    echo "  ./install.sh --device F8:1E:49:E9:51:34" >&2
    exit 1
fi
DEVICE="${DEVICE^^}"

if [[ ! -r /etc/os-release ]]; then
    echo "Unsupported system: /etc/os-release is unavailable." >&2
    exit 1
fi

# shellcheck disable=SC1091
source /etc/os-release
case "${ID:-}" in
    ubuntu|debian) ;;
    *)
        echo "Unsupported distribution: ${ID:-unknown}. Ubuntu/Debian is required for this installer." >&2
        exit 1
        ;;
esac

packages=(build-essential pkg-config libdbus-1-dev libfdk-aac-dev bluez pulseaudio-utils)
missing_packages=()
for package in "${packages[@]}"; do
    if ! dpkg-query -W -f='${Status}' "${package}" 2>/dev/null | grep -q '^install ok installed$'; then
        missing_packages+=("${package}")
    fi
done

if ((${#missing_packages[@]})); then
    echo "Installing build dependencies: ${missing_packages[*]}"
    sudo apt-get update
    sudo apt-get install -y "${missing_packages[@]}"
fi

for command in cargo rustc pactl bluetoothctl pkg-config systemctl; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        if [[ "${command}" == "cargo" || "${command}" == "rustc" ]]; then
            echo "Rust is missing. Install or update Rust explicitly with rustup, then run this installer again." >&2
        else
            echo "Required command is unavailable after dependency installation: ${command}" >&2
        fi
        exit 1
    fi
done

if ! systemctl --user show-environment >/dev/null 2>&1; then
    echo "The systemd user manager is unavailable. Run this installer inside your desktop user session." >&2
    exit 1
fi

echo "Building ${APP_NAME}..."
cargo build --manifest-path "${PROJECT_DIR}/Cargo.toml" --release

install -d -m 0755 "${CONFIG_DIR}" "${USER_SYSTEMD_DIR}"
for target in "${ENV_FILE}" "${SERVICE_FILE}"; do
    if [[ -L "${target}" || (-e "${target}" && ! -f "${target}") ]]; then
        echo "Refusing to overwrite unsafe path: ${target}" >&2
        exit 1
    fi
done

if [[ -f "${SERVICE_FILE}" ]]; then
    systemctl --user stop "${SERVICE_NAME}"
fi
if pgrep -f '(^|/)airpods-hires-mic( |$)' >/dev/null 2>&1; then
    echo "${APP_NAME} is running outside its user service. Stop it before installing." >&2
    exit 1
fi

install -d -m 0755 "${INSTALL_DIR}"
install -m 0755 "${PROJECT_DIR}/target/release/${APP_NAME}" "${INSTALL_DIR}/${APP_NAME}"
printf 'AIRPODS_DEVICE=%s\nAIRPODS_MIC_GAIN_DB=%s\nAIRPODS_MIC_LIMITER_DBFS=%s\n' \
    "${DEVICE}" "${MIC_GAIN_DB}" "${MIC_LIMITER_DBFS}" > "${ENV_FILE}"
chmod 0644 "${ENV_FILE}"
install -m 0644 "${PROJECT_DIR}/systemd/${SERVICE_NAME}" "${SERVICE_FILE}"

systemctl --user daemon-reload
systemctl --user enable "${SERVICE_NAME}"
systemctl --user restart "${SERVICE_NAME}"

echo "Installed ${INSTALL_DIR}/${APP_NAME}"
echo "Enabled ${SERVICE_NAME} for AirPods ${DEVICE}"
echo "The virtual microphone is created when the service receives valid AirPods audio."
echo "Check status: systemctl --user status ${SERVICE_NAME}"
echo "Check logs:   journalctl --user -u ${SERVICE_NAME} -f"
echo "This installer does not change Bluetooth profiles, A2DP, AAC, PipeWire configuration, or /etc."
if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    echo "Note: ${INSTALL_DIR} is not in PATH. Add it yourself before using ${APP_NAME}."
fi
