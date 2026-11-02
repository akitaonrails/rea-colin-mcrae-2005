//! What a launch changes, worked out without touching anything: the
//! registry values, the game's arguments, the components in the runtime,
//! the generated files, Wine's DLL overrides and the game's environment.
//! `main` carries out exactly this plan, so the end-to-end option tests
//! and the golden snapshots (`tests/snapshots/`) see what a launch does.

use std::fmt::Write as _;
use std::path::Path;

use crate::components::{self, APP_OVERRIDES, CLOSE_HOOK_CONFIG, DXVK_DLL};
use crate::config::Picture;
use crate::registry::{self, RegWrite};
use crate::runtime::DXVK_CONF;

/// The command-line switch for the game's widescreen view.
pub const WIDESCREEN_ARG: &str = "WIDESCREENDISPLAY";

/// Everything one launch changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Values under `registry::KEY`.
    pub registry: Vec<RegWrite>,
    /// Arguments `cmr5.exe` gets.
    pub args: Vec<&'static str>,
    /// Bundled DLLs installed into the runtime.
    pub installed: Vec<&'static str>,
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
}

/// The plan for `picture` with the game running from `runtime` and saving
/// to `saves`, under Wine or not.
pub fn plan(picture: &Picture, runtime: &Path, saves: &Path, wine: bool) -> Plan {
    let mut args = Vec::new();
    if picture.widescreen() {
        args.push(WIDESCREEN_ARG);
    }
    let (installed, taken_out, dxvk_conf) = if wine {
        (
            vec![DXVK_DLL],
            Vec::new(),
            Some(components::dxvk_conf(picture)),
        )
    } else {
        (Vec::new(), vec![DXVK_DLL], None)
    };
    Plan {
        registry: registry::writes(runtime, saves, picture.size()),
        args,
        installed,
        taken_out,
        files: vec![(DXVK_CONF, dxvk_conf)],
        app_dll_overrides: if wine {
            APP_OVERRIDES.iter().map(|(dll, _)| *dll).collect()
        } else {
            Vec::new()
        },
        env: vec![(rea_close_hook::ENV, CLOSE_HOOK_CONFIG)],
        prefer_fast_gpu: !wine,
    }
}

impl Plan {
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
        let _ = writeln!(out, "prefer fast GPU: {}", self.prefer_fast_gpu);
        let _ = writeln!(out, "\n[registry HKLM\\{}]", registry::KEY);
        for write in &self.registry {
            let _ = writeln!(out, "{write}");
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
            Path::new("/r"),
            Path::new("/s"),
            true,
        );
        assert_eq!(plan.args, [WIDESCREEN_ARG]);
        assert_eq!(plan.registry.len(), 4);
        assert_eq!(plan.installed, [DXVK_DLL]);
        assert_eq!(plan.app_dll_overrides, ["d3d9"]);
        assert!(!plan.prefer_fast_gpu);
    }

    #[test]
    fn original_keeps_the_games_mode_and_view() {
        let plan = plan(
            &picture(Resolution::Original),
            Path::new("/r"),
            Path::new("/s"),
            false,
        );
        assert!(plan.args.is_empty());
        assert_eq!(plan.registry.len(), 2, "only the two paths");
        assert_eq!(plan.taken_out, [DXVK_DLL]);
        assert_eq!(plan.files, [(DXVK_CONF, None)]);
        assert!(plan.app_dll_overrides.is_empty());
        assert!(plan.prefer_fast_gpu);
    }

    #[test]
    fn other_shapes_keep_the_4_by_3_view() {
        for size in [(1280, 800), (2560, 1080), (1024, 768)] {
            let plan = plan(
                &picture(Resolution::Size(size.0, size.1)),
                Path::new("/r"),
                Path::new("/s"),
                true,
            );
            assert!(plan.args.is_empty(), "{size:?}");
        }
    }
}
