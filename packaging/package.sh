#!/bin/sh
# packaging/package.sh [--no-appimage] <package-folder>
# Builds the zip folder (e.g. dist/CMR2005Launcher-v0.1.0/), its .zip beside it,
# and the AppImage (portable Wine, sdk/packaging/appimage-wine.sh) in the
# same parent folder. CI calls it from launcher-ci.yml's package-command
# (rea-launcher-sdk docs/playbooks/ci-and-releases.md).
#
# Environment: REA_RELEASE_DIR holds CMR2005Launcher.exe (CI sets it; default
# target/i686-pc-windows-gnu/release); REA_CACHE caches pinned downloads.
# Everything the folder holds must be listed in packaging/package.allow.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
appimage=1
if [ "${1:-}" = --no-appimage ]; then appimage='' && shift; fi
[ $# -eq 1 ] || { echo "usage: $0 [--no-appimage] <package-folder>" >&2; exit 2; }
pkg=$1
name=$(basename "$pkg")
case $name in '' | *[!A-Za-z0-9._-]*) echo "package: bad package name '$name'" >&2; exit 2 ;; esac
dist=$(dirname "$pkg")
release=${REA_RELEASE_DIR:-$ROOT/target/i686-pc-windows-gnu/release}
[ -f "$release/CMR2005Launcher.exe" ] || { echo "package: no $release/CMR2005Launcher.exe (cargo build --release --target i686-pc-windows-gnu)" >&2; exit 1; }

rm -rf "$pkg"
mkdir -p "$pkg/linux" "$pkg/licenses"
cp "$release/CMR2005Launcher.exe" "$pkg/"
cp "$ROOT/README.md" "$ROOT/THIRD_PARTY.md" "$ROOT/CHANGELOG.md" \
  "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$pkg/"
cp "$ROOT/linux/cmr2005-play" "$ROOT/linux/README.md" "$pkg/linux/"
# The helper finds the SDK play library beside itself.
cp "$ROOT/sdk/rea-play.sh" "$pkg/linux/rea-play.sh"
chmod +x "$pkg/linux/cmr2005-play"
cp "$ROOT"/packaging/licenses/* "$pkg/licenses/"

# Third-party components from packaging/third-party.txt, each checked by
# SHA-256 (the archive, then the one file taken out of it).
mkdir -p "$pkg/components" "$pkg/licenses/innoextract"
if [ "${REA_STANDIN_COMPONENTS:-}" = 1 ]; then
  # packaging/tests: stand-ins, no network and no build.
  for f in components/d3d9.dll components/innoextract.exe components/rea_close_hook.dll \
    licenses/innoextract/innoextract.LICENSE.txt licenses/innoextract/Boost.LICENSE_1_0.txt \
    licenses/innoextract/liblzma.COPYING.txt licenses/innoextract/libstdc++.COPYING3.txt \
    licenses/innoextract/libstdc++.COPYING.RUNTIME.txt; do
    printf 'stand-in\n' > "$pkg/$f"
  done
else
  . "$ROOT/sdk/packaging/pins.sh"
  PIN_FILE=$ROOT/packaging/third-party.txt
  [ -n "${REA_CACHE:-}" ] && PIN_CACHE=$REA_CACHE/pins
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT

  tarball=$(pin_fetch dxvk "$work")
  tar -xzf "$tarball" -C "$work" dxvk-3.1.1/x32/d3d9.dll
  pin_verify "$work/dxvk-3.1.1/x32/d3d9.dll" "$(pin_sha dxvk-d3d9)"
  cp "$work/dxvk-3.1.1/x32/d3d9.dll" "$pkg/components/d3d9.dll"

  zip=$(pin_fetch innoextract "$work")
  unzip -q "$zip" innoextract.exe 'license/*' -d "$work/innoextract"
  pin_verify "$work/innoextract/innoextract.exe" "$(pin_sha innoextract-exe)"
  cp "$work/innoextract/innoextract.exe" "$pkg/components/innoextract.exe"
  cp "$work/innoextract/license/"*.txt "$pkg/licenses/innoextract/"

  # The SDK's close hook, built from the pinned SDK (a dependency, not a
  # workspace member; CI builds it next to the launcher).
  hook=$release/rea_close_hook.dll
  if [ ! -f "$hook" ]; then
    (cd "$ROOT" && cargo build --release --locked --target i686-pc-windows-gnu -p rea-launcher-close-hook)
    hook=$ROOT/target/i686-pc-windows-gnu/release/rea_close_hook.dll
  fi
  cp "$hook" "$pkg/components/rea_close_hook.dll"
fi

(cd "$dist" && rm -f "$name.zip" && zip -qr "$name.zip" "$name")

if [ -n "$appimage" ]; then
  sh "$ROOT/sdk/packaging/appimage-wine.sh" --app cmr2005 --exe CMR2005Launcher.exe \
    --env-prefix CMR2005 \
    --icon "$ROOT/packaging/appimage/cmr2005-launcher.png" \
    --desktop "$ROOT/packaging/appimage/cmr2005-launcher.desktop" \
    --payload components \
    --doc licenses \
    ${REA_CACHE:+--cache "$REA_CACHE"} \
    --name "$name" "$pkg" "$dist"
fi
