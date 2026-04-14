#!/usr/bin/env bash
set -euo pipefail

write_fixture_midi() {
    local destination="$1"
    mkdir -p "$(dirname "$destination")"
    printf '\x4d\x54\x68\x64\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60\x4d\x54\x72\x6b\x00\x00\x00\x1b\x00\xff\x51\x03\x07\xa1\x20\x00\x90\x3c\x64\x30\x90\x40\x64\x30\x80\x3c\x40\x30\x80\x40\x40\x00\xff\x2f\x00' >"$destination"
}

run_smoke() {
    local bin_dir="$1"
    local tmp_dir="${2:-/tmp/meridian-smoke}"

    mkdir -p "$tmp_dir"
    write_fixture_midi "$tmp_dir/fixture.mid"

    PATH="$bin_dir:$PATH" meridian --help >/dev/null
    PATH="$bin_dir:$PATH" meridian-ui --help >/dev/null
    PATH="$bin_dir:$PATH" meridian debug piano-trail-classic-geometry --width 64 --height 64 >"$tmp_dir/geometry.json"
    test -s "$tmp_dir/geometry.json"

    echo "smoke ok: $bin_dir"
}
