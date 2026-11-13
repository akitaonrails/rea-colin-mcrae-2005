//! What a launch changes, worked out without touching anything: the
//! registry values, the game's arguments, the components in the runtime,
//! the generated files, Wine's DLL overrides and the game's environment.
//! `main` carries out exactly this plan, so the end-to-end option tests
//! and the golden snapshots (`tests/snapshots/`) see what a launch does.

use std::fmt::Write as _;
use std::path::Path;

use crate::components::{self, APP_OVERRIDES, CLOSE_HOOK_CONFIG, CLOSE_HOOK_DLL, DXVK_DLL};
use crate::config::Picture;
use rea_launcher::wheel::dinput;
use rea_launcher_win32::reg::Value;

use crate::registry;
use crate::runtime::DXVK_CONF;
use crate::wheel;

/// The command-line switch for the game's widescreen view.
pub const WIDESCREEN_ARG: &str = "WIDESCREENDISPLAY";

/// The switch that makes the game take the Hyper-Threading path it was
/// built for, the only one where its Advanced particles option is offered
/// (`0x5880b0`/`0x5880c0` test mode 2, which `FORCEHT` sets; docs/RESEARCH.md).
pub const FORCE_HT_ARG: &str = "FORCEHT";

/// Everything one launch changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Values under `HKCU\registry::USER_KEY`, which the game reads
    /// through the close hook's redirect (`registry::REDIRECT`).
    pub registry: Vec<Value>,
    /// Arguments `cmr5.exe` gets.
    pub args: Vec<&'static str>,
    /// Bundled DLLs installed into the runtime.
    pub installed: Vec<&'static str>,
    /// DLLs added to the import table of the runtime's `cmr5.exe`
    /// (`runtime::write_game_exe`), loaded by Windows with the game.
    pub exe_imports: Vec<&'static str>,
    /// The launcher's DLLs taken out of the runtime (the game's own come back).
    pub taken_out: Vec<&'static str>,
    /// Generated runtime files: (name, contents); `None` removes it.
    pub files: Vec<(&'static str, Option<String>)>,
    /// Wine only: `cmr5.exe`'s per-program DLL overrides, set when absent.
    pub app_dll_overrides: Vec<&'static str>,
    /// The game's environment additions.
    pub env: Vec<(&'static str, &'static str)>,
    /// Windows only: the fast-GPU preference for the runtime's exe.
    pub prefer_fast_gpu: bool,
    /// Under Wine outside gamescope with a portrait main monitor: the game
    /// runs in a Wine virtual desktop of this size, the one landscape
    /// screen it then sees (it only looks at the main monitor's modes).
    pub virtual_desktop: Option<(u32, u32)>,
}

/// The name of the Wine virtual desktop (`explorer /desktop=NAME,WxH`).
pub const DESKTOP_NAME: &str = "CMR2005";

/// Where the game should run. `monitors`: what Windows (Wine) reports;
/// `gamescope`: inside gamescope, which shows Wine one landscape screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The size the looks fit to.
    pub size: Option<(u32, u32)>,
    /// Whether the game needs a virtual desktop (Wine, no gamescope, a
    /// portrait main monitor beside a landscape one).
    pub virtual_desktop: bool,
}

pub fn screen(monitors: &[rea_launcher::display::Monitor], wine: bool, gamescope: bool) -> Screen {
    use rea_launcher::display;
    match display::game_monitor(monitors) {
        Some(index) => Screen {
            size: Some(monitors[index].size()),
            virtual_desktop: wine && !gamescope,
        },
        None => Screen {
            size: display::primary(monitors).map(display::Monitor::size),
            virtual_desktop: false,
        },
    }
}

/// `explorer.exe`'s arguments for the game in the virtual desktop.
pub fn desktop_args(size: (u32, u32), exe: &str, args: &[&str]) -> Vec<String> {
    let mut out = vec![
        format!("/desktop={DESKTOP_NAME},{}x{}", size.0, size.1),
        exe.to_owned(),
    ];
    out.extend(args.iter().map(|arg| (*arg).to_owned()));
    out
}

/// The plan for `picture` (and the game's Advanced particles unlocked or
/// not) with the game running from `runtime` and saving to `saves`, under
/// Wine or not.
pub fn plan(
    picture: &Picture,
    advanced_particles: bool,
    runtime: &Path,
    saves: &Path,
    wine: bool,
) -> Plan {
    plan_on(picture, advanced_particles, runtime, saves, wine, None)
}

/// [`plan`] with the virtual desktop's screen size when one is needed
/// (`desktop`: the landscape monitor's size).
pub fn plan_on(
    picture: &Picture,
    advanced_particles: bool,
    runtime: &Path,
    saves: &Path,
    wine: bool,
    desktop: Option<(u32, u32)>,
) -> Plan {
    let mut args = Vec::new();
    if picture.widescreen() {
        args.push(WIDESCREEN_ARG);
    }
    if advanced_particles {
        args.push(FORCE_HT_ARG);
    }
    // The close hook always: the runtime's cmr5.exe imports it, so it
    // must sit next to it.
    let (installed, taken_out, dxvk_conf) = if wine {
        (
            vec![DXVK_DLL, CLOSE_HOOK_DLL],
            Vec::new(),
            Some(components::dxvk_conf(picture)),
        )
    } else {
        (vec![CLOSE_HOOK_DLL], vec![DXVK_DLL], None)
    };
    Plan {
        registry: registry::writes(runtime, saves, picture.size()),
        args,
        installed,
        exe_imports: vec![CLOSE_HOOK_DLL],
        taken_out,
        files: vec![(DXVK_CONF, dxvk_conf)],
        app_dll_overrides: if wine {
            APP_OVERRIDES.iter().map(|(dll, _)| *dll).collect()
        } else {
            Vec::new()
        },
        // The hook DLL, loaded through the import table, installs itself
        // on the game's window thread (`ATTACH_ENV`): the launcher never
        // touches the running game. It also sends the game's opens of its
        // HKLM key to the HKCU key the launcher writes.
        env: vec![
            (rea_close_hook::ENV, CLOSE_HOOK_CONFIG),
            (rea_close_hook::ATTACH_ENV, "1"),
            (rea_close_hook::registry::ENV, registry::REDIRECT),
        ],
        prefer_fast_gpu: !wine,
        // The desktop is the game's whole screen: its mode, else the monitor.
        virtual_desktop: desktop
            .filter(|_| wine)
            .map(|screen| picture.size().unwrap_or(screen)),
    }
}

impl Plan {
    /// This plan with the in-game wheel layer's part (`crate::wheel`):
    /// `rea_wheel.ini` (`ini`) in the runtime, and whether the game is
    /// shown the virtual wheel in its environment.
    pub fn with_wheel(mut self, shown: bool, ini: String) -> Plan {
        self.files.push((dinput::FILE, Some(ini)));
        self.env.push((wheel::MODE_ENV, wheel::mode_value(shown)));
        self
    }

    /// The plan as text, for the golden snapshots and the log.
    pub fn render(&self) -> String {
        let list = |items: &[&str]| {
            if items.is_empty() {
                "(none)".to_owned()
            } else {
                items.join(", ")
            }
        };
        let mut out = String::new();
        let _ = writeln!(out, "arguments: {}", list(&self.args));
        let _ = writeln!(out, "components installed: {}", list(&self.installed));
        let _ = writeln!(out, "components taken out: {}", list(&self.taken_out));
        let _ = writeln!(out, "cmr5.exe imports: {}", list(&self.exe_imports));
        let _ = writeln!(out, "prefer fast GPU: {}", self.prefer_fast_gpu);
        if let Some((w, h)) = self.virtual_desktop {
            let _ = writeln!(out, "Wine virtual desktop: {DESKTOP_NAME},{w}x{h}");
        }
        let _ = writeln!(out, "\n[registry HKCU\\{}]", registry::USER_KEY);
        for value in &self.registry {
            let _ = writeln!(out, "{}", registry::shown(value));
        }
        if !self.app_dll_overrides.is_empty() {
            let _ = writeln!(
                out,
                "\n[Wine per-program DLL overrides for cmr5.exe, set when absent]"
            );
            for dll in &self.app_dll_overrides {
                let _ = writeln!(out, "{dll} = native,builtin");
            }
        }
        let _ = writeln!(out, "\n[environment]");
        for (name, value) in &self.env {
            let _ = writeln!(out, "{name}={value}");
        }
        for (name, contents) in &self.files {
            match contents {
                Some(text) => {
                    let _ = write!(out, "\n[file {name}]\n{text}");
                }
                None => {
                    let _ = writeln!(out, "\n[file {name}: removed]");
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::cfg::Resolution;

    fn picture(resolution: Resolution) -> Picture {
        Picture {
            resolution: Some(resolution),
            fps_limit: None,
            vsync: None,
        }
    }

    #[test]
    fn a_16_by_9_size_gets_the_widescreen_view_and_its_mode() {
        let plan = plan(
            &picture(Resolution::Size(1920, 1080)),
            false,
            Path::new("/r"),
            Path::new("/s"),
            true,
        );
        assert_eq!(plan.args, [WIDESCREEN_ARG]);
        assert_eq!(plan.registry.len(), 4);
        assert_eq!(plan.installed, [DXVK_DLL, CLOSE_HOOK_DLL]);
        assert_eq!(plan.app_dll_overrides, ["d3d9"]);
        assert!(!plan.prefer_fast_gpu);
    }

    #[test]
    fn original_keeps_the_games_mode_and_view() {
        let plan = plan(
            &picture(Resolution::Original),
            false,
            Path::new("/r"),
            Path::new("/s"),
            false,
        );
        assert!(plan.args.is_empty());
        assert_eq!(plan.registry.len(), 2, "only the two paths");
        assert_eq!(plan.taken_out, [DXVK_DLL]);
        assert_eq!(plan.installed, [CLOSE_HOOK_DLL], "next to cmr5.exe");
        assert_eq!(plan.files, [(DXVK_CONF, None)]);
        assert!(plan.app_dll_overrides.is_empty());
        assert!(plan.prefer_fast_gpu);
    }

    #[test]
    fn a_portrait_main_monitor_gets_a_virtual_desktop_outside_gamescope() {
        use rea_launcher::display::Monitor;
        let portrait = Monitor {
            width: 2160,
            height: 3840,
            primary: true,
        };
        let landscape = Monitor {
            width: 3840,
            height: 2160,
            primary: false,
        };
        let both = [portrait, landscape];
        assert_eq!(
            screen(&both, true, false),
            Screen {
                size: Some((3840, 2160)),
                virtual_desktop: true
            }
        );
        assert!(
            !screen(&both, true, true).virtual_desktop,
            "gamescope shows one screen"
        );
        assert!(
            !screen(&both, false, false).virtual_desktop,
            "Windows: not Wine"
        );
        assert_eq!(screen(&both, false, false).size, Some((3840, 2160)));
        let usual = [
            Monitor {
                primary: true,
                ..landscape
            },
            Monitor {
                primary: false,
                ..portrait
            },
        ];
        assert_eq!(
            screen(&usual, true, false),
            Screen {
                size: Some((3840, 2160)),
                virtual_desktop: false
            }
        );
        let only_portrait = [portrait];
        assert!(!screen(&only_portrait, true, false).virtual_desktop);

        let plan = plan_on(
            &picture(Resolution::Size(2560, 1440)),
            true,
            Path::new("/r"),
            Path::new("/s"),
            true,
            Some((3840, 2160)),
        );
        assert_eq!(plan.virtual_desktop, Some((2560, 1440)), "the game's mode");
        let original = plan_on(
            &picture(Resolution::Original),
            true,
            Path::new("/r"),
            Path::new("/s"),
            true,
            Some((3840, 2160)),
        );
        assert_eq!(
            original.virtual_desktop,
            Some((3840, 2160)),
            "else the monitor"
        );
        assert_eq!(
            desktop_args((2560, 1440), r"C:\r\cmr5.exe", &["WIDESCREENDISPLAY"]),
            [
                "/desktop=CMR2005,2560x1440",
                r"C:\r\cmr5.exe",
                "WIDESCREENDISPLAY"
            ]
        );
    }

    /// The close hook is loaded with the game, never installed from the
    /// launcher (antivirus heuristics take that for a keylogger): the
    /// runtime's cmr5.exe imports it, the DLL sits next to it, and the
    /// game's environment tells it to attach itself.
    #[test]
    fn the_close_hook_is_imported_by_the_game_and_attaches_itself() {
        for wine in [true, false] {
            let plan = plan(
                &picture(Resolution::Original),
                true,
                Path::new("/r"),
                Path::new("/s"),
                wine,
            );
            assert_eq!(plan.exe_imports, [CLOSE_HOOK_DLL]);
            assert!(plan.installed.contains(&CLOSE_HOOK_DLL), "wine: {wine}");
            assert!(plan.env.contains(&(rea_close_hook::ENV, CLOSE_HOOK_CONFIG)));
            assert!(plan.env.contains(&(rea_close_hook::ATTACH_ENV, "1")));
            assert!(
                plan.env
                    .contains(&(rea_close_hook::registry::ENV, registry::REDIRECT)),
                "the game reads its key from HKCU (wine: {wine})"
            );
        }
        assert_eq!(CLOSE_HOOK_DLL, rea_close_hook::DLL);
    }

    /// `FORCEHT` unlocks the game's Advanced particles, which it otherwise
    /// offers only on a 2004 Intel Hyper-Threading CPU (docs/RESEARCH.md).
    #[test]
    fn advanced_particles_pass_the_hyper_threading_switch() {
        let run = |resolution, particles| {
            plan(
                &picture(resolution),
                particles,
                Path::new("/r"),
                Path::new("/s"),
                false,
            )
            .args
        };
        assert_eq!(run(Resolution::Original, true), [FORCE_HT_ARG]);
        assert!(run(Resolution::Original, false).is_empty());
        assert_eq!(
            run(Resolution::Size(1920, 1080), true),
            [WIDESCREEN_ARG, FORCE_HT_ARG]
        );
        assert_eq!(FORCE_HT_ARG, "FORCEHT");
    }

    /// The layer reads its file next to it and is told whether to show
    /// the wheel, the launcher's decision.
    #[test]
    fn the_wheel_layer_gets_its_file_and_the_decision() {
        for shown in [true, false] {
            let plan = plan(
                &picture(Resolution::Original),
                true,
                Path::new("/r"),
                Path::new("/s"),
                true,
            )
            .with_wheel(shown, "mode = auto\n".into());
            assert!(
                plan.files
                    .contains(&(dinput::FILE, Some("mode = auto\n".into())))
            );
            assert!(
                plan.env
                    .contains(&(wheel::MODE_ENV, wheel::mode_value(shown)))
            );
        }
    }

    #[test]
    fn other_shapes_keep_the_4_by_3_view() {
        for size in [(1280, 800), (2560, 1080), (1024, 768)] {
            let plan = plan(
                &picture(Resolution::Size(size.0, size.1)),
                false,
                Path::new("/r"),
                Path::new("/s"),
                true,
            );
            assert!(plan.args.is_empty(), "{size:?}");
        }
    }
}
