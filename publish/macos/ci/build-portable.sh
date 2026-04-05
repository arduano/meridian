#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

repo_root="$(get_repo_root)"
initialize_stage_dir "$repo_root"

rm -f "$ZIP_PATH"
ditto -c -k --sequesterRsrc --keepParent "$STAGE_DIR" "$ZIP_PATH"

printf 'Created portable artifact: %s\n' "$ZIP_PATH"
