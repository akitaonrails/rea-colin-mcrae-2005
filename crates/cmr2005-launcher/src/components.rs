//! The bundled components (`components/` beside the launcher) and the
//! files the launcher generates for them.
//!
//! - DXVK's 32-bit `d3d9.dll` (zlib), under Wine and Proton only: Wine's
//!   own Direct3D drew a stage at about 20 fps, DXVK at well over 1000
//!   (docs/RUNTIME_NOTES.md). Installed into the runtime through the
//!   ledger, so a player's own `d3d9.dll` is never overwritten, and taken
//!   out again on Windows, where the game keeps Microsoft's Direct3D.
//! - `dxvk.conf`: the frame-rate cap and VSync.
//! - The SDK's close hook (`rea_close_hook.dll`): the game ignores Alt+F4
//!   and window-close requests (run r13/r14), so a close request for its
//!   window `PC_CMR5` ends it, and Alt+F4 read from the keyboard state
//!   (`alt_f4_state`) too. Installed next to the runtime's `cmr5.exe`,
//!   which imports it (`runtime::write_game_exe`): Windows loads it with
//!   the game and it attaches itself; the launcher never hooks the
//!   running game.
//! - innoextract (`innoextract.exe`, zlib), which unpacks GOG's installer
//!   (`rea_launcher_win32::innoextract`, [`crate::source`]); needed only
//!   when the game folder holds the installer, never copied into the
//!   runtime.

use std::path::Path;

use rea_launcher::identify;
use rea_launcher_win32::wine::DllMode;

use crate::config::Picture;

/// The folder beside the launcher's exe that holds the bundled files.
pub const DIR: &str = "components";

pub const DXVK_DLL: &str = "d3d9.dll";
pub const CLOSE_HOOK_DLL: &str = "rea_close_hook.dll";
/// `rea_launcher_win32::innoextract::EXE`.
pub const INNOEXTRACT: &str = "innoextract.exe";

/// The close hook's configuration, set on the game as `REA_CLOSE_HOOK`:
/// a close request (`SC_CLOSE` from the window's or the compositor's close,
/// or Alt+F4) for the game's window ends the process. `alt_f4_state`:
/// the game's exclusive DirectInput keyboard keeps Alt+F4 from its window,
/// so the hook reads it from the keyboard state while the window is in
/// front.
pub const CLOSE_HOOK_CONFIG: &str =
    "class=PC_CMR5;close_on=sc_close,alt_f4,alt_f4_state;strategy=terminate";

/// The runtime's record of the components the launcher installed.
pub const LEDGER: &str = ".cmr2005-components";

/// What each bundled file is, for `launcher.log`.
pub fn version(name: &str) -> &'static str {
    match name {
        DXVK_DLL => "DXVK 3.1.1",
        CLOSE_HOOK_DLL => "rea-launcher-close-hook",
        INNOEXTRACT => "innoextract 1.9",
        _ => "",
    }
}

/// The bundled files a launch needs: DXVK under Wine, the close hook
/// always, innoextract when the game is still GOG's installer.
pub fn needed(wine: bool, installer: bool) -> Vec<&'static str> {
    let mut names = Vec::new();
    if wine {
        names.push(DXVK_DLL);
    }
    names.push(CLOSE_HOOK_DLL);
    if installer {
        names.push(INNOEXTRACT);
    }
    names
}

/// The missing ones of [`needed`] in `components`, as a message.
pub fn validate(components: &Path, wine: bool, installer: bool) -> Result<(), String> {
    let missing: Vec<&str> = needed(wine, installer)
        .into_iter()
        .filter(|name| !components.join(name).is_file())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        let verb = if missing.len() == 1 { "is" } else { "are" };
        let names = match missing.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
            _ => missing.join(""),
        };
        Err(format!(
            "The launcher's {names} {verb} missing in {}. Unpack the whole launcher package again.",
            components.display()
        ))
    }
}

/// `name: SHA-256 (version)` of each bundled file there, for the log.
pub fn describe(components: &Path) -> Vec<String> {
    [DXVK_DLL, CLOSE_HOOK_DLL, INNOEXTRACT]
        .into_iter()
        .filter_map(|name| {
            let digest = identify::sha256_file(&components.join(name)).ok()?;
            Some(format!(
                "{name}: {} ({})",
                identify::hex(&digest),
                version(name)
            ))
        })
        .collect()
}

/// DXVK's settings for `picture`: the frame-rate cap (`0` is none) and
/// VSync (`presentInterval` 1 waits for the screen, 0 does not). An unset
/// value is left to DXVK (no cap; the game's own presentation).
pub fn dxvk_conf(picture: &Picture) -> String {
    let mut text = String::from(
        "# Written by CMR2005Launcher on every start; change the settings in the launcher.\n",
    );
    if let Some(fps) = picture.fps_limit {
        text.push_str(&format!("d3d9.maxFrameRate = {fps}\n"));
    }
    if let Some(vsync) = picture.vsync {
        text.push_str(&format!("d3d9.presentInterval = {}\n", u8::from(vsync)));
    }
    text
}

/// Wine's per-program DLL overrides for `cmr5.exe` (Wine's loader ignores
/// `WINEDLLOVERRIDES` in the environment the launcher gives the game):
/// DXVK's `d3d9` first, Wine's as the fallback.
pub const APP_OVERRIDES: [(&str, DllMode); 1] = [("d3d9", DllMode::NativeThenBuiltin)];

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::cfg::Resolution;
    use std::fs;

    #[test]
    fn dxvk_gets_the_cap_and_vsync() {
        let picture = Picture {
            resolution: Some(Resolution::Size(1280, 720)),
            fps_limit: Some(60),
            vsync: Some(true),
        };
        let text = dxvk_conf(&picture);
        assert!(text.contains("d3d9.maxFrameRate = 60\n"), "{text}");
        assert!(text.contains("d3d9.presentInterval = 1\n"), "{text}");
        let off = dxvk_conf(&Picture {
            fps_limit: Some(0),
            vsync: Some(false),
            ..picture
        });
        assert!(off.contains("d3d9.maxFrameRate = 0\n"), "{off}");
        assert!(off.contains("d3d9.presentInterval = 0\n"), "{off}");
        let unset = dxvk_conf(&Picture::default());
        assert!(!unset.contains("d3d9."), "{unset}");
    }

    #[test]
    fn dxvk_is_needed_only_under_wine_and_innoextract_only_for_the_installer() {
        assert_eq!(needed(true, false), [DXVK_DLL, CLOSE_HOOK_DLL]);
        assert_eq!(needed(false, false), [CLOSE_HOOK_DLL]);
        assert_eq!(needed(false, true), [CLOSE_HOOK_DLL, INNOEXTRACT]);
        assert_eq!(needed(true, true), [DXVK_DLL, CLOSE_HOOK_DLL, INNOEXTRACT]);
    }

    #[cfg(windows)]
    #[test]
    fn innoextract_has_the_sdks_name() {
        assert_eq!(INNOEXTRACT, rea_launcher_win32::innoextract::EXE);
    }

    #[test]
    fn missing_components_are_named() {
        let dir = std::env::temp_dir().join(format!("cmr2005-components-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let error = validate(&dir, true, false).unwrap_err();
        assert!(
            error.contains("d3d9.dll and rea_close_hook.dll are missing"),
            "{error}"
        );
        let error = validate(&dir, true, true).unwrap_err();
        assert!(
            error.contains("d3d9.dll, rea_close_hook.dll and innoextract.exe are missing"),
            "{error}"
        );
        fs::write(dir.join(CLOSE_HOOK_DLL), b"hook").unwrap();
        assert!(validate(&dir, false, false).is_ok());
        let error = validate(&dir, false, true).unwrap_err();
        assert!(
            error.contains("launcher's innoextract.exe is missing"),
            "{error}"
        );
        let error = validate(&dir, true, false).unwrap_err();
        assert!(error.contains("launcher's d3d9.dll is missing"), "{error}");
        let described = describe(&dir);
        assert_eq!(described.len(), 1);
        assert!(
            described[0].starts_with("rea_close_hook.dll: "),
            "{described:?}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_close_hook_configuration_parses() {
        let config = rea_close_hook::Config::parse(CLOSE_HOOK_CONFIG);
        assert!(config.is_ok(), "{config:?}");
    }

    /// The game takes the keyboard through DirectInput in exclusive mode,
    /// so Alt+F4 never reaches its window as a message (runs r13, e06,
    /// e09): the hook also reads it from the keyboard state. The window's
    /// and the compositor's close stay on.
    #[test]
    fn the_close_hook_also_reads_alt_f4_from_the_keyboard_state() {
        let requests = rea_close_hook::Config::parse(CLOSE_HOOK_CONFIG)
            .unwrap()
            .requests;
        assert_eq!(
            requests,
            rea_close_hook::Requests {
                sc_close: true,
                alt_f4: true,
                wm_close: false,
                alt_f4_state: true,
            }
        );
    }
}
