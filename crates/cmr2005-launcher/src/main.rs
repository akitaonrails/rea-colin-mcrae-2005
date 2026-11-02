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
    use std::path::PathBuf;

    use std::path::Path;

    use cmr2005_launcher::cli::{self, Invocation, Mode};
    use cmr2005_launcher::components::{self, APP_OVERRIDES};
    use cmr2005_launcher::registry::{self, RegValue};
    use cmr2005_launcher::saves::{self, Folders};
    use cmr2005_launcher::source::{self, Source};
    use cmr2005_launcher::{config, edition, plan, runtime, ui};
    use rea_launcher::ledger::{Ledger, Outcome};
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, paths};
    use rea_launcher_win32::console::{self, Output};
    use rea_launcher_win32::dialog::{self, Icon};
    use rea_launcher_win32::instance::{self, Window};
    use rea_launcher_win32::process::Launch;
    use rea_launcher_win32::reg::Key;
    use rea_launcher_win32::{disk, display, gpu, wine};

    /// The single-instance name (a named mutex per user session).
    const INSTANCE: &str = "cmr2005";

    const TITLE: &str = "Colin McRae Rally 2005";

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

    /// Where messages go: the starting terminal, a dialog when a person
    /// started it without one, or only `launcher.log`.
    struct Out {
        log: Log,
        attached: bool,
        interactive: bool,
    }

    impl Out {
        fn say(&self, text: &str, icon: Icon) {
            match console::output(self.attached, console::stdout_usable(), self.interactive) {
                Output::Terminal if icon == Icon::Info => println!("{text}"),
                Output::Terminal => eprintln!("{text}"),
                Output::Dialog => dialog::message(TITLE, text, icon),
                Output::Nowhere => {}
            }
        }

        fn warn(&self, message: &str) {
            self.log.line(&format!("warning: {message}"));
        }
    }

    pub fn run() -> i32 {
        let log = Log::new(home().join("launcher.log"));
        log.line(concat!("CMR2005Launcher ", env!("CARGO_PKG_VERSION")));
        let parsed = cli::parse(std::env::args().skip(1));
        let (attached, interactive) = match &parsed {
            Ok(invocation) => (
                invocation.wants_console() && console::attach_parent(),
                invocation.interactive(),
            ),
            Err(_) => (console::attach_parent(), true),
        };
        let out = Out {
            log,
            attached,
            interactive,
        };
        let invocation = match parsed {
            Ok(invocation) => invocation,
            Err(error) => {
                out.log.line(&format!("error: {error}"));
                out.say(&error, Icon::Error);
                return cli::USAGE_ERROR;
            }
        };
        match run_mode(&out, invocation) {
            Ok(code) => code,
            Err(error) => {
                out.log.line(&format!("error: {error}"));
                out.say(&error, Icon::Error);
                cli::REFUSED
            }
        }
    }

    fn run_mode(out: &Out, invocation: Invocation) -> Result<i32, String> {
        if invocation.mode == Mode::Help {
            out.say(&config::usage(), Icon::Info);
            return Ok(cli::OK);
        }
        // Held until the game exits: a second start brings the window
        // forward, or says the game is running; an action flag is refused.
        let instance = match instance::single(INSTANCE) {
            Ok(guard) => guard,
            Err(running) => {
                if invocation.mode == Mode::Check {
                    return Err(
                        "The launcher is already open; close it first, or use its window.".into(),
                    );
                }
                if !running.bring_to_front(Window::Title(TITLE)) {
                    out.say("Colin McRae Rally 2005 is already running.", Icon::Info);
                }
                out.log.line("second start: another launcher is running");
                return Ok(cli::OK);
            }
        };
        let path = settings_path();
        if let Mode::Window { settings_only } = invocation.mode {
            if wine::is_wine() {
                match gpu::prefer_glx() {
                    Ok(true) => {
                        // Wine chose EGL when this process started; on EGL the
                        // window's OpenGL setup fails (a fresh prefix's first
                        // start). The setting counts from the next process, so
                        // run again; that one finds it set and goes on.
                        out.log.line(
                            "settings window: OpenGL through GLX (UseEGL=N); starting again to use it",
                        );
                        let exe = std::env::current_exe()
                            .map_err(|error| format!("Cannot find the launcher: {error}"))?;
                        // The next process takes the single-instance lock.
                        drop(instance);
                        let code = Launch::new(&exe).args(std::env::args().skip(1)).run()?;
                        return Ok(code as i32);
                    }
                    Ok(false) => {}
                    Err(error) => out.warn(&error.to_string()),
                }
            }
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
        let screen = display::screen_size();
        let existing = cfg::read_file(&path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        let first_run = existing.is_none();
        let text = existing.unwrap_or_else(|| config::first_run_text(screen));
        let (saved, warnings) = config::parse_file(&text);
        for warning in &warnings {
            out.warn(warning);
        }
        let settings = saved.clone().merged(invocation.flags);
        let cwd = std::env::current_dir().unwrap_or_default();
        let (game, source) = cli::check(&settings, &cwd)?;
        out.log
            .line(&format!("game: {} ({})", game.display(), source.describe()));
        if invocation.mode == Mode::Check {
            out.say(&cli::check_report(&game, &source), Icon::Info);
            return Ok(cli::OK);
        }
        if first_run || saved.game.as_ref() != Some(&game) {
            let text = config::set(&text, "game", &game.display().to_string());
            fsx::write_atomic(&path, text)
                .map_err(|error| format!("Cannot save {}: {error}", path.display()))?;
        }

        let wine = wine::is_wine();
        let components_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(components::DIR)))
            .ok_or("Cannot find the launcher's components folder.")?;
        components::validate(&components_dir, wine)?;
        for line in components::describe(&components_dir) {
            out.log.line(&format!("component {line}"));
        }

        let folders = Folders::new(&home(), settings.runtime.as_deref());
        let game_dir = match &source {
            Source::Installed { dir, .. } => dir.clone(),
            Source::Installer(installer) => {
                unpack(out, installer, &folders.unpacked, &components_dir)?;
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
        if let Some((w, h)) = picture.size() {
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
        let plan = plan::plan(&picture, &folders.runtime, &folders.saves, wine);
        install_components(out, &plan, &game_dir, &folders.runtime, &components_dir)?;
        write_registry(out, &plan)?;
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
        let mut launch = Launch::new(&exe)
            .cwd(&folders.runtime)
            .args(plan.args.iter().copied());
        for (name, value) in &plan.env {
            launch = launch.env(*name, *value);
        }
        out.log.line("starting the game");
        let game_process = launch.spawn()?;
        let hook_dll = components_dir.join(components::CLOSE_HOOK_DLL);
        let _hook = match game_process.message_hook(&hook_dll, c"close_hook") {
            Ok(hook) => Some(hook),
            Err(error) => {
                out.warn(&format!("close hook not installed: {error}"));
                None
            }
        };
        let code = game_process.wait()?;
        out.log.line(&format!("game exited with {code}"));
        Ok(code as i32)
    }

    /// Unpacks GOG's installer with the bundled innoextract, once.
    fn unpack(
        out: &Out,
        installer: &source::Installer,
        dest: &Path,
        components_dir: &Path,
    ) -> Result<(), String> {
        if source::is_unpacked(dest, installer) {
            return Ok(());
        }
        let parent = dest.parent().unwrap_or(dest);
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create {}: {error}", parent.display()))?;
        if let Some(free) = disk::free_space(parent)
            && free < source::UNPACK_BYTES
        {
            return Err(format!(
                "Unpacking the game needs about {} GB free in {}; there are {:.1} GB.",
                source::UNPACK_BYTES / 1_000_000_000,
                parent.display(),
                free as f64 / 1e9
            ));
        }
        let innoextract = components_dir.join(components::INNOEXTRACT);
        out.log.line(&format!(
            "unpacking {} into {} (a few minutes)",
            installer.setup.display(),
            dest.display()
        ));
        let found = source::unpack(installer, dest, |args| {
            let code = Launch::new(&innoextract)
                .args(args.iter().map(String::as_str))
                .hidden(true)
                .run()?;
            if code == 0 {
                Ok(())
            } else {
                Err(format!(
                    "Unpacking the game from {} failed (innoextract exit code {code}).",
                    installer.setup.display()
                ))
            }
        })?;
        out.log.line(&format!(
            "unpacked: {} ({})",
            found.exe.display(),
            found.edition.name
        ));
        Ok(())
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

    /// The game's registry values: HKLM (32-bit view), or the per-user
    /// VirtualStore when HKLM can't be written.
    fn write_registry(out: &Out, plan: &plan::Plan) -> Result<(), String> {
        let (key, place) = Key::machine_or_virtual_store(registry::KEY)
            .map_err(|error| format!("Cannot open the game's registry key: {error}"))?;
        for write in &plan.registry {
            match &write.value {
                RegValue::String(text) => key.set_string(write.name, text),
                RegValue::Dword(n) => key.set_dword(write.name, *n),
            }
            .map_err(|error| format!("Cannot write the game's registry: {error}"))?;
        }
        out.log.line(&format!(
            "registry ({}): {}",
            place.describe(),
            plan.registry
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        ));
        Ok(())
    }
}
