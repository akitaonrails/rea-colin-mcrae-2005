#!/bin/sh
# packaging/build-dlls.sh: builds the SDK's close hook and wheel layer DLLs
# for components/, into target/i686-pc-windows-gnu/dll/.
#
# Both crates are dependencies, not workspace members, so their `dll`
# feature (DllMain and the exports) is turned on through this launcher's
# `layer-dlls` feature; --lib builds only libraries, so the launcher exe, which
# links both crates and can hold neither DllMain, is never linked with it.
# The `dll` profile unwinds on panic: the DLLs' guards then keep a panic
# from ending the game.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
exec cargo build --locked --profile dll --target "${RUST_TARGET:-i686-pc-windows-gnu}" --lib \
  -p cmr2005-launcher -p rea-launcher-close-hook -p rea-launcher-dinput \
  --features cmr2005-launcher/layer-dlls "$@"
