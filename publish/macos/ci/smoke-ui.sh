#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

repo_root="$(get_repo_root)"
initialize_publish_paths "$repo_root"

ui_path="$STAGE_DIR/meridian-ui"
if [[ ! -f "$ui_path" ]]; then
    echo "Missing staged UI binary: $ui_path" >&2
    exit 1
fi

"$ui_path" --help >/dev/null

ui_log="$(mktemp)"
pid=""
cleanup() {
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    fi
    rm -f "$ui_log"
}
trap cleanup EXIT

"$ui_path" >"$ui_log" 2>&1 &
pid="$!"
sleep 5

if ! kill -0 "$pid" 2>/dev/null; then
    set +e
    wait "$pid"
    status="$?"
    set -e

    if [[ "$status" -ne 0 ]]; then
        echo "meridian-ui exited early with code $status" >&2
        cat "$ui_log" >&2
        exit 1
    fi
fi
