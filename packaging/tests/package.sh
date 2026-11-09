#!/bin/sh
# packaging/package.sh builds a zip folder that passes packaging/package.allow
# (no AppImage, no network; a stand-in exe): sh packaging/tests/package.sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd -P)
T=$(mktemp -d "${TMPDIR:-/tmp}/cmr2005-package-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }

mkdir -p "$T/release"
printf 'MZ stand-in\n' > "$T/release/CMR2005Launcher.exe"
if REA_STANDIN_COMPONENTS=1 REA_RELEASE_DIR=$T/release sh "$ROOT/packaging/package.sh" --no-appimage \
  "$T/dist/CMR2005Launcher-v0.0.0" > "$T/out" 2>&1; then
  ok "package.sh builds the folder and zip"
else
  fail "package.sh: $(cat "$T/out")"
fi
for p in "$T/dist/CMR2005Launcher-v0.0.0" "$T/dist/CMR2005Launcher-v0.0.0.zip"; do
  if sh "$ROOT/sdk/packaging/smoke-package.sh" "$p" "$ROOT/packaging/package.allow" > "$T/out" 2>&1; then
    ok "allowlist passes: $(basename "$p")"
  else
    fail "allowlist: $(basename "$p"): $(cat "$T/out")"
  fi
done
# innoextract (GOG's installer is unpacked with it, never run) ships in the
# zip with its licences, and in the AppImage's payload.
for f in components/innoextract.exe licenses/innoextract/innoextract.LICENSE.txt \
  licenses/innoextract/Boost.LICENSE_1_0.txt licenses/innoextract/liblzma.COPYING.txt; do
  if [ -f "$T/dist/CMR2005Launcher-v0.0.0/$f" ] \
    && unzip -l "$T/dist/CMR2005Launcher-v0.0.0.zip" "CMR2005Launcher-v0.0.0/$f" > /dev/null 2>&1; then
    ok "folder and zip hold $f"
  else
    fail "$f is not in the folder and the zip"
  fi
done
if grep -q 'innoextract_bundle "$pkg"' "$ROOT/packaging/package.sh" \
  && grep -q -- '--payload components' "$ROOT/packaging/package.sh"; then
  ok "innoextract comes from the SDK's innoextract_bundle and rides in the AppImage payload"
else
  fail "package.sh must call innoextract_bundle and pass --payload components to appimage-wine.sh"
fi
if grep -q '^innoextract-exe - ' "$ROOT/packaging/third-party.txt" \
  && sh -c ". '$ROOT/sdk/packaging/innoextract.sh'; innoextract_pins" | grep -v '^#' \
  | while read -r line; do grep -qxF "$line" "$ROOT/packaging/third-party.txt" || exit 1; done; then
  ok "third-party.txt pins innoextract as the SDK does"
else
  fail "third-party.txt's innoextract pins differ from sdk/packaging/innoextract.sh"
fi

if REA_STANDIN_COMPONENTS=1 REA_RELEASE_DIR=$T/nowhere sh "$ROOT/packaging/package.sh" --no-appimage "$T/dist/x" > /dev/null 2>&1; then
  fail "a missing exe must fail"
else
  ok "a missing exe fails"
fi

if [ "$fails" = 0 ]; then echo "all passed"; exit 0; fi
echo "$fails failed"; exit 1
