#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

build_id="ubuntu2404"
stage_dir="$BUILD_ROOT/$build_id/stage"

build_binaries_in_container "$build_id" "$UBUNTU_BUILD_IMAGE" "$(debian_build_prereqs_script)"
stage_install_tree "$stage_dir" "$build_id"

tool_cache="$ARTIFACTS_DIR/.cache"
mkdir -p "$tool_cache"
artifact_path="$ARTIFACTS_DIR/Meridian-${PACKAGE_VERSION}-${ARCH}.AppImage"

docker_run "$UBUNTU_BUILD_IMAGE" "
$(debian_build_prereqs_script)
apt-get install -y desktop-file-utils libfuse2 >/dev/null
appdir='$BUILD_ROOT/appimage/AppDir'
tool_cache='$tool_cache'
artifact_path='$artifact_path'
rm -rf \"\$appdir\"
mkdir -p \"\$appdir\"
cp -a '$stage_dir/usr' \"\$appdir/usr\"
install -m 0755 '$ROOT/publish/linux/appimage/AppRun' \"\$appdir/AppRun\"
cp '$stage_dir/usr/share/applications/$APP_ID.desktop' \"\$appdir/$APP_ID.desktop\"
cp '$stage_dir/usr/share/icons/hicolor/256x256/apps/$APP_ID.png' \"\$appdir/$APP_ID.png\"
mkdir -p \"\$tool_cache\"
appimagetool=\"\$tool_cache/appimagetool-x86_64.AppImage\"
if [[ ! -x \"\$appimagetool\" ]]; then
    curl -L -o \"\$appimagetool\" https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x \"\$appimagetool\"
fi
APPIMAGE_EXTRACT_AND_RUN=1 ARCH='$ARCH' \"\$appimagetool\" \"\$appdir\" \"\$artifact_path\"
"

printf '%s\n' "$artifact_path"
