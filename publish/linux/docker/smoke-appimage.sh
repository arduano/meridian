#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

artifact_path="${1:-$ARTIFACTS_DIR/Meridian-${PACKAGE_VERSION}-${ARCH}.AppImage}"
test -f "$artifact_path"

run_appimage_smoke() {
    local image="$1"
    local install_cmd="$2"
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" "$image" bash -lc "
        $install_cmd >/dev/null &&
        chmod +x '$artifact_path' &&
        APPIMAGE_EXTRACT_AND_RUN=1 '$artifact_path' meridian --help >/dev/null &&
        APPIMAGE_EXTRACT_AND_RUN=1 '$artifact_path' meridian-ui --help >/dev/null &&
        APPIMAGE_EXTRACT_AND_RUN=1 '$artifact_path' meridian debug-piano-trail-classic-geometry --width 64 --height 64 >/tmp/geometry.json &&
        test -s /tmp/geometry.json
    "
}

run_appimage_smoke debian:12-slim "apt-get update && apt-get install -y ${debian_smoke_runtime_deps[*]} libfuse2"
run_appimage_smoke ubuntu:24.04 "apt-get update && apt-get install -y ${ubuntu_runtime_deps[*]} libfuse2"
run_appimage_smoke fedora:41 "dnf install -y ${fedora_runtime_deps[*]} fuse-libs"
run_appimage_smoke opensuse/tumbleweed "zypper --non-interactive refresh && zypper --non-interactive install ${opensuse_runtime_deps[*]} fuse"
