#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

usage() {
    cat <<'EOF'
Usage:
  ./install.sh --deb <path-to-package.deb>
  ./install.sh --dev --device <AIRPODS_MAC>

Use --deb for a normal package installation. The development option creates
only the isolated airpods-hires-mic-dev.service and never changes production.
EOF
}

if ((EUID == 0)); then
    echo "Run this helper as the desktop user, not with sudo." >&2
    exit 1
fi

case "${1:-}" in
    --deb)
        [[ $# -eq 2 ]] || { usage >&2; exit 1; }
        [[ -f "$2" && ! -L "$2" ]] || { echo "Package does not exist or is unsafe: $2" >&2; exit 1; }
        package_path="$(realpath -- "$2")"
        exec sudo apt-get install "${package_path}"
        ;;
    --dev)
        shift
        exec "${PROJECT_DIR}/scripts/dev-setup.sh" "$@"
        ;;
    --help|-h|"")
        usage
        ;;
    *)
        usage >&2
        exit 1
        ;;
esac
