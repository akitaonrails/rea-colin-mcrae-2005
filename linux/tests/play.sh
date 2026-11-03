#!/bin/sh
# linux/cmr2005-play end to end with a recording wrapper instead of Wine and
# gamescope, in the repository layout (library in ../sdk) and the package
# layout (library beside the script): sh linux/tests/play.sh
set -u
HERE=$(cd "$(dirname "$0")" && pwd -P)
ROOT=$(cd "$HERE/../.." && pwd -P)
T=$(mktemp -d "${TMPDIR:-/tmp}/cmr2005-play-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
export HOME=$T
unset DISPLAY WAYLAND_DISPLAY XDG_CONFIG_HOME GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD \
  WINEDLLOVERRIDES CMR2005_LAUNCHER CMR2005_WAIT CMR2005_GPU 2>/dev/null
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
check() { if [ "$2" = "$3" ]; then ok "$1"; else fail "$1: got '$2', want '$3'"; fi; }
contains() { case "$2" in *"$3"*) ok "$1" ;; *) fail "$1: '$2' lacks '$3'" ;; esac; }

# The wrapper: gamescope is "installed"; the settings window exits with
# $T/rc; everything else is recorded in $T/ran.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in
  *"command -v gamescope"*) exit 0 ;;
  *"wineserver"*) exit 0 ;;
  *"--settings"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc" 2>/dev/null || echo 0)" ;;
esac
echo "$*" > "$T/ran"
FAKE
chmod +x "$T/fake-run"
export T CMR2005_RUN=$T/fake-run CMR2005_PREFIX=$T/prefix CMR2005_GPU=

mkdir -p "$T/repo/linux" "$T/repo/sdk" "$T/pkg/linux"
cp "$ROOT/linux/cmr2005-play" "$T/repo/linux/"
cp "$ROOT/sdk/rea-play.sh" "$T/repo/sdk/"
cp "$ROOT/linux/cmr2005-play" "$ROOT/sdk/rea-play.sh" "$T/pkg/linux/"
: > "$T/repo/CMR2005Launcher.exe"
: > "$T/pkg/CMR2005Launcher.exe"

for layout in repo pkg; do
  play=$T/$layout/linux/cmr2005-play
  check "[$layout] status" "$(sh "$play" status)" idle
  rm -f "$T/ran" "$T/settings"
  echo 0 > "$T/rc"
  sh "$play" > "$T/out" 2>&1; rc=$?
  check "[$layout] window, then the game: exit 0" "$rc" 0
  contains "[$layout] the window opens through the helper" "$(cat "$T/settings" 2>/dev/null)" "CMR2005Launcher.exe --settings --linux"
  contains "[$layout] the game runs in gamescope" "$(cat "$T/ran" 2>/dev/null)" "gamescope -W 1920 -H 1080"
  contains "[$layout] without the window" "$(cat "$T/ran" 2>/dev/null)" "CMR2005Launcher.exe --no-window"
  # A launcher that crashes or is killed must not take the game with it,
  # and the session ends only when the game does (RUNTIME_NOTES w07 d).
  contains "[$layout] the session waits for the game itself" "$(cat "$T/ran" 2>/dev/null)" "REA_GAME_PROCS=cmr5.exe "
  rm -f "$T/ran"
  echo 3 > "$T/rc"
  sh "$play" > "$T/out" 2>&1; rc=$?
  check "[$layout] window closed without Play: nothing starts" "$(cat "$T/ran" 2>/dev/null)" ""
  check "[$layout] stop" "$(sh "$play" stop)" stopped
done

echo "# the launcher killed while the game runs: the session waits for the game"
INNER=$T/.config/cmr2005-launcher/inner.sh
mkdir -p "$T/fake" "$T/proc/4242"
printf '4242 (cmr5.exe) S 1 4242 4242\n' > "$T/proc/4242/stat"
printf 'WINEPREFIX=%s\0' "$T/prefix" > "$T/proc/4242/environ"
cat > "$T/fake/wine" <<'W'
#!/bin/sh
echo "launcher killed" >> "$T/order"; exit 137
W
cat > "$T/fake/pgrep" <<'P'
#!/bin/sh
[ "$2" = cmr5.exe ] && [ -e "$T/alive" ] && echo 4242
exit 0
P
cat > "$T/fake/wineserver" <<'S'
#!/bin/sh
echo "wineserver $*" >> "$T/order"
S
chmod +x "$T/fake/wine" "$T/fake/pgrep" "$T/fake/wineserver"
rm -f "$T/order"; : > "$T/alive"
( sleep 2; rm -f "$T/alive"; echo "game ended" >> "$T/order" ) &
# The game's process names exactly as cmr2005-play hands them to the session.
rm -f "$T/ran"; sh "$T/repo/linux/cmr2005-play" --play > /dev/null 2>&1
sh "$T/repo/linux/cmr2005-play" stop > /dev/null 2>&1
procs=$(sed -n 's/.* REA_GAME_PROCS=\([^ ]*\) .*/\1/p' "$T/ran" 2>/dev/null)
PATH=$T/fake:$PATH REA_TEST=1 REA_PROC=$T/proc REA_GAME_PROCS=$procs REA_GAME_SETTLE=0 \
  REA_GAME_PREFIX=$T/prefix sh "$INNER" CMR2005Launcher.exe --no-window
wait
check "the prefix is ended only after the game" "$(tr '\n' ';' < "$T/order")" "launcher killed;game ended;wineserver -k;"

echo "# tests source the script: REA_FUNCTIONS_ONLY=1 + REA_PLAY_HERE"
got=$(REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$T/repo/linux sh -c '
  . "$1"
  echo "app=$REA_APP exe=$REA_EXE cfg_appdata=$REA_CFG_APPDATA"
  echo "dropped=${REA_FUNCTIONS_ONLY-unset}/${REA_PLAY_HERE-unset}"' sh "$T/repo/linux/cmr2005-play" 2>&1)
contains "sourced: configured" "$got" "app=cmr2005-launcher exe=$T/repo/CMR2005Launcher.exe cfg_appdata=CMR2005Launcher"
contains "sourced: both variables dropped once loaded" "$got" "dropped=unset/unset"

echo "# gamescope's size follows the look and the resolution, as the launcher's does"
CFGDIR=$T/prefix/drive_c/users/me/AppData/Local/CMR2005Launcher
mkdir -p "$CFGDIR"
CFG=$CFGDIR/launcher.cfg
size() { rm -f "$T/ran"; USER=me sh "$T/repo/linux/cmr2005-play" --play "$@" > /dev/null 2>&1; cat "$T/ran" 2>/dev/null; }
printf 'look = steam_deck\n' > "$CFG"
contains "look steam_deck, no resolution: the Deck's screen" "$(size)" "gamescope -W 1280 -H 800 -w 1280 -h 800"
printf 'look = original\n' > "$CFG"
contains "look original: the game's own 640x480" "$(size)" "gamescope -W 640 -H 480"
printf 'look = widescreen\n' > "$CFG"
contains "look widescreen, unknown screen: 1920x1080" "$(size)" "gamescope -W 1920 -H 1080"
printf 'look = widescreen\nresolution = 1280x720\n' > "$CFG"
contains "the file's own resolution wins over its look" "$(size)" "gamescope -W 1280 -H 720"
contains "--look wins over the file's resolution" "$(size --look steam_deck)" "gamescope -W 1280 -H 800"
contains "--resolution wins over --look" "$(size --look steam_deck --resolution 2560x1440)" "gamescope -W 2560 -H 1440"
contains "the flags still reach the launcher" "$(size --look steam_deck)" "--no-window --look steam_deck"
rm -f "$CFG"
contains "no launcher.cfg: the first start's Widescreen look" "$(size)" "gamescope -W 1920 -H 1080"
printf 'game = C:\\CMR\n' > "$CFG"
contains "launcher.cfg without a look: Widescreen too" "$(size)" "gamescope -W 1920 -H 1080"
echo "# on a 2560x1440 screen (xrandr's current mode)"
mkdir -p "$T/bin"
cat > "$T/bin/xrandr" <<'XR'
#!/bin/sh
echo "Screen 0: minimum 8 x 8, current 2560 x 1440, maximum 32767 x 32767"
echo "   2560x1440     59.95*+"
XR
chmod +x "$T/bin/xrandr"
wide() { PATH=$T/bin:$PATH size "$@"; }
rm -f "$CFG"
contains "no launcher.cfg: Widescreen fills the screen" "$(wide)" "gamescope -W 2560 -H 1440"
printf 'look = widescreen\n' > "$CFG"
contains "look widescreen: the screen" "$(wide)" "gamescope -W 2560 -H 1440"
contains "--look original on that screen: 640x480" "$(wide --look original)" "gamescope -W 640 -H 480"
printf 'look = steam_deck\nresolution = original\n' > "$CFG"
contains "resolution original: the game's 640x480" "$(wide)" "gamescope -W 640 -H 480"
contains "--resolution for one run" "$(wide --resolution 1600x900)" "gamescope -W 1600 -H 900"
printf 'game = C:\\CMR\nfps_limit = 75\n' > "$CFG"
contains "launcher.cfg without a look or resolution: the screen" "$(wide)" "gamescope -W 2560 -H 1440"
sh "$T/repo/linux/cmr2005-play" stop > /dev/null 2>&1

rm -f "$T/repo/CMR2005Launcher.exe"
out=$(sh "$T/repo/linux/cmr2005-play" --play 2>&1); rc=$?
check "no launcher: exit 1" "$rc" 1
contains "no launcher: names the variable" "$out" "CMR2005_LAUNCHER"

if [ "$fails" = 0 ]; then echo "all passed"; exit 0; fi
echo "$fails failed"; exit 1
