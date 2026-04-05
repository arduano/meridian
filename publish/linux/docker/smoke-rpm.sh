#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

artifact_path="${1:-$(find "$ARTIFACTS_DIR" -name '*.rpm' -print -quit)}"
test -n "$artifact_path"
test -f "$artifact_path"

docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" fedora:41 bash -lc "
    dnf install -y '$artifact_path' >/dev/null &&
    source publish/linux/docker/smoke-common.sh &&
    run_smoke /usr/bin
"

docker run --rm -v "$ROOT:$ROOT" -w "$ROOT" opensuse/tumbleweed bash -lc "
    zypper --non-interactive refresh >/dev/null &&
    zypper --non-interactive install --allow-unsigned-rpm '$artifact_path' >/dev/null &&
    source publish/linux/docker/smoke-common.sh &&
    run_smoke /usr/bin
"
