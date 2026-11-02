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
//!   window `PC_CMR5` ends it.
//! - innoextract (`innoextract.exe`, zlib), which unpacks GOG's installer
//!   ([`crate::source`]); it is never copied into the runtime.

use std::path::Path;

use rea_launcher::identify;
use rea_launcher_win32::wine::DllMode;

use crate::config::Picture;

/// The folder beside the launcher's exe that holds the bundled files.
pub const DIR: &str = "components";

pub const DXVK_DLL: &str = "d3d9.dll";
pub const CLOSE_HOOK_DLL: &str = "rea_close_hook.dll";
pub const INNOEXTRACT: &str = "innoextract.exe";

/// The close hook's configuration, set on the game as `REA_CLOSE_HOOK`:
/// a close request (`SC_CLOSE` from the window's or the compositor's close,
/// or Alt+F4) for the game's window ends the process.
pub const CLOSE_HOOK_CONFIG: &str = "class=PC_CMR5;strategy=terminate";

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
/// always.
pub fn needed(wine: bool) -> Vec<&'static str> {
    let mut names = Vec::new();
    if wine {
        names.push(DXVK_DLL);
    }
    names.push(CLOSE_HOOK_DLL);
    names
}

/// The missing ones of [`needed`] in `components`, as a message.
pub fn validate(components: &Path, wine: bool) -> Result<(), String> {
    let missing: Vec<&str> = needed(wine)
        .into_iter()
        .filter(|name| !components.join(name).is_file())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        let verb = if missing.len() == 1 { "is" } else { "are" };
        Err(format!(
            "The launcher's {} {verb} missing in {}. Unpack the whole launcher package again.",
            missing.join(" and "),
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
    fn dxvk_is_needed_only_under_wine() {
        assert_eq!(needed(true), [DXVK_DLL, CLOSE_HOOK_DLL]);
        assert_eq!(needed(false), [CLOSE_HOOK_DLL]);
    }

    #[test]
    fn missing_components_are_named() {
        let dir = std::env::temp_dir().join(format!("cmr2005-components-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let error = validate(&dir, true).unwrap_err();
        assert!(
            error.contains("d3d9.dll and rea_close_hook.dll are missing"),
            "{error}"
        );
        fs::write(dir.join(CLOSE_HOOK_DLL), b"hook").unwrap();
        assert!(validate(&dir, false).is_ok());
        let error = validate(&dir, true).unwrap_err();
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
}
