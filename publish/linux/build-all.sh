#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"$root/archive/build.sh"
"$root/deb/build.sh"
"$root/rpm/build.sh"
"$root/appimage/build.sh"
