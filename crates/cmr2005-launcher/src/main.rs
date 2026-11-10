//! CMR2005Launcher.exe: a Windows GUI program (it also runs under Proton and
//! Wine). Everything testable lives in the library; this file wires it to
//! Windows: the console for `--help`/`--check`/`--no-window`, the settings
//! window, `launcher.log`, the runtime copy and the game's process.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    std::process::exit(app::run());
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "CMR2005Launcher.exe is a Windows program; on Linux run it with Proton, Wine or linux/cmr2005-play."
    );
    std::process::exit(cmr2005_launcher::cli::USAGE_ERROR);
}

#[cfg(windows)]
mod app {

    use std::path::{Path, PathBuf};

    use cmr2005_launcher::cli::{self, Invocation, Mode};
    use cmr2005_launcher::components::{self, APP_OVERRIDES};
    use cmr2005_launcher::registry;
    use cmr2005_launcher::saves::{self, Folders};
    use cmr2005_launcher::source::{self, Source};
    use cmr2005_launcher::{INSTANCE, WINDOW_TITLE, config, edition, plan, runtime, ui};
    use rea_launcher::ledger::{Ledger, Outcome};
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, paths};
    use rea_launcher_win32::dialog::{self, Icon};
    use rea_launcher_win32::gpu::{self, Glx};
    use rea_launcher_win32::innoextract::Unpacked;
    use rea_launcher_win32::instance::{self, SecondStart, Window};
    use rea_launcher_win32::process::{self, Launch};
    use rea_launcher_win32::reg::Value;
    use rea_launcher_win32::report;
    use rea_launcher_win32::{display, wine};

    const TITLE: &str = cmr2005_launcher::GAME_TITLE;

    /// `%LOCALAPPDATA%\CMR2005Launcher`: launcher.cfg, launcher.log and the
    /// runtime copy.
    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR2005Launcher")
    }

    /// `launcher.cfg` beside the exe (a portable install), else in [`home`].
    fn settings_path() -> PathBuf {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(config::FILE_NAME)));
        match portable {
            Some(path) if path.is_file() => path,
            _ => home().join(config::FILE_NAME),
        }
    }

    /// Where messages go (`Invocation::audience`, the SDK's rule): printed
    /// on stdout like the family (the terminal, a file `--check >
    /// report.txt` names, or the log `cmr2005-play`, the AppImage and
    /// Proton keep), and a dialog whenever a person may be watching a
    /// play-time error.
    struct Out {
        log: Log,
        mode: report::Mode,
    }

    impl Out {
        /// A command's result (`--help`, `--check`).
        fn say(&self, text: &str) {
            let route = report::route_here(self.mode);
            if route.print {
                println!("{text}");
            }
            if route.dialog {
                dialog::message(TITLE, text, Icon::Info);
            }
        }

        /// An error or a refusal: stdout and/or a dialog (`report::fatal`).
        fn fail(&self, text: &str) {
            report::fatal(TITLE, text, self.mode);
        }

        fn warn(&self, message: &str) {
            self.log.line(&format!("warning: {message}"));
        }

        /// Information for the player while preparing to play: the log,
        /// and stdout when it leads somewhere; never a dialog to wait on.
        fn tell(&self, text: &str) {
            self.log.line(text);
            if report::route_here(self.mode).print {
                println!("{TITLE}: {text}");
            }
        }
    }

    pub fn run() -> i32 {
        let log = Log::new(home().join("launcher.log"));
        log.line(concat!("CMR2005Launcher ", env!("CARGO_PKG_VERSION")));
        let args: Vec<String> = std::env::args().skip(1).collect();
        let parsed = cli::parse(args.iter().cloned());
        let audience = match &parsed {
            Ok(invocation) => invocation.audience(wine::is_wine()),
            Err(_) => cli::usage_error_audience(&args, wine::is_wine()),
        };
        let mode = audience.mode();
        let out = Out { log, mode };
        let invocation = match parsed {
            Ok(invocation) => invocation,
            Err(error) => {
                out.log.line(&format!("error: {error}"));
                out.fail(&error);
                return cli::USAGE_ERROR;
            }
        };
        match run_mode(&out, invocation, instance::is_action(&args, &[])) {
            Ok(code) => code,
            Err(error) => {
                out.log.line(&format!("error: {error}"));
                out.fail(&error);
                cli::REFUSED
            }
        }
    }

    /// `action`: the command line holds a family action flag (`--check`,
    /// `instance::is_action`), refused while the window is open.
    fn run_mode(out: &Out, invocation: Invocation, action: bool) -> Result<i32, String> {
        if invocation.mode == Mode::Help {
            out.say(&config::usage());
            return Ok(cli::OK);
        }
        // Held until the game exits (`--check` takes it too): a second
        // start brings the settings window forward, or says the game is
        // running (an action: the window is open) and exits 5.
        let instance = match instance::single(INSTANCE) {
            Ok(guard) => guard,
            Err(running) => {
                let second = running.second_start(action, Window::Own(WINDOW_TITLE));
                out.log.line(second.log_line());
                if let Some(text) = second.message(TITLE) {
                    out.fail(&text);
                }
                return Ok(second.exit_code());
            }
        };
        let path = settings_path();
        // A fresh Wine prefix draws eframe windows through EGL, which fails
        // on many drivers: switch it to GLX and run once more, so the first
        // start already gets the settings window (or the unpack window of
        // a --no-window first start).
        // `None` only when starting again failed after the lock was let go.
        let instance = if invocation.mode == Mode::Check {
            Some(instance)
        } else {
            match gpu::prefer_glx_or_restart(Some(instance)) {
                Glx::Restarted { exit_code } => {
                    out.log
                        .line("windows: OpenGL through GLX (UseEGL=N) set on a fresh prefix, started again");
                    return Ok(exit_code as i32);
                }
                Glx::Ready { instance, warning } => {
                    if let Some(warning) = warning {
                        out.warn(&warning);
                    }
                    instance
                }
            }
        };
        if let Mode::Window { settings_only } = invocation.mode {
            let saved = cfg::read_file(&path).ok().flatten();
            match ui::show(&path, saved.as_deref(), invocation.linux) {
                Ok(ui::Outcome::Play) if settings_only => return Ok(cli::OK),
                Ok(ui::Outcome::Play) => {}
                Ok(ui::Outcome::Closed) if settings_only => return Ok(cli::WINDOW_CLOSED),
                Ok(ui::Outcome::Closed) => return Ok(cli::OK),
                Err(error) => {
                    out.warn(&format!("settings window unavailable: {error}"));
                    if settings_only {
                        return Ok(cli::WINDOW_UNAVAILABLE);
                    }
                }
            }
        }

        // Held until the game exits.
        let _instance = instance;
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
        let placement = plan::screen(&display::monitors(), wine::is_wine(), gamescope);
        let screen = placement.size.or_else(display::screen_size);
        let existing = cfg::read_file(&path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        let first_run = existing.is_none();
        let text = existing.unwrap_or_else(|| config::first_run_text(screen));
        let (saved, warnings) = config::parse_file(&text);
        for warning in &warnings {
            out.warn(warning);
        }
        let game_from_command_line = invocation.flags.game.is_some();
        let settings = saved.clone().merged(invocation.flags);
        let cwd = std::env::current_dir().unwrap_or_default();
        let (game, source) = cli::check(&settings, &cwd)?;
        out.log
            .line(&format!("game: {} ({})", game.display(), source.describe()));
        let folders = Folders::new(&home(), settings.runtime.as_deref());
        let components_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(components::DIR)))
            .ok_or("Cannot find the launcher's components folder.")?;
        if invocation.mode == Mode::Check {
            let ready = cli::check_report(&game, &source, &folders.unpacked);
            let key = registry::check(&check_key(&components_dir, &source, &folders))?;
            out.say(&format!("{ready}\n{key}"));
            return Ok(cli::OK);
        }
        let save = |text: &str, game: &Path| -> Result<String, String> {
            let text = config::set(text, "game", &game.display().to_string());
            fsx::write_atomic(&path, &text)
                .map_err(|error| format!("Cannot save {}: {error}", path.display()))?;
            Ok(text)
        };
        let mut text = text;
        if first_run || saved.game.as_ref() != Some(&game) {
            text = save(&text, &game)?;
        }

        let wine = wine::is_wine();
        let from_installer = matches!(source, Source::Installer(_));
        components::validate(&components_dir, wine, from_installer)?;
        for line in components::describe(&components_dir) {
            out.log.line(&format!("component {line}"));
        }

        // A launcher that was killed released the instance, but its game
        // may still run from the runtime copy: never prepare under it.
        if !process::find_running(&folders.runtime.join(edition::GAME_EXE)).is_empty() {
            let busy = SecondStart::Busy;
            out.log.line(busy.log_line());
            out.fail(&busy.message(TITLE).unwrap_or_default());
            return Ok(busy.exit_code());
        }
        let game_dir = match &source {
            Source::Installed { dir, .. } => dir.clone(),
            Source::Installer(installer) => {
                if !unpack(out, installer, &folders.unpacked, &components_dir)? {
                    return Ok(cli::OK);
                }
                // The next start finds an installed game: the game setting
                // points at the unpacked folder (unless it came from --game).
                let unpacked =
                    source::setting_after_unpack(&folders.unpacked, game_from_command_line);
                if let Some(dir) = &unpacked.save {
                    save(&text, dir)?;
                }
                out.tell(&unpacked.message);
                folders.unpacked.clone()
            }
        };
        paths::check_separate(&game_dir, &folders.runtime)?;
        let report = runtime::prepare(&game_dir, &folders.runtime)?;
        for warning in &report.warnings {
            out.warn(warning);
        }
        out.log.line(&format!(
            "runtime: {} ({} brought, {} linked)",
            folders.runtime.display(),
            report.brought,
            report.linked
        ));
        match saves::import(&game_dir, &folders.saves) {
            Ok(copied) if !copied.is_empty() => out.log.line(&format!(
                "saves: copied {} from the game folder into {}",
                copied.join(", "),
                folders.saves.display()
            )),
            Ok(_) => {}
            Err(error) => {
                return Err(format!(
                    "Cannot prepare the saves folder {}: {error}",
                    folders.saves.display()
                ));
            }
        }

        let mut picture = settings.picture(screen);
        out.log.line(&format!(
            "screen: {}; picture: {}",
            screen.map_or("unknown".to_owned(), |(w, h)| format!("{w}x{h}")),
            picture.describe()
        ));
        if let Some((w, h)) = picture.size().filter(|_| !placement.virtual_desktop) {
            let fitted = config::fit_mode((w, h), display::mode_supported);
            if fitted != Some((w, h)) {
                out.warn(&format!(
                    "the display lists no {w}x{h} mode; the game gets {}",
                    fitted.map_or("its own mode".to_owned(), |(w, h)| format!("{w}x{h}"))
                ));
                picture.resolution = Some(fitted.map_or(cfg::Resolution::Original, |(w, h)| {
                    cfg::Resolution::Size(w, h)
                }));
            }
        }
        let plan = plan::plan_on(
            &picture,
            settings.advanced_particles(),
            &folders.runtime,
            &folders.saves,
            wine,
            placement.virtual_desktop.then_some(screen).flatten(),
        );
        install_components(out, &plan, &game_dir, &folders.runtime, &components_dir)?;
        // From the player's bytes on every start, with the close hook in
        // its import table: Windows loads the hook with the game, which
        // installs itself (REA_CLOSE_HOOK_ATTACH) and gives the game its
        // registry key from HKCU (REA_REGISTRY); nothing crosses processes.
        runtime::write_game_exe(&game_dir, &folders.runtime)
            .map_err(|error| registry::not_ready(&[error]))?;
        out.log.line(&format!(
            "runtime {}: imports {}",
            edition::GAME_EXE,
            plan.exe_imports.join(", ")
        ));
        let before = write_registry(out, plan.registry.clone())?;
        if !plan.app_dll_overrides.is_empty() {
            let applied = wine::app_dll_overrides(edition::GAME_EXE, &APP_OVERRIDES)
                .map_err(|error| error.to_string())?;
            out.log
                .line(&format!("Wine DLL overrides for cmr5.exe: {applied:?}"));
        }
        out.log.line(&format!("arguments: {:?}", plan.args));

        let exe = folders.runtime.join(edition::GAME_EXE);
        if plan.prefer_fast_gpu {
            match gpu::prefer_fast_gpu(&exe) {
                Ok(true) => out.log.line("Windows: high-performance GPU preferred"),
                Ok(false) => {}
                Err(error) => out.warn(&error.to_string()),
            }
        }
        let mut launch = match plan.virtual_desktop {
            // The main monitor is portrait: the game sees one landscape
            // screen, Wine's virtual desktop (Wine's explorer starts it there).
            Some(size) => {
                out.log.line(&format!(
                    "the main monitor is portrait: the game runs in a {}x{} Wine virtual desktop",
                    size.0, size.1
                ));
                let exe_text = exe.display().to_string();
                Launch::new(r"C:\windows\explorer.exe")
                    .cwd(&folders.runtime)
                    .args(plan::desktop_args(size, &exe_text, &plan.args))
            }
            None => Launch::new(&exe)
                .cwd(&folders.runtime)
                .args(plan.args.iter().copied()),
        };
        for (name, value) in &plan.env {
            launch = launch.env(*name, *value);
        }
        out.log.line("starting the game");
        let game_process = launch.spawn()?;
        let code = game_process.wait()?;
        out.log.line(&format!("game exited with {code}"));
        check_after_run(out, picture.size(), &before);
        Ok(code as i32)
    }

    /// Unpacks GOG's installer into `dest` with the bundled innoextract,
    /// once ([`source::unpack`]), showing its progress; with no window it
    /// is waited for all the same. `Ok(false)`: the player stopped it.
    fn unpack(
        out: &Out,
        installer: &source::Installer,
        dest: &Path,
        components_dir: &Path,
    ) -> Result<bool, String> {
        let mut log = |line: &str| out.log.line(line);
        let exe = components_dir.join(components::INNOEXTRACT);
        match source::unpack(&exe, installer, dest, &mut log, |job| {
            ui::show_unpack(installer, job)
        })? {
            Unpacked::Ready(done) => {
                out.log.line(&format!(
                    "unpacked game: {} ({})",
                    done.game.exe.display(),
                    done.game.edition.name
                ));
                Ok(true)
            }
            Unpacked::Cancelled => Ok(false),
        }
    }

    /// DXVK in or out through the ledger (a player's own `d3d9.dll`
    /// stays), and the generated files.
    fn install_components(
        out: &Out,
        plan: &plan::Plan,
        game: &Path,
        runtime_dir: &Path,
        components_dir: &Path,
    ) -> Result<(), String> {
        let failed =
            |error: std::io::Error| format!("Cannot set up {}: {error}", runtime_dir.display());
        let mut ledger = Ledger::load(game, runtime_dir, components::LEDGER);
        for name in &plan.installed {
            let bytes = std::fs::read(components_dir.join(name)).map_err(failed)?;
            match ledger.install(name, &bytes, &[]).map_err(failed)? {
                Outcome::NotOurs => out.warn(&format!(
                    "{name} in the private copy is not the launcher's; it was kept"
                )),
                outcome => out.log.line(&format!("component {name}: {outcome:?}")),
            }
        }
        for name in &plan.taken_out {
            let outcome = ledger.take_out(name, &[]).map_err(failed)?;
            if outcome != Outcome::Absent {
                out.log
                    .line(&format!("component {name} taken out: {outcome:?}"));
            }
        }
        for (name, contents) in &plan.files {
            let file = runtime_dir.join(name);
            match contents {
                Some(text) => fsx::write_atomic(&file, text).map_err(failed)?,
                None if file.is_file() => fsx::remove(&file).map_err(failed)?,
                None => {}
            }
        }
        Ok(())
    }

    /// Writes the plan's values and the game's old ones (once) to its HKCU
    /// key and reads them back; stops with what the game would miss.
    /// Returns the game's screen values from before, for
    /// [`check_after_run`].
    fn write_registry(out: &Out, values: Vec<Value>) -> Result<registry::Snapshot, String> {
        let written = registry::GAME
            .write(values)
            .map_err(|mismatches| registry::not_ready(&mismatches))?;
        if written.carried {
            out.log.line(&format!(
                "registry: the game's values carried over from HKLM\\{} (or its VirtualStore copy)",
                registry::MACHINE_KEY
            ));
        }
        out.log.line(&format!(
            "registry (HKCU\\{}): {}",
            registry::USER_KEY,
            registry::described(&written.values)
        ));
        Ok(written.before)
    }

    /// `--check`'s problems with the game's key: the redirect's rules, the
    /// close hook DLL, the player's `cmr5.exe` taking its import (the
    /// runtime copy is made from it on every Play), and the paths written
    /// and read back (the game's old values carried over, as Play does).
    fn check_key(components_dir: &Path, source: &Source, folders: &Folders) -> Vec<String> {
        let mut problems = Vec::new();
        if let Err(error) = registry::redirect() {
            problems.push(error.to_string());
        }
        let dll = components_dir.join(components::CLOSE_HOOK_DLL);
        if !dll.is_file() {
            problems.push(format!("{} is missing", dll.display()));
        }
        let exe_dir = match source {
            Source::Installed { dir, .. } => Some(dir),
            // Not unpacked yet: the first Play unpacks it and checks then.
            Source::Installer(installer) => {
                source::unpacked_already(&folders.unpacked, installer).then_some(&folders.unpacked)
            }
        };
        let import = exe_dir.map(|dir| {
            runtime::read_game_exe(dir).and_then(|(_, bytes)| runtime::game_exe(&bytes))
        });
        if let Some(Err(error)) = import {
            problems.push(error);
        }
        let paths = registry::writes(&folders.runtime, &folders.saves, None);
        if let Err(mismatches) = registry::GAME.write(paths) {
            problems.extend(mismatches.iter().map(ToString::to_string));
        }
        problems
    }

    /// After the game exits: when it could not use the size the launcher
    /// set, it ran at 640x480 and reset its Graphics options; put them and
    /// its last size back, and say so (`registry::after_run`).
    fn check_after_run(out: &Out, wanted: Option<(u32, u32)>, before: &registry::Snapshot) {
        let outcome = registry::after_run(wanted, before, &registry::GAME.snapshot());
        let registry::AfterRun::Fallback { restore, .. } = &outcome else {
            return;
        };
        if !restore.is_empty() {
            if let Err(mismatches) = registry::GAME.set(restore) {
                return out.warn(&registry::not_ready(&mismatches));
            }
            out.log.line(&format!(
                "registry restored: {}",
                registry::described(restore)
            ));
        }
        let message = outcome.message().unwrap_or_default();
        if outcome.needs_the_player() {
            out.log.line(&format!("warning: {message}"));
            out.say(&message);
        } else {
            out.tell(&message);
        }
    }
}
