# Third-party components and assets

Nothing from Colin McRae Rally 2005 itself is part of this repository or its packages.
Every component below is fetched from a pinned URL and checked by SHA-256
(`packaging/third-party.txt`), or is an asset with its licence recorded.

| Component | Version | What it does here | Licence | Source | Bundled? |
|---|---|---|---|---|---|
| [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk) | v0.3.3 | Shared launcher code, Linux helper, packaging | MIT OR Apache-2.0 | https://github.com/akitaonrails/rea-launcher-sdk | compiled in, `sdk/` |
| Barlow fonts | from the SDK | The settings window's type | SIL OFL 1.1 (`licenses/Barlow-OFL.txt`) | https://github.com/jpt/barlow | compiled in |
| [DXVK](https://github.com/doitsujin/dxvk) | 3.1.1 | `components/d3d9.dll` (only `x32/d3d9.dll` of the release): Direct3D 9 on Vulkan, installed into the private copy under Wine and Proton | zlib (`licenses/DXVK-LICENSE.txt`) | https://github.com/doitsujin/dxvk/releases/tag/v3.1.1 | zip, AppImage |
| [innoextract](https://constexpr.org/innoextract/) | 1.9 (official Windows build) | `components/innoextract.exe`: unpacks GOG's installer | zlib; built with Boost (BSL-1.0), liblzma (public domain) and libstdc++ (GPL-3.0 with the runtime exception), texts in `licenses/innoextract/` | https://github.com/dscharrer/innoextract/releases/tag/1.9 | zip, AppImage |
| rea-launcher-close-hook (from the SDK) | v0.3.3 | `components/rea_close_hook.dll`: a window close ends the game | MIT OR Apache-2.0 | https://github.com/akitaonrails/rea-launcher-sdk | zip, AppImage |
| Wine | AppImage only (SDK `packaging/appimage/tools.pins`) | Runs the launcher in the AppImage | LGPL-2.1-or-later | SDK `packaging/appimage/wine/SOURCE.txt` | AppImage |

## Images

| File | Author | Licence | Source |
|---|---|---|---|
| (the window's hero image, once researched) | | | |

## Owner decisions

- 2026-11-02: the launcher accepts GOG's installer files and unpacks them
  with innoextract (the installer hangs under Wine). No DRM is involved:
  the GOG build is DRM-free. The "64-bit patch" is never used (it brings
  back StarForce's `protect.dll`).
