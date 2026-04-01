#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
WORKSPACE_PARENT="${WORKSPACE_PARENT:-$(dirname "$ROOT")}"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$ROOT/publish/linux/artifacts}"
BUILD_ROOT="$ARTIFACTS_DIR/build"
CARGO_BUILD_PROFILE="${CARGO_BUILD_PROFILE:-release}"
RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-nightly-2026-03-29}"
PACKAGE_NAME="${PACKAGE_NAME:-meridian}"
UI_BINARY_NAME="${UI_BINARY_NAME:-meridian-ui}"
CLI_SOURCE_BINARY="${CLI_SOURCE_BINARY:-meridian-cli}"
CLI_BINARY_NAME="${CLI_BINARY_NAME:-meridian}"
APP_ID="${APP_ID:-io.github.arduano.meridian}"
ARCH="${ARCH:-x86_64}"
PACKAGE_LICENSE="${PACKAGE_LICENSE:-LicenseRef-Unknown}"
PACKAGE_MAINTAINER="${PACKAGE_MAINTAINER:-Meridian Maintainers}"
PACKAGE_URL="${PACKAGE_URL:-https://github.com/arduano/meridian}"
PACKAGE_DESCRIPTION="${PACKAGE_DESCRIPTION:-Desktop MIDI visualizer and CLI toolkit for Meridian}"
PACKAGE_SECTION="${PACKAGE_SECTION:-sound}"
RPM_RELEASE="${RPM_RELEASE:-1}"
DEBIAN_BUILD_IMAGE="${DEBIAN_BUILD_IMAGE:-debian:12-slim}"
UBUNTU_BUILD_IMAGE="${UBUNTU_BUILD_IMAGE:-ubuntu:24.04}"
FEDORA_BUILD_IMAGE="${FEDORA_BUILD_IMAGE:-fedora:41}"
OPENSUSE_BUILD_IMAGE="${OPENSUSE_BUILD_IMAGE:-opensuse/tumbleweed}"
ARCH_BUILD_IMAGE="${ARCH_BUILD_IMAGE:-archlinux:base-devel}"
WORKSPACE_VERSION="${WORKSPACE_VERSION:-$(cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" | jq -r '.packages[] | select(.name == "meridian-ui") | .version' | head -n 1)}"
GIT_SHA="${GIT_SHA:-$(git -C "$ROOT" rev-parse --short HEAD)}"
PACKAGE_VERSION="${PACKAGE_VERSION:-$WORKSPACE_VERSION}"
PACKAGE_ITERATION="${PACKAGE_ITERATION:-git$GIT_SHA}"
TARBALL_PREFIX="${PACKAGE_NAME}-${PACKAGE_VERSION}-linux-${ARCH}"
CARGO_HOME_DIR="$ARTIFACTS_DIR/.cargo-home"
RUSTUP_HOME_DIR="$ARTIFACTS_DIR/.rustup-home"

debian_runtime_deps=(
    "libasound2 | libasound2t64"
    libdbus-1-3
    libfontconfig1
    libfreetype6
    libgl1
    libwayland-client0
    libx11-6
    libxcb1
    libxcursor1
    libxext6
    libxfixes3
    libxi6
    libxkbcommon0
    libxkbfile1
    libxrandr2
    libxrender1
)

ubuntu_runtime_deps=(
    libasound2t64
    libdbus-1-3
    libfontconfig1
    libfreetype6
    libgl1
    libwayland-client0
    libx11-6
    libxcb1
    libxcursor1
    libxext6
    libxfixes3
    libxi6
    libxkbcommon0
    libxkbfile1
    libxrandr2
    libxrender1
)

fedora_runtime_deps=(
    alsa-lib
    dbus-libs
    fontconfig
    freetype
    libX11
    libXcursor
    libXext
    libXfixes
    libXi
    libxkbcommon
    libxkbfile
    libXrandr
    libXrender
    libxcb
    mesa-libGL
    wayland
)

arch_runtime_deps=(
    alsa-lib
    dbus
    fontconfig
    freetype2
    libx11
    libxcb
    libxcursor
    libxext
    libxfixes
    libxi
    libxkbcommon
    libxkbfile
    libxrandr
    libxrender
    mesa
    wayland
)

opensuse_runtime_deps=(
    alsa-lib
    dbus-1
    fontconfig
    freetype2
    libX11-6
    libXcursor1
    libXext6
    libXfixes3
    libXi6
    libxkbcommon0
    libxkbfile1
    libXrandr2
    libXrender1
    libxcb1
    Mesa-libGL1
    libwayland-client0
)

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "missing required command: $1" >&2
        exit 1
    }
}

ensure_workspace_layout() {
    [[ -d "$WORKSPACE_PARENT/midi-toolkit-rs" ]] || {
        echo "missing sibling repo: $WORKSPACE_PARENT/midi-toolkit-rs" >&2
        exit 1
    }
    [[ -d "$WORKSPACE_PARENT/xsynth" ]] || {
        echo "missing sibling repo: $WORKSPACE_PARENT/xsynth" >&2
        exit 1
    }
}

ensure_artifacts_dir() {
    mkdir -p "$ARTIFACTS_DIR" "$BUILD_ROOT" "$CARGO_HOME_DIR" "$RUSTUP_HOME_DIR"
}

reset_dir() {
    rm -rf "$1"
    mkdir -p "$1"
}

target_profile_dir() {
    case "$CARGO_BUILD_PROFILE" in
        dev)
            printf 'debug'
            ;;
        *)
            printf '%s' "$CARGO_BUILD_PROFILE"
            ;;
    esac
}

cargo_profile_args() {
    case "$CARGO_BUILD_PROFILE" in
        release)
            printf '%s' '--release'
            ;;
        dev)
            printf '%s' ''
            ;;
        *)
            printf '%s %s' '--profile' "$CARGO_BUILD_PROFILE"
            ;;
    esac
}

docker_run() {
    local image="$1"
    local script="$2"

    require_cmd docker
    ensure_workspace_layout
    ensure_artifacts_dir

    docker run --rm \
        -v "$WORKSPACE_PARENT:$WORKSPACE_PARENT" \
        -v "$CARGO_HOME_DIR:/root/.cargo" \
        -v "$RUSTUP_HOME_DIR:/root/.rustup" \
        -w "$ROOT" \
        "$image" \
        bash -lc "$script"
}

bootstrap_rustup_script() {
    cat <<EOF
set -euo pipefail
export PATH="/root/.cargo/bin:/usr/local/cargo/bin:\$PATH"
if ! command -v rustup >/dev/null 2>&1; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain none
fi
rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal >/dev/null
rustup default "$RUST_TOOLCHAIN" >/dev/null
rustc --version
cargo --version
EOF
}

debian_build_prereqs_script() {
    cat <<'EOF'
apt-get update >/dev/null
DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential \
    ca-certificates \
    clang \
    curl \
    file \
    libasound2-dev \
    libdbus-1-dev \
    libfontconfig1-dev \
    libfreetype6-dev \
    libgl1-mesa-dev \
    libwayland-dev \
    libx11-dev \
    libxcb1-dev \
    libxcursor-dev \
    libxext-dev \
    libxfixes-dev \
    libxi-dev \
    libxkbcommon-dev \
    libxkbfile-dev \
    libxrandr-dev \
    libxrender-dev \
    pkg-config \
    tar \
    xz-utils >/dev/null
EOF
}

fedora_build_prereqs_script() {
    cat <<'EOF'
dnf install -y \
    alsa-lib-devel \
    ca-certificates \
    clang \
    curl \
    dbus-devel \
    file \
    fontconfig-devel \
    freetype-devel \
    gcc \
    gcc-c++ \
    libX11-devel \
    libxcb-devel \
    libXcursor-devel \
    libXext-devel \
    libXfixes-devel \
    libXi-devel \
    libxkbcommon-devel \
    libxkbfile-devel \
    libXrandr-devel \
    libXrender-devel \
    make \
    mesa-libGL-devel \
    pkgconf-pkg-config \
    rpm-build \
    tar \
    wayland-devel \
    xz >/dev/null
EOF
}

build_binaries_in_container() {
    local build_id="$1"
    local image="$2"
    local prereq_script="$3"
    local target_dir="$BUILD_ROOT/$build_id/target"
    local cargo_args

    cargo_args="$(cargo_profile_args)"
    mkdir -p "$BUILD_ROOT/$build_id"

    docker_run "$image" "
$(printf '%s\n' "$prereq_script")
$(bootstrap_rustup_script)
export CARGO_TARGET_DIR='$target_dir'
cargo build $cargo_args -p meridian-cli -p meridian-ui
"
}

write_desktop_entry() {
    local destination="$1"
    cat >"$destination" <<EOF
[Desktop Entry]
Type=Application
Name=Meridian
GenericName=MIDI Visualizer
Comment=$PACKAGE_DESCRIPTION
Exec=$UI_BINARY_NAME
Icon=$APP_ID
Categories=AudioVideo;Audio;Music;
Terminal=false
StartupNotify=true
EOF
}

stage_install_tree() {
    local stage_dir="$1"
    local build_id="$2"
    local target_dir="$BUILD_ROOT/$build_id/target/$(target_profile_dir)"

    reset_dir "$stage_dir"
    mkdir -p \
        "$stage_dir/usr/bin" \
        "$stage_dir/usr/share/applications" \
        "$stage_dir/usr/share/doc/$PACKAGE_NAME" \
        "$stage_dir/usr/share/icons/hicolor/256x256/apps"

    install -m 0755 "$target_dir/$CLI_SOURCE_BINARY" "$stage_dir/usr/bin/$CLI_BINARY_NAME"
    install -m 0755 "$target_dir/$UI_BINARY_NAME" "$stage_dir/usr/bin/$UI_BINARY_NAME"
    install -m 0644 "$ROOT/README.md" "$stage_dir/usr/share/doc/$PACKAGE_NAME/README.md"
    install -m 0644 \
        "$ROOT/assets/icons/meridian-256.png" \
        "$stage_dir/usr/share/icons/hicolor/256x256/apps/$APP_ID.png"
    write_desktop_entry "$stage_dir/usr/share/applications/$APP_ID.desktop"
}

prepare_tarball_root() {
    local destination="$1"
    local stage_dir="$2"
    local payload_root="$destination/$TARBALL_PREFIX"

    reset_dir "$destination"
    mkdir -p "$payload_root"
    cp -a "$stage_dir/usr/." "$payload_root/"
}

join_by() {
    local separator="$1"
    shift
    local first=1
    for value in "$@"; do
        if [[ $first -eq 1 ]]; then
            printf '%s' "$value"
            first=0
        else
            printf '%s%s' "$separator" "$value"
        fi
    done
}

write_fixture_midi() {
    local destination="$1"
    mkdir -p "$(dirname "$destination")"
    printf '\x4d\x54\x68\x64\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60\x4d\x54\x72\x6b\x00\x00\x00\x1b\x00\xff\x51\x03\x07\xa1\x20\x00\x90\x3c\x64\x30\x90\x40\x64\x30\x80\x3c\x40\x30\x80\x40\x40\x00\xff\x2f\x00' >"$destination"
}
