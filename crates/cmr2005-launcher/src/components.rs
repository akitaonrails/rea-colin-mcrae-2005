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
//! - The SDK's wheel layer (`rea_dinput.dll`), imported by the runtime's
//!   `cmr5.exe` after the close hook: it shows the game one virtual wheel
//!   made from the player's wheel and pedals (`crate::wheel`); off, it
//!   hands the game its DirectInput untouched.
//! - Xidi 5.0.0 (BSD-3-Clause): `dinput8.dll` and the `Xidi.32.dll` it
//!   loads, with the Microsoft VC++ runtime DLLs Xidi needs, and our
//!   `Xidi.ini` ([`XIDI_CONF`]), with the `controller` option on. The game
//!   has no XInput: Xidi shows it an XInput pad as a DirectInput one, with
//!   the triggers on axes of their own and the game's force feedback on the
//!   pad's motors. The wheel layer sits in front of it (it calls the
//!   `DirectInput8Create` the game's import resolved to, Xidi's here).
//!   Installed through the ledger, so a player's own `dinput8.dll` or
//!   `Xidi.ini` in the private copy stays theirs.
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
pub const WHEEL_DLL: &str = crate::wheel::DLL;
/// `rea_launcher_win32::innoextract::EXE`.
pub const INNOEXTRACT: &str = "innoextract.exe";

/// Xidi 5's DirectInput wrapper, which the game loads in place of the
/// system's.
pub const XIDI_DLL: &str = "dinput8.dll";
/// Xidi 5's main binary: the wrapper loads it from beside the game.
pub const XIDI_MAIN_DLL: &str = "Xidi.32.dll";
/// The Microsoft VC++ 2015–2022 runtime DLLs `Xidi.32.dll` imports.
pub const VC_DLLS: [&str; 4] = [
    "msvcp140.dll",
    "msvcp140_atomic_wait.dll",
    "concrt140.dll",
    "vcruntime140.dll",
];
/// Everything Xidi needs next to `cmr5.exe`.
pub const XIDI_DLLS: [&str; 6] = [
    XIDI_DLL,
    XIDI_MAIN_DLL,
    VC_DLLS[0],
    VC_DLLS[1],
    VC_DLLS[2],
    VC_DLLS[3],
];
/// Xidi's settings, next to its DLL.
pub const XIDI_INI: &str = "Xidi.ini";

/// Xidi's view of the pad (golden copy: `tests/fixtures/xidi.ini`).
///
/// The layout stays Xidi's StandardGamepad (left stick X/Y, right stick
/// Z/RotZ, the hat, buttons 1–12), so the game's own defaults for a pad
/// apply. Only these change:
/// - The triggers get axes of their own, RT on RotX and LT on RotY, each
///   centred at rest. For a pad (not a driving device) the game reads
///   accelerate from the half of its axis below the centre and brake from
///   the half above it (`cmr5.exe` 0x46e9bd and 0x46e995, modes 3 and 2
///   at 0x47b928/0x47b944), so RT moves RotX down and LT moves RotY up;
///   both pedals can be pressed at once. `controls` binds them.
/// - Start also presses Escape, the game's pause; its button stays, so
///   the button numbering the game stored doesn't move.
/// - The game's force feedback reaches both motors: each takes the
///   strength of an effect on X and Y (`MagnitudeProjection`).
/// - Only the first pad is shown (`ActiveVirtualControllerMask`): the game
///   binds one controller per player.
pub const XIDI_CONF: &str = r"[Mapper]
Type = CMR2005Gamepad

[CustomMapper:CMR2005Gamepad]
Template = StandardGamepad
TriggerRT = Axis(RotX, -)
TriggerLT = Axis(RotY, +)
ButtonStart = Compound(Button(10), Keyboard(Escape))
ForceFeedback.LeftMotor = MagnitudeProjection(X, Y)
ForceFeedback.RightMotor = MagnitudeProjection(X, Y)

[Properties]
UseBuiltInProperties = yes
DeadzonePercentStickLeft = 5
DeadzonePercentStickRight = 5
DeadzonePercentTriggerLT = 0
DeadzonePercentTriggerRT = 0
SaturationPercentStickLeft = 100
SaturationPercentStickRight = 100
SaturationPercentTriggerLT = 100
SaturationPercentTriggerRT = 100

[Log]
Enabled = no
Level = 1

[Workarounds]
ActiveVirtualControllerMask = 0x01
";

/// Wine's per-program DLL overrides for `cmr5.exe`, all native-then-builtin
/// (Wine's loader ignores `WINEDLLOVERRIDES` in the environment the
/// launcher gives the game): DXVK's `d3d9` …
pub const DXVK_OVERRIDES: [&str; 1] = ["d3d9"];
/// … and, with the gamepad on, Xidi's `dinput8` and the VC++ runtime it
/// imports (Wine has its own of each).
pub const XIDI_OVERRIDES: [&str; 5] = [
    "dinput8",
    "msvcp140",
    "msvcp140_atomic_wait",
    "concrt140",
    "vcruntime140",
];
/// The mode of every override the launcher sets: the bundled DLL first,
/// Wine's as the fallback.
pub const OVERRIDE_MODE: DllMode = DllMode::NativeThenBuiltin;

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
        WHEEL_DLL => "rea-launcher-dinput",
        INNOEXTRACT => "innoextract 1.9",
        XIDI_DLL | XIDI_MAIN_DLL => "Xidi 5.0.0",
        name if VC_DLLS.contains(&name) => "Microsoft VC++ 2015-2022 runtime, x86",
        _ => "",
    }
}

/// The bundled files a launch needs: DXVK under Wine, the close hook and
/// the wheel layer always, Xidi with the gamepad on, innoextract when the
/// game is still GOG's installer.
pub fn needed(wine: bool, installer: bool, gamepad: bool) -> Vec<&'static str> {
    let mut names = Vec::new();
    if wine {
        names.push(DXVK_DLL);
    }
    names.push(CLOSE_HOOK_DLL);
    names.push(WHEEL_DLL);
    if gamepad {
        names.extend(XIDI_DLLS);
    }
    if installer {
        names.push(INNOEXTRACT);
    }
    names
}

/// The missing ones of [`needed`] in `components`, as a message.
pub fn validate(
    components: &Path,
    wine: bool,
    installer: bool,
    gamepad: bool,
) -> Result<(), String> {
    let missing: Vec<&str> = needed(wine, installer, gamepad)
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
    [DXVK_DLL, CLOSE_HOOK_DLL, WHEEL_DLL, INNOEXTRACT]
        .into_iter()
        .chain(XIDI_DLLS)
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
        assert_eq!(
            needed(true, false, false),
            [DXVK_DLL, CLOSE_HOOK_DLL, WHEEL_DLL]
        );
        assert_eq!(needed(false, false, false), [CLOSE_HOOK_DLL, WHEEL_DLL]);
        assert_eq!(
            needed(false, true, false),
            [CLOSE_HOOK_DLL, WHEEL_DLL, INNOEXTRACT]
        );
        assert_eq!(
            needed(true, true, false),
            [DXVK_DLL, CLOSE_HOOK_DLL, WHEEL_DLL, INNOEXTRACT]
        );
    }

    #[test]
    fn xidi_and_its_runtime_are_needed_only_with_the_gamepad_on() {
        let on = needed(false, false, true);
        assert_eq!(on[..2], [CLOSE_HOOK_DLL, WHEEL_DLL]);
        assert_eq!(
            on[2..],
            [
                "dinput8.dll",
                "Xidi.32.dll",
                "msvcp140.dll",
                "msvcp140_atomic_wait.dll",
                "concrt140.dll",
                "vcruntime140.dll"
            ]
        );
        assert!(!needed(true, true, false).contains(&XIDI_DLL));
        assert_eq!(version(XIDI_MAIN_DLL), "Xidi 5.0.0");
        assert!(version("concrt140.dll").contains("VC++"));
    }

    /// The generated file is reviewed like code: a change here must be
    /// made to `tests/fixtures/xidi.ini` too.
    #[test]
    fn xidi_ini_matches_the_golden_fixture() {
        assert_eq!(XIDI_CONF, include_str!("../tests/fixtures/xidi.ini"));
    }

    /// The pad as the game needs it: the layout of StandardGamepad, the
    /// triggers on the two spare axes (accelerate below the centre, brake
    /// above), Start also Escape with its button kept, both motors fed by
    /// the force on X and Y, and one pad. Xidi 5 finds the system's
    /// DirectInput itself: an `[Import]` section would make it ignore the
    /// whole file (CMR04, RUNTIME_NOTES 2026-11-09).
    #[test]
    fn xidi_keeps_the_pad_layout_with_separate_trigger_axes_and_rumble() {
        for line in [
            "Template = StandardGamepad",
            "TriggerRT = Axis(RotX, -)",
            "TriggerLT = Axis(RotY, +)",
            "ButtonStart = Compound(Button(10), Keyboard(Escape))",
            "ForceFeedback.LeftMotor = MagnitudeProjection(X, Y)",
            "ForceFeedback.RightMotor = MagnitudeProjection(X, Y)",
            "DeadzonePercentTriggerRT = 0",
            "ActiveVirtualControllerMask = 0x01",
        ] {
            assert!(XIDI_CONF.lines().any(|l| l == line), "{line}");
        }
        assert!(!XIDI_CONF.contains("[Import]"));
        assert!(
            !XIDI_CONF.lines().any(|l| l.starts_with("Stick")),
            "the sticks stay the template's"
        );
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
        let error = validate(&dir, true, false, false).unwrap_err();
        assert!(
            error.contains("d3d9.dll, rea_close_hook.dll and rea_dinput.dll are missing"),
            "{error}"
        );
        let error = validate(&dir, true, true, false).unwrap_err();
        assert!(
            error.contains(
                "d3d9.dll, rea_close_hook.dll, rea_dinput.dll and innoextract.exe are missing"
            ),
            "{error}"
        );
        fs::write(dir.join(WHEEL_DLL), b"layer").unwrap();
        let error = validate(&dir, false, false, false).unwrap_err();
        assert!(
            error.contains("launcher's rea_close_hook.dll is missing"),
            "{error}"
        );
        fs::write(dir.join(CLOSE_HOOK_DLL), b"hook").unwrap();
        assert!(validate(&dir, false, false, false).is_ok());
        let error = validate(&dir, false, false, true).unwrap_err();
        assert!(
            error.contains("dinput8.dll, Xidi.32.dll, msvcp140.dll, msvcp140_atomic_wait.dll, concrt140.dll and vcruntime140.dll are missing"),
            "{error}"
        );
        for name in XIDI_DLLS {
            fs::write(dir.join(name), name).unwrap();
        }
        assert!(validate(&dir, false, false, true).is_ok());
        let error = validate(&dir, false, true, false).unwrap_err();
        assert!(
            error.contains("launcher's innoextract.exe is missing"),
            "{error}"
        );
        let error = validate(&dir, true, false, false).unwrap_err();
        assert!(error.contains("launcher's d3d9.dll is missing"), "{error}");
        let described = describe(&dir);
        assert_eq!(described.len(), 8);
        assert!(
            described[2].starts_with("dinput8.dll: ") && described[2].ends_with("(Xidi 5.0.0)"),
            "{described:?}"
        );
        assert!(
            described[0].starts_with("rea_close_hook.dll: "),
            "{described:?}"
        );
        assert!(
            described[1].ends_with("(rea-launcher-dinput)"),
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
