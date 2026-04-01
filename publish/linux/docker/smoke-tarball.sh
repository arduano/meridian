#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

artifact_path="${1:-$ARTIFACTS_DIR/${TARBALL_PREFIX}.tar.gz}"
test -f "$artifact_path"

run_debian() {
    local image="$1"
    local deps="$2"
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" "$image" bash -lc "
        apt-get update >/dev/null &&
        apt-get install -y tar $deps >/dev/null &&
        rm -rf /tmp/meridian-extract &&
        mkdir -p /tmp/meridian-extract &&
        tar -C /tmp/meridian-extract -xzf '$artifact_path' &&
        publish/linux/docker/smoke-common.sh &&
        source publish/linux/docker/smoke-common.sh &&
        run_smoke /tmp/meridian-extract/$TARBALL_PREFIX/usr/bin
    "
}

run_fedora() {
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" fedora:41 bash -lc "
        dnf install -y tar ${fedora_runtime_deps[*]} >/dev/null &&
        rm -rf /tmp/meridian-extract &&
        mkdir -p /tmp/meridian-extract &&
        tar -C /tmp/meridian-extract -xzf '$artifact_path' &&
        source publish/linux/docker/smoke-common.sh &&
        run_smoke /tmp/meridian-extract/$TARBALL_PREFIX/usr/bin
    "
}

run_opensuse() {
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" opensuse/tumbleweed bash -lc "
        zypper --non-interactive refresh >/dev/null &&
        zypper --non-interactive install tar ${opensuse_runtime_deps[*]} >/dev/null &&
        rm -rf /tmp/meridian-extract &&
        mkdir -p /tmp/meridian-extract &&
        tar -C /tmp/meridian-extract -xzf '$artifact_path' &&
        source publish/linux/docker/smoke-common.sh &&
        run_smoke /tmp/meridian-extract/$TARBALL_PREFIX/usr/bin
    "
}

run_arch() {
    docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" archlinux:base-devel bash -lc "
        pacman -Sy --noconfirm tar ${arch_runtime_deps[*]} >/dev/null &&
        rm -rf /tmp/meridian-extract &&
        mkdir -p /tmp/meridian-extract &&
        tar -C /tmp/meridian-extract -xzf '$artifact_path' &&
        source publish/linux/docker/smoke-common.sh &&
        run_smoke /tmp/meridian-extract/$TARBALL_PREFIX/usr/bin
    "
}

run_debian debian:12-slim "${debian_runtime_deps[*]}"
run_debian ubuntu:24.04 "${ubuntu_runtime_deps[*]}"
run_fedora
run_opensuse
run_arch
