#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

build_id="debian12"
stage_dir="$BUILD_ROOT/$build_id/stage"

build_binaries_in_container "$build_id" "$DEBIAN_BUILD_IMAGE" "$(debian_build_prereqs_script)"
stage_install_tree "$stage_dir" "$build_id"

deb_root="$BUILD_ROOT/deb"
reset_dir "$deb_root"
cp -a "$stage_dir/." "$deb_root/"
mkdir -p "$deb_root/DEBIAN"

depends="$(join_by ', ' "${debian_runtime_deps[@]}")"
installed_size_kib="$(du -sk "$deb_root" | awk '{print $1}')"

cat >"$deb_root/DEBIAN/control" <<EOF
Package: $PACKAGE_NAME
Version: ${PACKAGE_VERSION}+${PACKAGE_ITERATION}
Section: $PACKAGE_SECTION
Priority: optional
Architecture: amd64
Maintainer: $PACKAGE_MAINTAINER
Depends: $depends
Installed-Size: $installed_size_kib
Homepage: $PACKAGE_URL
Description: $PACKAGE_DESCRIPTION
 Meridian ships a headless CLI and a Slint-based desktop frontend.
EOF

artifact_path="$ARTIFACTS_DIR/${PACKAGE_NAME}_${PACKAGE_VERSION}+${PACKAGE_ITERATION}_amd64.deb"
docker run --rm \
    -v "$ROOT:$ROOT" \
    -w "$ROOT" \
    "$DEBIAN_BUILD_IMAGE" \
    bash -lc "dpkg-deb --build '$deb_root' '$artifact_path'"

printf '%s\n' "$artifact_path"
