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
| `SELECTION` | REG_SZ | 40-letter string the game writes (options packed as text; not decoded yet) |
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

## Community work

| Project | What it does | Licence | Maintained | Needs | Decision |
|---|---|---|---|---|---|
| Official patch 1.1 (Codemasters, 2004) | white grass, flashing sun, `NOSUN`, texture option | proprietary | no | retail 1.0 | **Not needed**: the GOG build is 1.1 |
| "64-bit patch" `cmr5win64.zip` (Codemasters, 2005; PCGW file 2288) | Win64 compatibility for retail | proprietary | no | 1.1 retail | **Not used**: replaces `cmr5.exe` and restores `protect.dll` (StarForce) |
| `WIDESCREENDISPLAY` switch (built in) | Hor+ widescreen, HUD stretched (WSGF grade B) | n/a | n/a | any | **Configure**: candidate for the widescreen look, to be verified in-game |
| CMR3/04/2005 Force Feedback Restore Module (PCGW file 2873, v4.3.1, 2024-12) | a configured Xidi build: XInput rumble, Start/Back → Enter/Esc | Xidi is BSD-3; the package's own licence is not stated | yes (2024) | Win10/11 | **Build on Xidi directly** (as the family already does), with our own config; don't redistribute the PCGW zip |
| Xidi (samuelgr) | XInput controllers as DirectInput, force feedback | BSD-3 | yes | any | **Bundle** (family standard), mapping to be verified |
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
| DirectSound3D / EAX | to check (audio went to a null sink; no errors) |
| Timers / frame pacing at 150+ fps | to check in a stage |
| Multi-core affinity | the game imports affinity APIs; to check |
| Alt+F4, window close | **ignored by the game** (runs r13–r14): the close hook is needed |
| Focus loss | to check |
| DirectInput controllers | to check (Xidi) |
| Service Area crash (Wine bug 22232) | to check |

## Installing the game for players

**Observed:** the owner's copy is the GOG *installer*, and that installer
hangs under Wine. On Windows players normally run the GOG installer, which
writes the registry values itself.

**Decision (owner, 2026-11-02):** the launcher accepts either an
installed game folder or the GOG installer files. From the installer it
unpacks the game into its own folder (an Inno Setup reader, pinned, no
manual step), so Linux and Steam Deck players never run `setup_colin2005.exe`.

## Owner decisions (2026-11-02)

- Launcher id `cmr2005`, environment prefix `CMR2005_`, program
  `CMR2005Launcher.exe`, Linux helper `linux/cmr2005-play`.
- Game source: an installed folder **or** the GOG installer files (see above).
- Registry on Windows: the launcher writes `INSTALL_PATH`/`SAVEGAME_PATH`
  per user, under `HKCU\Software\Classes\VirtualStore\MACHINE\SOFTWARE\WOW6432Node\CODEMASTERS\Colin McRae Rally 2005`,
  which a non-elevated 32-bit program without a manifest reads in place of
  HKLM. No admin rights. Wine has no registry virtualization, so this needs
  one check on a real Windows PC; inside the launcher's own Wine prefix it
  writes HKLM directly.

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
- https://github.com/anzz1/openspy-client, https://github.com/kurtis2221/cmr05_hook, https://github.com/appoloin/install-scripts
