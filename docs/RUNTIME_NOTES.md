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

### Not verified yet

- Menu music (WMA) and sound in general (audio went to a null sink).
- Sound output (it went to the null sink; no errors, nothing measured).
- Driving a stage (car physics at different frame rates).
- Saves written to `SAVEGAME_PATH` (pointing at a folder other than `SG`).
- Controllers. The close hook with `PC_CMR5` (the launcher is not run yet).
- Windows: how the launcher provides the registry values without admin rights.
