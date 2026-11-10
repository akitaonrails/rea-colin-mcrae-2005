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
//! The game also keeps its Graphics options there, packed into `SELECTION`
//! ([`Selection`]). When it cannot use the screen mode it finds (a size
//! its mode list lacks, or a graphics card other than the one saved) it
//! starts at 640×480 and resets every one of those options to its defaults
//! (`0x58d1b0`, `0x569d30`; docs/RESEARCH.md); its very first start always
//! does (no card saved yet). [`after_run`] notices that after the game
//! exits and gives the player's options back.
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
pub const SELECTION: &str = "SELECTION";

/// The size the game falls back to.
pub const GAME_FALLBACK: (u32, u32) = (640, 480);

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

/// `SELECTION`: the game's five option words (`0xb114b0`–`0xb114c3`),
/// each byte written as two letters, low nibble first (`'B' + low`,
/// `'X' - high`), as `0x569e10` writes and `0x569fd0` reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Option bits, more option bits (the adapter index in the top
    /// nibble), the refresh choice, then the graphics card's device and
    /// vendor id as the game saw them.
    pub words: [u32; 5],
}

impl Selection {
    /// The first two words after the game's reset (`0x569d30`).
    const RESET_WORDS: [u32; 2] = [0xda08_2017, 0x0100_026b];

    pub fn parse(text: &str) -> Option<Selection> {
        let letters = text.as_bytes();
        if letters.len() != 40 {
            return None;
        }
        let mut bytes = [0u8; 20];
        for (byte, pair) in bytes.iter_mut().zip(letters.chunks(2)) {
            let low = pair[0].checked_sub(b'B').filter(|n| *n < 16)?;
            let high = b'X'.checked_sub(pair[1]).filter(|n| *n < 16)?;
            *byte = high << 4 | low;
        }
        let mut words = [0u32; 5];
        for (word, four) in words.iter_mut().zip(bytes.chunks(4)) {
            *word = u32::from_le_bytes([four[0], four[1], four[2], four[3]]);
        }
        Some(Selection { words })
    }

    pub fn text(&self) -> String {
        self.words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .flat_map(|byte| {
                [
                    char::from(b'B' + (byte & 15)),
                    char::from(b'X' - (byte >> 4)),
                ]
            })
            .collect()
    }

    /// Whether the option words are the game's defaults.
    pub fn is_reset(&self) -> bool {
        self.words[..2] == Self::RESET_WORDS
    }
}

/// The game's screen values, read before a launch and after the game exits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub selection: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Snapshot {
    fn size(&self) -> Option<(u32, u32)> {
        self.width.zip(self.height)
    }
}

/// Why the game fell back to 640×480.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Its very first start: no graphics card saved yet.
    FirstStart,
    /// It found another graphics card than the one saved (another GPU,
    /// driver or D3D9 layer); the next start uses the size.
    NewCard,
    /// Its mode list has no such size.
    Size,
}

/// What the launcher does once the game exits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterRun {
    Fine,
    /// The game ran at 640×480 instead of `wanted`; `restore` puts back
    /// its size from before and, when it reset them
    /// (`options_restored`), the player's Graphics options.
    Fallback {
        wanted: (u32, u32),
        cause: Cause,
        options_restored: bool,
        restore: Vec<RegWrite>,
    },
}

impl AfterRun {
    /// What the player is told, if anything.
    pub fn message(&self) -> Option<String> {
        let AfterRun::Fallback {
            wanted: (w, h),
            cause,
            options_restored,
            ..
        } = self
        else {
            return None;
        };
        let (fw, fh) = GAME_FALLBACK;
        let (reset, back) = if *options_restored {
            (
                " and reset its Graphics options",
                " The launcher put your graphics options back.",
            )
        } else {
            ("", "")
        };
        Some(match cause {
            Cause::FirstStart => format!(
                "On its first start the game always uses {fw}x{fh}; from the next start it uses {w}x{h}."
            ),
            Cause::NewCard => format!(
                "The game found another graphics card than last time, so it ran at {fw}x{fh}{reset}.\
                 {back} From the next start it uses {w}x{h}."
            ),
            Cause::Size => format!(
                "The game could not use {w}x{h}, so it ran at {fw}x{fh}{reset}.{back} Choose \
                 another Resolution, or the Original look, in the launcher's settings window."
            ),
        })
    }

    /// Whether the player has to change something (a dialog, not just the
    /// log): only a size the game cannot use comes back every start.
    pub fn needs_the_player(&self) -> bool {
        matches!(
            self,
            AfterRun::Fallback {
                cause: Cause::Size,
                ..
            }
        )
    }
}

/// Compares the game's values `before` the launch (read before the
/// launcher wrote `wanted`, `None` when it set no size) with those `after`
/// the game exited.
pub fn after_run(wanted: Option<(u32, u32)>, before: &Snapshot, after: &Snapshot) -> AfterRun {
    let Some(wanted) = wanted.filter(|size| *size != GAME_FALLBACK) else {
        return AfterRun::Fine;
    };
    let Some(reset) = after
        .selection
        .as_deref()
        .and_then(Selection::parse)
        .filter(|s| s.is_reset() && after.size() == Some(GAME_FALLBACK))
    else {
        return AfterRun::Fine;
    };
    let Some(players) = before.selection.as_deref().and_then(Selection::parse) else {
        return AfterRun::Fallback {
            wanted,
            cause: Cause::FirstStart,
            options_restored: false,
            restore: Vec::new(),
        };
    };
    let cause = if players.words[3..] == reset.words[3..] {
        Cause::Size
    } else {
        Cause::NewCard
    };
    let options_restored = !players.is_reset();
    let mut restore = Vec::new();
    if options_restored {
        // The player's options, with the graphics card the game found.
        let mut words = players.words;
        words[3..].copy_from_slice(&reset.words[3..]);
        restore.push(RegWrite {
            name: SELECTION,
            value: RegValue::String(Selection { words }.text()),
        });
    }
    if let Some((width, height)) = before.size() {
        restore.push(RegWrite {
            name: WIDTH,
            value: RegValue::Dword(width),
        });
        restore.push(RegWrite {
            name: HEIGHT,
            value: RegValue::Dword(height),
        });
    }
    AfterRun::Fallback {
        wanted,
        cause,
        options_restored,
        restore,
    }
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

    // SELECTION as the real game wrote it (docs/RUNTIME_NOTES.md, 2026-11-09,
    // runs r1–r3): options changed by the player, then the same after the
    // game fell back to 640x480 and reset them to its defaults.
    const PLAYERS: &str = "IWBVJXLKMRDXBXGXNUIXBXBXOLHXBXBXPKBWBXBX";
    const RESET: &str = "IWBVJXLKMRDXBXCXMUIXBXBXOLHXBXBXPKBWBXBX";

    fn snapshot(selection: Option<&str>, size: Option<(u32, u32)>) -> Snapshot {
        Snapshot {
            selection: selection.map(str::to_owned),
            width: size.map(|s| s.0),
            height: size.map(|s| s.1),
        }
    }

    fn shown(writes: &[RegWrite]) -> Vec<String> {
        writes.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn selection_reads_and_writes_the_games_option_words() {
        let reset = Selection::parse(RESET).unwrap();
        assert_eq!(
            reset.words,
            [0xda08_2017, 0x0100_026b, 0x73b, 0x6cd, 0x10de]
        );
        assert!(reset.is_reset());
        let players = Selection::parse(PLAYERS).unwrap();
        assert!(!players.is_reset());
        assert_eq!(players.text(), PLAYERS);
        assert_eq!(Selection::parse("short"), None);
        assert_eq!(Selection::parse(&RESET.replace('I', "a")), None);
    }

    fn fallback(outcome: &AfterRun) -> (Cause, bool, Vec<String>) {
        match outcome {
            AfterRun::Fallback {
                cause,
                options_restored,
                restore,
                ..
            } => (*cause, *options_restored, shown(restore)),
            AfterRun::Fine => panic!("no fallback"),
        }
    }

    #[test]
    fn a_size_the_game_refused_brings_the_players_options_back() {
        let before = snapshot(Some(PLAYERS), Some((1920, 1080)));
        let after = snapshot(Some(RESET), Some((640, 480)));
        let outcome = after_run(Some((2560, 1440)), &before, &after);
        assert_eq!(
            fallback(&outcome),
            (
                Cause::Size,
                true,
                vec![
                    format!("SELECTION = string:{PLAYERS:?}"),
                    "WIDTH = dword:1920".to_owned(),
                    "HEIGHT = dword:1080".to_owned(),
                ]
            )
        );
        let message = outcome.message().unwrap();
        assert!(message.contains("could not use 2560x1440"), "{message}");
        assert!(
            message.contains("put your graphics options back"),
            "{message}"
        );
        assert!(message.contains("Choose another Resolution"), "{message}");
        assert!(outcome.needs_the_player());
    }

    /// Seen in the real game (2026-11-09, run v2b): another graphics card
    /// (here DXVK reporting the real one where WineD3D reported another)
    /// resets the options as well; they come back with the new card's ids,
    /// so the next start keeps them.
    #[test]
    fn another_graphics_card_resets_the_options_and_they_come_back() {
        let old_card = Selection {
            words: [0xda08_2017, 0x0500_026b, 0x73c, 0x2b85, 0x1002],
        };
        let found = Selection {
            words: [0xda08_2017, 0x0100_026b, 0x73c, 0x2b85, 0x10de],
        };
        let before = snapshot(Some(&old_card.text()), Some((1920, 1080)));
        let after = snapshot(Some(&found.text()), Some((640, 480)));
        let outcome = after_run(Some((1920, 1080)), &before, &after);
        let (cause, restored, writes) = fallback(&outcome);
        assert_eq!((cause, restored), (Cause::NewCard, true));
        let back = Selection::parse(
            writes[0]
                .trim_start_matches("SELECTION = string:\"")
                .trim_end_matches('"'),
        )
        .unwrap();
        assert_eq!(
            back.words,
            [0xda08_2017, 0x0500_026b, 0x73c, 0x2b85, 0x10de]
        );
        let message = outcome.message().unwrap();
        assert!(message.contains("another graphics card"), "{message}");
        assert!(
            message.contains("From the next start it uses 1920x1080"),
            "{message}"
        );
        assert!(!outcome.needs_the_player());
    }

    #[test]
    fn options_already_at_their_defaults_only_get_the_size_back() {
        let before = snapshot(Some(RESET), Some((1920, 1080)));
        let after = snapshot(Some(RESET), Some((640, 480)));
        let outcome = after_run(Some((2560, 1440)), &before, &after);
        assert_eq!(
            fallback(&outcome),
            (
                Cause::Size,
                false,
                vec![
                    "WIDTH = dword:1920".to_owned(),
                    "HEIGHT = dword:1080".to_owned()
                ]
            )
        );
        let message = outcome.message().unwrap();
        assert!(message.contains("640x480"), "{message}");
        assert!(
            !message.contains("put your graphics options back"),
            "{message}"
        );
    }

    #[test]
    fn the_games_first_start_is_640x480_whatever_the_launcher_set() {
        let after = snapshot(Some(RESET), Some((640, 480)));
        let outcome = after_run(Some((1920, 1080)), &Snapshot::default(), &after);
        assert_eq!(fallback(&outcome), (Cause::FirstStart, false, Vec::new()));
        let message = outcome.message().unwrap();
        assert!(
            message.contains("from the next start it uses 1920x1080"),
            "{message}"
        );
        assert!(!outcome.needs_the_player());
    }

    #[test]
    fn a_game_that_used_the_size_or_was_left_alone_changes_nothing() {
        let before = snapshot(Some(PLAYERS), Some((1920, 1080)));
        let kept = snapshot(Some(PLAYERS), Some((2560, 1440)));
        assert_eq!(
            after_run(Some((2560, 1440)), &before, &kept),
            AfterRun::Fine
        );
        let reset = snapshot(Some(RESET), Some((640, 480)));
        assert_eq!(after_run(None, &before, &reset), AfterRun::Fine, "Original");
        assert_eq!(after_run(Some((640, 480)), &before, &reset), AfterRun::Fine);
        // Changed in the game to its defaults while still at the size set.
        let in_game = snapshot(Some(RESET), Some((2560, 1440)));
        assert_eq!(
            after_run(Some((2560, 1440)), &before, &in_game),
            AfterRun::Fine
        );
        assert_eq!(AfterRun::Fine.message(), None);
    }
}
