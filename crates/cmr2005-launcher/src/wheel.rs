//! The in-game wheel layer (the SDK's `rea_dinput.dll`) on a start:
//! whether the game is shown the virtual wheel, and what the layer gets.
//!
//! The layer wraps the game's DirectInput and shows it one virtual wheel
//! made from the player's wheel and pedals (`rea_launcher::wheel::dinput`).
//! It reads `rea_wheel.ini` next to it in the runtime, made at each start
//! from the `wheel*` options (`launcher.cfg` and the flags) and the pedal
//! calibration the Wheel tab keeps in its own `rea_wheel.ini` beside
//! `launcher.cfg` ([`runtime_ini`]). The launcher decides before the
//! start whether the wheel will be shown ([`shown`], the rule the layer
//! applies too) and tells the layer through [`MODE_ENV`], so the game's
//! `OPTIONS` is seeded for the same devices the game will see
//! (`crate::controls`).

use rea_launcher::wheel::dinput::{HEADER, Mode, Profile, Setup};
use rea_launcher::wheel::rig::{self, Device};

/// The layer's DLL, next to the runtime's `cmr5.exe`, which imports it
/// (`rea_dinput::import_into`).
pub const DLL: &str = rea_dinput::DLL;

/// The layer's on/off for this start, in the game's environment.
pub const MODE_ENV: &str = rea_dinput::MODE_ENV;

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

/// The runtime's `rea_wheel.ini`: this start's `setup` and the
/// calibration file's text (`""`: none yet, the layer guesses one).
pub fn runtime_ini(setup: &Setup, calibration: &str) -> String {
    rig::save("", setup, Profile::from_cfg(calibration).as_ref(), HEADER)
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
    fn the_runtime_file_has_this_starts_setup_and_the_calibration() {
        let csl = device(Kind::Wheel);
        let profile = Profile::guess(csl.id, &csl.initial, None);
        let calibration = profile.to_cfg("# mine\n", "");
        let start = setup(Mode::Off);
        let text = runtime_ini(&start, &calibration);
        assert!(text.starts_with(&format!("# {HEADER}\n")), "{text}");
        assert_eq!(Setup::from_cfg(&text), start);
        assert_eq!(Profile::from_cfg(&text), Some(profile), "{text}");
        assert_eq!(Profile::from_cfg(&runtime_ini(&start, "")), None);
    }
}
