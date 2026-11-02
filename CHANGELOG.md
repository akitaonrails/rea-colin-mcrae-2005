# Changelog

All notable changes to the Colin McRae Rally 2005 launcher. Versions are git tags
(`vX.Y.Z`) matching the `cmr2005-launcher` crate version.

## [Unreleased]

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
