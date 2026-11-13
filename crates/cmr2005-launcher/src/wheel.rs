//! The in-game wheel layer (the SDK's `rea_dinput.dll`) on a start:
//! whether the game is shown the virtual wheel, and what the layer gets.
//!
//! The layer wraps the game's DirectInput and shows it one virtual wheel
//! made from the player's wheel and pedals (`rea_launcher::wheel::dinput`).
//! It reads `rea_wheel.ini` next to it in the runtime: the Wheel tab's
//! file with the `wheel*` options of this start on top
//! ([`runtime_ini`]). The launcher decides before the start whether the
//! wheel will be shown ([`shown`], the rule the layer applies too) and
//! tells the layer through [`MODE_ENV`], so the game's `OPTIONS` is seeded
//! for the same devices the game will see (`crate::controls`).

use rea_launcher::wheel::dinput::{Mode, Setup};
use rea_launcher::wheel::rig::{self, Device};

/// The layer's DLL, next to the runtime's `cmr5.exe`, which imports it.
pub const DLL: &str = "rea_dinput.dll";

/// The comment a new `rea_wheel.ini` starts with, as the window writes it
/// (`rea_launcher_ui::wheel::HEADER`; that crate builds only for Windows
/// and the tests here).
pub const HEADER: &str = "rea_wheel.ini - the wheel settings, written by the launcher";

/// The layer's on/off for this start, in the game's environment.
pub const MODE_ENV: &str = "REA_WHEEL_MODE";

/// Whether the game is shown the virtual wheel: `On` and `Off` as set;
/// `Auto` asks `attached` for the controllers plugged in now (only then:
/// listing them takes a moment) and shows it with a wheel among them.
pub fn shown(setup: &Setup, attached: impl FnOnce() -> Vec<Device>) -> bool {
    match setup.mode {
        Mode::Auto => rig::shows_virtual_wheel(setup, &attached()),
        mode => mode == Mode::On,
    }
}

/// [`MODE_ENV`]'s value: the launcher's decision, so the layer never
/// shows a wheel the launcher didn't seed `OPTIONS` for.
pub fn mode_value(shown: bool) -> &'static str {
    if shown { "on" } else { "off" }
}

/// The runtime's `rea_wheel.ini`: the Wheel tab's file (`ini`, its
/// calibration and other keys kept) with this start's `setup`.
pub fn runtime_ini(ini: &str, setup: &Setup) -> String {
    setup.to_cfg(ini, HEADER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::wheel::{DeviceId, DeviceState, Kind};

    fn device(kind: Kind) -> Device {
        let state = DeviceState {
            axes: vec![0; 8],
            buttons: vec![false; 12],
        };
        Device {
            id: DeviceId {
                name: "a controller".into(),
                vid: 0x0eb7,
                pid: 0x0020,
                guid: "{a}".into(),
            },
            kind,
            axes: 8,
            buttons: 12,
            ffb: kind == Kind::Wheel,
            initial: state.clone(),
            state,
        }
    }

    fn setup(mode: Mode) -> Setup {
        Setup {
            mode,
            ..Setup::default()
        }
    }

    #[test]
    fn auto_shows_the_wheel_only_with_a_wheel_attached() {
        assert!(shown(&setup(Mode::Auto), || vec![device(Kind::Wheel)]));
        assert!(!shown(&setup(Mode::Auto), || vec![device(Kind::Pad)]));
        assert!(!shown(&setup(Mode::Auto), Vec::new));
    }

    #[test]
    fn on_and_off_never_list_the_controllers() {
        let unlisted = || -> Vec<Device> { panic!("listed") };
        assert!(shown(&setup(Mode::On), unlisted));
        assert!(!shown(&setup(Mode::Off), unlisted));
        assert_eq!((mode_value(true), mode_value(false)), ("on", "off"));
    }

    #[test]
    fn the_runtime_file_has_this_starts_settings_and_keeps_the_rest() {
        let ini = "# mine\nmode = auto\nwheel_vid = 3767\nlock = 540\n";
        let mut start = Setup::from_cfg(ini);
        start.mode = Mode::Off;
        let text = runtime_ini(ini, &start);
        assert!(text.starts_with("# mine\n"), "{text}");
        assert!(text.contains("wheel_vid = 3767"), "the calibration stays");
        assert_eq!(Setup::from_cfg(&text), start);
        assert!(runtime_ini("", &start).starts_with(&format!("# {HEADER}\n")));
    }

    #[test]
    fn the_header_is_the_windows() {
        assert_eq!(HEADER, rea_launcher_ui::wheel::HEADER);
    }
}
