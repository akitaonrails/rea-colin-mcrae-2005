# Colin McRae Rally 2005 Launcher

Plays **your own copy** of Colin McRae Rally 2005 on Windows 10/11, the
Steam Deck and Linux (Proton, Wine or the AppImage). Point it at your game,
or at the GOG installer you downloaded, and press **Play**. Everything the
game needs is bundled and set up for you: no patchers, no winetricks, no
DLL overrides, no registry edits by hand. Your game folder is only read,
never changed; the launcher plays from a private copy.

This project contains no game files: you need your own copy of the game.

> Played end to end on Linux (Wine 11 inside gamescope): unpacking the GOG
> installer, the intro videos, the menus and a stage, every look, quitting
> by closing the window. Not tested yet: Windows itself, the Steam Deck,
> Proton, controllers, and sound beyond "no errors". Reports are welcome:
> [open an issue](https://github.com/akitaonrails/rea-colin-mcrae-2005/issues/new?template=bug-report.yml).

<!-- Showcase video: a poster image linked to the video, both hosted outside
the repository. Caption: what is shown, on which platform, length. -->

## Problems this launcher solves

| Problem | What the launcher does |
| --- | --- |
| On Linux the game crashes as soon as it starts, before the first menu (the "Wine dead-end"). | The game looks for its files through settings its installer writes. Without them it loads no fonts and crashes. The launcher writes them on every start, pointing at its private copy. |
| GOG's installer hangs under Wine, so Linux players can't install the game at all. | Point the launcher at the installer files. It unpacks the game itself on the first start (with innoextract, bundled) and never runs the installer. |
| The game starts at 640×480 in the corner of the screen. | The picture follows your screen: the **Widescreen** look uses the largest 16:9 size that fits, with the game's own widescreen view. |
| Under Wine a stage runs at about 20 frames per second. | The launcher brings DXVK for Wine and Proton: the same stage runs at hundreds of frames per second, with a frame-rate cap and VSync you can set. |
| Alt+F4 and the window's close button do nothing, so the game had to be killed. | Closing the window (or Super+W, or your desktop's close shortcut) ends the game and the launcher. |
| Saves sit inside the game folder, and copying or patching the game risked them. | Your saves get a folder of their own, outside the game and the launcher's copy. Saves already in your game folder are copied there once. |

## Screenshots

| Play | Picture |
| --- | --- |
| ![The Play tab with the game folder and the three looks](docs/images/launcher-play.jpg) | ![The Picture tab with resolution, frame-rate cap and VSync](docs/images/launcher-picture.jpg) |

![A stage in Australia at 1920×1080 with the widescreen view](docs/images/gameplay.jpg)

## What you need

- Colin McRae Rally 2005 for PC, the **GOG** release (version 1.1). Either
  the installed game, the folder with `cmr5.exe`, or the three installer
  files you downloaded from GOG (`setup_colin2005.exe`,
  `setup_colin2005-1.bin`, `setup_colin2005-2.bin`).
- About 3.5 GB free if you start from the installer files: the launcher
  unpacks the game once.
- The retail DVD versions use StarForce copy protection and are not
  supported.

If your `cmr5.exe` or `setup_colin2005.exe` is a version the launcher does
not know, it stops and shows a code (SHA-256). Please
[report it](https://github.com/akitaonrails/rea-colin-mcrae-2005/issues/new?template=unknown-edition.yml)
with that code and where your copy came from. Report the code, never the
file itself.

## Download and install

Download from the [latest release](https://github.com/akitaonrails/rea-colin-mcrae-2005/releases/latest):
`CMR2005Launcher-<version>.zip` for Windows, Proton and your own Wine, or
`CMR2005Launcher-<version>-x86_64.AppImage` for Linux.

### Windows

1. Unzip anywhere outside the game folder.
2. Keep the `components` folder next to `CMR2005Launcher.exe`.
3. Run `CMR2005Launcher.exe`.

### Steam Deck

1. In Desktop Mode, keep the AppImage (or unzip the zip) somewhere in your
   home folder.
2. Steam → **Add a Non-Steam Game** → choose it. For the zip's
   `CMR2005Launcher.exe`, open its Properties → Compatibility and force a
   Proton version.
3. Back in Game Mode, start it. The **Steam Deck** look is picked for you.

Game Mode has not been tested yet.

### Linux: AppImage

```sh
chmod +x CMR2005Launcher-*-x86_64.AppImage
./CMR2005Launcher-*-x86_64.AppImage
```

It brings its own Wine and keeps its Wine prefix in
`~/.local/share/cmr2005-launcher/prefix`. When gamescope is installed the
game runs inside it. Your Linux files are drive `Z:` in the window.

### Linux: Steam (Proton)

Add the zip's `CMR2005Launcher.exe` as a non-Steam game and force a Proton
version, as on the Steam Deck. Proton brings its own DXVK. Not tested yet.

### Linux: your own Wine

From the zip:

```sh
linux/cmr2005-play            # the settings window, then the game
linux/cmr2005-play --play     # the game with your saved settings
linux/cmr2005-play stop       # end the game
linux/cmr2005-play status     # running or idle
```

More, including a distrobox, in [linux/README.md](linux/README.md).

## First run

1. Press **Browse…** and choose the folder with `cmr5.exe`, or the folder
   with GOG's `setup_colin2005.exe`. Under Wine your Linux files are drive
   `Z:`, for example `Z:\home\you\Games\Colin McRae Rally 2005`.
2. Pick a look. **Widescreen** is recommended (**Steam Deck** on a Deck).
3. Press **Play** (or Enter).

From the installer files, the first start unpacks the game, about 3.4 GB.
It took about three minutes in testing and nothing shows on screen
meanwhile. After that, starts are quick.

## The settings window

**Play** holds the game folder and the looks. **Picture** holds the
resolution, the frame-rate cap and VSync. Changing one of those by hand
leaves the look cards unselected (your own mix).

| Look | Resolution | Frame-rate cap | VSync | When to pick it |
| --- | --- | --- | --- | --- |
| **Original** | the game's own (it starts at 640×480) | none | off | to see the game as it shipped |
| **Widescreen** | the largest 16:9 size that fits your screen, widescreen view | none | on | most screens (picked on the first run) |
| **Steam Deck** | 1280×720, widescreen view | 60 fps | on | the Deck (picked there on the first run) |

The game's widescreen view is only correct at 16:9 sizes. Other shapes
(16:10, 21:9, 4:3) keep the game's 4:3 view.

The window works with the keyboard alone: Tab moves, Enter or Space
activates, Enter plays. A control that can't be used says why.

## Options

Every option has the same name in the window, in `launcher.cfg` and as a
flag (`fps_limit` in the file is `--fps-limit` on the command line).
`launcher.cfg` is described under **Where things live**. The first run
writes it with every option commented and the recommended look's values
filled in.

| `launcher.cfg` / flag | In the window | Values | Unset | What it changes |
| --- | --- | --- | --- | --- |
| `game` / `--game` | Play: Game folder | a folder | asked in the window | Your installed game, or the folder with GOG's installer files. Only read. |
| `look` / `--look` | Play: the look cards | `original`, `widescreen`, `steam_deck` | none | Sets the resolution, cap and VSync at once; options set after it win. |
| `resolution` / `--resolution` | Picture: Resolution | `WIDTHxHEIGHT`, `original` | the game's own | The picture size. A 16:9 size gets the widescreen view. |
| `fps_limit` / `--fps-limit` | Picture: Frame-rate cap | `0`–`1000`, `off` | no cap | The most frames per second. Under Wine and Proton only. |
| `vsync` / `--vsync` | Picture: VSync | `on`, `off` | the game's own | Waits for the screen: no tearing. Under Wine and Proton only. |
| `runtime` / `--runtime` | — (file and flag only) | a folder | beside `launcher.log` | Where the launcher keeps its private copy. |

Unset keeps the game's own value.

Any option on the command line skips the window and plays, overriding
`launcher.cfg` for that run; only `--game` is remembered. The launcher's
own switches:

| Switch | What it does |
| --- | --- |
| `--settings` | only the window: exit 0 after **Play**, 3 when closed, 4 when no window can open |
| `--no-window` | play with the saved settings, no window |
| `--check` | check the game folder and exit: 0 ready, 1 not (details in `launcher.log`) |
| `--linux` | started by `linux/cmr2005-play` |
| `--help` | the options |

A bad flag or value exits with 2 and says what is wrong.

## Where things live

| What | Windows | Linux (AppImage, `cmr2005-play`) |
| --- | --- | --- |
| Settings (`launcher.cfg`) and log (`launcher.log`) | `%LOCALAPPDATA%\CMR2005Launcher\` | in the prefix, `drive_c/users/<you>/AppData/Local/CMR2005Launcher/` |
| Your saves | `%LOCALAPPDATA%\CMR2005Launcher\saves\` | in the prefix, `…/AppData/Local/CMR2005Launcher/saves/` |
| The launcher's copy of the game | `%LOCALAPPDATA%\CMR2005Launcher\runtime\` | `…/CMR2005Launcher/runtime/` |
| The game unpacked from GOG's installer | `%LOCALAPPDATA%\CMR2005Launcher\game\` | `…/CMR2005Launcher/game/` |
| Wine prefix | — | `~/.local/share/cmr2005-launcher/prefix` |
| `cmr2005-play`'s logs and lock | — | `~/.config/cmr2005-launcher` |

A `launcher.cfg` next to `CMR2005Launcher.exe` is used instead (a portable
install). Under Proton everything is inside Steam's
`compatdata/<appid>/pfx`.

The copy of the game links the big data files instead of copying them
where the disk allows, so it takes little extra space.

The launcher writes the game's own registry key,
`HKEY_LOCAL_MACHINE\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005`
(`INSTALL_PATH`, `SAVEGAME_PATH`, and `WIDTH`/`HEIGHT` when a look or
resolution is set). Without administrator rights Windows keeps them per
user, and the game reads them from there.

## Troubleshooting

- **"This cmr5.exe is not a build this launcher knows"** (or the same for
  `setup_colin2005.exe`): your copy is a version the launcher has not seen.
  [Report the code it shows](https://github.com/akitaonrails/rea-colin-mcrae-2005/issues/new?template=unknown-edition.yml).
- **A .bin file "is missing" or "has … bytes":** the installer download is
  incomplete. Download it again from GOG.
- **"Unpacking the game needs about 3 GB free":** free some space, or set
  `runtime` to a folder on another drive (the unpacked game goes beside it).
- **Alt+F4 does nothing in the game:** the game keeps the keyboard to
  itself. Close the window instead, or quit from the game's menu.
- **Anything else:** run `CMR2005Launcher.exe --check`, then open an issue
  with the [bug form](https://github.com/akitaonrails/rea-colin-mcrae-2005/issues/new?template=bug-report.yml)
  and attach `launcher.log` (remove personal paths). Never attach game files.

## Known limitations

- The game's widescreen view is exact only at 16:9. On 16:10 and ultrawide
  screens the Widescreen look uses a 16:9 picture inside the screen.
- Alt+F4 pressed inside the game is not seen. Closing the window works.
- The frame-rate cap and VSync work under Wine and Proton only. On Windows
  the game keeps its own behaviour.
- No progress is shown while the installer unpacks (a few minutes).
- Online play used GameSpy, which is gone.
- Not tested yet: Windows 10/11, the Steam Deck, Proton, controllers,
  multi-monitor setups, and driving physics at high frame rates. See
  [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md).

## Uninstall

1. **Back up your saves**: `%LOCALAPPDATA%\CMR2005Launcher\saves` on
   Windows, or `drive_c/users/<you>/AppData/Local/CMR2005Launcher/saves`
   inside the prefix on Linux. The next steps delete them.
2. Delete the launcher's folder (or the AppImage).
3. Windows: delete `%LOCALAPPDATA%\CMR2005Launcher` (settings, log, the
   private copy, and the unpacked game, about 3.5 GB). Keep the registry key
   above if you still play the game without the launcher; otherwise you can
   delete it.
4. Linux: copy the saves out, then delete `~/.local/share/cmr2005-launcher`
   and `~/.config/cmr2005-launcher`. Under Proton, removing the Steam
   shortcut offers to delete its prefix, saves included.

Your game folder was never changed, so there is nothing to undo there.

## For developers

Rust (edition 2024), MinGW-w64 for `i686-pc-windows-gnu` linked with lld
(GNU ld mis-links the import tables), Wine for the Windows-target tests,
`cargo-deny`. The `gaming` distrobox from
[distrobox-gaming](https://github.com/akitaonrails/distrobox-gaming) has them all.

```sh
cargo fmt --all --check
cargo test --workspace --locked                 # host tests, the window headless
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --target i686-pc-windows-gnu --workspace --all-targets -- -D warnings
cargo deny check
for t in linux/tests/*.sh packaging/tests/*.sh; do sh "$t"; done
cargo build --release --locked --target i686-pc-windows-gnu
sdk/scripts/wine-test --locked --workspace      # Windows-target tests under Wine, headless
sh <sdk>/scripts/sdk-sync --check .             # sdk/ matches the pinned SDK tag
```

- Moving to a new SDK: `sdk/scripts/sdk-bump vX.Y.Z`
  (`sdk/docs/playbooks/sdk-change.md`). Freeing disk space:
  `sdk/scripts/clean`, then `--yes`.
- `packaging/package.sh` builds the zip and the AppImage. DXVK and
  innoextract are fetched from pinned URLs and checked by SHA-256
  (`packaging/third-party.txt`); the close hook is built from the pinned
  SDK. GitHub Actions run the checks and publish a release on a `v*` tag.
- [AGENTS.md](AGENTS.md) has the rules for contributors and coding agents;
  [docs/RESEARCH.md](docs/RESEARCH.md) has the reverse engineering (why the
  game crashed, the widescreen view); [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md)
  says what was verified in the game and how.

## How it works

1. Identifies the game folder: a known `cmr5.exe`, or GOG's installer (the
   first start unpacks it with innoextract).
2. Brings the game into a private copy, linking the big data files.
3. Under Wine or Proton, adds DXVK's `d3d9.dll` and writes its `dxvk.conf`
   (frame-rate cap, VSync).
4. Writes the game's registry values: where its files are, where to save,
   the screen mode.
5. Starts the game, with `WIDESCREENDISPLAY` at 16:9 sizes, and a small
   hook that turns a window close into quitting.

## Built on rea-launcher-sdk

The runtime copy, `launcher.cfg` and the option table, the settings window's
widgets, the registry and Wine helpers, the close hook, `rea-play.sh`,
packaging and CI come from
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk), pinned
by tag in `Cargo.toml` and vendored in `sdk/` (the tag is in
`sdk/VERSION`). This repository holds what is Colin McRae Rally 2005's own:
the editions, the registry fix, the installer unpacking, the looks.

## Contributing

- **About Colin McRae Rally 2005** (its editions, fixes, settings,
  `cmr2005-play`'s game settings, this README): issues and pull requests here.
- **Shared code** (the runtime folder, settings-window widgets, the option
  table, `rea-play.sh`, packaging, CI workflows, anything under `sdk/`):
  issues and pull requests at
  [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk/issues).
- **How to tell:** code under `sdk/` or from a `rea_launcher*` crate is the
  SDK's. If the same problem would happen with another rea launcher, it's
  the SDK's. Not sure? Open it here and it will be moved.
- Never patch `sdk/` or copy SDK code here: fix the SDK and bump the tag.
- Never attach game files (executables, saves, data) to an issue.
- Test reports are welcome: the
  [bug form](https://github.com/akitaonrails/rea-colin-mcrae-2005/issues/new?template=bug-report.yml).

## Credits

- [DXVK](https://github.com/doitsujin/dxvk) by Philip Rebohle and
  contributors: Direct3D 9 on Vulkan under Wine and Proton.
- [innoextract](https://constexpr.org/innoextract/) by Daniel Scharrer:
  unpacks GOG's installer.
- [Wine](https://www.winehq.org) and Kron4ek's
  [portable builds](https://github.com/Kron4ek/Wine-Builds), and the
  AppImage tools, in the AppImage.
- [Barlow](https://github.com/jpt/barlow) by Jeremy Tribby (SIL OFL): the
  window's type.
- KevBow's [photo of Colin McRae at Wales Rally GB 2005](https://commons.wikimedia.org/wiki/File:Colin_McRae.jpg)
  (CC BY 2.0): the window's picture.
- [PCGamingWiki](https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_2005),
  the [WineHQ AppDB](https://appdb.winehq.org/objectManager.php?sClass=version&iId=6421)
  and [Wine bug 38198](https://bugs.winehq.org/show_bug.cgi?id=38198), where
  the earlier reports were.
- [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk).

Full list with licences: [THIRD_PARTY.md](THIRD_PARTY.md).

## Legal

Colin McRae Rally 2005 is © Codemasters. This project is not affiliated
with or endorsed by Codemasters or GOG. It contains no game files and only
runs a copy you own. Our code is licensed under
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE); bundled parts keep their
own licences ([THIRD_PARTY.md](THIRD_PARTY.md)).
