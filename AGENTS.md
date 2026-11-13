# Colin McRae Rally 2005 launcher — instructions for coding agents

This repository makes the player's own copy of **Colin McRae Rally 2005** run on modern
Windows and Linux (Proton/Wine, AppImage) with a settings window, sensible
defaults and no manual steps. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk) v0.9.0.

**Read `sdk/docs/STANDARDS.md` before any work; the playbooks are in
`sdk/docs/playbooks/`** (vendored from the SDK at the tag in `sdk/VERSION`).

## Always

- The game's files are read-only: never committed, copied into the repo or published.
- Everything outside this repository is the user's and read-only (game installs,
  other repos, Wine prefixes, saves, home configs, containers, the SDK checkout)
  unless the user asked for or approved that specific change: back up first,
  never delete by a broad glob, ask when unsure.
- Headless by default; open a visible window only when a check needs a person, and say so first.
- Never print, log or commit credentials.
- Push, tag, release or post only with the user's go-ahead.
- Research community fixes first (`docs/RESEARCH.md`); patch only what nobody solved.
- Never edit `sdk/`; SDK gaps and bugs go to the SDK repository as issues or PRs.
- Clean up after a release or a long session: `sdk/scripts/clean`, then `--yes` (log scratch in `.work/.task-created`).

Running the real game in a test: `sdk/docs/STANDARDS.md`, "Running the real
game in a test" (under "Testing").

## Layout

| Path | What |
|---|---|
| `crates/cmr2005-launcher/src/edition.rs` | Known builds (SHA-256), the unknown-edition refusal |
| `crates/cmr2005-launcher/src/config.rs` | The option table: `launcher.cfg`, flags, `--help`, template, looks |
| `crates/cmr2005-launcher/src/cli.rs` | Modes (`--settings`, `--no-window`, `--check`, `--help`), exit codes |
| `crates/cmr2005-launcher/src/form.rs`, `ui.rs`, `palette.rs` | The settings window and its headless tests |
| `crates/cmr2005-launcher/src/runtime.rs` | The runtime copy's `Policy`, `cmr5.exe` with the close hook and wheel layer imports |
| `crates/cmr2005-launcher/src/wheel.rs`, `controls.rs` | The wheel layer on a start (shown or not, its `rea_wheel.ini`); the virtual wheel's record in `SG\OPTIONS` (seed, stash, put back) |
| `crates/cmr2005-launcher/src/main.rs` | Windows wiring: console, window, `launcher.log`, the game's process |
| `linux/cmr2005-play`, `linux/tests/` | The Linux helper (sources `sdk/rea-play.sh`) and its tests |
| `packaging/` | Pins, allowlist, package script, AppImage desktop entry and icon |
| `sdk/` | Vendored by the SDK's `scripts/sdk-sync`; never edit by hand |
| `docs/RESEARCH.md`, `docs/RUNTIME_NOTES.md`, `docs/CONTRIBUTIONS.md` | Findings, verification, drafts to share |

## Checks (all must pass before a commit)

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --target i686-pc-windows-gnu --workspace --all-targets -- -D warnings
cargo deny check
for t in linux/tests/*.sh packaging/tests/*.sh; do sh "$t"; done
../rea-launcher-sdk/scripts/sdk-sync --check .
cargo build --release --locked --target i686-pc-windows-gnu
sh packaging/build-dlls.sh                     # rea_close_hook.dll, rea_dinput.dll (profile dll)
sdk/scripts/wine-test --locked --workspace     # Windows-target tests under Wine, headless
```

MinGW, lld and Wine may live in the `gaming` distrobox: run the last two as
`distrobox enter gaming -- sh -c 'cd /absolute/path && …'`.

Conventional Commits; every behaviour change gets a CHANGELOG "Unreleased"
entry, and every change that still needs a real-game check a line in
`docs/RUNTIME_NOTES.md`.

## Colin McRae Rally 2005: what is specific to this game

Details and evidence are in `docs/RESEARCH.md`; keep this list current.

- **Editions:** GOG v1.1 (offline installer `setup_colin2005.exe` + two
  `.bin`), `cmr5.exe` SHA-256 `a7ea2d33…13bc`. Retail DVDs carry StarForce
  and are not supported unless a DRM-free build of theirs turns up.
- **Renderer, input, audio:** D3D9, DirectInput 8, DirectSound (EAX),
  Windows Media through COM for `.wmv` videos and `.wma` music. Math
  helpers from `MathCPU`/`MathPIII`/`Math3dNow.dll`.
- **The registry is required.** The game reads
  `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005` → `INSTALL_PATH`,
  `SAVEGAME_PATH` (32-bit view) and otherwise looks in
  `C:\Program Files\Codemasters\Colin McRae Rally 2005`, where nothing is:
  the fonts never load and it crashes at `0x41DA27` on the first text.
  This was the "Wine dead-end". The launcher always writes both values,
  pointing at its runtime copy and its saves folder. It also reads/writes
  `LANGUAGE`, `WIDTH`, `HEIGHT`, `SELECTION`, `CD_KEY` there.
  Since 0.4.1 the launcher writes them to
  `HKCU\Software\Codemasters\Colin McRae Rally 2005` (`registry::USER_KEY`,
  `GAME.write`: `reg::write_verified`, the old HKLM/VirtualStore values
  carried over once) and the close hook DLL redirects the game's HKLM
  opens there (`REA_REGISTRY` = `registry::REDIRECT`). Never write HKLM
  or the VirtualStore (SDK v0.6.0; rea-colin-mcrae#6).
- **The game resets its own Graphics options** (`SELECTION`, the option
  words and the adapter's D3D9 ids) and drops to 640×480 when it can't use
  `WIDTH`×`HEIGHT`, finds another adapter, or starts for the first time.
  The launcher snapshots `SELECTION`/`WIDTH`/`HEIGHT` before writing and
  restores them after the game exits (`registry::after_run`).
- **`FORCEHT`** unlocks the game's Advanced particles on non-HT CPUs (AMD,
  modern Intel); passed unless `advanced_particles = off`.
- **Wheels:** the runtime's `cmr5.exe` also imports the SDK's
  `rea_dinput.dll` (after the close hook), which shows the game one
  virtual wheel. The launcher decides before each start
  (`wheel::shown`), passes `REA_WHEEL_MODE`, writes the runtime's
  `rea_wheel.ini` from the `wheel*` options and the calibration beside
  `launcher.cfg`, and seeds player 1's slot 2 of `OPTIONS` for the
  virtual wheel (no checksum; never the 8-byte trailer), putting the
  displaced record back on a start without it (`controls`,
  `wheel-records`). Both DLLs are built by `packaging/build-dlls.sh`
  (`--profile dll`, which unwinds, and the `layer-dlls` feature; the
  launcher links both crates with `default-features = false`).
  Docs/RESEARCH.md "Wheels", RUNTIME_NOTES 2026-11-13.
- **Files the game writes:** `SG\OPTIONS` and saves under `SAVEGAME_PATH`
  (`BESTTIMES_GHOSTS`, `CAREER_SLOT_*`, `CHAMPIONSHIP_SLOT_*`), plus the
  registry values above (in the HKCU key, through the redirect).
- **Community components used:** none yet; Xidi planned (see `docs/RESEARCH.md`).
- **Patches:** none to the game's code; the startup crash needs no patch.
  The runtime copy's `cmr5.exe` imports `rea_close_hook.dll` (SDK v0.4.3,
  `rea_close_hook::import_into`) and `rea_dinput.dll`, rebuilt from the player's bytes on every
  start; the import is required (it carries the registry redirect), so an
  exe that can't take it stops Play and `--check`; the launcher never hooks the running game (antivirus).
- **Owner decisions (2026-11-02):** id `cmr2005`; accept an installed
  folder or the GOG installer files (the launcher unpacks the installer,
  which hangs under Wine, with the SDK's `rea_launcher::installer`;
  `source.rs` holds the installer table and renames 0.2's
  `.cmr2005-unpacked` marker to `.rea-unpacked`; afterwards the `game`
  setting points at the unpacked folder unless it came from `--game`); write the registry values per user
  (HKCU, no admin; since 0.4.1 never through the VirtualStore). Never use the "64-bit patch" (it
  restores StarForce's `protect.dll`).
- **Known problems and their status:** startup crash fixed (verified to the
  menus, headless). Verified headless: intro videos, a stage, saves,
  16:9 widescreen, DXVK, every way of quitting under `cmr2005-play`, the
  settings window under Wine and play-time error dialogs, the wheel layer
  with the virtual wheel bound and the seed kept (no real wheel). Sound,
  driving with a real wheel and pads are not verified yet
  (`docs/RUNTIME_NOTES.md`).
