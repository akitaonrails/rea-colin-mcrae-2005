# Changelog

All notable changes to the Colin McRae Rally 2005 launcher. Versions are git tags
(`vX.Y.Z`) matching the `cmr2005-launcher` crate version.

## [Unreleased]

### Fixed

- **No administrator rights needed for the game's settings, on any
  Windows.** The launcher used to write the game's registry key under
  `HKEY_LOCAL_MACHINE`; without administrator rights Windows put it in
  a per-user copy (the VirtualStore) that the game is not guaranteed to
  see, and then the game can't find its files and crashes at start while
  `--check` said ready (the same problem as Colin McRae Rally's issue
  #6). The game's settings now live in
  `HKEY_CURRENT_USER\Software\Codemasters\Colin McRae Rally 2005`, and
  `rea_close_hook.dll`, which the game already loads, shows them to the
  game where it looks for them. Your language, Graphics options and
  screen size are carried over from where the game kept them before.
  The launcher reads the values back after writing them: Play and
  `--check` stop with what is wrong instead of starting a game that
  can't run. `--check` also says where the settings are, and fails
  when `rea_close_hook.dll` is missing or the game's exe can't load it.
- Restoring the Graphics options after the game resets them (0.4.0)
  works on the new key.

## [0.4.0] - 2026-11-09

From a player's report on Quiver and issue #1.

### Added

- **Advanced particles** (Picture tab, `advanced_particles`,
  `--advanced-particles`), on by default: the launcher starts the game
  with its `FORCEHT` switch, so the game's own Advanced particles option
  (Options, Graphics) is available on AMD and on Intel processors newer
  than 2004. Without it the game greys the option out. Turn it off here or
  in the game; heavy snow on the Swedish stages can hide the road with it
  on.

### Fixed

- Graphics options lost after changing the resolution: when the game
  can't use the picture size it is given, or finds a graphics card other
  than last time (another GPU, a driver update, DXVK in or out), it drops
  to 640×480 and resets every Graphics option to its defaults. The
  launcher now notices this after the game exits, puts your graphics
  options and the last size back, and says which happened. When the size
  was the cause, it also shows a message asking for another Resolution
  or the Original look.
- `launcher.log` explains the game's very first start, which always runs
  at 640×480 (and the menus look squeezed in the Widescreen look); the
  size you chose applies from the second start.

## [0.3.0] - 2026-11-09

### Added

- Once the game is unpacked from GOG's installer, the **Game folder**
  setting points at the unpacked folder (`launcher.cfg`), so later starts
  find an installed game and the installer files are no longer needed;
  `launcher.log` says so. A folder given with `--game` is never saved
  over: the log names the unpacked folder instead. Players who unpacked
  with 0.2 get the same on their next start, without unpacking again.
- `--check` on the installer folder says what the first Play does, where
  the game is unpacked and how much free space it needs (or that it is
  unpacked already); it never unpacks.
- The settings window's note under the game folder says the same for the
  installer, and for the unpacked folder that the installer files are no
  longer needed.
- Any other Inno Setup installer in the game folder is left alone and
  reported with its SHA-256, like an unknown `setup_colin2005.exe`.

### Changed

- Updated rea-launcher-sdk to v0.5.0. The installer unpacking, its
  progress window and innoextract's packaging are now the SDK's (they
  were moved there from this launcher); the behaviour is the same. The
  unpack window's button is now **Stop unpacking** (also Escape and the
  pad's B), and the free-space check asks for the game plus a tenth
  (about 3.9 GB). The unpacked folder's marker is now `.rea-unpacked`;
  the 0.2 marker `.cmr2005-unpacked` is renamed on the next start.
- innoextract is checked at start like the other components when the game
  folder holds the installer.

## [0.2.3] - 2026-11-07

### Changed

- Updated rea-launcher-sdk to v0.4.4: the AppImage's portable Wine is
  now 11.21, which fixes the startup crash on machines that cannot
  reserve ~64 GiB of address space (16 GB handhelds).
- The Widescreen look now caps the game at 60 fps (VSync stays on), which
  keeps its frame-rate-timed animations steady; the Picture tab's
  Frame-rate cap raises it if wanted. As before, the cap works under Wine
  and Proton only.
- The README says where the game's own options live: advanced particles,
  post processing effects, texture and shadow settings, EAX 4.0, Vsync and
  refresh rate, and controller calibration and vibration. It also says
  how to cap animations at 60 fps on Windows, and that saves are per Windows
  user.

## [0.2.2] - 2026-11-05

To avoid antivirus false positives: a player's antivirus flagged another
rea launcher on Windows, so this launcher no longer looks like the
programs those checks are built to catch.

### Changed

- Updated rea-launcher-sdk to v0.4.3.
- `CMR2005Launcher.exe` now says what it is: its name, version, publisher
  and icon show in Explorer's Properties and in the task manager.
  Antivirus programs are less likely to flag a program that does.
- The launcher no longer reaches into the running game to make Alt+F4 and
  closing the window work. Instead, the game's private copy loads the
  launcher's `rea_close_hook.dll` itself when it starts. Your own game
  files are still never changed. Quitting works as before: closing the
  window or the game's own Exit end it, and Alt+F4 does while the mouse
  moves (the game keeps the keyboard to itself).

## [0.2.1] - 2026-11-03

From a native Windows 10 test of the launchers (rea-launcher-sdk#1).

### Changed

- Updated rea-launcher-sdk to v0.4.2.
- The README says what to do when Windows SmartScreen stops the first
  start ("Windows protected your PC": More info, then Run anyway), and
  lists every registry entry the launcher writes, with how to remove them
  when you uninstall.
- New releases of DXVK and innoextract are now watched every week, so
  the launcher hears of them.

### Fixed

- `--check` refusals and every error are now printed to standard output,
  like the other rea launchers. Before, they went to standard error only,
  so `CMR2005Launcher.exe --check > report.txt` gave an empty file.

## [0.2.0] - 2026-11-03

### Changed

- Updated rea-launcher-sdk to v0.4.1.
- Starting the launcher a second time while the game runs now says so
  and exits with code 5. Before, it quietly exited with 0. If the
  settings window is open, it comes to the front as before.
- The settings window is titled "Colin McRae Rally 2005 Launcher", so it
  can't be mistaken for the game's own window.

### Fixed

- Linux (`cmr2005-play`, the AppImage): if the launcher crashes or is
  closed by force while you play, the game keeps running, and the session
  ends when the game does. Before, the game was ended with it.
- If the launcher was closed by force while the game kept running, the
  next start no longer prepares the game's files under the running game.
  It says the game is still running instead.
- Alt+F4 in the game: the launcher now reads it from the keyboard, which
  the game keeps to itself. The game only notices it when something else
  happens at the same moment, such as the mouse moving, so with only the
  keyboard Alt+F4 still does nothing. Closing the window and the game's
  own Exit work as before.
- Under `cmr2005-play`, the AppImage and Proton, an error when Play
  starts the game (the game folder is gone, an unknown `cmr5.exe`) is now
  shown in a dialog. Before, it only went to `play.log` and the game
  simply never appeared. It is still printed for the log as well
  (rea-launcher-sdk's `report` rule).

## [0.1.0] - 2026-11-02

### Added

- Scaffold from rea-launcher-sdk v0.3.3 (`scripts/new-launcher`): edition
  table and unknown-edition refusal, option table (`look`, `resolution`,
  `vsync` as examples), settings window, `--check`, `--no-window`, the
  Linux helper, packaging and CI.
- Fixes the startup crash at `0x41DA27` (the "Wine dead-end"): every
  launch writes the game's `INSTALL_PATH` and `SAVEGAME_PATH` registry
  values (HKLM 32-bit view, or the per-user VirtualStore without admin
  rights), pointing at the private copy and the saves folder.
- Plays from GOG's installer files: the first start unpacks the game with
  the bundled innoextract 1.9, so the installer (which hangs under Wine) is
  never run. Unknown installers and missing or wrong-sized parts are
  refused with a reason.
- The GOG v1.1 `cmr5.exe` is a known edition.
- Looks: Original, Widescreen (the largest 16:9 size that fits the screen,
  with the game's `WIDESCREENDISPLAY` view) and Steam Deck (1280x720,
  60 fps). Options `resolution`, `fps_limit` and `vsync`.
- Under Wine and Proton: DXVK 3.1.1's `d3d9.dll` (a stage went from about
  20 fps to over 1000), `dxvk.conf` for the frame-rate cap and VSync, and
  `cmr5.exe`'s per-program DLL override.
- Saves live in their own folder; saves already in an installed game's
  `SG` folder are copied there once.
- Closing the game's window ends the game (the SDK's close hook,
  `PC_CMR5`): the window's close button, the compositor's close (Super+W,
  a desktop's Alt+F4 binding). The game itself ignores these.
- A screen mode the display does not list is replaced by the nearest
  listed 16:9 mode (the game quits at once otherwise).
- One launcher at a time.
- The first start from GOG's installer shows a window with the unpack's
  progress and the time left; Cancel stops it and removes the partial
  copy.
- Under Wine outside gamescope with a portrait main monitor, the game runs
  in a Wine virtual desktop sized to the landscape monitor (it stopped
  with "Adapter has no modes" otherwise).
- A new Wine prefix's first start no longer fails to open the settings
  window ("failed to find a matching configuration"): after switching Wine
  to GLX the launcher starts itself again, so the switch applies.
- The zip and the AppImage carry `components/` (DXVK's `d3d9.dll`,
  innoextract, the close hook), fetched by SHA-256 at package time; CI
  builds the close hook.
- The settings window takes the game's identity: the menus' white band,
  warm-black headings and sepia, the logo's red for Play, and a sepia
  photo of Colin McRae's Škoda Fabia WRC at Wales Rally GB 2005 (KevBow,
  CC BY 2.0). Sliders and number boxes are light with dark text.
- `linux/cmr2005-play`: gamescope takes the size the look gives (the
  screen for Widescreen and the first start, 1280x800 for Steam Deck,
  640x480 for Original); a `resolution` or flag wins.
