#!/usr/bin/env bash
# Update the vendored librqbit and keep our streaming fixes on top.
#
# vendor/librqbit is a patched copy of upstream (see
# patches/librqbit-streaming-fixes.patch and vendor/librqbit/README.md). A
# plain re-vendor would silently drop the fixes, so this script re-fetches
# the pristine crate, re-applies the patch, and regenerates the patch file.
# A patch that fails to apply is the merge mechanism: resolve the conflict
# by hand in vendor/librqbit, then regenerate.
#
# Usage:
#   scripts/update-vendor.sh <new-version>   e.g. scripts/update-vendor.sh 9.0.2
#   scripts/update-vendor.sh regen-patch     rebuild the patch file from the
#                                            committed pristine baseline
#                                            (after resolving a conflict)
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE=librqbit
VENDOR_DIR=vendor/librqbit
PATCH_FILE=patches/librqbit-streaming-fixes.patch
VERSION_DECL='crates/kitsune-core/Cargo.toml'
# Scratch space for the current command; cleaned by the EXIT trap. Global
# (not a function local) because bash tears down locals before the trap runs.
WORK_DIR=

current_version() {
    # Either `librqbit = { version = "9.0.1", ... }` or `librqbit = "9.0.1"`.
    sed -n 's/^librqbit = { version = "\([^"]*\)".*/\1/p' "${VERSION_DECL}" | head -1
    sed -n 's/^librqbit = "\([^"]*\)"/\1/p' "${VERSION_DECL}" | head -1
}

# Download one version's pristine sources into $2 (directory).
fetch_pristine() {
    local version=$1 dest=$2
    rm -rf "${dest}" && mkdir -p "${dest}"
    curl -fsSL "https://crates.io/api/v1/crates/${CRATE}/${version}/download" \
        | tar -xz -C "${dest}" --strip-components=1
}

# Diff two source trees and normalise the paths to a/vendor/librqbit,
# b/vendor/librqbit, so the patch applies from the repo root. Cargo
# bookkeeping files that are not part of the fixes are excluded.
make_patch() {
    local pristine_dir=$1 current_dir=$2 out=$3
    local stage
    stage=$(mktemp -d)
    mkdir -p "${stage}/aa/vendor" "${stage}/bb/vendor"
    cp -a "${pristine_dir}" "${stage}/aa/vendor/${CRATE}"
    cp -a "${current_dir}" "${stage}/bb/vendor/${CRATE}"
    rm -f "${stage}/aa/vendor/${CRATE}/.cargo-ok" \
        "${stage}/aa/vendor/${CRATE}/.cargo_vcs_info.json" \
        "${stage}/aa/vendor/${CRATE}/.cargo-checksum.json"
    rm -f "${stage}/bb/vendor/${CRATE}/.cargo-ok" \
        "${stage}/bb/vendor/${CRATE}/.cargo_vcs_info.json" \
        "${stage}/bb/vendor/${CRATE}/.cargo-checksum.json"
    (cd "${stage}" && git diff --no-index aa bb || true) \
        | sed -e 's|a/aa/vendor|a/vendor|g' -e 's|b/bb/vendor|b/vendor|g' \
        > "${out}"
    rm -rf "${stage}"
}

# Files a patch legitimately touches (cargo bookkeeping does not count).
patch_has_changes() {
    ! grep -q '^diff --git' "$1" && return 1
    return 0
}

cmd_regen_patch() {
    local version
    version=$(current_version)
    echo "Rebuilding ${PATCH_FILE} against pristine ${CRATE} ${version}..."
    WORK_DIR=$(mktemp -d)
    trap 'rm -rf "$WORK_DIR"' EXIT
    fetch_pristine "${version}" "${WORK_DIR}/pristine"
    make_patch "${WORK_DIR}/pristine" "${VENDOR_DIR}" "${PATCH_FILE}"
    if ! patch_has_changes "${PATCH_FILE}"; then
        echo "No differences found -- vendor matches pristine; leaving an empty patch." >&2
        rm -f "${PATCH_FILE}"
        exit 1
    fi
    echo "Rewrote ${PATCH_FILE}:"
    grep '^diff --git' "${PATCH_FILE}"
    echo "Review with: git diff -- ${PATCH_FILE}"
}

cmd_update() {
    local new_version=$1
    WORK_DIR=$(mktemp -d)
    trap 'rm -rf "$WORK_DIR"' EXIT

    echo "Fetching pristine ${CRATE} ${new_version}..."
    fetch_pristine "${new_version}" "${WORK_DIR}/pristine"

    echo "Replacing ${VENDOR_DIR} with pristine ${new_version}..."
    rsync -a --delete "${WORK_DIR}/pristine/" "${VENDOR_DIR}/"

    echo "Checking whether ${PATCH_FILE} applies..."
    if ! git apply --check "${PATCH_FILE}" 2>"${WORK_DIR}/apply-err"; then
        echo "" >&2
        echo "PATCH DOES NOT APPLY: upstream changed the patched code." >&2
        cat "${WORK_DIR}/apply-err" >&2
        echo "" >&2
        echo "${VENDOR_DIR} is now pristine ${new_version}; the fixes are NOT" >&2
        echo "applied. Merge by hand:" >&2
        echo "  1. re-read the rejected hunks above and rework the fixes in ${VENDOR_DIR}" >&2
        echo "  2. bump the version: ${0} regen-patch rewrites the patch file" >&2
        echo "  3. run: ${0} ${new_version}   (will apply cleanly once merged)" >&2
        echo "  4. cargo build && cargo test -p kitsune-core" >&2
        exit 1
    fi
    git apply "${PATCH_FILE}"
    echo "Patch applied."

    # Keep the declared dependency and the vendored README in step.
    sed -i -e "s/^\(${CRATE} = { version = \"\)[^\"]*\"/\1${new_version}\"/" \
        -e "s/^\(${CRATE} = \"\)[^\"]*\"/\1${new_version}\"/" \
        "${VERSION_DECL}"
    sed -i "s/vendors ${CRATE} [0-9][0-9.]*/vendors ${CRATE} ${new_version}/" \
        "${VENDOR_DIR}/README.md"

    echo "Refreshing Cargo.lock..."
    cargo build -p kitsune-core > /dev/null

    cat <<EOF

Done. Now verify and commit:
  cargo test -p kitsune-core --lib
  cargo test -p kitsune-core --test ram_stream_e2e -- --ignored
  git add -A && git commit -m "vendor: update ${CRATE} to ${new_version}"
EOF
}

case "${1:-}" in
    regen-patch) cmd_regen_patch ;;
    ""|"-h"|"--help")
        sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
        [ -n "${1:-}" ] || { echo "error: pass a version (e.g. $(basename "$0") 9.0.2) or 'regen-patch'" >&2; exit 1; }
        ;;
    *)
        [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]] \
            || { echo "error: '$1' does not look like a crate version" >&2; exit 1; }
        cmd_update "$1"
        ;;
esac
