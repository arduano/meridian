#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/common/lib.sh"

build_id="fedora41"
stage_dir="$BUILD_ROOT/$build_id/stage"

build_binaries_in_container "$build_id" "$FEDORA_BUILD_IMAGE" "$(fedora_build_prereqs_script)"
stage_install_tree "$stage_dir" "$build_id"

rpm_root="$BUILD_ROOT/rpm"
spec_root="$rpm_root/rpmbuild"
reset_dir "$rpm_root"
mkdir -p \
    "$spec_root/BUILD" \
    "$spec_root/BUILDROOT" \
    "$spec_root/RPMS" \
    "$spec_root/SOURCES" \
    "$spec_root/SPECS" \
    "$spec_root/SRPMS"

tarball_root="$rpm_root/tarball"
prepare_tarball_root "$tarball_root" "$stage_dir"
source_tarball="$spec_root/SOURCES/${TARBALL_PREFIX}.tar.gz"
tar -C "$tarball_root" -czf "$source_tarball" "$TARBALL_PREFIX"

{
    echo "Name:           $PACKAGE_NAME"
    echo "Version:        $PACKAGE_VERSION"
    echo "Release:        ${RPM_RELEASE}.${PACKAGE_ITERATION}"
    echo "Summary:        $PACKAGE_DESCRIPTION"
    echo "License:        $PACKAGE_LICENSE"
    echo "URL:            $PACKAGE_URL"
    echo "Source0:        ${TARBALL_PREFIX}.tar.gz"
    echo "BuildArch:      x86_64"
    for dep in "${fedora_runtime_deps[@]}"; do
        echo "Requires:       $dep"
    done
    cat <<'EOF'

%description
Meridian ships a headless CLI and a Slint-based desktop frontend.

%prep
%autosetup -n __TARBALL_PREFIX__

%build

%install
mkdir -p %{buildroot}
cp -a * %{buildroot}/

%files
/usr/bin/meridian
/usr/bin/meridian-ui
/usr/share/applications/io.github.arduano.meridian.desktop
/usr/share/doc/meridian/README.md
/usr/share/icons/hicolor/256x256/apps/io.github.arduano.meridian.png
EOF
} | sed "s/__TARBALL_PREFIX__/${TARBALL_PREFIX}/g" >"$spec_root/SPECS/$PACKAGE_NAME.spec"

docker run --rm \
    -v "$ROOT:$ROOT" \
    -w "$ROOT" \
    "$FEDORA_BUILD_IMAGE" \
    bash -lc "dnf install -y rpm-build >/dev/null && rpmbuild --define '_topdir $spec_root' -bb '$spec_root/SPECS/$PACKAGE_NAME.spec'"

artifact_path="$(find "$spec_root/RPMS" -name '*.rpm' -print -quit)"
test -n "$artifact_path"
printf '%s\n' "$artifact_path"
