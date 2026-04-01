#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

build_id="debian12"
stage_dir="$BUILD_ROOT/$build_id/stage"

build_binaries_in_container "$build_id" "$DEBIAN_BUILD_IMAGE" "$(debian_build_prereqs_script)"
stage_install_tree "$stage_dir" "$build_id"

tarball_root="$BUILD_ROOT/tarball-root"
prepare_tarball_root "$tarball_root" "$stage_dir"

archive_path="$ARTIFACTS_DIR/${TARBALL_PREFIX}.tar.gz"
tar -C "$tarball_root" -czf "$archive_path" "$TARBALL_PREFIX"

printf '%s\n' "$archive_path"
