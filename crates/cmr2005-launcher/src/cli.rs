//! What a command line asks for (docs/STANDARDS.md "Family consistency":
//! `--game`, `--settings`, `--no-window`, `--check`, `--linux`, `--look`).

use std::path::{Path, PathBuf};

use rea_launcher::cfg::ArgError;
use rea_launcher::paths;
use rea_launcher_win32::report;

use crate::config::{self, Settings};
use crate::edition;
use crate::source::{self, Source};

/// Exit codes. The Linux helper reads the `--settings` ones.
pub const OK: i32 = 0;
/// Refused: an unknown edition, no game folder, a failed `--check`.
pub const REFUSED: i32 = 1;
/// A bad command line.
pub const USAGE_ERROR: i32 = 2;
/// `--settings`: the window was closed without Play.
pub const WINDOW_CLOSED: i32 = 3;
/// `--settings`: no window could be made (the helper then plays anyway).
pub const WINDOW_UNAVAILABLE: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `--help`, `-h`, `/?`.
    Help,
    /// The settings window; then the game, unless `settings_only`
    /// (`--settings`).
    Window { settings_only: bool },
    /// `--check`: identify the game folder and report, never launch.
    Check,
    /// `--no-window`, or any option flag: play with the saved settings and
    /// the flags on top.
    Play,
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub mode: Mode,
    /// Started by the Linux helper (`--linux`): never show a dialog.
    pub linux: bool,
    /// `--no-window`: on Windows usually a script.
    pub no_window: bool,
    /// The settings the flags set, to go on top of `launcher.cfg`.
    pub flags: Settings,
}

impl Invocation {
    /// Whether a person may be watching: messages may become dialogs when
    /// there is no terminal. Never for the Linux helper's runs.
    pub fn interactive(&self) -> bool {
        !self.linux
    }

    /// How `rea_launcher_win32::report` routes this run's messages
    /// (`wine`: running under Wine).
    ///
    /// Play time (the window, the game run): an error is a dialog whenever
    /// a person may be watching, and printed for the log as well.
    /// `--no-window` counts as a script on Windows, but not under Wine:
    /// `cmr2005-play`'s game run, the AppImage and Proton start the
    /// launcher for a player in front of the screen, and its stdout goes
    /// to a log nobody reads. The helper's window run (`--linux`) stays
    /// quiet: the helper reads its exit code.
    ///
    /// `--help` and `--check` are commands: printed, and a dialog only on
    /// Windows when nothing printed can be seen. Under Wine they are only
    /// printed, so a script or `sdk/scripts/wine-test` never waits on a
    /// dialog nobody can answer.
    pub fn message_mode(&self, wine: bool) -> report::Mode {
        match self.mode {
            Mode::Help | Mode::Check => report::Mode::Command {
                interactive: self.interactive() && !wine,
            },
            Mode::Window { .. } | Mode::Play => report::Mode::Play {
                interactive: self.interactive() && (wine || !self.no_window),
            },
        }
    }
}

/// How a bad command line's message is routed: as a command's
/// ([`Invocation::message_mode`]).
pub fn usage_error_mode(wine: bool) -> report::Mode {
    report::Mode::Command { interactive: !wine }
}

/// Reads the command line (without the program name). The error is the
/// message for the player.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Invocation, String> {
    let args: Vec<String> = args.into_iter().collect();
    let (flags, switches) = match config::parse_args(args) {
        Ok(parsed) => parsed,
        Err(ArgError::Help(_)) => {
            return Ok(Invocation {
                mode: Mode::Help,
                linux: false,
                no_window: false,
                flags: Settings::default(),
            });
        }
        Err(error) => {
            return Err(format!(
                "{error}\n\nRun CMR2005Launcher.exe --help for the options."
            ));
        }
    };
    let has = |switch: &str| switches.iter().any(|s| s == switch);
    let options = flags != Settings::default();
    let mode = if has("--check") {
        Mode::Check
    } else if has("--settings") {
        if has("--no-window") {
            return Err("--settings and --no-window cannot be used together.".into());
        }
        Mode::Window {
            settings_only: true,
        }
    } else if has("--no-window") || options {
        Mode::Play
    } else {
        Mode::Window {
            settings_only: false,
        }
    };
    Ok(Invocation {
        mode,
        linux: has("--linux"),
        no_window: has("--no-window"),
        flags,
    })
}

/// The game folder from the settings, made absolute against `cwd`.
pub fn game_folder(settings: &Settings, cwd: &Path) -> Result<PathBuf, String> {
    let game = settings.game.as_deref().ok_or_else(|| {
        format!(
            "No game folder yet. Choose it in the settings window, or run:\n\n\
             CMR2005Launcher.exe --game <the folder with {} or {}>",
            edition::GAME_EXE,
            source::SETUP_EXE
        )
    })?;
    paths::resolve(game, cwd)
}

/// `--check`: the game folder and what it holds, or why it cannot be played.
pub fn check(settings: &Settings, cwd: &Path) -> Result<(PathBuf, Source), String> {
    let game = game_folder(settings, cwd)?;
    let source = source::detect(&game)?;
    Ok((game, source))
}

/// The `--check` report for a playable folder.
pub fn check_report(game: &Path, source: &Source) -> String {
    match source {
        Source::Installed { found, .. } => format!(
            "Ready to play: {} ({}) in {}",
            edition::GAME_EXE,
            found.edition.name,
            game.display()
        ),
        Source::Installer(installer) => format!(
            "Ready to play: {} in {}. The first start unpacks the game (about 3.6 GB).",
            installer.build.name,
            game.display()
        ),
    }
}
