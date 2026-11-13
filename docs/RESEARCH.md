# Research: Colin McRae Rally 2005 (PC)

Phase 1 (community fixes) and Phase 2 (static analysis with REA) of the
SDK's new-launcher playbook. Runtime evidence is in `RUNTIME_NOTES.md`.
Observations, inferences and decisions are kept apart.

## The build we have

| Item | Value |
|---|---|
| Source | GOG offline installer: `setup_colin2005.exe` + `setup_colin2005-1.bin` + `setup_colin2005-2.bin` (Inno Setup, not passworded) |
| Game version | 1.1 (GOG and CD Projekt releases ship with the official 1.1 patch, per PCGamingWiki) |
| `cmr5.exe` | 2,654,208 bytes, SHA-256 `a7ea2d33337d764e541b0871a5b0ce5a91c2e66c94c8459e811823054fb113bc`, MD5 `aba2fae91a34a0ec2d96991ba69d8b49` (matches the GOG listing), FileVersion 1.0.0.0, PDB `cmr5.pdb` GUID `ed4f25cd-69a5-4450-8e7a-32fc9a40b1d1` |
| `MathCPU.dll` | SHA-256 `b913baecb125d4458f3183e2593c8390531de54eaf36cce125df46281bd951d6` |
| `MathPIII.dll` | SHA-256 `6b0a646f1cb556b1f2a5624574c1ed572635be4609244ed0e81d12dcecd7d1de` |
| `Math3dNow.dll` | SHA-256 `ce41ccda8b820401c55e3906756d939e521859265cb79367b8c6c7d89c832faf` |
| Extras in the installer | `tmp/wmfdist.exe`, `tmp/wmpcdcs8.exe` (Windows Media runtime and codecs), Adobe Reader; GOG's Galaxy build also carries `goggame-1207658663.dll` (not in this offline set) |

The owner's two local copies of the installer are byte-identical
(distrobox-gaming issue #4). Other editions (retail DVD with StarForce,
Galaxy build 311) are not available here yet.

## Previous attempts (distrobox-gaming issue #4, 2026-06 to 2026-10)

Declared a "Wine dead-end". Under Wine 9.19-staging and 11.x the
`innoextract`-ed install always crashed with

```text
wine: Unhandled page fault on read access to 00000010 at address 0041DA27
```

Tried without success: a clean 32-bit prefix, XP SP3 mode, DXVK, WineD3D,
a Wine virtual desktop, disabling XVidMode/XRandR, the `WIDESCREENDISPLAY`,
`NOVIDEOMEMORYCHECK` and `NOVIDEO` switches, corefonts/vcrun6, and
PCGamingWiki's "64-bit patch". The real GOG installer hung under Wine and
the box had to be force-stopped.

## Root cause of the crash (solved)

**Observed (REA, `cmr5.exe` above):**

- `0x41DA27` is in `FUN_0041da10`, the text-width routine of the bitmap
  font system (`.\Game\Source\CMR3Font.cpp`). It indexes the font table at
  `[0x7b5548]` by font id (36-byte entries) and reads the font header
  pointer at `entry+0` and then `header+0x10` **before** checking the
  slot's "loaded" flag at `entry+0x14`. The draw routine (`FUN_0041d290`)
  checks the flag first; the width routine does not.
- A font slot is filled by `FUN_0041d900`: it reads `fonts\NAME.dds`,
  creates a texture from it (`FUN_00581ce0`), and only when that succeeds
  reads `fonts\NAME.pcf` and marks the slot loaded (`FUN_0041d800`). A
  missing file leaves the slot empty.
- The game builds data paths from the registry:
  `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005`, values
  `INSTALL_PATH` and `SAVEGAME_PATH`. When they are missing it falls back
  to the hard-coded `C:\Program Files\Codemasters\Colin McRae Rally 2005`
  and `...\SG` (strings at `0x5e9660`, `0x5e9628`).
- Runtime (`RUNTIME_NOTES.md`, run r03): with no registry values the game
  opens `C:\Program Files\Codemasters\Colin McRae Rally 2005\fonts\HelNu_10.dds`,
  which does not exist; on the first text drawn it crashes at `0x41DA27`.

**Inference:** every earlier attempt crashed because an `innoextract`
install never writes the installer's registry values, and the one route
that would (the real installer) hung under Wine. Wine bug 38198 reports
the same crash on the GOG build; its reporter fixed it by "reinstalling
with the GOG installer" and guessed "relation with register keys". Nobody
had identified the mechanism. On Windows the same crash would happen to
anyone who copies the game folder instead of installing it.

**Decision:** the launcher writes `INSTALL_PATH` (the folder it runs the
game from) and `SAVEGAME_PATH` (the saves folder). No binary patch is
needed for this crash. Verified in runs r04–r06.

## Registry and files the game uses

**Observed (strings next to the key name, runtime runs):**

| Value | Type | Notes |
|---|---|---|
| `INSTALL_PATH` | REG_SZ | data root; required (see above) |
| `SAVEGAME_PATH` | REG_SZ | save folder (`SG`) |
| `LANGUAGE` | REG_SZ | `E` written by the game on first run; strings are `data\Strings\Whole_{E,F,G,I,S}.Lng` |
| `WIDTH`, `HEIGHT` | REG_DWORD | screen mode; the game writes 640×480 on first run; 1920×1080 set here renders at 1920×1080 (run r05) |
| `SELECTION` | REG_SZ | the Graphics options and the adapter's ids, 40 letters (decoded below, "The Graphics options reset") |
| `CD_KEY` | REG_SZ | online only (GameSpy, dead); openspy-client injects a fake one |

Other key read: `Software\Microsoft\Direct3D` (D3D debug settings).
PCGamingWiki: options in `<game>\SG\OPTIONS`; saves in `<game>\SG\`
(`BESTTIMES_GHOSTS`, `CAREER_SLOT_*`, `CHAMPIONSHIP_SLOT_*`). With
`SAVEGAME_PATH` they go to that folder instead (to be confirmed at runtime).

## Platform (imports)

D3D9 (`Direct3DCreate9`), DirectSound (ordinals 1 and 2, EAX via property
sets), DirectInput 8, ole32 COM (Windows Media / DirectShow for the
`.wmv` videos and `.wma` music), WSOCK32 (online), VERSION, GDI only for a
stock object. Math helpers come from one of three DLLs picked by CPU
(`MathCPU`, `MathPIII`, `Math3dNow`). `SetProcessAffinityMask` and
`SetThreadAffinityMask` are imported (multi-core handling to be checked).

Command-line switches (readme plus strings): `NOSOUND`, `NOHARDWARESOUND`,
`ALLOWHARDWARESOUND`, `NOEAX`, `BLENDEAX`, `NOVIDEO`, `LQVIDEO`, `HQVIDEO`,
`ANALOGUECONTROLS`, `STATICBUFFERS`, `FORCECLIPPING`, `RESETDISPLAY`,
`RESETCONTROLS`, `RESETNETWORK`, `MULTIDISPLAY1..3`, `ROTATEDDISPLAY`,
`WIDESCREENDISPLAY`, `TRIPLEBUFFER`, `NOVIDEOMEMORYCHECK`, `NOSUN` (1.1),
`FORCEHT`, `DISABLEHT`, `LONGPAUSE`, `SAFEMODE`, network `-HOST`/`-JOIN`/
`-PORT`/`-NAME`/`-PASSWORD`/`-PLAYERS`/`-HOSTNAME`.

## Advanced particles and `FORCEHT` (2026-11-09)

**Observed (REA, `cmr5.exe` above):** `FORCEHT` (string `0x5e9e1c`) sets
the switch byte `0xb6412b`; `0x588220` turns the switches into the
Hyper-Threading mode `0xb674c4`: `DISABLEHT` → 0, `FORCEHT` → 2, otherwise
the CPU detection at `0x599eb0` decides (only its "HT present" answer gives
2). Only two functions read the mode: `0x5880b0` (mode == 2), which the
Graphics menu (`0x42d980`) uses to enable or grey out the Advanced
particles row, and `0x5880c0` (mode == 2 *and* bit 31 of the option word
`0xb114b0`), which the particle code (`0x5ad2d3`, `0x5af31e`) asks. The
default option word `0xda082017` has bit 31 set.

**Community:** the GOG forum thread "advanced particles" (2009–2022)
quotes Codemasters' FAQ ("requires an Intel Pentium Processor with
Hyper-Threading enabled"), and players on Athlon 64 X2 and Core 2 Duo
found it greyed out; `FORCEHT` on the shortcut is the known fix (posts
from 2012, 2019, 2022). One poster reports that with Advanced particles on,
snow on the Swedish stages hides the road. The issue #1 reporter says the
same about AMD CPUs.

**Runtime (RUNTIME_NOTES.md, 2026-11-09):** on an AMD Ryzen the row is
greyed without the switch and selectable (On) with it; a stage runs.

**Decision:** the launcher passes `FORCEHT` unless `advanced_particles =
off`. It only makes the game's own option available; the in-game toggle
stays the player's.

## The Graphics options reset (`SELECTION`) (2026-11-09)

**Observed (REA):** `0x569fd0` reads `WIDTH`, `HEIGHT`, `LANGUAGE`,
`INSTALL_PATH`, `SAVEGAME_PATH`, `CD_KEY` and `SELECTION`; `0x569e10`
writes them back. `SELECTION` is 40 letters: the 20 bytes at
`0xb114b0`–`0xb114c3`, each byte as `'B' + low nibble`, `'X' - high
nibble`. The five words are the Graphics option bits (`0xb114b0`; bit 31
Advanced particles, bit 27 32-bit colour), more option bits (`0xb114b4`;
bit 26 the in-game widescreen, top nibble the adapter index), the refresh
choice (`0xb114b8`), and the adapter's D3D9 device and vendor id
(`0xb114bc` = identifier `+0x430`, `0xb114c0` = `+0x42c`).
`0x569d30` is the reset: defaults `0xda082017`, `0x0100026b`, `0x73c`,
the current adapter's ids, and (on the very first read) 640×480.

The display setup `0x58d1b0` calls the reset, then forces 640×480, when:

1. the saved vendor/device ids match no adapter (unless `RESETDISPLAY`),
2. or the mode list (`0x588850`: the adapter's 32-bit and 16-bit landscape
   modes, at most 128 entries and 32 refresh rates) has no
   `WIDTH`×`HEIGHT` in the saved colour depth (`0x588550` returns -1);

`0x478030` does the same when creating the device fails. Then it writes
the registry at once (`0x569e10`), so the player's options are gone for
good.

**Runtime:** the first start (no `SELECTION`: the ids read as unknown)
always falls back; a size the screen lacks resets everything; another
vendor id does too; DXVK and WineD3D report different device ids for the
same GPU (RUNTIME_NOTES.md, 2026-11-09).

**Inference:** this is the reporter's "savedata of all settings lost after
resolution change in the launcher": a size the game refused, or another
adapter (a hybrid iGPU/dGPU PC, a driver update, DXVK in or out), and the
game wipes its Graphics options itself. The first-start fallback with
`WIDESCREENDISPLAY` also explains the reported wrong menu aspect at 4K. On
Windows the launcher checks a size with `EnumDisplaySettings` (any depth
and refresh); the game's own list is D3D9's, capped at 128 entries, so a
4K monitor with many modes can list a size the game drops.

**Decision:** the launcher reads `SELECTION`, `WIDTH` and `HEIGHT` before
writing its values and again after the game exits. When the game ran at
640×480 with its default options instead of the size the launcher set, it
writes back the player's option words (with the card ids the game found)
and the previous size, and says why (a dialog only when the size was the
cause, the one the player must change). Seeding `SELECTION` before the
first start would need the adapter's D3D9 ids, which the SDK does not
expose (proposal in the 2026-11-09 report).

## Wheels: the virtual wheel and `SG/OPTIONS` (2026-11-13)

Static study of GOG `cmr5.exe` 1.1 (`a7ea2d33…`) for the SDK's wheel work
(rea-launcher-sdk `.work/cmr-wheels-plan.md`, seed study
`seed-cmr2005.md`); V = seen in the binary, U = not verified.

- **Input (V):** `EnumDevices(DI8DEVCLASS_GAMECTRL, ATTACHEDONLY)` opens
  up to 16 controllers in DirectInput's order, `c_dfDIJoystick2`,
  exclusive and foreground; every axis is set to `DIPROP_RANGE` 0..255
  by object ID (`0x584e63`), so steering has 256 steps. A device whose
  `dwDevType` is DRIVING or FLIGHT is the game's wheel type (type 2,
  `0x46dfe0`), the only one with separate pedals (`0x46e93e`).
  Force feedback is native (Spring, Damper, a Sine, Constant force on X
  and Y; `DIPROP_AUTOCENTER` toggled). No steering lock anywhere.
- **No community wheel fix** exists for this game (PCGamingWiki, the
  forums); the PCGW "Force Feedback Restore Module" is a Xidi setup for
  pad rumble only.
- **`SG/OPTIONS` has no checksum (V).** Saved by `0x401140` (the
  `0xe34e8` bytes at `0x8f9030`); the CRC-32 at `0x4067e0` covers a
  separate copy and is not written. Loading needs the exact size and the
  version byte. The 8-byte trailer is a random profile id (`+0xe34e0`)
  and the version `0x1b` with three flag bytes (`+0xe34e4`): a version
  mismatch makes the game delete the other save slots, so the launcher
  never writes it.
- **Controller records (V):** 18 per player at `0x60 + (player·18 +
  slot)·0x261c`; slot 0 keyboard, 1 mouse, 2–17 the controllers in
  `EnumDevices` order. Player 1's slot is the u32 at `0xab888`. A record
  keeps its device's product and vendor ids at `+0x74`/`+0x78`; when they
  don't match the device in its slot (`0x406a00`) the game resets the
  record's bindings and ids to that device's defaults (`0x46f0d0`) but
  **keeps byte 0, the calibration tables and the autocentre word
  `+0x253c`**.
- **Controls (V):** buttons 0–31, the hat 32/33, then the axes in
  `EnumObjects` order: the virtual wheel's X = 34, Y = 35, Z = 36,
  Rz = 37. Bindings at `+0x04..+0x70` (steering `+0x24`/`+0x28`,
  accelerate `+0x5c`, brake `+0x60`). Byte 0: bit 1 force feedback, bits
  2–4 strength 0–4 (`min(1, (s+1)·0.2)`, `0x4afc77`); byte 1: digital
  steering, digital pedals, inverted pedals.
- **Calibration (V, `0x5842e0`):** eight i32 per mapper entry at
  `+0x7c + entry·0x20` (offset, deadzone edges, saturation, 100, invert);
  steering is entry 7, accelerate 9, brake 10; applied only when the ids
  match. The launcher's seed: entry 7 `[0x7fff,0,0,-0x7fff,0x7fff,0,100,0]`
  (raw 0/127/128/255 → −32767/−128/129/32767, no deadzone), entries 9
  and 10 `[0,0,0,0,0xffff,0,100,0]` (released at the axis minimum).
- **The seed** (`controls::seed`): the game's own wheel defaults
  (`0x46ebd0`) with steering on X and the pedals on Z and Rz, byte 0
  `0x12` (force feedback on at full; the layer applies the player's
  strength), byte 1 0, autocentre off, ids 5245/1209, player 1 on slot 2.
  On the owner's `OPTIONS` it changes the same 48 bytes as the research's
  `seed.py`, byte for byte (`controls::tests::a_real_options_file_round_trips`
  with `CMR2005_OPTIONS`/`CMR2005_OPTIONS_SEEDED`).
- **Why the stash:** a start without the wheel puts the pad in slot 2; the
  game's reset would keep the seed's calibration, byte 0 and autocentre
  and apply them to the pad. The launcher puts the record it displaced
  back instead, and keeps the wheel's record (with the player's in-game
  changes) for the next wheel start (`controls`, `wheel-records`).
- **No `OPTIONS` before the first save (V):** a new player has no file,
  and building one is unverified (U), so the launcher seeds from the
  next start.

## Community work

| Project | What it does | Licence | Maintained | Needs | Decision |
|---|---|---|---|---|---|
| Official patch 1.1 (Codemasters, 2004) | white grass, flashing sun, `NOSUN`, texture option | proprietary | no | retail 1.0 | **Not needed**: the GOG build is 1.1 |
| "64-bit patch" `cmr5win64.zip` (Codemasters, 2005; PCGW file 2288) | Win64 compatibility for retail | proprietary | no | 1.1 retail | **Not used**: replaces `cmr5.exe` and restores `protect.dll` (StarForce) |
| `WIDESCREENDISPLAY` switch (built in) | Hor+ widescreen, HUD stretched (WSGF grade B) | n/a | n/a | any | **Configure**: candidate for the widescreen look, to be verified in-game |
| CMR3/04/2005 Force Feedback Restore Module (PCGW file 2873, v4.3.1, 2024-12) | a configured Xidi build: XInput rumble, Start/Back → Enter/Esc | Xidi is BSD-3; the package's own licence is not stated | yes (2024) | Win10/11 | **Build on Xidi directly** (as the family already does), with our own config; don't redistribute the PCGW zip |
| Xidi (samuelgr) | XInput controllers as DirectInput, force feedback | BSD-3 | yes | any | **Bundle** (family standard), mapping to be verified |
| `FORCEHT` switch (GOG forum, 2012–2022) | Advanced particles on non-HT CPUs | n/a | n/a | any | **Configure** (0.4.0): passed unless `advanced_particles = off` |
| DSOAL | EAX 4 over OpenAL Soft | LGPL | yes | any | **Later / optional**: keep a player's own `dsound.dll` (CMR4 rule); decide after audio is verified |
| DXVK | D3D9 → Vulkan | zlib | yes | any | **To test**: WineD3D already runs at ~150 fps headless; earlier DXVK failure was a Vulkan-instance error on an old setup |
| dgVoodoo2 | D3D wrapper (Windows) | freeware, no licence | yes | any | **Not planned**: no need found so far |
| "CMR2005Widescreen.zip" (MediaFire link in a 2025-10 MyAbandonware comment) | claims an unreleased widescreen patch | none | unknown | ? | **Not used**: no author, no licence, flagged as possible malware |
| anzz1/openspy-client | GameSpy replacement as `dinput8.dll` proxy | none in repo | yes | 1.1 | **Not used**: online play is out of scope; licence missing; conflicts with a DirectInput wrapper |
| kurtis2221/cmr05_hook | disables out-of-bounds car reset | none | no | 1.1 | **Not used**: gameplay mod |
| Strings-file fix (GameBurnWorld) | Service Area crash on mixed-language copies | none | no | pirated copies | **Not used**: our files are the original GOG set; watch for Wine bug 22232 (Service Area crash) at runtime |
| appoloin/install-scripts | runs the GOG setup under Wine after winetricks (directplay, directshow, lavfilters, wmp9/11, xact) | n/a | ? | GOG | **Not used**: the launcher avoids the installer and writes the registry itself |
| Wine bug 38198 | same crash, "fixed" by reinstalling | — | — | — | Explained above |

## The widescreen view (static analysis)

`WIDESCREENDISPLAY` sets the flag byte `0xb64132` (command-line parser at
`0x585fcb`); `IsWidescreen()` at `0x588170` returns that flag **or** bit 26
of the saved options word at `0xb114b4` (an in-game setting). The camera
code at `0x4670a0` and `0x467300` picks fixed scale pairs: 1.33 × 0.75
normally, 1.209 × 0.682 widescreen (ratio 1.773, i.e. 16:9), 0.76 × 1.011
for `ROTATEDDISPLAY`.

**Inference:** the widescreen view is exact only at 16:9. At 16:10 it is
about 10% too narrow; at 21:9 it is stretched by a third (run r12).

**Decision:** like CMR4, the launcher gives the widescreen view only at
16:9 sizes, and the Widescreen look uses the largest 16:9 size that fits
the screen. A patch that writes `0.682 × width/height` into the two
`0x3f9ac083` constants (at `0x4670d2` and `0x467332`) would make any
shape exact; it is a candidate for a later version, after checking the
cockpit and replay cameras.

## Old-game trouble spots

| Spot | Status |
|---|---|
| Missing install registry → crash | **Solved** (launcher writes the values) |
| Resolution / aspect | `WIDTH`/`HEIGHT` registry values work; `WIDESCREENDISPLAY` is exact at 16:9 (HUD fine), see above |
| Intro videos (WMV) | play under Wine 11.21 with GStreamer (run r07); music (WMA) not checked |
| Renderer speed | WineD3D: about 20 fps in a stage; DXVK: over 1000 (runs r08–r10) |
| Frame-rate independence | the stage timer keeps real time at 20–1450 fps |
| DirectSound3D / EAX | to check (audio went to a null sink; no errors); EAX needs DSOAL (proposal) |
| Graphics options reset to defaults | **Solved** (0.4.0): the launcher restores them after the game's own reset |
| Advanced particles greyed out (non-HT CPUs) | **Solved** (0.4.0): `FORCEHT` |
| Timers / frame pacing at 150+ fps | to check in a stage |
| Multi-core affinity | the game imports affinity APIs; to check |
| Alt+F4, window close | **ignored by the game** (runs r13–r14): the close hook is needed |
| Focus loss | to check |
| DirectInput controllers | pads: the game's own Controllers menu; wheels: the SDK's in-game layer (`rea_dinput.dll`, see "Wheels") |
| Service Area crash (Wine bug 22232) | to check |

## Installing the game for players

**Observed:** the owner's copy is the GOG *installer*, and that installer
hangs under Wine. On Windows players normally run the GOG installer, which
writes the registry values itself.

**Decision (owner, 2026-11-02):** the launcher accepts either an
installed game folder or the GOG installer files. From the installer it
unpacks the game into its own folder (an Inno Setup reader, pinned, no
manual step), so Linux and Steam Deck players never run `setup_colin2005.exe`.

### GOG's installer in the launcher (SDK v0.5.0, 2026-11-09)

| Item | Value |
|---|---|
| Setup program | `setup_colin2005.exe`, SHA-256 `690a5e3f203e84a49fa0bad1a5e9bb628adce3f0b5a6e0349409484b737a8240` |
| Data parts (exact sizes) | `setup_colin2005-1.bin` 2,097,272,064 bytes; `setup_colin2005-2.bin` 985,406,510 bytes |
| Format | Inno Setup 5.4.2, not passworded; GOG's official offline installer, not a repack |
| Unpacked game (`app\`) | 5,218 files, 3,564,462,813 bytes; `cmr5.exe` is the known GOG v1.1 build |
| Collisions / files the launcher bundles | none: innoextract's `--collisions=rename` leaves no `X#component` copies; the installer adds no wrapper or fix (`prefer` and `drop` are empty) |

**Observed (2026-11-09):** the Linux innoextract 1.9 and the launcher's
own code path (the SDK's `innoextract::unpack` with innoextract 1.9's
Windows build under Wine, `tests/real_installer.rs`) produced identical
folders (`diff -r`: only the launcher's `.rea-unpacked` marker differs).
There is no installed copy of this game to compare against (both of the
owner's folders are the installer); the reference is the Linux
innoextract's output, already checked against the GOG listing's
`cmr5.exe` MD5 above and played (RUNTIME_NOTES.md).

**Decision:** the unpacking code moved from this launcher into the SDK
(`rea_launcher::installer`, v0.5.0); the launcher keeps the installer
table, its game check and the rename of its 0.2 marker
(`.cmr2005-unpacked` → `.rea-unpacked`). After an unpack the `game`
setting points at the unpacked folder (unless it came from `--game`).

## Owner decisions (2026-11-02)

- Launcher id `cmr2005`, environment prefix `CMR2005_`, program
  `CMR2005Launcher.exe`, Linux helper `linux/cmr2005-play`.
- Game source: an installed folder **or** the GOG installer files (see above).
- Registry (0.4.1, SDK v0.6.0): the launcher writes the game's values to
  `HKCU\Software\Codemasters\Colin McRae Rally 2005` on Windows and Wine
  alike (`reg::write_verified`, read back), and the close hook DLL in the
  runtime `cmr5.exe`'s import table sends the game's opens of
  `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005` there
  (`REA_REGISTRY`). No admin rights, and no reliance on the VirtualStore,
  which the game sees only while UAC registry virtualization is on for it
  (rea-colin-mcrae#6). Before 0.4.1 the launcher wrote HKLM, falling back
  to the VirtualStore; the game's own values there (`LANGUAGE`, `WIDTH`,
  `HEIGHT`, `SELECTION`, `CD_KEY`) are carried over once.

## The problems the launcher will solve

1. The startup crash at `0x41DA27` (Wine and copied installs): the launcher
   writes the install and save paths the game needs.
2. The GOG installer hanging under Wine: the launcher unpacks the game
   from the installer itself.
3. 640×480 on first start: the resolution follows the player's screen
   (`WIDTH`/`HEIGHT`), with widescreen through the game's own
   `WIDESCREENDISPLAY` switch once verified.
4. Saves kept outside the runtime copy (`SAVEGAME_PATH`), mirrored safely.
5. Controllers: XInput pads with rumble through Xidi (to verify).
6. The rest of the trouble-spot list above, as each is verified.

## Sources

- PCGamingWiki, Colin McRae Rally 2005 (snapshot 2026-09-09): https://web.archive.org/web/20260909083522/https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_2005
- Wine bug 38198: https://bugs.winehq.org/show_bug.cgi?id=38198 (also 38193, 14988, 14971, 21899, 22232, 25038)
- WineHQ AppDB version 6421: https://appdb.winehq.org/objectManager.php?sClass=version&iId=6421
- GOG build listing: https://www.gogdb.org/product/1207658663/build/311
- distrobox-gaming issue #4: https://github.com/akitaonrails/distrobox-gaming/issues/4
- GOG forum, "advanced particles": https://www.gog.com/forum/colin_mcrae_rally_2005/advanced_particles
- https://github.com/anzz1/openspy-client, https://github.com/kurtis2221/cmr05_hook, https://github.com/appoloin/install-scripts
