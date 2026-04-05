#!/usr/bin/env bash
set -euo pipefail

get_repo_root() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd
}

get_workspace_version() {
    local repo_root="${1:?repo root required}"
    local cargo_toml="$repo_root/Cargo.toml"
    local version

    version="$(
        python - "$cargo_toml" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as fh:
    cargo = tomllib.load(fh)

print(cargo["workspace"]["package"]["version"])
PY
    )"

    if [[ -z "$version" ]]; then
        echo "Could not determine workspace version from $cargo_toml" >&2
        exit 1
    fi

    printf '%s\n' "$version"
}

get_macos_arch() {
    case "$(uname -m)" in
        x86_64)
            printf 'x86_64\n'
            ;;
        arm64|aarch64)
            printf 'arm64\n'
            ;;
        *)
            echo "Unsupported macOS architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac
}

initialize_publish_paths() {
    REPO_ROOT="${1:-$(get_repo_root)}"
    VERSION="$(get_workspace_version "$REPO_ROOT")"
    ARCH="$(get_macos_arch)"
    ARTIFACTS_ROOT="$REPO_ROOT/publish/macos/artifacts"
    DIST_ROOT="$ARTIFACTS_ROOT/dist"
    STAGE_ROOT="$ARTIFACTS_ROOT/stage"
    PREFIX="meridian-$VERSION-macos-$ARCH"
    STAGE_DIR="$STAGE_ROOT/$PREFIX"
    ZIP_PATH="$DIST_ROOT/$PREFIX.zip"
    TARGET_RELEASE="$REPO_ROOT/target/release"
}

initialize_stage_dir() {
    initialize_publish_paths "${1:-}"

    mkdir -p "$ARTIFACTS_ROOT" "$DIST_ROOT" "$STAGE_ROOT"
    rm -rf "$STAGE_DIR"
    mkdir -p "$STAGE_DIR"

    local cli_source="$TARGET_RELEASE/meridian-cli"
    local ui_source="$TARGET_RELEASE/meridian-ui"

    if [[ ! -f "$cli_source" ]]; then
        echo "Missing CLI binary: $cli_source" >&2
        exit 1
    fi

    if [[ ! -f "$ui_source" ]]; then
        echo "Missing UI binary: $ui_source" >&2
        exit 1
    fi

    cp "$cli_source" "$STAGE_DIR/meridian"
    cp "$ui_source" "$STAGE_DIR/meridian-ui"

    if [[ -f "$REPO_ROOT/README.md" ]]; then
        cp "$REPO_ROOT/README.md" "$STAGE_DIR/README.md"
    fi

    if [[ -f "$REPO_ROOT/assets/icons/meridian.icns" ]]; then
        cp "$REPO_ROOT/assets/icons/meridian.icns" "$STAGE_DIR/meridian.icns"
    fi

    cat >"$STAGE_DIR/KNOWN_ISSUES.txt" <<EOF
Current macOS prototype notes:
- meridian and meridian-ui were built natively on a GitHub Actions macOS runner.
- The smoke tests verify CLI help, geometry generation, UI help, and a short UI launch.
EOF
}
