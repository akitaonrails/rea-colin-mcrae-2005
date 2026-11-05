# Colin McRae Rally 2005 launcher — instructions for coding agents

This repository makes the player's own copy of **Colin McRae Rally 2005** run on modern
Windows and Linux (Proton/Wine, AppImage) with a settings window, sensible
defaults and no manual steps. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk) v0.4.3.

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
| `crates/cmr2005-launcher/src/runtime.rs` | The runtime copy's `Policy` |
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
- **Files the game writes:** `SG\OPTIONS` and saves under `SAVEGAME_PATH`
  (`BESTTIMES_GHOSTS`, `CAREER_SLOT_*`, `CHAMPIONSHIP_SLOT_*`), plus the
  registry values above.
- **Community components used:** none yet; Xidi planned (see `docs/RESEARCH.md`).
- **Patches:** none. The startup crash needs no patch.
- **Owner decisions (2026-11-02):** id `cmr2005`; accept an installed
  folder or the GOG installer files (the launcher unpacks the installer,
  which hangs under Wine); on Windows write the registry values per user
  through the VirtualStore, no admin. Never use the "64-bit patch" (it
  restores StarForce's `protect.dll`).
- **Known problems and their status:** startup crash fixed (verified to the
  menus, headless). Verified headless: intro videos, a stage, saves,
  16:9 widescreen, DXVK, every way of quitting under `cmr2005-play`, the
  settings window under Wine and play-time error dialogs. Sound, driving
  and controllers are not verified yet (`docs/RUNTIME_NOTES.md`).
