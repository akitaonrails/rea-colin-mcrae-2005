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
| Look: Original | — | — | |
| Look: Recommended | — | — | |
| Look: Steam Deck | — | — | device check pending |
| `resolution` (example) | — | — | |
| `vsync` (example) | — | — | |

### Platforms

| Platform | Status | Notes |
|---|---|---|
| Windows 11 | — | |
| Windows 10 | — | |
| Proton, without our Linux helper (Steam, non-Steam game) | — | |
| Linux AppImage (portable Wine) | — | |
| Linux, `linux/cmr2005-play` with system Wine | — | |
| Steam Deck, Game Mode (controller, 1280×800, suspend/resume) | — | |
| Multi-monitor | — | |
| HiDPI / 4K | — | |
| Ultrawide | — | |

### Robustness

| Check | Status | Notes |
|---|---|---|
| Hostile paths (spaces, apostrophes, accents) for install, game and runtime | — | |
| `launcher.cfg` saved in a Windows code page | — | |
| Upgrade from an earlier release's files | — (no earlier release) | |
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

### Not verified yet

- Sound on real speakers beyond what the owner heard while playing.
- Sound output (it went to the null sink; no errors, nothing measured).
- Driving a stage (car physics at different frame rates).
- The game's own Quit, gamescope closed by itself.
- The settings window (only `--no-window` was run).
- Windows: how the launcher provides the registry values without admin rights.
