#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

artifact_path="${1:-$ARTIFACTS_DIR/${PACKAGE_NAME}_${PACKAGE_VERSION}+${PACKAGE_ITERATION}_amd64.deb}"
test -f "$artifact_path"

run_deb_smoke() {
    local image="$1"
    local deps="$2"
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" "$image" bash -lc "
        apt-get update >/dev/null &&
        apt-get install -y $deps >/dev/null &&
        dpkg -i '$artifact_path' >/dev/null &&
        source publish/linux/docker/smoke-common.sh &&
        run_smoke /usr/bin
    "
}

run_deb_smoke debian:12-slim "${debian_runtime_deps[*]}"
run_deb_smoke ubuntu:24.04 "${ubuntu_runtime_deps[*]}"
