# Runtime notes: Colin McRae Rally 2005

What was verified in the real game, how, and what was not. An option
reaches the settings window only after its effect has been seen here
(rea-launcher-sdk `docs/STANDARDS.md`, "No unverified options").

Runs are headless unless a line says otherwise: `gamescope --backend
headless` with a PipeWire null sink, a throwaway Wine prefix under `.work/`.

## Verification matrix

Status: ✅ verified · ⚠️ partly (see notes) · ❌ fails · — not yet checked.

### Editions

| Edition (SHA-256 prefix) | `--check` | Starts | Plays a session | Notes |
|---|---|---|---|---|
| | — | — | — | |
| unknown file | — (refused with SHA-256 and issue link) | — | — | |

### Options and looks

| Option / look | Window → `launcher.cfg` → game file | Effect seen in game | Notes |
|---|---|---|---|
| Look: Original | ✅ window card → `look = original`, `resolution = original` → only `INSTALL_PATH`/`SAVEGAME_PATH` written (v12) | — | |
| Look: Widescreen | ✅ `launcher.cfg` → window card selected (v10); `WIDTH`/`HEIGHT` 1920×1080 + `WIDESCREENDISPLAY` (v01–v05) | ✅ r08, e06 | |
| Look: Steam Deck | ⚠️ window card → selected, its values shown (v10) | — | device check pending |
| `resolution` | ✅ typed 1600x900 → `resolution = 1600x900` → `WIDTH`/`HEIGHT` 1600×900; back into the window (v10, v11) | ✅ r05, r08 | |
| `fps_limit` | ✅ value box 75 → `fps_limit = 75` → `dxvk.conf` `d3d9.maxFrameRate = 75` (v10, v11) | ✅ r15 (45 fps cap) | |
| `vsync` | ✅ switch off → `vsync = off` → `d3d9.presentInterval = 0` (v10, v11) | ✅ r16 (on: 59.9 fps) | |
| `advanced_particles` | ✅ switch off → `advanced_particles = off` → no `FORCEHT` (ui e2e test); on → `FORCEHT` (v1–v3b) | ✅ m6/m7, v3b: without `FORCEHT` the game's Graphics menu shows "Advanced particles Off" greyed out on this AMD Ryzen; with it the row is selectable and On; a stage loads and runs (s1) | |
| `wheel` | ✅ window chip → `wheel = on` → `REA_WHEEL_MODE=on`, `rea_wheel.ini` `mode = on` (ui e2e tests); the launcher's decision and the seed in `OPTIONS` (b1, d1) | ✅ b1: the game lists and binds "rea wheel"; d1: Auto without a wheel leaves the game its DirectInput | real wheel pending |
| `wheel_lock`, `wheel_deadzone`, `wheel_curve`, `wheel_invert_*`, `wheel_device`, `wheel_pedals` | ✅ window control → key → the runtime's `rea_wheel.ini` (ui e2e tests, cli tests) | — | need a real wheel |
| `wheel_ffb`, `wheel_ffb_invert` | ✅ window chip → key → `ffb_strength` (ui e2e tests) | ⚠️ c1: the game's four effects made on the fake wheel at 50 % | the force itself needs a real wheel |
| `wheel_log` | ✅ `launcher.cfg` → `log = on` → `rea_wheel.log` next to the game (b1, c1) | ✅ | |
| `game` | ✅ typed `Z:\…\extract\app` → `game = …`, "Found PC v1.1 (GOG)" (v12) | — | Browse… opens Wine's folder picker (v09) |

### Platforms

| Platform | Status | Notes |
|---|---|---|
| Windows 11 | — | |
| Windows 10 | — | |
| Proton, without our Linux helper (Steam, non-Steam game) | — | |
| Linux AppImage (portable Wine) | — | |
| Linux, `linux/cmr2005-play` with system Wine | ⚠️ headless gamescope only (h01–h02, v01–v12, w01–w11) | every way of quitting ends everything (Alt+F4 only with the mouse moving); play-time errors are dialogs; a second start exits 5 |
| Steam Deck, Game Mode (controller, 1280×800, suspend/resume) | — | the helper inside an existing gamescope (no nested one) ran the window and the game headless (v10–v12) |
| Multi-monitor | — | |
| HiDPI / 4K | — | |
| Ultrawide | — | |

### Robustness

| Check | Status | Notes |
|---|---|---|
| Hostile paths (spaces, apostrophes, accents) for install, game and runtime | — | |
| `launcher.cfg` saved in a Windows code page | — | |
| Upgrade from an earlier release's files | ✅ 0.2's unpacked folder (`.cmr2005-unpacked`) reused and the setting moved to it (2026-11-09) | |
| Gamepad-only use of the window | — | |

## Log

Dated entries: what ran, on what, what was seen.


### 2026-11-02: reproducing and fixing the startup crash (original game, no launcher yet)

**Setup.** GOG installer unpacked with `innoextract 1.9` into `.work/extract`
(local, not in Git), run from a reflinked copy in `.work/game`. Wine 11.21
(plain `wine`, WineD3D, no DXVK, no DLL overrides), gamescope 3.17.0
`--backend headless` at 1920×1080, a private PipeWire with one null sink
and WirePlumber's hardware monitors disabled, a throwaway prefix in
`.work/realgame/prefix`, finite timeouts inside the `gaming` distrobox.
Harness: `.work/realgame/bin/raw.sh` (adapted from CMR4's). RTX 5090.

| Run | Change | Result |
|---|---|---|
| r01 | fresh prefix, no registry | invalid: another project's gamescope took the same X display seconds later; Wine had no display |
| r02 | same, retried | **crash reproduced**: `Unhandled page fault on read access to 00000010 at address 0041DA27` |
| r03 | `WINEDEBUG=trace+file` | the game opens `C:\Program Files\Codemasters\Colin McRae Rally 2005\fonts\HelNu_10.dds` (missing): the hard-coded fallback path |
| r04 | `INSTALL_PATH` and `SAVEGAME_PATH` added under `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005` (32-bit view) | **no crash**; title screen "press ENTER to play" after 45 s, drawn 640×480 in the corner of the 1080p output |
| r05 | plus `WIDTH`=1920, `HEIGHT`=1080 | title at full 1920×1080; Enter → main menu with the 3D background |
| r06 | Enter through Challenges → Time Trial → Stages → name entry → nationality | menus work, no errors, about 150 fps (WineD3D present trace) |

The game wrote `LANGUAGE`=`E`, `WIDTH`=640, `SELECTION` and an empty
`CD_KEY` into the key on its first run. `SG` stayed empty (no save made yet).

After every run the process table was checked by command line and by
`WINEPREFIX`/`XDG_RUNTIME_DIR` in `/proc/*/environ` (match only). The one
leftover (r01's gamescope, after its child failed) was ended by its exact PID.
gamescope 3.17.0 segfaults on exit, as CMR4 recorded.

### 2026-11-02: stage, renderer, widescreen, quitting (original game, no launcher yet)

Same setup as above. `stage.sh` clears `SG`, presses Enter through
Challenges → Time Trial → Stages → name → nationality → car → stage, and
takes screenshots 10 s and 20 s into the stage (Australia, Subaru).

| Run | Change | Result |
|---|---|---|
| r07 | screenshots every 4 s from start | **intro videos play** (Codemasters, NVIDIA, Sound Blaster, GameSpy, attract) through Wine's GStreamer; the stage loads; `SG\OPTIONS` and `SG\BESTTIMES_GHOSTS` written |
| r08 | `WIDESCREENDISPLAY`, WineD3D, `WINEDEBUG=fps` | correct proportions at 1920×1080 (round dials, car no longer stretched); menus 130–260 fps, **the stage only about 20 fps** |
| r09/r10 | DXVK 3.1.1 `d3d9.dll`, `d3d9=n,b` | same picture; **about 1450 fps** in the stage (DXVK HUD) |
| r11 | 1280×800 (16:10), DXVK | runs (690 fps); proportions look close but not exact (see the widescreen analysis in `RESEARCH.md`) |
| r12 | 2560×1080 (21:9) | **stretched**: the widescreen view is fixed at 16:9 |
| r13 | Alt+F4 at the main menu (XTest) | **ignored**: game still running, screen dark |
| r14 | `WM_DELETE_WINDOW` to every `cmr5.exe` window | **ignored** |
| r15 | `dxvk.conf`: `d3d9.maxFrameRate = 45` | 44.9 fps in the stage |
| r16 | `dxvk.conf`: `d3d9.presentInterval = 1` | 59.9 fps (gamescope headless at 60 Hz) |

The stage timer kept wall-clock time at 20, 45 and 1450 fps (12.2–12.3 s
of stage time over 12–13 s of wall time between screenshots). Car physics
at different frame rates needs a person driving.

Files the game wrote in its own folder during these runs: only
`SG\OPTIONS` and `SG\BESTTIMES_GHOSTS` (checked with a marker file
before r11).

innoextract 1.9's Windows build unpacked the owner's installer under Wine
11.21 in 162 s (from the NAS), byte-identical to the Linux innoextract's output.

### 2026-11-02: the launcher end to end (package from `packaging/package.sh --no-appimage`)

`CMR2005Launcher.exe --no-window` from `.work/pkg/CMR2005Launcher-dev`
(release i686 build, DXVK 3.1.1 `d3d9.dll`, innoextract 1.9, the SDK's
close hook), plain `wine` 11.21 inside a headless gamescope 1920×1080,
throwaway prefixes `.work/realgame/prefix-e2e*`; harness `bin/launch.sh`.

| Run | What | Result |
|---|---|---|
| e01 | `--game` = the owner's GOG installer folder (NAS), fresh prefix | **unpacked** (GOG installer identified, then `PC v1.1 (GOG)`), runtime 5219 files (5207 hard-linked), registry written (HKLM 32-bit view), DXVK + per-program `d3d9` override, `WIDESCREENDISPLAY`; but the screen read 1024×768 and the game quit after 1 s (see e05) |
| e02–e04 | same prefix | the game runs (intro videos); 1920×1080 and 1024×576 both kept by the game |
| e05 | fresh prefix made by a headless `wineboot` | screen 1024×768 and **no 1024×576 mode listed: the game quits at once with code 0**. My harness had made the prefix with no display; the Linux helper never does. The launcher now falls back to a listed mode (`config::fit_mode`) and logs it |
| e06 | fresh prefix made inside gamescope, `--game` = an installed folder | screen 1920×1080, the stage at 1920×1080 widescreen; first start fine |
| e07 | close hook | `rea_close_hook.dll` mapped into `cmr5.exe` |
| e08 | `WM_SYSCOMMAND SC_CLOSE` posted to `PC_CMR5` (window on the main thread) | **game and launcher gone** |
| e11 | `WM_DELETE_WINDOW` (compositor close, as Super+W) | **game and launcher gone** |
| e06, e09 | Alt+F4 as keys (XTest, and `SendInput` from inside Wine) | **not seen by the game**: still running |

**Inference (not verified):** the game takes the keyboard through
DirectInput in exclusive mode, which keeps Alt+F4 from its window, so the
hook never sees the keys. On desktops where Alt+F4 is the compositor's
close (most Linux desktops, Hyprland's binding) it works as e11 shows. A
close hook that also polls the key state would cover the keys everywhere:
an SDK change (`rea-launcher-close-hook`), not done here.

After every run the process table was checked as above; nothing was left.

**Saves:** after e06–e11 the game's `OPTIONS` and `BESTTIMES_GHOSTS` were
in the launcher's `saves` folder (`SAVEGAME_PATH`), and the runtime copy
had no `SG` folder.

### 2026-11-02: `linux/cmr2005-play` with the real game

`helper.sh`: `linux/cmr2005-play --play` with `CMR2005_LAUNCHER` = the dev
package and `CMR2005_PREFIX` = `prefix-e2e2`, gamescope wrapped to
`--backend headless` on `PATH`, private PipeWire as above.

| Run | Action | Result |
|---|---|---|
| h01 | `cmr2005-play stop` with the game running | the helper chose the NVIDIA GPU and 1920×1080; **game and helper gone in 0.28 s**, nothing left |
| h02 | SIGINT to the helper's process group (Ctrl+C) | **game and helper gone in 0.40 s** (exit 143), nothing left |

### 2026-11-02: the AppImage (`packaging/package.sh`, portable Wine 11.20)

`APPIMAGE_EXTRACT_AND_RUN=1 CMR2005Launcher-dev-x86_64.AppImage --play --game <installed folder>`
through `helper.sh` (gamescope headless on `PATH`), fresh prefix `prefix-e2e3`.

| Run | Result |
|---|---|
| a01 | **failed**: the AppImage had no `components/` ("d3d9.dll and rea_close_hook.dll is missing"); fixed with `--payload components` in `package.sh` (and the message's grammar) |
| a02 | components found, runtime built (5207 linked), registry and DXVK set, the game ran at 1920×1080 with `WIDESCREENDISPLAY`; `stop`: **game and helper gone in 0.71 s**, nothing left |

gamescope 3.17.0 segfaulted on exit again (exit 139 of the helper in a01),
as CMR4 recorded; it happens after the game is gone.

### 2026-11-02: the owner's desktop (Hyprland), the AppImage

The first start in a new prefix: the settings window failed (exit 9,
eframe "failed to find a matching configuration for creating glutin
config"). The launcher had just written `UseEGL=N`, but Wine reads it when
a process starts, so that process still drew through EGL. The second start
opened the window. Fixed: after writing the value the launcher starts
itself again (single-instance lock released first). Checked in a fresh
headless prefix (w06): the log shows the restart and the window opens.

### 2026-11-02: the owner plays (Hyprland, RTX 5090, through gamescope)

- First AppImage start without gamescope on the host: after the unpack the
  game stopped with "GraphicsSS_SetupRender Adapter has no modes!". The
  primary monitor is portrait (the launcher saw 2160×3840); the game only
  enumerates the primary adapter. Through `cmr2005-play` with the gaming
  box's gamescope it ran. A Wine virtual desktop
  (`explorer /desktop=…`) is the candidate fix outside gamescope; the
  per-program `AppDefaults\cmr5.exe\Explorer\Desktop` key is ignored by
  Wine 11 (run r18: the game still opened its own X windows). Not done yet.
- **Played a full Finland stage at 4K** (resolution changed in the game's
  graphics menu) with a gamepad set up in the game's own controller menu.
  Recorded (showcase video in the README).
- **Reported:** about four or five freezes of roughly half a second during
  the stage; otherwise smooth at about 60 fps. Under investigation.

### 2026-11-02: the unpack window and the portrait main monitor

- **Unpack window** (e13, e14; fresh prefixes, headless gamescope): the
  first start from the installer shows the window with the photo, a
  progress bar and the time left ("793 MB of 3.6 GB, about 2 min left"
  after 40 s), then starts the game by itself. Closing the window (or
  Cancel) stops innoextract, removes the partial `game.unpacking` folder
  and ends the launcher; no game folder is left.
- **Portrait main monitor, no gamescope** (the owner's Hyprland desktop,
  the AppImage with `--play`, a visible window announced first): the
  launcher logged "the main monitor is portrait: the game runs in a
  3840x2160 Wine virtual desktop"; the game ran in the "CMR2005 - Wine
  Desktop" window on the landscape monitor, past the point where it had
  stopped with "Adapter has no modes". Closing that window
  (`WM_DELETE_WINDOW`) ended the game, Wine's explorer, the launcher and
  the helper within about 20 s.
- **Freezes reported by the owner** (about 0.5 s, five or six per stage,
  at the moment a co-driver call appears on screen, found in the owner's
  recording with ffmpeg `freezedetect`): **not reproduced** headless in
  three measured drives (MangoHud frame logs, 1080p and 4K, the owner's
  saved options; no frame over 100 ms while driving). Differences left:
  real audio output, the pad in use, OBS recording at the same time. Also
  seen: with the owner's options the game opens the pad's HID device once
  per frame (8,500 opens in 160 s), apparently re-acquiring it through
  DirectInput; it caused no stall in these runs. Candidates if it comes
  back: Wine's sound path for co-driver calls; Xidi for the pad.

### 2026-11-02: quitting, the settings window and play-time errors (dev package)

Release i686 build packaged with `packaging/package.sh --no-appimage`,
Wine 11.21 and gamescope 3.17.0 in the `gaming` distrobox, every run started
from `env -i`, a private PipeWire null sink (WirePlumber's device monitors
off in the test HOME), throwaway prefixes `.work/realgame/prefix-v1..3`, the
game from `innoextract` of the owner's GOG installer into `.work/extract`.
Harness: `.work/realgame/bin/vhelper.sh` (the helper's `--play` with its own
gamescope, made headless by a wrapper on `PATH`) and `vouter.sh` (the helper
inside one headless gamescope, as in Game Mode: it starts no gamescope of its
own). Input through XTest; evidence is screenshots, logs and the process table.
After every run the table was checked by environment (`WINEPREFIX`,
`XDG_RUNTIME_DIR`, match only), by recorded session and by name, zombies
included; none of these runs left anything. (A `wineserver` and `explorer.exe`
listed by name in v04–v08 belonged to `sdk/scripts/wine-test`'s prefix,
running at the same time.)

**Every way of quitting under `cmr2005-play`:**

| Run | How | Result |
|---|---|---|
| v01, v02 | the game's menu: Exit → Yes (main menu, one Left from Challenges) | the game exited with 0; launcher, helper and gamescope gone; nothing left |
| v10 | the same, the helper inside gamescope | game, launcher and session gone within about 1 s of Yes; helper exit 0 |
| v03 | TERM to the helper's gamescope (gamescope closed) | everything gone in 0.9 s (helper exit 139: gamescope's segfault on exit) |
| v04 | SIGINT to the helper's pid | everything gone in 0.6 s (exit 143) |
| v05 | SIGTERM to the helper's pid | everything gone in 0.5 s (exit 143) |
| v12 | `cmr2005-play stop`, the helper inside gamescope | everything gone within 1 s (exit 143) |

**The settings window in the real Wine run** (the helper's default command,
window then game):

| Run | What | Result |
|---|---|---|
| v09 | fresh prefix | the launcher restarts once for GLX (`UseEGL=N`), then the window opens (880×640); Play stays disabled until a game is found; Browse… opens Wine's "Browse for Folder"; closing the window: "settings window closed without Play; nothing started", no `launcher.cfg` written, nothing left |
| v10 | `launcher.cfg` with the game, Widescreen, 1920x1080, 0, on | shown in the window ("Found PC v1.1 (GOG)", Widescreen selected); Steam Deck card, then Resolution 1600x900, Frame-rate cap 75, VSync off; Play: `launcher.cfg` has `resolution = 1600x900`, `fps_limit = 75`, `vsync = off` (editing the picture clears `look`, as designed); the game started 16 s later with `WIDTH`/`HEIGHT` 1600×900, `WIDESCREENDISPLAY` and `dxvk.conf` `d3d9.maxFrameRate = 75`, `d3d9.presentInterval = 0` |
| v11 | reopened | the Picture tab shows 1600x900, 75 and VSync off; closed without Play, nothing left |
| v12 | fresh prefix, folder typed | `Z:\…\extract\app` typed into the field → "Found PC v1.1 (GOG)"; Original card; Play: `game`, `look = original`, `resolution = original`, `fps_limit = 0`, `vsync = off` saved; the game started |

**Play-time errors under `cmr2005-play --play`** (`launcher.cfg` naming a
missing folder, then a folder whose `cmr5.exe` is random bytes):

| Run | Build | Result |
|---|---|---|
| v06-old, v07-old | before the fix | **no dialog**: the launcher printed the error into `play.log` and ended, so the game simply never appeared |
| v06, v07 | the fix (`report`, see CHANGELOG; rerun with the committed build) | a dialog: "There is no folder Z:\…\no-such-folder." and the unknown-`cmr5.exe` refusal with its SHA-256 and the issue link; also in `play.log`; `cmr2005-play stop` while it shows ends everything |
| v08 | the fix, Enter on OK | launcher, helper and gamescope gone; nothing left |

### 2026-11-02: rea-launcher-sdk v0.4.1 (dev package)

Release i686 build of the `sdk-update/v0.4.1` branch packaged with
`packaging/package.sh --no-appimage`, Wine 11.21 and gamescope 3.17.0 in
the `gaming` distrobox, every run started from `env -i`, a private PipeWire
null sink (WirePlumber's device monitors off), one throwaway prefix
`.work/realgame/prefix-w1`, the game unpacked by the launcher from the
owner's GOG installer (read only, on the NAS). Harness:
`.work/realgame/bin/w.sh` (`vhelper.sh` with more steps), `wraces.sh`,
`waltf4.sh`, `wsecond.sh`; evidence in `.work/realgame/logs/w0*`–`w11*`.
After every run the process table was checked as before; runs that ended
the game left nothing (w07 below is the exception, cleaned up by the harness).

| Run | What | Result |
|---|---|---|
| w01 | fresh prefix, `cmr2005-play --play --game <the installer folder>` | the launcher wrote `UseEGL=N` and started itself again once (`gpu::prefer_glx_or_restart`), the unpack window ran, the game started (5219 files, 5207 linked) |
| w03 | `--look original`, a Time Trial stage | gamescope 640x480 (`REA_LOOK_SIZES`), the game 4:3 at 640x480; the stage runs (timer 00:32) |
| w04 | `--look widescreen` | gamescope and the game 1920x1080 with `WIDESCREENDISPLAY`; the stage runs |
| w05 | `--look steam_deck` | gamescope 1280x800, the game 1280x720 with `fps_limit 60`; the stage runs |
| w07 a | `CMR2005Launcher.exe --check` while the game runs | exit 5, "Colin McRae Rally 2005 is already running from this launcher…" printed, no dialog |
| w07 b | `CMR2005Launcher.exe --no-window` while the game runs | the same message as a dialog; OK → exit 5 |
| w07 c | `cmr2005-play --play` while the game runs | the helper says "already running", exit 5, nothing started |
| w07 d | the launcher killed (SIGKILL), the game left running, a new `--no-window` start | refused by `process::find_running` after the game folder check, before the runtime copy is touched; exit 5 |
| w08 | `launcher.cfg` naming a missing folder, `cmr2005-play --play` | dialog "There is no folder Z:\…\no-such-folder."; OK → launcher, helper and gamescope gone |
| w09, w10 | the settings window (the helper's default start) | titled "Colin McRae Rally 2005 Launcher"; Play tab as in `docs/images` (the SDK's light widget fills give the same resting colours as the removed local override: panel fill, ink text); closing it starts nothing |

**Quitting:**

| Run | How | Result |
|---|---|---|
| w03 | SIGTERM to the helper during the stage | everything gone in 0.5 s (exit 143) |
| w04 | `WM_DELETE_WINDOW` to `cmr5.exe` (compositor close) during the stage | "game exited with 0"; everything gone in 0.9 s |
| w05 | TERM to the helper's gamescope during the stage | everything gone in 0.9 s (exit 139: gamescope's segfault on exit) |
| w06 | the game's menu: Exit → Yes | the game gone 0.4 s after Yes, "game exited with 0"; nothing left |
| w01, w02 | Alt+F4 (XTest) at the intro videos and at the title screen, alone and held for 2 s | **ignored**: the game kept running |
| w02 | mouse moves alone | the game kept running |
| w02 | Alt+F4 held while the mouse moves | **the game ended** within 4 s ("game exited with 0"), launcher, helper and gamescope gone |

`close_on = alt_f4_state` reads Alt+F4 from the keyboard state when the
game's thread takes a message. With the exclusive DirectInput keyboard no
message arrives for the keys, so Alt+F4 alone is still not seen; any other
message (the mouse) lets the hook see it. Drafted for the SDK in
`CONTRIBUTIONS.md` (a timer in the hook).

**Found, then fixed (w07 d, w11):** in w07 d the harness killed every
process whose command line named the launcher: the launcher, but also the
session's inner script and gamescope. `cmr5.exe` was left running in the
prefix without a display. In real use only the launcher dies (a crash, a
task manager), and the session's inner script then ran `wineserver -k`,
taking the game with it, because the helper waited for the launcher, not
the game. `cmr2005-play` now sets `REA_GAME_PROCS=cmr5.exe`: the session
waits for the game, and ends the prefix after it.

| Run | What | Result |
|---|---|---|
| w11 | fresh prefix `prefix-w2`, the game at the title screen, SIGKILL to the launcher's two Wine processes only (found by name and prefix, the exe link Wine's preloader) | the game kept running in the session; 8 s later no launcher, gamescope up |
| w11 | a new `--no-window` start | the "already running" dialog (`process::find_running`); OK → exit 5; the game still running |
| w11 | `WM_DELETE_WINDOW` to `cmr5.exe` | game, inner script, helper and gamescope gone in 1.3 s; nothing left in the prefix |

**Not explained:** in w09 the game started without input a few seconds
after the settings window had been captured (as if Play was pressed);
w10, the same start, stayed idle for 20 s and closed without Play. A pad
on the host seen through the box's `/dev/input` is the likeliest cause;
not reproduced.

### 2026-11-05: rea-launcher-sdk v0.4.3, the close hook loaded with the game

To avoid antivirus false positives the launcher no longer installs the
close hook into the running game (`Running::message_hook`): the runtime's
`cmr5.exe`, rebuilt from the player's bytes on every start, imports
`rea_close_hook.dll` (`rea_close_hook::import_into`, a `.reahook` section),
and `REA_CLOSE_HOOK_ATTACH=1` lets the DLL install the hook itself.

Release i686 build packaged with `packaging/package.sh --no-appimage`,
Wine 11.21 and gamescope 3.17.0 (`--backend headless`) in the `gaming`
distrobox, started from `env -i`, a private PipeWire null sink, one
throwaway prefix `.work/realgame/prefix-h1`, the game unpacked by the
launcher from the owner's GOG installer (read only, on the NAS). Harness
`.work/realgame/bin/h.sh` (processes found by the helper's session and by
exact name only); evidence in `.work/realgame/logs/h01`, `h02`.

| Run | What | Result |
|---|---|---|
| h01 | fresh prefix, `cmr2005-play --play --game <the installer folder>` | unpacked (`PC v1.1 (GOG)`, 5219 files, 5207 linked), `runtime cmr5.exe: imports rea_close_hook.dll` in the log, the game reached the title screen; `rea_close_hook.dll` mapped into `cmr5.exe` from the runtime folder |
| h01 | `WM_DELETE_WINDOW` to `cmr5.exe` (compositor close) at the title screen | "game exited with 0"; game, launcher and helper gone in 3.9 s; nothing left in the session, no wineserver |
| h02 | second start, same prefix (the runtime exe already hooked) | the exe rebuilt from the player's bytes, unchanged; the hook mapped once; the title screen |
| h02 | Alt+F4 alone (XTest), then mouse moves alone | **ignored**, as with v0.4.2 (w02): the exclusive DirectInput keyboard |
| h02 | Alt+F4 held while the mouse moves | "game exited with 0"; everything gone in 4.3 s; nothing left |

The helper's exit 139 in both runs is gamescope 3.17.0's segfault on exit,
as before. The host test `runtime::tests::the_real_cmr5_exe_takes_the_hook_once`
passed against the unpacked GOG `cmr5.exe` (SHA-256 `a7ea2d33…`,
`CMR2005_GAME_DIR`): one import of the DLL, a second pass changes nothing,
the player's file untouched.

**Still to check on Windows:** the same quitting checks on a real Windows
10/11 install (the import path is the same loader, so no difference is
expected), and that the player's antivirus no longer flags the launcher.

### 2026-11-07: triage of issue #1 (high frame rates, particles, pads, EAX, saves)

The issue's claims were checked against the game's own files (read only:
the GOG installer's `readme.txt`, `Whole_E.Lng` and `cmr5.exe`) and the
launcher code; no game run was needed.

- **Advanced particles / EAX / Vsync / refresh rate:** the game's Advanced
  Options contain "Advanced particles", "Post processing effects",
  "Environmental Audio (EAX) 4.0", "Vsync" and "Refresh Rate" (strings in
  `Whole_E.Lng`), so these are the game's own settings, not launcher ones.
- **`FORCEHT`:** the switch sets the game's Hyper-Threading mode
  (`0xb6412b`, read at `0x588236`: 2 = forced HT, `DISABLEHT` = off), the
  `IntelHT` build's path; it is not a particle switch. The launcher does
  not pass it.
  **Corrected 2026-11-09:** that mode is exactly what gates Advanced
  particles (`0x5880b0`/`0x5880c0` test mode 2); without `FORCEHT` the
  option is greyed out on any CPU the game's 2004 detection does not call
  Hyper-Threading, AMD included (see the 2026-11-09 entry). The launcher
  now passes it.
- **Pads:** the game's Controllers menu calibrates deadzone and saturation
  and holds Vibration and Vibration Strength (same language file and
  `readme.txt` §IX). The launcher adds no mapping of its own yet.
- **Saves:** the launcher has no profile switch and no code path that
  deletes a save; its `saves` folder is per Windows user. The reported loss
  after a "profile change" was not reproduced and needs the exact steps.
  **Found 2026-11-09:** the "settings" are the game's Graphics options in
  the registry (`SELECTION`), which the game itself resets when it can't
  use the size it was given or finds another graphics card (see the
  2026-11-09 entry).
- **Widescreen cap:** the Widescreen look now sets `fps_limit = 60` (the
  value already seen in the game in w05); no new game behaviour.

Still unverified: whether capping at 60 fixes the reported driver
animations (the reporter's system is Windows, where the launcher's cap does
not apply), and the 4:3 shape of the intro videos (inferred from the
stretching report, not measured).

### 2026-11-09: GOG's installer through the SDK's installer module (v0.5.0)

Release i686 build; innoextract 1.9's Windows build
(`946bd06d…`); the owner's GOG installer, read-only from the NAS; system
Wine in the `gaming` box, headless (`sdk/scripts/wine-test`'s prefix, null
graphics driver, no display variables). Output under `.work/inst-0.3/`,
deleted after the checks.

| Check | Result |
|---|---|
| Linux `innoextract 1.9 --collisions=rename --include app` (reference) | 63 s, exit 0, 5,218 files, 3,564,462,813 bytes, no `#` collision copies |
| `tests/real_installer.rs` under `sdk/scripts/wine-test` (`source::detect` → `source::unpack`, no window: the SDK waits for innoextract) | ✅ 140 s; `cmr5.exe` PC v1.1 (GOG); no preferred/removed/dropped files; work folder gone |
| The launcher's folder against the reference, `diff -r` | ✅ identical; only `.rea-unpacked` (the setup's SHA-256) added |
| Same test: `.rea-unpacked` renamed back to 0.2's `.cmr2005-unpacked`, then `source::unpack` with no innoextract present | ✅ reused at once, marker renamed back, never unpacked again |
| `CMR2005Launcher.exe --check --game <unpacked folder>` | ✅ "Ready to play: cmr5.exe (PC v1.1 (GOG)) in …" |
| `CMR2005Launcher.exe --check --game <installer folder>` | ✅ "Found GOG offline installer (v1.1). The first Play unpacks the game from it once into C:\users\…\CMR2005Launcher\game (about 3.9 GB free space needed there)…"; nothing written |
| Play (`--no-window`) as a 0.2 player: `launcher.cfg` `game` = the installer folder, the unpacked folder in `%LOCALAPPDATA%\CMR2005Launcher\game` with the 0.2 marker | ✅ log: marker renamed, "unpacked game: …cmr5.exe (PC v1.1 (GOG))", "The game was unpacked into …\game. The game folder setting now points there; the installer files are no longer needed."; `launcher.cfg` now `game = C:\users\…\CMR2005Launcher\game`; runtime prepared and the game started (stopped by the test's timeout); the next `--check` reads it as an installed game |

Not checked here: the SDK unpack window on a real desktop (the launcher
falls back to waiting when no window opens, as above), and the unpack on
Windows and in the AppImage (unchanged innoextract; 0.2's runs above).

### 2026-11-09: Quiver feedback and issue #1 (settings reset, advanced particles)

The owner's GOG installer unpacked with innoextract into `.work/extract`
(read only from the NAS); the game run headless (gamescope 3.17.0
`--backend headless`, system Wine in the `gaming` box, a private
PipeWire null sink, every run from `env -i`, prefixes `.work/q/prefix*`).
Registry values read with `wine reg query` after each run. AMD Ryzen 9
7950X3D, RTX 5090.

**The game's own resets (plain `cmr5.exe`, no launcher, WineD3D):**

| Run | Before | Result |
|---|---|---|
| r1 | no `SELECTION`, `WIDTH`/`HEIGHT` 1920×1080 | ran at 640×480 (screenshot: picture in the top-left quarter); wrote `WIDTH` 640, `HEIGHT` 480 and `SELECTION` with its default option words and this card's ids (vendor `0x10de`, device `0x6cd` as WineD3D reports it) |
| r2 | r1's `SELECTION` with one option bit changed (bit 26 of the second word), 1920×1080 | kept: 1920×1080, the changed bit still there (only the refresh choice moved, `0x73b` → `0x73c`) |
| r3 | r2's values, but 2560×1440 (not a mode of the 1920×1080 screen) | **reset**: 640×480, `SELECTION` back to the defaults, the changed bit gone |

**The launcher (dev package, DXVK):**

| Run | What | Result |
|---|---|---|
| v1 | fresh prefix, `--no-window --linux`, defaults (Widescreen look) | arguments `WIDESCREENDISPLAY, FORCEHT`; first start at 640×480 (intro video stretched to the screen); the log says "On its first start the game always uses 640x480; from the next start it uses 1920x1080." |
| v2 | `SELECTION` with a changed option and another *device* id (`0x2b85`) | no reset: under DXVK the game sees the real device id `0x2b85` (WineD3D reported `0x6cd`) |
| v2b, v3a | same, with another *vendor* id too | the game reset to 640×480 and its defaults; after it exited the launcher wrote back the player's options with the card's ids the game found, and 1920×1080; log: "The game found another graphics card than last time, so it ran at 640x480 and reset its Graphics options. The launcher put your graphics options back. From the next start it uses 1920x1080." |
| v3b | next normal start | the game kept the restored options and 1920×1080 (registry unchanged after the run); its Graphics menu shows Advanced particles selectable and On |

**Advanced particles:** m6 (no `FORCEHT`): the Graphics menu's
"Advanced particles Off" is greyed and the cursor skips it. m7 (`FORCEHT`):
"Advanced particles On - Off", selectable. s1 (`FORCEHT`): Time Trial,
Spain stage 1, the stage started and ran (timer at 5.57 s); dust itself was
not driven through.

Not reproduced here: the reporter's exact trigger on Windows (the size
the launcher set was refused, or the game saw another adapter). Under
Wine the launcher only writes sizes the display lists (`fit_mode`), so the
"size refused" message was checked with the game alone (r3) and unit
tests.

### 2026-11-10: the game's key in HKCU (SDK v0.6.0, `REA_REGISTRY`)

Why: the launcher wrote `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005`,
falling back to the VirtualStore without admin rights; the game reads that
copy only while UAC registry virtualization is on for it
(rea-colin-mcrae#6). Now the values go to
`HKCU\Software\Codemasters\Colin McRae Rally 2005` and the close hook DLL
redirects the game's opens of the HKLM key there.

Setup: dev package (`packaging/package.sh --no-appimage`, the SDK v0.6.0
`rea_close_hook.dll`); the game from the owner's GOG installer, unpacked
by the launcher in an earlier run (`.work/realgame/prefix-h1`, read only);
gamescope 3.17 `--backend headless` 1920×1080, system Wine in the
`gaming` box, a private PipeWire null sink, every run from `env -i`;
**new prefixes booted inside gamescope with no key for the game in HKLM
(both views), HKCU or the VirtualStore**. Registry read with `wine reg
query` before and after each run; the game ended with SIGTERM after
45 s.

| Run | What | Result |
|---|---|---|
| a1 | fresh prefix, `--no-window`, defaults (Widescreen) | title screen with its text (fonts load: `INSTALL_PATH` read through the redirect); `rea_close_hook.dll` mapped. Afterwards **only the HKCU key exists**: the launcher's paths plus what the game wrote back itself (`LANGUAGE` E, `SELECTION`, `CD_KEY`, its first-start 640×480). HKLM (both views) and the VirtualStore: no key |
| a2 | next start | the game kept 1920×1080 (`WIDTH`/`HEIGHT` 0x780/0x438 after the run): the launcher's resolution reaches the game |
| a3 | HKCU `SELECTION` seeded with a changed option bit and another vendor id (as v2b) | the game reset to 640×480; the launcher wrote back the player's options with the card's ids into HKCU and 1920×1080; log: "The game found another graphics card than last time, … The launcher put your graphics options back." 0.4.0's restore works on the new key |
| b1 | **control**: the same runtime `cmr5.exe` (hook imported, `REA_CLOSE_HOOK_ATTACH=1`) without `REA_REGISTRY`, only the HKCU key present | `Unhandled page fault on read access to 00000010 at address 0041DA27`: the old crash, the game finds no `INSTALL_PATH` |
| c1 | carry-over: fresh prefix seeded with HKLM 32-bit `LANGUAGE` F, `INSTALL_PATH` `C:\old\cmr2005` and VirtualStore `LANGUAGE` G | log "the game's values carried over …", `LANGUAGE = string:"G", if missing`; HKCU got `LANGUAGE` G (the VirtualStore first, as the game read it) and the launcher's own `INSTALL_PATH`; the title says "Drücken Sie die Eingabetaste, um zu spielen". HKLM and the VirtualStore left as they were |
| c2 | carry-over from HKLM alone (`LANGUAGE` F) | HKCU `LANGUAGE` F; the old keys untouched (no screenshot: gamescopectl failed that run) |
| k1 | `--check` (prefix of a1) | exit 0: "Ready to play: …" and "The game's settings are in HKEY_CURRENT_USER\Software\Codemasters\Colin McRae Rally 2005 (no administrator rights needed)." |
| k2 | `--check` with `components\rea_close_hook.dll` removed | exit 1: "Not ready: the game would not find its settings …" naming the missing DLL and what to do |

Pending: **native Windows, unelevated** (the redirect with no VirtualStore
involved; a player of 0.4.0 keeps language and Graphics options).

### Not verified yet

- Sound on real speakers beyond what the owner heard while playing.
- Sound output (it went to the null sink; no errors, nothing measured).
- Driving a stage (car physics at different frame rates).
- The settings window on a real desktop (only headless gamescope so far), and with a gamepad.
- Windows, unelevated: the game reads its key from HKCU through the close
  hook's redirect (verified under Wine only, 2026-11-10).
- Windows: which of the game's resets a player hits (a refused size or
  another adapter, e.g. a hybrid iGPU + dGPU machine); the restore runs the
  same code there (registry read through the same key as the writes).
- Advanced particles while driving (dust, spray) and the snow visibility
  reported on GOG's forum for the Swedish stages.

### 2026-11-13: the wheel layer with the real game (headless, no wheel)

Release i686 build and the DLLs from `packaging/build-dlls.sh`
(`--profile dll`, the `layer-dlls` feature; `objdump -p`: the close hook
exports `DllMain`, `close_hook`, `rea_close_hook_attach`, the layer
`DllMain`, `rea_dinput_load` and the test-only `rea_dinput_fake_*`; the
launcher exe exports no `DllMain`), packaged with
`packaging/package.sh --no-appimage`. Wine 11.21 and gamescope 3.17.0
(`--backend headless`) in the `gaming` box, from `env -i`, a private
PipeWire null sink, the throwaway prefix `.work/realgame/prefix-h1` (the
game unpacked there from the owner's GOG installer). Harness
`.work/realgame/bin/wheel.sh` (through `cmr2005-play --play`) and
`bin/fake.sh` (the runtime's `cmr5.exe` started directly); processes found
by exact name and our own session only. `tools/opts.py` reads player 1's
controller fields from `OPTIONS` or from the game's copy at `0x8f9030`.
Evidence in `.work/realgame/logs/{a1-off,b1-on,c1-fake,d1-auto-altf4}`.
Every run: Return through the menus into a Time Trial stage, then a close.

| Run | What | Result |
|---|---|---|
| a1 | `wheel = off`, no `OPTIONS` yet | `rea_close_hook.dll` and `rea_dinput.dll` both mapped into `cmr5.exe`; the launcher logged "no OPTIONS yet … controls not seeded"; the stage ran; the game wrote `OPTIONS` (slot 2 empty: no controller); `WM_DELETE_WINDOW` ended everything in 3.6 s |
| b1 | `wheel = on`, `wheel_log = on` | launcher: "the virtual wheel seeded in slot 2 for player 1", `wheel-records` written. Layer log: mode on (`REA_WHEEL_MODE`), `CreateDevice` of the virtual wheel, `c_dfDIJoystick2`, `DIPROP_RANGE` on X, Y, Z, Rz by object ID, `DIPROP_AUTOCENTER`, then Spring, Damper, Sine and Constant effects. In the stage the game's own copy of `OPTIONS` still held the seed: ids 5245/1209, steering 34/34, accelerate 36, brake 37, calibration entries 7/9, autocentre off, player 1 on slot 2, byte 0 `0x13` (bit 0 set by the game at load); the re-saved file the same; the trailer unchanged. Closed in 3.6 s; "the virtual wheel's controls kept for the next start" |
| c1 | the same runtime started directly, `fake = wheel` appended to its `rea_wheel.ini` by hand (the launcher's next start wrote the file again without it) | the layer found the fake CSL DD, hid it from the game, took it exclusive in the background, and made the game's four effects on it at the 50 % strength (saturation 10000 → 5000); the seeded record held in memory as in b1; closed in 1 s |
| d1 | `wheel = auto`, no wheel attached, Alt+F4 held while the mouse moves | launcher: "the game doesn't see the virtual wheel", "no wheel this time; the controller record … back in slot 2"; layer: "off: the game gets its DirectInput untouched"; `OPTIONS` afterwards **byte for byte the file a1 left** (the seed fully undone); Alt+F4 ended everything in 3.95 s |

gamescope 3.17.0 still segfaults on exit (the helper's exit 139), as
before. Not seen: the slot-type word at `0xab848` the seed study expected
to read 2 for the wheel read 0 in the game's memory; the game kept the
record all the same, so the classification it stands for was not needed.

**Needs a real wheel** (phase 3 of the SDK's wheel plan): the force's
direction and feel at a given strength, the game's effect updates while
driving (the car stood still here, so no `SetParameters` were seen), the
autocentre value the game sets in a stage, steering and pedals reaching
the car, separate USB pedals, the steering lock's feel, a replug mid-stage,
and Windows 10/11 with the vendors' drivers.
