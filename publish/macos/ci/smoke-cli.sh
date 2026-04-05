#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

repo_root="$(get_repo_root)"
initialize_publish_paths "$repo_root"

cli_path="$STAGE_DIR/meridian"
if [[ ! -f "$cli_path" ]]; then
    echo "Missing staged CLI binary: $cli_path" >&2
    exit 1
fi

"$cli_path" --help >/dev/null

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

geometry_path="$tmp_dir/geometry.json"
"$cli_path" debug-piano-trail-classic-geometry --width 64 --height 64 >"$geometry_path"

if [[ ! -s "$geometry_path" ]]; then
    echo "Geometry smoke output was empty: $geometry_path" >&2
    exit 1
fi
