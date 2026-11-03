# Contributions back

Fixes found while building this launcher that the community lacks, drafted
for PCGamingWiki, upstream projects, ProtonDB or the WineHQ AppDB
(rea-launcher-sdk `docs/playbooks/contribute-back.md`). These are public
posts under the owner's name: agents only draft them here; the owner
reviews and posts. Only fixes verified in the real game.

| Target (page or project) | Fix | Evidence | Status |
|---|---|---|---|
| PCGamingWiki, Colin McRae Rally 2005 (Issues fixed; Linux) | The startup crash: the install-path registry values | `RUNTIME_NOTES.md` r02–r06, e06 | draft |
| WineHQ Bugzilla, bug 38198 (comment) | The cause of the `0x41DA27` crash | same | draft |
| WineHQ AppDB, version 6421 (test report) | What works under Wine 11.21 | `RUNTIME_NOTES.md` | draft |
| rea-launcher-sdk (issue) | Light palettes: the theme leaves egui's dark widget fills | `src/palette.rs::light_widgets` and its test | done in SDK v0.4.0 (adopted; local fix removed) |
| rea-launcher-sdk (issue) | Close hook: Alt+F4 never reaches a game with an exclusive DirectInput keyboard | `RUNTIME_NOTES.md` e06, e09 | done in SDK v0.4.0/v0.4.1 (`close_on = alt_f4_state`, adopted); follow-up draft below: it only acts when a message arrives (w02) |
| rea-launcher-sdk (issue) | Close hook `alt_f4_state`: no message arrives while only the keyboard is used, so Alt+F4 alone still does nothing | `RUNTIME_NOTES.md` w02 | draft |
| rea-launcher-sdk (issue) | `report::Audience`: `--help`/`--check` under Wine become a dialog, so a script waits on it | `src/cli.rs::commands_print_only` and its tests | draft |

## Drafts

### PCGamingWiki: "Game crashes on startup / after the logos" (Issues fixed)

Applies to: the GOG release, `cmr5.exe` version 1.1, SHA-256
`a7ea2d33337d764e541b0871a5b0ce5a91c2e66c94c8459e811823054fb113bc`
(MD5 `aba2fae91a34a0ec2d96991ba69d8b49`). Likely any copy installed without
its installer (copied, or unpacked with innoextract).

> **Game crashes at startup (page fault at `0041DA27`)**
>
> The game finds its files through two registry values its installer
> writes. When they are missing (a copied install, an installer that
> didn't finish, innoextract on Linux) it looks in
> `C:\Program Files\Codemasters\Colin McRae Rally 2005`, loads no fonts,
> and crashes the first time it draws text.
>
> Create these values (32-bit view; on 64-bit Windows the key is under
> `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\`):
>
> `HKEY_LOCAL_MACHINE\SOFTWARE\Codemasters\Colin McRae Rally 2005`
> - `INSTALL_PATH` (string): the game folder, e.g. `D:\Games\Colin McRae Rally 2005`
> - `SAVEGAME_PATH` (string): the save folder, e.g. `D:\Games\Colin McRae Rally 2005\SG`
>
> Optional: `WIDTH` and `HEIGHT` (DWORD) set the screen mode before the
> first start.
>
> Under Wine: `wine reg add "HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005" /v INSTALL_PATH /t REG_SZ /d "<folder>" /reg:32`
> (and the same for `SAVEGAME_PATH`). With these the GOG build runs under
> Wine 11.21 through the intro videos, the menus and stages; DXVK is
> recommended (Wine's own Direct3D drew about 20 fps in a stage).
>
> Found while building https://github.com/akitaonrails/rea-colin-mcrae-2005,
> which sets this up automatically.

### WineHQ bug 38198 (comment)

> The cause is in the game, not Wine: `cmr5.exe` reads `INSTALL_PATH` and
> `SAVEGAME_PATH` from `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005`
> and otherwise looks in `C:\Program Files\Codemasters\Colin McRae Rally 2005`.
> With no font files there, the font slots stay empty, and the text-width
> routine at `0x41DA10` reads the font header (`mov ecx,[eax]` then
> `mov ebp,[ecx+0x10]` at `0x41DA27`) before checking whether the slot is
> loaded. That is why reinstalling with the GOG installer fixed it for the
> reporter: the installer writes those values. Setting the two values in a
> prefix with an innoextract-ed copy makes the GOG build run under Wine
> 11.21 (tested to the menus and stages).
>
> Details: https://github.com/akitaonrails/rea-colin-mcrae-2005/blob/main/docs/RESEARCH.md

### WineHQ AppDB, version 6421 (test report)

> Wine 11.21 (Arch, WoW64), GOG build v1.1, NVIDIA RTX 5090, gamescope.
> Works after adding the two registry values above. Intro videos (WMV)
> play with GStreamer. With DXVK 3.1.1 (`d3d9=n,b`) a stage runs at well
> over 1000 fps uncapped; with Wine's own Direct3D about 20 fps. Alt+F4
> does not close the game (closing the window does through a launcher's
> hook). The GOG installer itself hangs under Wine; innoextract unpacks it.
> Not tested: sound output, controllers.

### rea-launcher-sdk issue: light palettes get dark widgets

> `theme::set_palette` starts from `egui::Visuals::dark()` and sets the
> widgets' strokes but not `bg_fill`/`weak_bg_fill`. With a light palette
> (CMR2005's window: base `#fbfaf8`, ink `#1c1612`) sliders, number boxes
> and buttons stay egui's dark grey under dark text: about 1.5:1 contrast,
> and `contrast_problems` can't see it because it only checks the palette.
> CMR2005 works around it in `src/palette.rs::light_widgets` (panel fill
> for inactive widgets, a slightly darker one for hovered/active, ink for
> `fg_stroke`) with a test that fails without it. Proposal: set the widget
> fills from the palette (panel; a hover tint), pick `Visuals::light()` when
> the base is light, and add a contrast check of ink on the widget fills.

### rea-launcher-sdk issue: Alt+F4 with an exclusive DirectInput keyboard (done in v0.4.x)

> CMR2005 ignores window-close requests like CMR04. With
> `class=PC_CMR5;strategy=terminate` the hook ends it for `SC_CLOSE` and a
> compositor's `WM_DELETE_WINDOW`, but Alt+F4 pressed in the game (XTest,
> or `SendInput` inside Wine) never reaches the window, most likely because
> the game acquires the keyboard through DirectInput in exclusive mode. A
> `close_on = alt_f4_state` option, where the hook checks
> `GetAsyncKeyState(VK_MENU)` and `VK_F4` on each message it sees, would
> cover such games. Not verified on Windows.

### rea-launcher-sdk issue: commands under Wine and dialogs

> `report::Audience::of` makes a command (`--help`, `--check`, a bad
> command line) interactive under Wine unless `--linux` is given, and
> `route` then shows a dialog because Wine's stdout never counts as seen.
> `wine CMR2005Launcher.exe --check` from a script, or the launcher's own
> program tests under `sdk/scripts/wine-test`, then wait on a message box
> nobody answers (the template avoids it by adding `--linux` to its program
> tests). CMR2005 verified and keeps "commands under Wine are only
> printed" (`src/cli.rs::commands_print_only`, on top of the SDK's
> audience). The case for the SDK's rule: a `--check` added to Proton's
> launch options prints into a log nobody reads. Proposal: decide it in
> the SDK, e.g. a `Run::script` flag or a documented rule for commands
> under Wine, so launchers stop choosing differently.

### rea-launcher-sdk issue: `alt_f4_state` needs a message to arrive

> v0.4.1's `close_on = alt_f4_state` reads Alt+F4 from the keyboard state
> in the `WH_GETMESSAGE` hook, so it only acts when the game's thread
> takes a message from its queue. CMR2005 (exclusive DirectInput
> keyboard, Wine 11.21, headless gamescope, XTest input, run w02): Alt+F4
> alone, and Alt+F4 held for 2 s, left the game running; mouse moves alone
> too; Alt+F4 held while the mouse moved ended it within 4 s ("game exited
> with 0"). With only the keyboard (or a pad) in use, the game gets no
> messages, so the press is never seen. Proposal: when `alt_f4_state` is
> on, the hook also gives the game's window thread a timer
> (`SetTimer(NULL, …)` from inside the hook on its first call, e.g. every
> 100 ms) so a `WM_TIMER` message arrives and the state is read; or a
> watcher thread in the hook DLL that polls `GetAsyncKeyState` and posts
> to the window. Not verified on Windows.
