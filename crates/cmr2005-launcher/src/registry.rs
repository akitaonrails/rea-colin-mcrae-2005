//! The game's registry key and what a launch writes there.
//!
//! CMR2005 builds every data path from `INSTALL_PATH` and its save folder
//! from `SAVEGAME_PATH`; without them it looks in
//! `C:\Program Files\Codemasters\Colin McRae Rally 2005`, loads no fonts and
//! crashes at `0x41DA27` (docs/RESEARCH.md). So both are written on every
//! launch, pointing at the launcher's private copy and saves folder. The
//! screen mode is `WIDTH`/`HEIGHT`, which the game also writes itself
//! (640×480 on its first start).
//!
//! On Windows the key is HKLM (32-bit view); without the rights to write
//! it the launcher writes the per-user VirtualStore copy, which the game
//! reads instead (`rea_launcher_win32::reg::Key::machine_or_virtual_store`).

use std::fmt;
use std::path::Path;

/// Under `HKEY_LOCAL_MACHINE`, 32-bit view.
pub const KEY: &str = r"SOFTWARE\CODEMASTERS\Colin McRae Rally 2005";

pub const INSTALL_PATH: &str = "INSTALL_PATH";
pub const SAVEGAME_PATH: &str = "SAVEGAME_PATH";
pub const WIDTH: &str = "WIDTH";
pub const HEIGHT: &str = "HEIGHT";

/// One value under [`KEY`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegWrite {
    pub name: &'static str,
    pub value: RegValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    String(String),
    Dword(u32),
}

impl fmt::Display for RegWrite {
    /// `NAME = string:"…"` or `NAME = dword:N`, for the log and snapshots.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            RegValue::String(text) => write!(f, "{} = string:{text:?}", self.name),
            RegValue::Dword(n) => write!(f, "{} = dword:{n}", self.name),
        }
    }
}

/// A folder as the game wants it: no trailing separator (it appends
/// `\fonts\…` itself).
pub fn folder(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned()
}

/// The values a launch writes: both paths always; the screen mode only
/// when the launcher sets one (`size`), so Original keeps the game's own.
pub fn writes(runtime: &Path, saves: &Path, size: Option<(u32, u32)>) -> Vec<RegWrite> {
    let mut out = vec![
        RegWrite {
            name: INSTALL_PATH,
            value: RegValue::String(folder(runtime)),
        },
        RegWrite {
            name: SAVEGAME_PATH,
            value: RegValue::String(folder(saves)),
        },
    ];
    if let Some((width, height)) = size {
        out.push(RegWrite {
            name: WIDTH,
            value: RegValue::Dword(width),
        });
        out.push(RegWrite {
            name: HEIGHT,
            value: RegValue::Dword(height),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_paths_are_always_written_without_a_trailing_separator() {
        let writes = writes(
            Path::new(r"C:\Users\José\AppData\Local\CMR2005Launcher\runtime\"),
            Path::new(r"C:\Users\José\AppData\Local\CMR2005Launcher\saves"),
            None,
        );
        let text: Vec<String> = writes.iter().map(ToString::to_string).collect();
        assert_eq!(
            text,
            [
                r#"INSTALL_PATH = string:"C:\\Users\\José\\AppData\\Local\\CMR2005Launcher\\runtime""#,
                r#"SAVEGAME_PATH = string:"C:\\Users\\José\\AppData\\Local\\CMR2005Launcher\\saves""#,
            ]
        );
    }

    #[test]
    fn the_screen_mode_is_written_only_when_the_launcher_sets_one() {
        let writes = writes(Path::new("/r"), Path::new("/s"), Some((1920, 1080)));
        assert_eq!(writes[2].to_string(), "WIDTH = dword:1920");
        assert_eq!(writes[3].to_string(), "HEIGHT = dword:1080");
        assert_eq!(writes.len(), 4);
    }
}
