//! What a command line asks for (docs/STANDARDS.md "Family consistency":
//! `--game`, `--settings`, `--no-window`, `--check`, `--linux`, `--look`).

use std::path::{Path, PathBuf};

use rea_launcher::cfg::ArgError;
use rea_launcher::paths;
use rea_launcher_win32::report::{Audience, Run};

use crate::config::{self, Settings};
use crate::edition;
use crate::source::{self, Source};

/// Exit codes. The Linux helper reads the `--settings` ones.
pub const OK: i32 = 0;
/// Refused: an unknown edition, no game folder, a failed `--check`.
pub const REFUSED: i32 = 1;
/// A bad command line (the family code, `report::BAD_COMMAND_LINE`).
pub const USAGE_ERROR: i32 = rea_launcher_win32::report::BAD_COMMAND_LINE;
/// `--settings`: the window was closed without Play.
pub const WINDOW_CLOSED: i32 = 3;
/// `--settings`: no window could be made (the helper then plays anyway).
pub const WINDOW_UNAVAILABLE: i32 = 4;
/// A second start that did nothing: the game or another launcher run holds
/// the instance (`rea_launcher_win32::instance::ALREADY_RUNNING`).
pub const ALREADY_RUNNING: i32 = rea_launcher_win32::instance::ALREADY_RUNNING;

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
    /// Started by the Linux helper (`--linux`).
    pub linux: bool,
    /// `--no-window` was given (a script on native Windows; the helper's
    /// game run under Wine).
    pub no_window: bool,
    /// The settings the flags set, to go on top of `launcher.cfg`.
    pub flags: Settings,
}

impl Invocation {
    /// The family flags of this run, for the SDK's audience rule.
    pub fn run(&self) -> Run {
        Run {
            command: matches!(self.mode, Mode::Help | Mode::Check),
            no_window: self.no_window,
            settings_only: self.mode
                == (Mode::Window {
                    settings_only: true,
                }),
            linux: self.linux,
        }
    }

    /// Who may see this run's messages (`wine`: running under Wine): the
    /// SDK's [`Audience::of`] (STANDARDS "Messages reach the player").
    /// Play time (the window, the game run) is the SDK's rule as is: under
    /// Wine `cmr2005-play`'s `--no-window` game run, the AppImage and
    /// Proton get a dialog for an error, and it is printed for the log.
    /// The window run of the helper (`--settings --linux`) shows none.
    ///
    /// One narrowing, see [`commands_print_only`]: `--help` and `--check`
    /// under Wine are only printed.
    pub fn audience(&self, wine: bool) -> Audience {
        commands_print_only(Audience::of(self.run(), wine), wine)
    }

    /// Whether the run does work without the window (`--check`): a second
    /// start then never brings the open window forward and exits non-zero.
    pub fn is_action(&self) -> bool {
        self.mode == Mode::Check
    }
}

/// A command's output (`--help`, `--check`, a bad command line) under Wine
/// is printed and never a dialog: `wine CMR2005Launcher.exe --check` from a
/// script or `sdk/scripts/wine-test` must not wait on a dialog nobody can
/// answer (docs/RUNTIME_NOTES.md, play-time errors). The SDK's rule would
/// show one under Wine when no console is attached; the difference is
/// recorded for rea-launcher-sdk in docs/CONTRIBUTIONS.md.
fn commands_print_only(audience: Audience, wine: bool) -> Audience {
    if audience.command && wine {
        Audience {
            interactive: false,
            ..audience
        }
    } else {
        audience
    }
}

/// Who sees the message for a command line that did not parse (`args`
/// without the program name): the SDK's [`Run::bad_command_line`], a
/// command, answered on the terminal and exiting [`USAGE_ERROR`]. Only
/// [`commands_print_only`] is ours: the SDK has no rule for commands under
/// Wine yet.
pub fn usage_error_audience<S: AsRef<str>>(args: &[S], wine: bool) -> Audience {
    commands_print_only(Audience::of(Run::bad_command_line(args), wine), wine)
}

/// Reads the command line (without the program name). The error is the
/// message for the player.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Invocation, String> {
    let args: Vec<String> = args.into_iter().collect();
    let (flags, switches) = match config::parse_args(args.clone()) {
        Ok(parsed) => parsed,
        Err(ArgError::Help(_)) => {
            return Ok(Invocation {
                mode: Mode::Help,
                linux: args.iter().any(|arg| arg == "--linux"),
                no_window: args.iter().any(|arg| arg == "--no-window"),
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
