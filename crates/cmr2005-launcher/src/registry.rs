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
//! The game opens `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005`
//! (`RegCreateKeyExA`, full access) and reads and writes its values there.
//! An unelevated launcher on Windows can't write HKLM, and the VirtualStore
//! copy Windows makes instead reaches the game only while UAC registry
//! virtualization is on for it, which Windows doesn't promise
//! (akitaonrails/rea-colin-mcrae#6). So the launcher writes [`USER_KEY`]
//! under HKCU, on Windows and Wine alike, reads it back
//! (`rea_launcher_win32::reg::write_verified`), and the close hook DLL in
//! the game's import table sends the game's opens of [`MACHINE_KEY`] there
//! ([`REDIRECT`], `REA_REGISTRY`). The game's own values come over once
//! from where it read them before ([`carry_over`]).

use std::fmt;
use std::path::Path;

use rea_close_hook::registry::{Config, ConfigError};
use rea_launcher_win32::reg::{Data, Value, When};

/// The key the game opens, under `HKEY_LOCAL_MACHINE` (32-bit view).
pub const MACHINE_KEY: &str = r"SOFTWARE\CODEMASTERS\Colin McRae Rally 2005";

/// Where the launcher writes the game's values and the game reads them
/// through [`REDIRECT`], under `HKEY_CURRENT_USER`.
pub const USER_KEY: &str = r"Software\Codemasters\Colin McRae Rally 2005";

/// The close hook's registry rule (`rea_close_hook::registry::ENV`): the
/// game's opens of [`MACHINE_KEY`] go to [`USER_KEY`].
pub const REDIRECT: &str = r"HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 2005 -> HKCU\Software\Codemasters\Colin McRae Rally 2005";

pub const INSTALL_PATH: &str = "INSTALL_PATH";
pub const SAVEGAME_PATH: &str = "SAVEGAME_PATH";
pub const WIDTH: &str = "WIDTH";
pub const HEIGHT: &str = "HEIGHT";
pub const SELECTION: &str = "SELECTION";
pub const LANGUAGE: &str = "LANGUAGE";
pub const CD_KEY: &str = "CD_KEY";

/// The type of a value the game keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    String,
    Dword,
}

/// The game's own values (`0x569fd0` reads them, `0x569e10` writes them
/// back), carried over from the old key by [`carry_over`].
pub const GAME_VALUES: [(&str, Kind); 5] = [
    (LANGUAGE, Kind::String),
    (WIDTH, Kind::Dword),
    (HEIGHT, Kind::Dword),
    (SELECTION, Kind::String),
    (CD_KEY, Kind::String),
];

/// The size the game falls back to.
pub const GAME_FALLBACK: (u32, u32) = (640, 480);

/// [`REDIRECT`] parsed, as the DLL will read it.
pub fn redirect() -> Result<Config, ConfigError> {
    Config::parse(REDIRECT)
}

/// `NAME = string:"…"` or `NAME = dword:N` (`, if missing` for a value
/// only seeded), for the log and snapshots.
pub fn shown(value: &Value) -> String {
    let data = match &value.data {
        Data::String(text) => format!("string:{text:?}"),
        Data::Dword(n) => format!("dword:{n}"),
    };
    match value.when {
        When::Always => format!("{} = {data}", value.name),
        When::IfMissing => format!("{} = {data}, if missing", value.name),
    }
}

/// Values as one log line.
pub fn described(values: &[Value]) -> String {
    values.iter().map(shown).collect::<Vec<_>>().join("; ")
}

fn always(name: &str, data: Data) -> Value {
    Value::new(name, data, When::Always)
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
pub fn writes(runtime: &Path, saves: &Path, size: Option<(u32, u32)>) -> Vec<Value> {
    let mut out = vec![
        always(INSTALL_PATH, Data::String(folder(runtime))),
        always(SAVEGAME_PATH, Data::String(folder(saves))),
    ];
    if let Some((width, height)) = size {
        out.push(always(WIDTH, Data::Dword(width)));
        out.push(always(HEIGHT, Data::Dword(height)));
    }
    out
}

/// `values` plus the game's own `old` ones (read from where it kept them
/// before: the VirtualStore or HKLM), seeded when [`USER_KEY`] lacks them,
/// so a player keeps the language, Graphics options and screen mode. A
/// value the launch sets itself wins.
pub fn carry_over(mut values: Vec<Value>, old: &[(&str, Data)]) -> Vec<Value> {
    for (name, data) in old {
        if !values.iter().any(|value| value.name == *name) {
            values.push(Value::new(*name, data.clone(), When::IfMissing));
        }
    }
    values
}

/// Why the game would not find its values, and what to do: Play and
/// `--check` stop with it, never "ready" when the game wouldn't see them.
/// `problems`: the read-back's mismatches, a missing close hook DLL or
/// import, rules that don't parse.
pub fn not_ready(problems: &[impl fmt::Display]) -> String {
    let list: Vec<String> = problems.iter().map(|p| format!("- {p}")).collect();
    format!(
        "Not ready: the game would not find its settings in HKEY_CURRENT_USER\\{USER_KEY}:\n{}\n\n\
         The launcher keeps them there, per user (no administrator rights needed), and the game \
         reads them through rea_close_hook.dll. If a file is missing, unpack the whole launcher \
         package again; if the registry could not be written, allow CMR2005Launcher.exe to \
         change it (security software, a registry policy). Then start it again. If it keeps \
         failing, report it with launcher.log at \
         https://github.com/akitaonrails/rea-colin-mcrae-2005/issues",
        list.join("\n")
    )
}

/// `--check`'s registry line: ready only without `problems`.
pub fn check(problems: &[String]) -> Result<String, String> {
    if problems.is_empty() {
        Ok(format!(
            "The game's settings are in HKEY_CURRENT_USER\\{USER_KEY} (no administrator rights needed)."
        ))
    } else {
        Err(not_ready(problems))
    }
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
    /// The screen values among the game's `values` ([`GAME_VALUES`] as
    /// read from its key).
    pub fn of(values: &[(&str, Data)]) -> Snapshot {
        let mut snapshot = Snapshot::default();
        for (name, data) in values {
            match (*name, data) {
                (SELECTION, Data::String(text)) => snapshot.selection = Some(text.clone()),
                (WIDTH, Data::Dword(n)) => snapshot.width = Some(*n),
                (HEIGHT, Data::Dword(n)) => snapshot.height = Some(*n),
                _ => {}
            }
        }
        snapshot
    }

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
        restore: Vec<Value>,
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
        restore.push(always(SELECTION, Data::String(Selection { words }.text())));
    }
    if let Some((width, height)) = before.size() {
        restore.push(always(WIDTH, Data::Dword(width)));
        restore.push(always(HEIGHT, Data::Dword(height)));
    }
    AfterRun::Fallback {
        wanted,
        cause,
        options_restored,
        restore,
    }
}

#[cfg(windows)]
pub use self::windows::{GAME, GameKey, Written};

/// Reading and writing the game's key (Windows and Wine;
/// `sdk/scripts/wine-test` runs the tests).
#[cfg(windows)]
mod windows {
    use rea_launcher_win32::reg::{self, Hive, Key, Mismatch};

    use super::{Data, GAME_VALUES, Kind, MACHINE_KEY, Snapshot, USER_KEY, Value};

    /// Where the launcher writes the game's values (`user`, under HKCU)
    /// and where the game kept them before (`machine`, HKLM or its
    /// VirtualStore copy). [`GAME`] is the game's; tests use others.
    #[derive(Debug, Clone, Copy)]
    pub struct GameKey<'a> {
        pub user: &'a str,
        pub machine: &'a str,
    }

    pub const GAME: GameKey<'static> = GameKey {
        user: USER_KEY,
        machine: MACHINE_KEY,
    };

    /// What [`GameKey::write`] did.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Written {
        /// Everything written, the carried-over values included.
        pub values: Vec<Value>,
        /// The game's screen values from before, for `after_run`.
        pub before: Snapshot,
        /// The game's old values were carried over (the HKCU key was new).
        pub carried: bool,
    }

    fn game_values(
        read: impl Fn(&str, Kind) -> Result<Option<Data>, reg::Error>,
    ) -> Vec<(&'static str, Data)> {
        GAME_VALUES
            .iter()
            .filter_map(|&(name, kind)| Some((name, read(name, kind).ok().flatten()?)))
            .collect()
    }

    impl GameKey<'_> {
        /// The game's values in the HKCU key; `None` when there is no key.
        pub fn current(&self) -> Option<Vec<(&'static str, Data)>> {
            let key = Key::open_read(Hive::CurrentUser, self.user).ok()??;
            Some(game_values(|name, kind| match kind {
                Kind::String => key.string(name).map(|v| v.map(Data::String)),
                Kind::Dword => key.dword(name).map(|v| v.map(Data::Dword)),
            }))
        }

        /// The game's values where it kept them before (the VirtualStore
        /// copy, else HKLM), read-only.
        pub fn old(&self) -> Vec<(&'static str, Data)> {
            let Ok(old) = Key::read_machine(self.machine) else {
                return Vec::new();
            };
            game_values(|name, kind| match kind {
                Kind::String => old.string(name).map(|v| v.map(Data::String)),
                Kind::Dword => old.dword(name).map(|v| v.map(Data::Dword)),
            })
        }

        /// Writes `values` to the HKCU key, with the game's old values
        /// ([`super::carry_over`]) while that key doesn't exist yet, and
        /// reads them back ([`reg::write_verified`]). Never writes HKLM or
        /// the VirtualStore.
        pub fn write(&self, values: Vec<Value>) -> Result<Written, Vec<Mismatch>> {
            let (values, game, carried) = match self.current() {
                Some(current) => (values, current, false),
                None => {
                    let old = self.old();
                    let carried = !old.is_empty();
                    (super::carry_over(values, &old), old, carried)
                }
            };
            reg::write_verified(Hive::CurrentUser, self.user, &values)?;
            Ok(Written {
                values,
                before: Snapshot::of(&game),
                carried,
            })
        }

        /// The game's screen values now.
        pub fn snapshot(&self) -> Snapshot {
            Snapshot::of(&self.current().unwrap_or_default())
        }

        /// Writes `values` as they are, read back.
        pub fn set(&self, values: &[Value]) -> Result<(), Vec<Mismatch>> {
            reg::write_verified(Hive::CurrentUser, self.user, values)
        }
    }

    #[cfg(test)]
    mod tests {
        use std::process::Command;

        use rea_launcher_win32::reg::{When, windows_is_64_bit};

        use super::*;
        use crate::registry::{
            HEIGHT, INSTALL_PATH, LANGUAGE, SELECTION, WIDTH, after_run, shown, writes,
        };

        const PLAYERS: &str = "IWBVJXLKMRDXBXGXNUIXBXBXOLHXBXBXPKBWBXBX";
        const RESET: &str = "IWBVJXLKMRDXBXCXMUIXBXBXOLHXBXBXPKBWBXBX";

        /// A scratch key in HKCU and HKLM, removed again (also when the
        /// test fails): the wine-test prefix is kept between runs.
        struct Scratch {
            user: String,
            machine: String,
            store: String,
        }

        impl Scratch {
            fn new(name: &str) -> Scratch {
                let leaf = format!("rea-cmr2005-test-{name}-{}", std::process::id());
                let machine = format!(r"SOFTWARE\{leaf}");
                Scratch {
                    user: format!(r"Software\{leaf}"),
                    store: reg::virtual_store_path(&machine, windows_is_64_bit()),
                    machine,
                }
            }

            fn key(&self) -> GameKey<'_> {
                GameKey {
                    user: &self.user,
                    machine: &self.machine,
                }
            }
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                for key in [
                    format!(r"HKCU\{}", self.user),
                    format!(r"HKCU\{}", self.store),
                    format!(r"HKLM\{}", self.machine),
                ] {
                    let _ = Command::new("reg").args(["delete", &key, "/f"]).output();
                }
            }
        }

        fn user(scratch: &Scratch) -> Key {
            Key::open_read(Hive::CurrentUser, &scratch.user)
                .unwrap()
                .expect("the HKCU key")
        }

        /// A player of 0.4.0 on Windows: the game's values in the
        /// VirtualStore (taken first) and HKLM. The first launch carries
        /// them into HKCU next to the launcher's paths; HKLM and the
        /// VirtualStore are only read.
        #[test]
        fn the_first_launch_carries_the_games_values_into_hkcu() {
            let scratch = Scratch::new("carry");
            let store = Key::create(Hive::CurrentUser, &scratch.store).unwrap();
            store.set_string(LANGUAGE, "F").unwrap();
            store.set_string(SELECTION, PLAYERS).unwrap();
            let machine = Key::machine(&scratch.machine).unwrap();
            machine.set_string(LANGUAGE, "G").unwrap();
            machine.set_dword(WIDTH, 1280).unwrap();
            machine.set_dword(HEIGHT, 720).unwrap();
            machine.set_string(INSTALL_PATH, r"C:\old").unwrap();
            drop((store, machine));

            let key = scratch.key();
            assert_eq!(key.current(), None, "no HKCU key yet");
            let written = key
                .write(writes(
                    std::path::Path::new(r"C:\r"),
                    std::path::Path::new(r"C:\s"),
                    Some((1920, 1080)),
                ))
                .unwrap();
            assert!(written.carried);
            assert_eq!(
                written.before,
                Snapshot {
                    selection: Some(PLAYERS.into()),
                    width: Some(1280),
                    height: Some(720),
                },
                "the game's own values from before, for after_run"
            );
            let hkcu = user(&scratch);
            assert_eq!(hkcu.string(INSTALL_PATH).unwrap().as_deref(), Some(r"C:\r"));
            assert_eq!(hkcu.string(LANGUAGE).unwrap().as_deref(), Some("F"));
            assert_eq!(hkcu.string(SELECTION).unwrap().as_deref(), Some(PLAYERS));
            assert_eq!(hkcu.dword(WIDTH).unwrap(), Some(1920), "the launch's size");
            assert_eq!(hkcu.dword(HEIGHT).unwrap(), Some(1080));

            // Never written: the old places keep what they had.
            let old = Key::read_machine(&scratch.machine).unwrap();
            assert_eq!(
                old.string(INSTALL_PATH).unwrap().as_deref(),
                Some(r"C:\old")
            );
            assert_eq!(old.dword(WIDTH).unwrap(), Some(1280));
            let machine = Key::open_read(Hive::LocalMachine, &scratch.machine)
                .unwrap()
                .unwrap();
            assert_eq!(machine.string(LANGUAGE).unwrap().as_deref(), Some("G"));
        }

        /// Once HKCU has the key, the game's values there are its own: a
        /// later launch never carries the old ones over again, and the
        /// game's changes (here its language) stay.
        #[test]
        fn later_launches_keep_the_games_hkcu_values() {
            let scratch = Scratch::new("later");
            let machine = Key::machine(&scratch.machine).unwrap();
            machine.set_string(LANGUAGE, "G").unwrap();
            drop(machine);
            let key = scratch.key();
            let paths = || {
                writes(
                    std::path::Path::new(r"C:\r"),
                    std::path::Path::new(r"C:\s"),
                    None,
                )
            };
            assert!(key.write(paths()).unwrap().carried);
            Key::create(Hive::CurrentUser, &scratch.user)
                .unwrap()
                .set_string(LANGUAGE, "E")
                .unwrap();
            let again = key.write(paths()).unwrap();
            assert!(!again.carried);
            assert!(again.values.iter().all(|v| v.when == When::Always));
            assert_eq!(
                user(&scratch).string(LANGUAGE).unwrap().as_deref(),
                Some("E")
            );
        }

        /// No key anywhere (a fresh prefix, a new player): the launcher's
        /// values alone, read back.
        #[test]
        fn a_new_player_gets_the_launchers_values_only() {
            let scratch = Scratch::new("new");
            let written = scratch
                .key()
                .write(writes(
                    std::path::Path::new(r"C:\r"),
                    std::path::Path::new(r"C:\s"),
                    Some((2560, 1440)),
                ))
                .unwrap();
            assert!(!written.carried);
            assert_eq!(written.before, Snapshot::default());
            let shown: Vec<String> = written.values.iter().map(shown).collect();
            assert_eq!(
                shown,
                [
                    r#"INSTALL_PATH = string:"C:\\r""#,
                    r#"SAVEGAME_PATH = string:"C:\\s""#,
                    "WIDTH = dword:2560",
                    "HEIGHT = dword:1440",
                ]
            );
            assert!(
                Key::open_read(Hive::LocalMachine, &scratch.machine)
                    .unwrap()
                    .is_none(),
                "HKLM is never written"
            );
        }

        /// v0.4.0's restore against the HKCU key: the game (through the
        /// redirect) reset its options and dropped to 640x480; the
        /// player's options and size come back there.
        #[test]
        fn the_graphics_options_are_restored_in_hkcu() {
            let scratch = Scratch::new("restore");
            let key = scratch.key();
            let hkcu = Key::create(Hive::CurrentUser, &scratch.user).unwrap();
            hkcu.set_string(SELECTION, PLAYERS).unwrap();
            hkcu.set_dword(WIDTH, 1920).unwrap();
            hkcu.set_dword(HEIGHT, 1080).unwrap();
            let written = key
                .write(writes(
                    std::path::Path::new(r"C:\r"),
                    std::path::Path::new(r"C:\s"),
                    Some((2560, 1440)),
                ))
                .unwrap();
            // What the game writes back when it can't use the size.
            hkcu.set_string(SELECTION, RESET).unwrap();
            hkcu.set_dword(WIDTH, 640).unwrap();
            hkcu.set_dword(HEIGHT, 480).unwrap();
            let outcome = after_run(Some((2560, 1440)), &written.before, &key.snapshot());
            let crate::registry::AfterRun::Fallback { restore, .. } = &outcome else {
                panic!("no fallback: {outcome:?}");
            };
            key.set(restore).unwrap();
            assert_eq!(
                key.snapshot(),
                Snapshot {
                    selection: Some(PLAYERS.into()),
                    width: Some(1920),
                    height: Some(1080),
                }
            );
        }
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
        let text = shown_all(&writes);
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
        assert_eq!(shown(&writes[2]), "WIDTH = dword:1920");
        assert_eq!(shown(&writes[3]), "HEIGHT = dword:1080");
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

    fn shown_all(values: &[Value]) -> Vec<String> {
        values.iter().map(shown).collect()
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
            } => (*cause, *options_restored, shown_all(restore)),
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

    /// The game opens its HKLM key with the full path at once; the close
    /// hook sends it, and anything below it, to the HKCU key the launcher
    /// writes (akitaonrails/rea-colin-mcrae#6).
    #[test]
    fn the_redirect_sends_the_games_key_to_the_launchers_hkcu_key() {
        use rea_close_hook::registry::{Key, Root};
        let rules = redirect().unwrap();
        assert_eq!(Config::parse(&rules.env_value()).unwrap(), rules);
        let target = |root, path: &str| rules.map(&Key::new(root, path)).map(|t| t.key);
        let user = Key::new(Root::CurrentUser, USER_KEY);
        assert_eq!(target(Root::LocalMachine, MACHINE_KEY), Some(user.clone()));
        assert_eq!(
            target(
                Root::LocalMachine,
                r"software\codemasters\colin mcrae rally 2005"
            ),
            Some(user.clone()),
            "any case"
        );
        assert_eq!(
            target(Root::LocalMachine, &format!(r"{MACHINE_KEY}\Sub")),
            Some(user.join("Sub"))
        );
        assert_eq!(target(Root::LocalMachine, r"SOFTWARE\Microsoft"), None);
        assert_eq!(target(Root::CurrentUser, USER_KEY), None);
    }

    #[test]
    fn the_games_own_values_are_carried_over_once_and_never_override_the_launch() {
        let launch = writes(Path::new("/r"), Path::new("/s"), Some((1920, 1080)));
        let old = [
            (LANGUAGE, Data::String("F".into())),
            (WIDTH, Data::Dword(1280)),
            (SELECTION, Data::String(PLAYERS.into())),
        ];
        assert_eq!(
            shown_all(&carry_over(launch, &old)),
            [
                r#"INSTALL_PATH = string:"/r""#,
                r#"SAVEGAME_PATH = string:"/s""#,
                "WIDTH = dword:1920",
                "HEIGHT = dword:1080",
                r#"LANGUAGE = string:"F", if missing"#,
                &format!("SELECTION = string:{PLAYERS:?}, if missing"),
            ]
        );
        assert_eq!(
            Snapshot::of(&old),
            Snapshot {
                selection: Some(PLAYERS.into()),
                width: Some(1280),
                height: None,
            }
        );
        let original = writes(Path::new("/r"), Path::new("/s"), None);
        let carried = carry_over(original, &old);
        assert_eq!(shown(&carried[3]), "WIDTH = dword:1280, if missing");
        assert_eq!(carry_over(Vec::new(), &[]), Vec::new());
        let kinds: Vec<&str> = GAME_VALUES.iter().map(|(name, _)| *name).collect();
        assert_eq!(kinds, [LANGUAGE, WIDTH, HEIGHT, SELECTION, CD_KEY]);
    }

    #[test]
    fn check_is_ready_only_when_the_game_will_see_its_key() {
        let ready = check(&[]).unwrap();
        assert!(ready.contains(r"HKEY_CURRENT_USER\Software\Codemasters\Colin McRae Rally 2005"));
        assert!(ready.contains("no administrator rights"), "{ready}");
        let error = check(&["rea_close_hook.dll is missing".to_owned()]).unwrap_err();
        assert!(error.starts_with("Not ready:"), "{error}");
        assert!(
            error.contains("\n- rea_close_hook.dll is missing\n"),
            "{error}"
        );
        assert!(error.contains("report it with launcher.log"), "{error}");
    }

    #[test]
    fn a_failed_read_back_names_every_value() {
        use rea_launcher_win32::reg::{Mismatch, Problem};
        let mismatches = [
            Mismatch {
                name: INSTALL_PATH.into(),
                problem: Problem::Missing,
            },
            Mismatch {
                name: WIDTH.into(),
                problem: Problem::Differs {
                    wanted: Data::Dword(1920),
                    found: Data::Dword(640),
                },
            },
        ];
        let text = not_ready(&mismatches);
        assert!(text.contains("- INSTALL_PATH is missing\n"), "{text}");
        assert!(text.contains("- WIDTH is 640, not 1920\n"), "{text}");
        assert!(text.contains("no administrator rights needed"), "{text}");
    }
}
