#!/usr/bin/env bash
# Fetch and build optional external Othello engines (Edax / Egaroucid)
# under `vendor/engines/`.
#
# These binaries are NOT bundled with the repository (see `.gitignore`).
# They are fetched on demand by this script so users can run the
# `#[ignore]`-gated real-engine integration tests under
# `crates/othello-player/tests/real_engine.rs`.
#
# Phase 6.1 of the design document (§10).
#
# Usage:
#   bash scripts/fetch_engines.sh <target> [--force]
#
#   target: edax | egaroucid | all
#
# Examples:
#   bash scripts/fetch_engines.sh edax
#   bash scripts/fetch_engines.sh egaroucid
#   bash scripts/fetch_engines.sh all --force
#
# Notes:
#   - Edax (https://github.com/abulmo/edax-reversi, GPL-2.0) is built from
#     source. The proprietary `eval.dat` evaluation file is NOT downloaded;
#     the user must obtain it manually from edax-reversi.org and place it
#     under `vendor/engines/edax/data/`.
#   - Egaroucid (https://github.com/Nyanyan/Egaroucid, GPL-3.0) prebuilt
#     binary URLs change between releases. The URLs in this script are
#     placeholders; update them before running. See
#     `docs/external-engines.md` for guidance.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="${SCRIPT_DIR}/.."
VENDOR_DIR="${REPO_ROOT}/vendor/engines"

FORCE="false"
TARGET=""

print_usage() {
    cat <<EOF
Usage: fetch_engines.sh <target> [--force]
  target: edax | egaroucid | all

Examples:
  bash scripts/fetch_engines.sh edax
  bash scripts/fetch_engines.sh egaroucid
  bash scripts/fetch_engines.sh all --force
EOF
}

# --- argument parsing -------------------------------------------------------

if [[ $# -eq 0 ]]; then
    print_usage
    exit 1
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            print_usage
            exit 0
            ;;
        --force)
            FORCE="true"
            shift
            ;;
        edax|egaroucid|all)
            if [[ -n "${TARGET}" ]]; then
                echo "error: target already set to '${TARGET}', got '$1'" >&2
                exit 2
            fi
            TARGET="$1"
            shift
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            print_usage
            exit 2
            ;;
    esac
done

if [[ -z "${TARGET}" ]]; then
    echo "error: target is required" >&2
    print_usage
    exit 2
fi

# --- helpers ----------------------------------------------------------------

write_license_note() {
    local dir="$1"
    local body="$2"
    mkdir -p "${dir}"
    printf '%s\n' "${body}" > "${dir}/LICENSE-NOTE.md"
}

# --- Edax -------------------------------------------------------------------

install_edax() {
    local edax_dir="${VENDOR_DIR}/edax"
    local edax_repo="https://github.com/abulmo/edax-reversi.git"
    local edax_tag="4.4"

    mkdir -p "${VENDOR_DIR}"
    if [[ -d "${edax_dir}" && "${FORCE}" != "true" ]]; then
        echo "Edax already installed at ${edax_dir} (use --force to reinstall)"
        return 0
    fi
    rm -rf "${edax_dir}"

    echo "Cloning Edax ${edax_tag} from ${edax_repo} ..."
    git clone --depth 1 --branch "${edax_tag}" "${edax_repo}" "${edax_dir}"

    echo "Building Edax for $(uname -s)-$(uname -m) ..."
    (
        cd "${edax_dir}/src"
        case "$(uname -s)-$(uname -m)" in
            Darwin-arm64)
                make build ARCH=arm BUILD=osx
                ;;
            Darwin-*)
                make build ARCH=x64-modern BUILD=osx
                ;;
            Linux-x86_64)
                make build ARCH=x64-modern BUILD=linux
                ;;
            Linux-aarch64|Linux-arm64)
                make build ARCH=arm BUILD=linux
                ;;
            *)
                echo "error: unsupported platform: $(uname -s)-$(uname -m)" >&2
                exit 1
                ;;
        esac
    )

    echo
    echo "Edax built at ${edax_dir}/bin/"
    echo "Note: please obtain eval.dat manually from edax-reversi.org and place"
    echo "      it under ${edax_dir}/data/eval.dat ."
    echo

    write_license_note "${edax_dir}" "$(cat <<'EOF'
# Edax

- Source: https://github.com/abulmo/edax-reversi
- License: GPL-2.0
- Author: Richard Delorme

This binary was fetched and built locally by `scripts/fetch_engines.sh`.
The `eval.dat` file (proprietary) is NOT bundled and must be downloaded
manually by the user from edax-reversi.org and placed under `data/`.
EOF
)"
}

# --- Egaroucid --------------------------------------------------------------

install_egaroucid() {
    local egaroucid_dir="${VENDOR_DIR}/egaroucid"
    local egaroucid_release_url

    # NOTE: Egaroucid release archives change names per version.
    # Update these URLs to point at the desired release before running.
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64)
            egaroucid_release_url="<TODO: fill release URL for macOS arm64>"
            ;;
        Darwin-*)
            egaroucid_release_url="<TODO: fill release URL for macOS x86_64>"
            ;;
        Linux-x86_64)
            egaroucid_release_url="<TODO: fill release URL for Linux x86_64>"
            ;;
        *)
            echo "error: unsupported platform: $(uname -s)-$(uname -m)" >&2
            exit 1
            ;;
    esac

    if [[ "${egaroucid_release_url}" == *"<TODO"* ]]; then
        cat >&2 <<EOF
error: Egaroucid release URL is not configured.
       Edit \`scripts/fetch_engines.sh\` and replace the <TODO> placeholders
       with concrete URLs from
       https://github.com/Nyanyan/Egaroucid/releases
       See docs/external-engines.md for details.
EOF
        return 1
    fi

    mkdir -p "${VENDOR_DIR}"
    if [[ -d "${egaroucid_dir}" && "${FORCE}" != "true" ]]; then
        echo "Egaroucid already installed at ${egaroucid_dir} (use --force to reinstall)"
        return 0
    fi
    rm -rf "${egaroucid_dir}"
    mkdir -p "${egaroucid_dir}"

    echo "Downloading Egaroucid from ${egaroucid_release_url} ..."
    curl -L "${egaroucid_release_url}" -o "${egaroucid_dir}/egaroucid.tar.gz"
    tar -xzf "${egaroucid_dir}/egaroucid.tar.gz" -C "${egaroucid_dir}"
    rm -f "${egaroucid_dir}/egaroucid.tar.gz"

    write_license_note "${egaroucid_dir}" "$(cat <<'EOF'
# Egaroucid

- Source: https://github.com/Nyanyan/Egaroucid
- License: GPL-3.0
- Author: Takuto Yamana (Nyanyan)

This binary was fetched by `scripts/fetch_engines.sh` from a published
GitHub release. The release URL is hardcoded in the script and must be
updated when Egaroucid publishes a new version.
EOF
)"
}

# --- main -------------------------------------------------------------------

mkdir -p "${VENDOR_DIR}"

case "${TARGET}" in
    edax)
        install_edax
        ;;
    egaroucid)
        install_egaroucid
        ;;
    all)
        install_edax
        install_egaroucid
        ;;
    *)
        echo "error: unknown target '${TARGET}'" >&2
        print_usage
        exit 2
        ;;
esac

echo "Done."
