//! The command line end to end: every flag, `--help`, bad values and their
//! messages, `--check`; on Windows (and under `sdk/scripts/wine-test`) the
//! real CMR2005Launcher.exe too.

use std::path::{Path, PathBuf};

use cmr2005_launcher::cli::{self, Invocation, Mode};
use cmr2005_launcher::config::{self, OPTIONS, Settings};
use cmr2005_launcher::edition;
use rea_launcher::cfg::{Kind, Resolution};
use rea_launcher::identify;

fn parse(args: &[&str]) -> Result<Invocation, String> {
    cli::parse(args.iter().map(|arg| (*arg).to_owned()))
}

fn mode(args: &[&str]) -> Mode {
    parse(args).unwrap().mode
}

/// A scratch folder with a hostile name (spaces, apostrophe, accents).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("cmr2005-cli-{name}-{}", std::process::id()))
        .join("Jogos d'Ação");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn modes_follow_the_family_flags() {
    assert_eq!(
        mode(&[]),
        Mode::Window {
            settings_only: false
        }
    );
    assert_eq!(
        mode(&["--settings"]),
        Mode::Window {
            settings_only: true
        }
    );
    assert_eq!(mode(&["--no-window"]), Mode::Play);
    assert_eq!(
        mode(&["--vsync", "off"]),
        Mode::Play,
        "an option flag plays"
    );
    assert_eq!(mode(&["--check", "--game", "C:\\Games"]), Mode::Check);
    for help in ["--help", "-h", "/?"] {
        assert_eq!(mode(&[help]), Mode::Help);
    }
    let helper = parse(&["--settings", "--linux"]).unwrap();
    assert!(helper.linux);
}

#[test]
fn every_option_has_a_working_flag() {
    for row in OPTIONS {
        let number;
        let value = match row.kind {
            Kind::Toggle => "off",
            Kind::Choice(choices) => choices[0].0,
            Kind::Size { .. } => "1280x720",
            Kind::Range { min, .. } => {
                number = min.to_string();
                &number
            }
            Kind::Path | Kind::Text => "C:\\Games\\Some Game",
        };
        let flag = row.flag_name(config::STYLE);
        let parsed = parse(&[&flag, value]).unwrap_or_else(|e| panic!("{flag} {value}: {e}"));
        assert_ne!(
            parsed.flags,
            Settings::default(),
            "{flag} {value} set nothing"
        );
        assert!(config::usage().contains(&flag), "--help lacks {flag}");
    }
    let flags = parse(&["--look", "original", "--resolution", "800X600"])
        .unwrap()
        .flags;
    assert_eq!(
        flags.picture(None).resolution,
        Some(Resolution::Size(800, 600))
    );
}

#[test]
fn bad_command_lines_say_what_is_wrong() {
    let message = |args: &[&str]| parse(args).unwrap_err();
    assert!(message(&["--vsync", "maybe"]).contains("--vsync needs on or off, not maybe"));
    assert!(message(&["--look", "shiny"]).contains("original, widescreen or steam_deck"));
    assert!(message(&["--resolution", "10x10"]).contains("WIDTHxHEIGHT or original"));
    assert!(message(&["--resolution"]).contains("--resolution needs a value"));
    assert!(message(&["--fps-limit", "fast"]).contains("--fps-limit"));
    assert!(message(&["--fps-limit", "5000"]).contains("--fps-limit"));
    assert!(message(&["--frobnicate"]).contains("unknown argument --frobnicate"));
    assert!(message(&["--frobnicate"]).contains("--help"));
    assert!(message(&["--settings", "--no-window"]).contains("cannot be used together"));
}

#[test]
fn help_lists_every_option_and_switch() {
    let usage = config::usage();
    for switch in config::SWITCHES.iter().chain(&["--help"]) {
        assert!(usage.contains(switch), "{switch}");
    }
    assert!(usage.contains("--vsync on|off"));
    assert!(usage.contains("--fps-limit"));
    assert!(usage.contains("--look original|widescreen|steam_deck"));
}

#[test]
fn check_needs_a_game_folder_with_a_known_edition() {
    let cwd = Path::new("/");
    let none = cli::check(&Settings::default(), cwd).unwrap_err();
    assert!(none.contains("--game"), "{none}");

    let dir = scratch("check");
    let game = parse(&["--check", "--game", &dir.display().to_string()])
        .unwrap()
        .flags;
    let missing = cli::check(&game, cwd).unwrap_err();
    assert!(missing.contains("There is no"), "{missing}");

    std::fs::write(dir.join(edition::GAME_EXE), b"some other build").unwrap();
    let unknown = cli::check(&game, cwd).unwrap_err();
    let digest = identify::hex(&identify::sha256(b"some other build"));
    assert!(unknown.contains(&digest), "{unknown}");
    assert!(
        unknown.contains(edition::UNKNOWN_EDITION_ISSUE),
        "{unknown}"
    );
}

/// The real program, as a player's terminal or the Linux helper runs it.
#[cfg(windows)]
mod program {
    use super::*;
    use std::process::{Command, Output};

    fn run(name: &str, args: &[&str]) -> (Output, PathBuf) {
        let home = scratch(name).join("AppData");
        let output = Command::new(env!("CARGO_BIN_EXE_CMR2005Launcher"))
            .args(args)
            .env("LOCALAPPDATA", &home)
            .output()
            .unwrap();
        (output, home.join("CMR2005Launcher").join("launcher.log"))
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn help_goes_to_the_terminal() {
        let (output, _) = run("help", &["--help"]);
        assert_eq!(output.status.code(), Some(cli::OK));
        assert!(text(&output.stdout).contains("--vsync on|off"));
    }

    #[test]
    fn a_bad_flag_exits_2_with_its_message() {
        let (output, log) = run("bad", &["--vsync", "maybe"]);
        assert_eq!(output.status.code(), Some(cli::USAGE_ERROR));
        assert!(text(&output.stdout).contains("--vsync needs on or off"));
        assert!(
            std::fs::read_to_string(log)
                .unwrap()
                .contains("--vsync needs on or off")
        );
    }

    /// rea-launcher-sdk#1: an unknown flag or a bad value is answered on
    /// stdout at once (no dialog to wait on) with exit 2, so
    /// `launcher --bogus-flag > out.txt` holds the message.
    #[test]
    fn an_unknown_flag_or_bad_value_prints_to_stdout_and_exits_2() {
        for (name, args, says) in [
            (
                "bogus",
                &["--bogus-flag"][..],
                "unknown argument --bogus-flag",
            ),
            (
                "look",
                &["--look", "invalid"],
                "original, widescreen or steam_deck",
            ),
        ] {
            let (output, _) = run(name, args);
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            let stdout = text(&output.stdout);
            assert!(stdout.contains(says), "{args:?}: {stdout}");
            assert!(stdout.contains("--help"), "{args:?}: {stdout}");
            assert!(text(&output.stderr).is_empty(), "{args:?}");
        }
    }

    #[test]
    fn check_refuses_an_unknown_edition_with_its_digest() {
        let dir = scratch("unknown-game");
        std::fs::write(dir.join(edition::GAME_EXE), b"some other build").unwrap();
        let game = dir.display().to_string();
        let (output, log) = run("unknown", &["--check", "--game", &game]);
        assert_eq!(output.status.code(), Some(cli::REFUSED));
        let digest = identify::hex(&identify::sha256(b"some other build"));
        // stdout like the family (rea-launcher-sdk#1): `--check > report.txt`.
        assert!(text(&output.stdout).contains(&digest));
        assert!(text(&output.stderr).is_empty());
        let log = std::fs::read_to_string(log).unwrap();
        assert!(
            log.contains(&digest) && log.contains(edition::UNKNOWN_EDITION_ISSUE),
            "{log}"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn the_program_says_it_is_for_windows() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_CMR2005Launcher"))
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(cli::USAGE_ERROR));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Windows program"));
}

/// Who sees a message (`rea_launcher_win32::report::Audience`, from the
/// parsed flags): `cmr2005-play`, the AppImage and Proton start the
/// launcher with no console and stdout to a log, so a play-time error that
/// is only printed never reaches the player.
mod messages {
    use super::parse;
    use cmr2005_launcher::cli;
    use rea_launcher_win32::report::{self, Audience, Mode, Route, Run, Terminal, route};

    const UNDER_HELPER: Terminal = Terminal {
        attached: false,
        redirected: true,
        wine: true,
    };

    const BOTH: Route = Route {
        print: true,
        dialog: true,
    };

    fn routed(args: &[&str], wine: bool, terminal: Terminal) -> Route {
        route(parse(args).unwrap().audience(wine).mode(), terminal)
    }

    fn bad_line(args: &[&str], wine: bool) -> Mode {
        cli::usage_error_audience(args, wine).mode()
    }

    #[test]
    fn the_runs_are_the_family_flags() {
        let run = |args: &[&str]| parse(args).unwrap().run();
        assert_eq!(
            run(&["--settings", "--linux"]),
            Run {
                settings_only: true,
                linux: true,
                ..Run::default()
            }
        );
        assert_eq!(
            run(&["--no-window", "--look", "original"]),
            Run {
                no_window: true,
                ..Run::default()
            }
        );
        for command in [&["--check"][..], &["--help"], &["--help", "--linux"]] {
            assert!(run(command).command, "{command:?}");
        }
        assert!(run(&["--help", "--linux"]).linux);
    }

    #[test]
    fn the_helpers_command_lines_follow_the_sdk() {
        // REA_SETTINGS_ARGS and REA_PLAY_ARGS of sdk/rea-play.sh: the
        // audience is the SDK's for every run that is not a command.
        for args in [
            &["--settings", "--linux"][..],
            &["--no-window"],
            &["--no-window", "--look", "steam_deck"],
            &[],
        ] {
            let invocation = parse(args).unwrap();
            for wine in [true, false] {
                assert_eq!(
                    invocation.audience(wine),
                    Audience::of(invocation.run(), wine),
                    "{args:?}, Wine {wine}"
                );
            }
        }
    }

    #[test]
    fn a_play_error_under_the_linux_helper_is_a_dialog_and_logged() {
        // cmr2005-play's game run (one-run flags too), and a Proton start.
        for run in [
            &["--no-window"][..],
            &["--no-window", "--look", "original"],
            &[],
        ] {
            assert_eq!(routed(run, true, UNDER_HELPER), BOTH, "{run:?}");
            assert!(matches!(
                parse(run).unwrap().audience(true).mode(),
                Mode::Play { .. }
            ));
        }
    }

    #[test]
    fn the_helpers_window_run_stays_quiet() {
        // `--settings --linux`: the helper reads the exit code.
        let window = routed(&["--settings", "--linux"], true, UNDER_HELPER);
        assert!(window.print && !window.dialog);
    }

    #[test]
    fn check_help_and_bad_flags_never_block_a_script_under_wine() {
        // `wine CMR2005Launcher.exe --check` from a terminal or a script,
        // and `sdk/scripts/wine-test`: printed, never a modal dialog that
        // nobody can answer.
        let printed = Route {
            print: true,
            dialog: false,
        };
        for command in [&["--check"][..], &["--help"]] {
            assert_eq!(routed(command, true, UNDER_HELPER), printed, "{command:?}");
        }
        assert_eq!(
            route(bad_line(&["--vsync", "maybe"], true), UNDER_HELPER),
            printed
        );
        assert_eq!(
            route(
                bad_line(&["--frobnicate", "--no-window"], true),
                UNDER_HELPER
            ),
            printed
        );
    }

    /// A command line that fails to parse is the SDK's
    /// `Run::bad_command_line` (rea-launcher-sdk#1): a command, never a
    /// play run whose modal dialog a terminal would wait on, exit 2. On
    /// native Windows the audience is the SDK's as is; under Wine only
    /// `commands_print_only` narrows it (never a dialog).
    #[test]
    fn a_bad_command_line_is_the_sdks_bad_command_line() {
        assert_eq!(cli::USAGE_ERROR, report::BAD_COMMAND_LINE);
        let terminal = Terminal {
            attached: true,
            redirected: false,
            wine: false,
        };
        for args in [
            &["--bogus-flag"][..],
            &["--look", "invalid"],
            &["--no-window", "--bogus-flag"],
            &["--settings", "--linux", "--vsync", "maybe"],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
            let sdk = Audience::of(Run::bad_command_line(args), false);
            assert_eq!(cli::usage_error_audience(args, false), sdk, "{args:?}");
            let mode = bad_line(args, false);
            assert!(matches!(mode, Mode::Command { .. }), "{args:?}");
            let from_terminal = route(mode, terminal);
            assert!(from_terminal.print && !from_terminal.dialog, "{args:?}");
            let under_wine = cli::usage_error_audience(args, true);
            assert!(under_wine.command && !under_wine.interactive, "{args:?}");
        }
    }

    #[test]
    fn check_help_and_bad_flags_on_windows_show_a_dialog_only_when_unseen() {
        let terminal = Terminal {
            attached: true,
            redirected: false,
            wine: false,
        };
        let shortcut = Terminal {
            attached: false,
            redirected: false,
            wine: false,
        };
        for command in [&["--check"][..], &["--help"]] {
            let from_terminal = routed(command, false, terminal);
            assert!(from_terminal.print && !from_terminal.dialog, "{command:?}");
            assert!(routed(command, false, shortcut).dialog, "{command:?}");
        }
        assert!(!route(bad_line(&["--vsync", "maybe"], false), terminal).dialog);
        assert!(route(bad_line(&["--vsync", "maybe"], false), shortcut).dialog);
        // A script's bad line on Windows (`--no-window`) never waits on one.
        assert!(!route(bad_line(&["--frobnicate", "--no-window"], false), shortcut).dialog);
    }

    #[test]
    fn a_windows_script_with_no_window_never_gets_a_dialog() {
        let redirected = Terminal {
            attached: false,
            redirected: true,
            wine: false,
        };
        assert!(!routed(&["--no-window"], false, redirected).dialog);
        // A double-clicked shortcut with a flag is a person.
        let nothing = Terminal {
            attached: false,
            redirected: false,
            wine: false,
        };
        assert!(routed(&["--look", "original"], false, nothing).dialog);
    }

    /// Under Wine (`sdk/scripts/wine-test` pipes stdout as cmr2005-play
    /// does): the game run's error would reach the player as a dialog.
    #[cfg(windows)]
    #[test]
    fn under_wine_the_game_run_error_is_a_dialog() {
        use rea_launcher_win32::{report, wine};
        if !wine::is_wine() {
            return;
        }
        let mode = parse(&["--no-window"]).unwrap().audience(true).mode();
        assert!(report::route_here(mode).dialog);
    }
}

/// One launcher at a time (STANDARDS "One instance"): a second start brings
/// the settings window forward, found by its own title in a process running
/// this exe (`instance::Window::Own`), else says the game is running and
/// exits 5, which `cmr2005-play` reads as "already running".
mod second_start {
    use super::parse;
    use cmr2005_launcher::{WINDOW_TITLE, cli};
    use rea_launcher_win32::instance::{self, SecondStart};

    #[test]
    fn only_check_is_an_action() {
        assert!(parse(&["--check"]).unwrap().is_action());
        for args in [&[][..], &["--no-window"], &["--settings", "--linux"]] {
            assert!(!parse(args).unwrap().is_action(), "{args:?}");
        }
    }

    #[test]
    fn a_refused_second_start_exits_5() {
        assert_eq!(cli::ALREADY_RUNNING, 5);
        assert_eq!(cli::ALREADY_RUNNING, instance::ALREADY_RUNNING);
        assert_eq!(SecondStart::Busy.exit_code(), cli::ALREADY_RUNNING);
        assert_eq!(SecondStart::Focused.exit_code(), cli::OK);
        let busy = SecondStart::Busy.message("Colin McRae Rally 2005").unwrap();
        assert!(busy.contains("already running"), "{busy}");
    }

    #[test]
    fn the_window_title_is_not_the_games() {
        // The game's window is "Colin McRae Rally 2005"; a second start
        // must never take it for the launcher's window.
        assert_eq!(WINDOW_TITLE, "Colin McRae Rally 2005 Launcher");
        assert!(instance::is_own_window(
            WINDOW_TITLE,
            WINDOW_TITLE,
            r"C:\Launcher\CMR2005Launcher.exe",
            "cmr2005launcher.exe"
        ));
        assert!(!instance::is_own_window(
            "Colin McRae Rally 2005",
            WINDOW_TITLE,
            r"C:\Launcher\CMR2005Launcher.exe",
            "CMR2005Launcher.exe"
        ));
        // The game's window, whatever its title.
        assert!(!instance::is_own_window(
            WINDOW_TITLE,
            WINDOW_TITLE,
            r"C:\users\player\AppData\Local\CMR2005Launcher\runtime\cmr5.exe",
            "CMR2005Launcher.exe"
        ));
    }
}
