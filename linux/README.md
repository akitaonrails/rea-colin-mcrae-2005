# Colin McRae Rally 2005 on Linux

The launcher is a Windows program (the same one as on Windows); on Linux
it runs through Wine or Proton. Three ways to play:

## The AppImage

```sh
chmod +x CMR2005Launcher-*-x86_64.AppImage
./CMR2005Launcher-*-x86_64.AppImage
```

It brings its own Wine, so nothing else needs to be installed. The
settings window opens: press **Browse…** and pick your game folder (your
Linux files are on drive `Z:`), choose a look and press **Play**. When
[gamescope](https://github.com/ValveSoftware/gamescope) is installed the
game runs in it; otherwise in a plain window. Its Wine prefix is
`~/.local/share/cmr2005-launcher/prefix`.

## Steam (Proton)

1. Unzip the release, for example into `~/Games/cmr2005-launcher`.
2. In Steam: **Games → Add a Non-Steam Game to My Library…** and pick
   `CMR2005Launcher.exe`.
3. **Properties → Compatibility**: force a Proton version.
4. Press **Play**, pick your game folder (under drive `Z:`), choose a look
   and press **Play**.

### Steam Deck

Follow the steps above in Desktop Mode, then play from Game Mode. The
window recommends the **Steam Deck** look and works with the controls.

Game Mode has not been tested yet.

## Your own Wine

`linux/cmr2005-play` runs the launcher and the game in a private Wine prefix,
inside gamescope when it is installed. It needs `rea-play.sh` beside it
(the zip and the AppImage ship it) or in `../sdk/` (a repository checkout).

```sh
linux/cmr2005-play             # the settings window, then the game
linux/cmr2005-play --play      # the game with the saved settings, no window
linux/cmr2005-play stop        # end the game
linux/cmr2005-play status      # running or idle
```

Launcher options after `--play` apply to that run, for example
`linux/cmr2005-play --play --look original`. When Wine and gamescope live in
a [distrobox](https://github.com/containers/distrobox), run it through the
container:

```sh
CMR2005_RUN="distrobox enter gaming --" linux/cmr2005-play
```

## Environment

| Variable | Meaning |
|---|---|
| `CMR2005_PREFIX` | the Wine prefix (default `~/.local/share/cmr2005-launcher/prefix`) |
| `CMR2005_LAUNCHER` | `CMR2005Launcher.exe` (default: beside the script, or one folder up) |
| `CMR2005_RUN` | a wrapper for wine and gamescope, e.g. `distrobox enter gaming --` |
| `CMR2005_GPU` | GPU environment instead of the detected one (empty: none) |

## Where things are

| What | Where |
|---|---|
| Settings (`launcher.cfg`) and log (`launcher.log`) | in the prefix, `drive_c/users/<you>/AppData/Local/CMR2005Launcher/` |
| The launcher's copy of the game | `…/AppData/Local/CMR2005Launcher/runtime` |
| Your saves | `…/AppData/Local/CMR2005Launcher/saves` |
| The game unpacked from GOG's installer | `…/AppData/Local/CMR2005Launcher/game` |
| `cmr2005-play`'s logs and lock | `~/.config/cmr2005-launcher` |

## Troubleshooting

- **GOG's installer hangs under Wine:** don't run it. Point the launcher
  at the folder with `setup_colin2005.exe` and its two `.bin` files; it
  unpacks the game itself.
- **The game window doesn't close on Alt+F4 inside gamescope:** close it
  with `linux/cmr2005-play stop`, Ctrl+C in the terminal, or your desktop's
  close shortcut.

- **Anything else:** run `CMR2005Launcher.exe --check` through the AppImage or
  `cmr2005-play`, and look at `launcher.log` (see the table above).
