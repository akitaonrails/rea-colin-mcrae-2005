# Changelog

All notable changes to the Colin McRae Rally 2005 launcher. Versions are git tags
(`vX.Y.Z`) matching the `cmr2005-launcher` crate version.

## [Unreleased]

### Changed

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
