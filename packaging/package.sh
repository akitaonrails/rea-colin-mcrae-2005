#!/bin/sh
# packaging/package.sh [--no-appimage] <package-folder>
# Builds the zip folder (e.g. dist/CMR2005Launcher-v0.1.0/), its .zip beside it,
# and the AppImage (portable Wine, sdk/packaging/appimage-wine.sh) in the
# same parent folder. CI calls it from launcher-ci.yml's package-command
# (rea-launcher-sdk docs/playbooks/ci-and-releases.md).
#
# Environment: REA_RELEASE_DIR holds CMR2005Launcher.exe (CI sets it; default
# target/i686-pc-windows-gnu/release); REA_DLL_DIR holds the close hook and
# wheel layer DLLs (packaging/build-dlls.sh; default
# target/i686-pc-windows-gnu/dll); REA_CACHE caches pinned downloads.
# Needs unzip, zip and cabextract (the VC++ runtime Xidi needs comes out of
# Microsoft's installer).
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
# SHA-256 (the archive, then the one file taken out of it). innoextract
# comes from the SDK's sdk/packaging/innoextract.sh.
# Xidi's DLLs and the VC++ runtime DLLs Xidi.32.dll imports.
XIDI_FILES="dinput8.dll Xidi.32.dll"
VC_DLLS="msvcp140 msvcp140_atomic_wait concrt140 vcruntime140"
XIDI_LICENSES="LICENSE README.md ThirdParty/Boost_LICENSE ThirdParty/Hookshot_LICENSE ThirdParty/XstdBitSet_LICENSE"
mkdir -p "$pkg/components" "$pkg/licenses/innoextract" "$pkg/licenses/Xidi/ThirdParty"
if [ "${REA_STANDIN_COMPONENTS:-}" = 1 ]; then
  # packaging/tests: stand-ins, no network and no build.
  for f in components/d3d9.dll components/innoextract.exe components/rea_close_hook.dll \
    components/rea_dinput.dll licenses/innoextract/innoextract.LICENSE.txt licenses/innoextract/Boost.LICENSE_1_0.txt \
    licenses/innoextract/liblzma.COPYING.txt licenses/innoextract/libstdc++.COPYING3.txt \
    licenses/innoextract/libstdc++.COPYING.RUNTIME.txt; do
    printf 'stand-in\n' > "$pkg/$f"
  done
  for f in $XIDI_FILES; do printf 'stand-in\n' > "$pkg/components/$f"; done
  for d in $VC_DLLS; do printf 'stand-in\n' > "$pkg/components/$d.dll"; done
  for f in $XIDI_LICENSES; do printf 'stand-in\n' > "$pkg/licenses/Xidi/$f"; done
else
  . "$ROOT/sdk/packaging/pins.sh"
  . "$ROOT/sdk/packaging/innoextract.sh"
  PIN_FILE=$ROOT/packaging/third-party.txt
  [ -n "${REA_CACHE:-}" ] && PIN_CACHE=$REA_CACHE/pins
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT

  tarball=$(pin_fetch dxvk "$work")
  tar -xzf "$tarball" -C "$work" dxvk-3.1.1/x32/d3d9.dll
  pin_verify "$work/dxvk-3.1.1/x32/d3d9.dll" "$(pin_sha dxvk-d3d9)"
  cp "$work/dxvk-3.1.1/x32/d3d9.dll" "$pkg/components/d3d9.dll"

  # innoextract 1.9, which unpacks GOG's installer (never runs it):
  # components/innoextract.exe and licenses/innoextract/*.txt, in the zip
  # and (below, --payload components) the AppImage.
  innoextract_bundle "$pkg" "$work"

  # Xidi 5.0.0 (the gamepad option): the Win32 wrapper and main binary,
  # each checked against its pin, and its licence texts.
  xidi=$(pin_fetch xidi "$work")
  mkdir -p "$work/xidi"
  unzip -q "$xidi" 'Xidi-v5.0.0/Win32/dinput8.dll' 'Xidi-v5.0.0/Win32/Xidi.32.dll' \
    'Xidi-v5.0.0/LICENSE' 'Xidi-v5.0.0/README.md' 'Xidi-v5.0.0/ThirdParty/*' -d "$work/xidi"
  pin_verify "$work/xidi/Xidi-v5.0.0/Win32/dinput8.dll" "$(pin_sha xidi-dinput8)"
  pin_verify "$work/xidi/Xidi-v5.0.0/Win32/Xidi.32.dll" "$(pin_sha xidi-main)"
  for f in $XIDI_FILES; do cp "$work/xidi/Xidi-v5.0.0/Win32/$f" "$pkg/components/$f"; done
  for f in $XIDI_LICENSES; do cp "$work/xidi/Xidi-v5.0.0/$f" "$pkg/licenses/Xidi/$f"; done

  # The VC++ 2015-2022 runtime (x86) Xidi.32.dll imports, out of
  # Microsoft's installer: the cab that holds vcruntime140.dll_x86.
  vcredist=$(pin_fetch vcredist "$work")
  cabextract -q -d "$work/vc" "$vcredist"
  for cab in "$work"/vc/a*; do
    if cabextract -l "$cab" 2>/dev/null | grep -q 'vcruntime140.dll_x86'; then
      for d in $VC_DLLS; do cabextract -q -d "$work/vcdll" -F "${d}.dll_x86" "$cab"; done
      break
    fi
  done
  for d in $VC_DLLS; do
    pin_verify "$work/vcdll/${d}.dll_x86" "$(pin_sha "vc-$d")"
    cp "$work/vcdll/${d}.dll_x86" "$pkg/components/$d.dll"
  done

  # The SDK's close hook and wheel layer, built from the pinned SDK by
  # packaging/build-dlls.sh (CI's extra-build) into REA_DLL_DIR.
  dlls=${REA_DLL_DIR:-$ROOT/target/i686-pc-windows-gnu/dll}
  for dll in rea_close_hook.dll rea_dinput.dll; do
    [ -f "$dlls/$dll" ] || sh "$ROOT/packaging/build-dlls.sh"
    cp "$dlls/$dll" "$pkg/components/$dll"
  done
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
