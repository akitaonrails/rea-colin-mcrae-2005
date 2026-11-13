//! The settings window, on `rea_launcher_ui`.
//!
//! [`draw`] is the whole window as a function of [`Window`], so the
//! headless tests below drive the real widgets (egui_kittest) on the host
//! and under `sdk/scripts/wine-test`; `show` wraps it in eframe on Windows.
//!
//! Layout: the hero image strip on the left, the title and status, the
//! Play tab (game folder and look cards), the Picture tab (the picture
//! rows of `config::OPTIONS`) and the Wheel tab (the SDK's `WheelTab`:
//! the `wheel*` options, and the pedal calibration it keeps in
//! `rea_wheel.ini` beside `launcher.cfg`), the footer with the Play plate.
//! Any change to the window's look or layout goes through a design skill
//! (docs/STANDARDS.md "Settings window and CLI").

use std::path::PathBuf;

use rea_launcher::cfg::{OptionSpec, spec};
use rea_launcher::wheel::dinput::options as wheel_options;
use rea_launcher::wheel::rig::{self, WheelDevices};
use rea_launcher_ui::egui::{self, Align2, Rect, RichText, TextureHandle, pos2, vec2};
use rea_launcher_ui::spec::{mark_rendered, spec_path_row, spec_row};
use rea_launcher_ui::theme::{body, condensed};
use rea_launcher_ui::wheel::WheelTab;
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::config::{self, Look, OPTIONS};
use crate::edition;
use crate::form::Form;
use crate::palette::{BASE, ERROR, INK, MUTED, PANEL, SEPIA};
use crate::source::{self, Source};

/// Width of the hero image strip.
pub const PHOTO_W: f32 = 280.0;

/// The hero image: Colin McRae's Škoda Fabia WRC at Wales Rally GB 2005,
/// photo by KevBow (CC BY 2.0, via Wikimedia Commons), cropped to the
/// strip and toned sepia like the game's menu backdrops (THIRD_PARTY.md).
pub const HERO_JPEG: Option<&[u8]> = Some(include_bytes!("../assets/hero.jpg"));

/// The image's credit, which its licence asks for.
pub const HERO_CREDIT: &str = "Photo: KevBow, CC BY 2.0 (Wikimedia Commons)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Play,
    Picture,
    Wheel,
}

impl Tab {
    pub const NAMES: [(Tab, &'static str); 3] = [
        (Tab::Play, "Play"),
        (Tab::Picture, "Picture"),
        (Tab::Wheel, "Wheel"),
    ];

    /// The next (`by` = 1) or previous (-1) tab, wrapping (pad LB/RB).
    pub fn step(self, by: isize) -> Tab {
        let at = Tab::NAMES
            .iter()
            .position(|(tab, _)| *tab == self)
            .unwrap_or(0) as isize;
        let len = Tab::NAMES.len() as isize;
        Tab::NAMES[(at + by).rem_euclid(len) as usize].0
    }
}

/// What the game-folder check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Empty,
    /// A playable folder: the note under the field ([`found_note`]).
    Found(String),
    /// Why the folder cannot be played (an unknown edition names its
    /// SHA-256 and the issue link).
    Problem(String),
}

/// Checks a game-folder text as typed or picked.
pub fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    match source::detect(std::path::Path::new(folder)) {
        Ok(found) => Check::Found(found_note(&found)),
        Err(problem) => Check::Problem(problem),
    }
}

/// The note under the game folder for a playable folder: the edition; for
/// the folder the launcher unpacked, that the installer is no longer
/// needed; for GOG's installer, what the first Play does with it.
pub fn found_note(found: &Source) -> String {
    match found {
        Source::Installed { dir, found } if source::is_unpacked_folder(dir) => format!(
            "Found {}, unpacked by the launcher from GOG's installer. \
             The installer files are no longer needed.",
            found.edition.name
        ),
        Source::Installed { found, .. } => format!("Found {}.", found.edition.name),
        Source::Installer(installer) => source::installer_plan(installer, None),
    }
}

/// The window's state.
pub struct Window {
    pub form: Form,
    pub tab: Tab,
    /// The screen the window opened on, for the looks.
    pub screen: Option<(u32, u32)>,
    /// Started by the Linux helper.
    pub linux: bool,
    pub check: Check,
    /// The folder text [`Window::check`] is for.
    checked: String,
    /// The folder picker (`dialog::pick_folder` on Windows).
    pub pick: fn() -> Option<PathBuf>,
    pub hero: Option<TextureHandle>,
    /// A message that replaces the status line (a failed save).
    pub error: Option<String>,
    /// Play was pressed while ready: the caller saves and closes.
    pub play: bool,
    /// The Wheel tab, over the `wheel*` options and the calibration.
    pub wheel: WheelTab,
    /// The controllers it shows: DirectInput on Windows, a fixed list in
    /// tests.
    pub devices: Box<dyn WheelDevices>,
}

impl Window {
    /// The window over a `launcher.cfg` text and the calibration file's
    /// (`None`: no file yet).
    pub fn new(text: &str, calibration: Option<&str>, screen: Option<(u32, u32)>) -> Window {
        let form = Form::from_text(text);
        let checked = form.get("game").to_owned();
        let wheel = WheelTab::new(form.settings().wheel, calibration);
        Window {
            check: check_game(&checked),
            checked,
            form,
            tab: Tab::Play,
            screen,
            linux: false,
            pick: || None,
            hero: None,
            error: None,
            play: false,
            wheel,
            devices: Box::new(rig::Fixed::default()),
        }
    }

    /// The calibration file's new text (`existing`, its other lines
    /// kept); `None` while there is no calibration.
    pub fn calibration_text(&self, existing: &str) -> Option<String> {
        self.wheel.calibration_text(existing)
    }

    /// The Wheel tab's settings into the form, so Play saves them to
    /// `launcher.cfg` as their flags spell them.
    fn sync_wheel(&mut self) {
        for row in wheel_options::OPTIONS {
            *self.form.value_mut(row.key) = self.wheel.setup.value(row.key).unwrap_or_default();
        }
    }

    /// A known edition is chosen and every value can be saved.
    pub fn ready(&self) -> bool {
        matches!(self.check, Check::Found(_)) && self.form.problems().is_empty()
    }

    /// Play (the plate, Enter or the pad's Start): only when ready; else
    /// back to the Play tab, where the reason is shown.
    pub fn press_play(&mut self) {
        if self.ready() {
            self.play = true;
        } else {
            self.tab = Tab::Play;
        }
    }

    fn refresh_check(&mut self) {
        if self.form.get("game") != self.checked {
            self.checked = self.form.get("game").to_owned();
            self.check = check_game(&self.checked);
        }
    }
}

fn row(key: &str) -> &'static OptionSpec {
    spec::find(OPTIONS, key).unwrap_or_else(|| panic!("no option row {key}"))
}

/// The whole window, filling `ui`.
pub fn draw(ui: &mut egui::Ui, w: &mut Window) {
    w.refresh_check();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, BASE);
    let layout = Layout::new(full, PHOTO_W);
    draw_hero(ui, layout.photo, w.hero.as_ref());
    ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
        ui.add(
            egui::Label::new(
                RichText::new(crate::GAME_TITLE)
                    .font(condensed(38.0))
                    .color(INK),
            )
            .wrap(),
        );
        status(ui, w);
        ui.add_space(6.0);
        widgets::tabs_bar(ui, &Tab::NAMES, &mut w.tab);
        ui.add_space(14.0);
        widgets::tab_body(ui, w.tab, |ui| match w.tab {
            Tab::Play => play_tab(ui, w),
            Tab::Picture => picture_tab(ui, w),
            Tab::Wheel => wheel_tab(ui, w),
        });
    });
    let hint = "Your game folder is never changed. Settings are saved when you press Play.";
    if widgets::footer(ui, layout.footer, hint, w.ready()).clicked() {
        w.press_play();
    }
}

fn draw_hero(ui: &egui::Ui, rect: Rect, hero: Option<&TextureHandle>) {
    let painter = ui.painter();
    if hero.is_none() {
        // The empty slot until the imagery is researched (HERO_JPEG).
        painter.rect_filled(rect, 0.0, PANEL);
    }
    widgets::hero_image(painter, rect, hero, Crop::Center, BASE);
    if !HERO_CREDIT.is_empty() {
        // Small text never sits on the photo itself: a solid band under it.
        let band = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 34.0), rect.max);
        painter.rect_filled(band, 0.0, BASE);
        painter.text(
            pos2(rect.left() + 14.0, rect.bottom() - 11.0),
            Align2::LEFT_BOTTOM,
            HERO_CREDIT,
            body(11.0),
            MUTED,
        );
    }
}

fn status(ui: &mut egui::Ui, w: &Window) {
    let problems = w.form.problems();
    let (text, color) = match (&w.error, &w.check) {
        (Some(error), _) => (error.clone(), ERROR),
        (None, Check::Found(_)) if !problems.is_empty() => (problems.join(" "), ERROR),
        (None, Check::Found(_)) => ("Ready. Press Play or Enter.".to_owned(), MUTED),
        (None, _) => (
            "First, show the launcher where the game is.".to_owned(),
            MUTED,
        ),
    };
    ui.label(RichText::new(text).font(body(14.0)).color(color));
}

fn play_tab(ui: &mut egui::Ui, w: &mut Window) {
    widgets::heading(ui, "Game");
    if spec_path_row(ui, row("game"), w.form.value_mut("game"), w.pick).changed() {
        w.error = None;
    }
    let (note, color) = match &w.check {
        Check::Empty if w.linux => (
            format!(
                "Choose the folder with {}, or the one with GOG's {} and its .bin files. \
                 Under Wine, Z:\\ is your Linux root folder.",
                edition::GAME_EXE,
                source::SETUP_EXE
            ),
            MUTED,
        ),
        Check::Empty => (
            format!(
                "Choose the folder with {}, or the one with GOG's {} and its .bin files.",
                edition::GAME_EXE,
                source::SETUP_EXE
            ),
            MUTED,
        ),
        Check::Found(note) => (note.clone(), SEPIA),
        Check::Problem(problem) => (problem.clone(), ERROR),
    };
    ui.add(egui::Label::new(RichText::new(note).font(body(13.0)).color(color)).selectable(true));
    ui.add_space(12.0);

    widgets::heading(ui, row("look").label);
    // The cards are the `look` option's window control.
    mark_rendered(ui.ctx(), "look");
    ui.add_space(6.0);
    let recommended = Look::recommended(w.screen);
    let gap = 10.0;
    let count = Look::ALL.len() as f32;
    let width = (ui.available_width() - (count - 1.0) * gap) / count;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for look in Look::ALL {
            let note = look_note(look, w.screen);
            let card = LookCard {
                title: look.label(),
                note: &note,
                selected: w.form.look() == Some(look),
                badge: (look == recommended).then_some("For this screen"),
                ..LookCard::default()
            };
            if widgets::look_card(ui, vec2(width, 96.0), &card).clicked() {
                w.form.choose_look(look, w.screen);
            }
        }
    });
    widgets::note(ui, row("look").description);
}

/// A look card's note: its values in words.
fn look_note(look: Look, screen: Option<(u32, u32)>) -> String {
    let picture = look.picture(screen);
    let size = match picture.resolution {
        Some(rea_launcher::cfg::Resolution::Size(w, h)) => format!("{w}×{h}"),
        Some(_) => "The game's own size".to_owned(),
        None => "Your current size".to_owned(),
    };
    let fps = match picture.fps_limit {
        Some(fps) if fps > 0 => format!(", {fps} fps"),
        _ => String::new(),
    };
    let vsync = match picture.vsync {
        Some(true) => ", VSync on.",
        Some(false) => ", VSync off.",
        None => ".",
    };
    format!("{size}{fps}{vsync}")
}

/// The SDK's Wheel tab: its controls are the `wheel*` options' (each
/// marks its key for the parity test).
fn wheel_tab(ui: &mut egui::Ui, w: &mut Window) {
    w.wheel.draw(ui, &mut *w.devices);
    w.sync_wheel();
}

/// Every option row with a window control that is not on the Play or the
/// Wheel tab.
fn picture_tab(ui: &mut egui::Ui, w: &mut Window) {
    let mut changed = false;
    for row in OPTIONS.iter().filter(|row| {
        !row.launcher_only && !matches!(row.key, "game" | "look") && !config::is_wheel_key(row.key)
    }) {
        changed |= spec_row(ui, row, w.form.value_mut(row.key)).changed();
        ui.add_space(10.0);
    }
    if changed {
        w.form.picture_edited(w.screen);
    }
}

/// What the unpack window shows for `build`: the game, the installer and
/// the photo's credit, which its licence asks for wherever it is shown.
pub fn unpacking(
    build: &rea_launcher::installer::InstallerBuild,
) -> rea_launcher_ui::unpack::Unpacking {
    rea_launcher_ui::unpack::Unpacking::new(crate::GAME_TITLE, build.name, build.unpacked)
        .with_credit(HERO_CREDIT)
}

#[cfg(windows)]
pub use app::{Outcome, show, show_unpack};

#[cfg(windows)]
mod app {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use rea_launcher::fsx;
    use rea_launcher::wheel::dinput;
    use rea_launcher_ui::eframe;
    use rea_launcher_ui::egui;
    use rea_launcher_ui::pad::{self, Nav, PadNav};
    use rea_launcher_ui::theme;
    use rea_launcher_ui::widgets;
    use rea_launcher_ui::window::{SIZE, WindowFit};
    use rea_launcher_win32::dinput::DirectInput;
    use rea_launcher_win32::{dialog, display, wine, xinput};

    use rea_launcher::installer::{Installer, UnpackJob, UnpackOutcome};
    use rea_launcher_ui::unpack;

    use super::{HERO_JPEG, PHOTO_W, Window, draw};
    use crate::config;
    use crate::palette::{METRICS, PALETTE};

    /// The SDK's unpack window over `job` (GOG's installer being unpacked
    /// on the first Play), sized like the settings window, with its photo.
    pub fn show_unpack(
        found: &Installer,
        job: &mut dyn UnpackJob,
    ) -> Result<UnpackOutcome, String> {
        let scale = display::wine_dpi_scale();
        let screen = display::screen_size();
        let fit = WindowFit::new(SIZE, wine::is_wine(), screen, scale).with_native_fit(true);
        unpack::show(
            &PALETTE,
            &METRICS,
            unpack::Window {
                title: crate::WINDOW_TITLE,
                fit,
                hero_jpeg: HERO_JPEG,
                photo_width: PHOTO_W,
            },
            super::unpacking(found.build),
            job,
            xinput::buttons,
        )
    }

    /// How the window ended.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Outcome {
        /// Play was pressed; the settings are saved.
        Play,
        /// Closed without Play.
        Closed,
    }

    struct App<'a> {
        window: Window,
        fit: WindowFit,
        path: PathBuf,
        /// The `launcher.cfg` text the window's values are saved into.
        text: String,
        /// The pedal calibration (`rea_wheel.ini`), beside `launcher.cfg`.
        calibration_path: PathBuf,
        pad: PadNav,
        pending: Vec<Nav>,
        outcome: &'a mut Outcome,
    }

    /// Shows the window over `saved` (the `launcher.cfg` text, or `None` on
    /// the first run) and saves to `path` when Play is pressed.
    pub fn show(path: &Path, saved: Option<&str>, linux: bool) -> Result<Outcome, String> {
        // Under Wine the window is created at its final size, with Wine's
        // DPI scale divided out (read first: it also makes the process DPI
        // aware before the screen size is taken).
        let scale = display::wine_dpi_scale();
        let screen = display::screen_size();
        let fit = WindowFit::new(SIZE, wine::is_wine(), screen, scale).with_native_fit(true);
        let options = fit.native_options(crate::WINDOW_TITLE);
        let text = saved.map_or_else(|| config::first_run_text(screen), str::to_owned);
        let calibration_path = path.with_file_name(dinput::FILE);
        let calibration = std::fs::read_to_string(&calibration_path).ok();
        let mut outcome = Outcome::Closed;
        eframe::run_native(
            crate::WINDOW_TITLE,
            options,
            Box::new(|cc| {
                theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
                let mut window = Window::new(&text, calibration.as_deref(), screen);
                window.devices = Box::new(DirectInput::start());
                window.linux = linux;
                window.pick = || {
                    dialog::pick_folder("Choose the folder with cmr5.exe or setup_colin2005.exe")
                };
                window.hero =
                    HERO_JPEG.and_then(|jpeg| widgets::load_jpeg(&cc.egui_ctx, "hero", jpeg));
                Ok(Box::new(App {
                    window,
                    fit,
                    path: path.to_path_buf(),
                    text: text.clone(),
                    calibration_path,
                    pad: PadNav::default(),
                    pending: Vec::new(),
                    outcome: &mut outcome,
                }))
            }),
        )
        .map_err(|error| error.to_string())?;
        Ok(outcome)
    }

    impl App<'_> {
        fn save_and_close(&mut self, ctx: &egui::Context) {
            let text = self.window.form.save(&self.text);
            let existing = std::fs::read_to_string(&self.calibration_path).unwrap_or_default();
            let saved = fsx::write_atomic(&self.path, text).and_then(|()| {
                match self.window.calibration_text(&existing) {
                    Some(calibration) => fsx::write_atomic(&self.calibration_path, calibration),
                    None => Ok(()),
                }
            });
            match saved {
                Ok(()) => {
                    *self.outcome = Outcome::Play;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(error) => {
                    self.window.error = Some(format!("Cannot save the settings: {error}"));
                }
            }
        }
    }

    impl eframe::App for App<'_> {
        fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
            let navs = self.pad.update(xinput::buttons());
            pad::inject(ctx, raw, &navs);
            self.pending.extend(navs);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            self.fit.first_frame(&ctx);
            // Taken, so a second egui pass in one frame can't play twice.
            for nav in std::mem::take(&mut self.pending) {
                match nav {
                    Nav::Play => self.window.press_play(),
                    Nav::NextTab => self.window.tab = self.window.tab.step(1),
                    Nav::PreviousTab => self.window.tab = self.window.tab.step(-1),
                    _ => {}
                }
            }
            if ctx.input(|input| input.key_pressed(egui::Key::Enter))
                && !ctx.egui_wants_keyboard_input()
            {
                self.window.press_play();
            }
            draw(ui, &mut self.window);
            if std::mem::take(&mut self.window.play) {
                self.save_and_close(&ctx);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, Settings};
    use crate::palette::{METRICS, PALETTE};
    use egui::Role;
    use rea_launcher::cfg::Resolution;
    use rea_launcher_ui::spec::rendered_keys;
    use rea_launcher_ui::testing::{self, Harness, Queryable};

    fn open(text: &str) -> Harness<'static, Window> {
        testing::harness(
            &PALETTE,
            &METRICS,
            Window::new(text, None, Some((1920, 1080))),
            draw,
        )
    }

    fn flags(args: &[&str]) -> Settings {
        let args = args.iter().map(|arg| (*arg).to_owned());
        config::parse_args(args).unwrap().0
    }

    /// GOG's installer, as `source::detect` finds it in `dir`.
    fn gog_installer(dir: &std::path::Path) -> Source {
        Source::Installer(source::Installer {
            setup: dir.join(source::SETUP_EXE),
            sha256: source::INSTALLERS[0].sha256,
            build: &source::INSTALLERS[0].value,
        })
    }

    #[test]
    fn the_installer_folder_says_it_is_unpacked_once_and_play_is_ready() {
        let note = found_note(&gog_installer(std::path::Path::new(r"D:\GOG\CMR2005")));
        assert!(
            note.starts_with(
                "Found GOG offline installer (v1.1). The first Play unpacks the game from it once"
            ),
            "{note}"
        );
        assert!(note.contains("about 3.9 GB free space"), "{note}");
        assert!(note.contains("never run"), "{note}");

        let mut ui = open("");
        ui.run();
        ui.state_mut().check = Check::Found(note.clone());
        ui.run();
        assert!(
            ui.query_by_label_contains("Found GOG offline installer (v1.1).")
                .is_some()
        );
        assert!(!testing::disabled(
            &ui.get_by_role_and_label(Role::Button, "Play")
        ));
    }

    #[test]
    fn the_unpacked_folder_says_the_installer_is_no_longer_needed() {
        let dir = std::env::temp_dir().join(format!("cmr2005-ui-unpacked-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let installed = |dir: &std::path::Path| Source::Installed {
            dir: dir.to_path_buf(),
            found: edition::Found {
                exe: dir.join(edition::GAME_EXE),
                edition: &edition::EDITIONS[0].value,
            },
        };
        assert_eq!(found_note(&installed(&dir)), "Found PC v1.1 (GOG).");
        for marker in [rea_launcher::installer::MARKER, source::OLD_MARKER] {
            std::fs::write(dir.join(marker), b"digest").unwrap();
            let note = found_note(&installed(&dir));
            assert!(note.contains("unpacked by the launcher"), "{note}");
            assert!(note.contains("no longer needed"), "{note}");
            std::fs::remove_file(dir.join(marker)).unwrap();
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The SDK's unpack window with this launcher's palette, title,
    /// installer name and the photo's credit (as in the settings window);
    /// "Stop unpacking" stops it.
    #[test]
    fn the_unpack_window_names_the_game_and_the_installer_and_can_be_stopped() {
        use rea_launcher_ui::unpack;
        let gog = &source::INSTALLERS[0].value;
        let mut ui = testing::harness(&PALETTE, &METRICS, unpacking(gog), unpack::draw_unpack);
        ui.run();
        ui.get_by_role_and_label(Role::Label, HERO_CREDIT);
        assert!(
            ui.query_by_label_contains("Colin McRae Rally 2005")
                .is_some()
        );
        assert!(
            ui.query_by_label_contains("from GOG offline installer (v1.1)")
                .is_some()
        );
        assert!(ui.query_by_label_contains("of 3.6 GB").is_some());
        ui.get_by_role_and_label(Role::Button, unpack::STOP).click();
        ui.run();
        assert!(ui.state().cancel);
    }

    #[test]
    fn the_window_has_a_control_for_every_option_and_no_other() {
        let mut ui = open("");
        for tab in ["Picture", "Wheel"] {
            ui.get_by_role_and_label(Role::Tab, tab).click();
            ui.run();
        }
        spec::assert_parity(OPTIONS, &rendered_keys(&ui.ctx));
    }

    /// A CSL DD with its pedals in the base, resting.
    fn csl() -> rig::Device {
        use rea_launcher::wheel::{DeviceId, DeviceState, Kind};
        let state = DeviceState {
            axes: vec![0, 32_767, 32_767, 0, 0, 0, 0, 0],
            buttons: vec![false; 12],
        };
        rig::Device {
            id: DeviceId {
                name: "FANATEC CSL DD".into(),
                vid: 0x0eb7,
                pid: 0x0020,
                guid: "{csl}".into(),
            },
            kind: Kind::Wheel,
            axes: 8,
            buttons: 12,
            ffb: true,
            initial: state.clone(),
            state,
        }
    }

    /// The window on its Wheel tab with a wheel attached, tall enough that
    /// the whole tab is in view.
    fn wheel_window(text: &str, ini: Option<&str>) -> Harness<'static, Window> {
        let mut window = Window::new(text, ini, Some((1920, 1080)));
        window.devices = Box::new(rig::Fixed {
            devices: vec![csl()],
            ..rig::Fixed::default()
        });
        let mut ui = testing::harness_sized(&PALETTE, &METRICS, vec2(1280.0, 2400.0), window, draw);
        ui.get_by_role_and_label(Role::Tab, "Wheel").click();
        ui.run();
        ui
    }

    fn choose(ui: &mut Harness<'static, Window>, role: Role, label: &str) {
        ui.get_by_role_and_label(role, label).click();
        ui.run();
    }

    /// Every wheel control saves its `launcher.cfg` key as its flag
    /// spells it, and reaches the layer's `rea_wheel.ini` in the runtime.
    #[test]
    fn every_wheel_control_saves_what_its_flag_sets() {
        let mut ui = wheel_window("", None);
        for (role, label, flag, value, line) in [
            (Role::RadioButton, "On", "--wheel", "on", "mode = on"),
            (
                Role::RadioButton,
                "900°",
                "--wheel-lock",
                "900",
                "lock = 900",
            ),
            (
                Role::RadioButton,
                "4 %",
                "--wheel-deadzone",
                "4",
                "deadzone = 4",
            ),
            (
                Role::RadioButton,
                "Mild",
                "--wheel-curve",
                "40",
                "linearity = 40",
            ),
            (
                Role::Switch,
                "Invert throttle",
                "--wheel-invert-throttle",
                "on",
                "invert_accel = on",
            ),
            (
                Role::Switch,
                "Invert brake",
                "--wheel-invert-brake",
                "on",
                "invert_brake = on",
            ),
            (
                Role::RadioButton,
                "75 %",
                "--wheel-ffb",
                "75",
                "ffb_strength = 75",
            ),
            (
                Role::Switch,
                "Reverse force feedback",
                "--wheel-ffb-invert",
                "on",
                "ffb_invert = on",
            ),
        ] {
            choose(&mut ui, role, label);
            let saved = ui.state().form.settings();
            let key = flag.trim_start_matches("--").replace('-', "_");
            assert_eq!(
                saved.wheel.value(&key),
                flags(&[flag, value]).wheel.value(&key),
                "{label}"
            );
            assert_eq!(saved.wheel, ui.state().wheel.setup, "{label}");
            let layer = layer_file(&game_gets(&ui));
            assert!(layer.contains(line), "{label}: {line} in\n{layer}");
        }
        let env = game_gets(&ui).env;
        assert!(env.contains(&(crate::wheel::MODE_ENV, "on")), "{env:?}");
    }

    /// `launcher.cfg` holds the settings; the calibration file only the
    /// calibration, which the layer's file also gets.
    #[test]
    fn the_tab_opens_with_launcher_cfg_and_the_calibration() {
        let csl = csl();
        let profile = rea_launcher::wheel::dinput::Profile::guess(csl.id, &csl.initial, None);
        let calibration = profile.to_cfg("# mine\n", "");
        let ui = wheel_window("wheel = off\nwheel_lock = 270\n", Some(&calibration));
        let setup = &ui.state().wheel.setup;
        assert_eq!(setup.mode, rea_launcher::wheel::dinput::Mode::Off);
        assert_eq!(setup.wheel.lock.to_setting(), 270);
        let saved = ui.state().calibration_text(&calibration).unwrap();
        assert!(saved.starts_with("# mine\n"), "{saved}");
        assert!(!saved.contains("lock ="), "no settings in it: {saved}");
        let layer = layer_file(&game_gets_with(&ui, &calibration));
        assert_eq!(
            rea_launcher::wheel::dinput::Profile::from_cfg(&layer),
            Some(profile)
        );
    }

    #[test]
    fn the_wheel_starts_at_540_degrees_and_auto() {
        let ui = wheel_window("", None);
        assert!(testing::checked(
            &ui.get_by_role_and_label(Role::RadioButton, "540°")
        ));
        assert!(layer_file(&game_gets(&ui)).contains("lock = 540"));
        assert!(ui.state().calibration_text("").is_none(), "nothing to save");
    }

    #[test]
    fn wheel_support_off_shows_nothing_else_and_saves_off() {
        let mut ui = wheel_window("", None);
        // The first "Off" is wheel support's; force feedback has one too.
        ui.get_all_by_role_and_label(Role::RadioButton, "Off")
            .next()
            .unwrap()
            .click();
        ui.run();
        assert!(ui.query_by_label("Steering lock").is_none());
        assert_eq!(
            ui.state().form.settings().wheel.mode,
            rea_launcher::wheel::dinput::Mode::Off
        );
        assert!(layer_file(&game_gets(&ui)).contains("mode = off"));
    }

    #[test]
    fn a_look_card_saves_what_its_flag_sets() {
        let mut ui = open("");
        ui.get_by_label("Steam Deck").click();
        ui.run();
        let saved = ui.state().form.settings();
        assert_eq!(saved.look, Some(Look::SteamDeck));
        let screen = ui.state().screen;
        let flag = flags(&["--look", "steam_deck"]);
        assert_eq!(saved.picture(screen), flag.picture(screen));
        assert_eq!(
            saved.picture(screen).resolution,
            Some(Resolution::Size(1280, 720))
        );
        assert!(testing::checked(&ui.get_by_label("Steam Deck")));
    }

    /// What the game gets from the window's saved settings (Wine).
    /// What the game gets from the window's saved files (Wine, no
    /// controller attached).
    fn game_gets(ui: &Harness<'static, Window>) -> crate::plan::Plan {
        game_gets_with(ui, "")
    }

    /// [`game_gets`] with a calibration file.
    fn game_gets_with(ui: &Harness<'static, Window>, calibration: &str) -> crate::plan::Plan {
        let settings = ui.state().form.settings();
        let picture = settings.picture(ui.state().screen);
        let setup = &settings.wheel;
        crate::plan::plan(
            &picture,
            settings.advanced_particles(),
            std::path::Path::new(r"C:\r"),
            std::path::Path::new(r"C:\s"),
            true,
        )
        .with_wheel(
            crate::wheel::shown(setup, Vec::new),
            crate::wheel::runtime_ini(setup, calibration),
        )
    }

    /// The runtime's `rea_wheel.ini` in a plan.
    fn layer_file(plan: &crate::plan::Plan) -> String {
        let file = plan.files.iter().find(|(name, _)| *name == "rea_wheel.ini");
        file.and_then(|(_, text)| text.clone()).unwrap_or_default()
    }

    fn registry(plan: &crate::plan::Plan) -> Vec<String> {
        plan.registry.iter().map(crate::registry::shown).collect()
    }

    #[test]
    fn the_resolution_field_reaches_the_registry_and_the_widescreen_view() {
        let mut ui = open("");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        let field = ui.get_by_role_and_label(Role::TextInput, "Resolution");
        field.focus();
        field.type_text("2560x1440");
        ui.run();
        let saved = ui.state().form.settings();
        assert_eq!(saved.resolution, Some(Resolution::Size(2560, 1440)));
        assert_eq!(
            saved.resolution,
            flags(&["--resolution", "2560x1440"]).resolution
        );
        let plan = game_gets(&ui);
        assert!(registry(&plan).contains(&"WIDTH = dword:2560".to_owned()));
        assert!(registry(&plan).contains(&"HEIGHT = dword:1440".to_owned()));
        assert_eq!(
            plan.args,
            [crate::plan::WIDESCREEN_ARG, crate::plan::FORCE_HT_ARG]
        );
    }

    #[test]
    fn the_frame_rate_cap_reaches_dxvk() {
        let mut ui = open("fps_limit = 59\n");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        let slider = ui.get_by_role_and_label(Role::Slider, "Frame-rate cap");
        slider.focus();
        ui.run();
        ui.key_press(egui::Key::ArrowRight);
        ui.run();
        let saved = ui.state().form.settings();
        assert_eq!(saved.fps_limit, Some(60));
        assert_eq!(saved.fps_limit, flags(&["--fps-limit", "60"]).fps_limit);
        let plan = game_gets(&ui);
        let conf = plan.files[0].1.as_deref().unwrap();
        assert!(conf.contains("d3d9.maxFrameRate = 60"), "{conf}");
    }

    #[test]
    fn the_vsync_switch_reaches_dxvk() {
        let mut ui = open("vsync = on\n");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        ui.get_by_role_and_label(Role::Switch, "VSync").click();
        ui.run();
        let conf = game_gets(&ui).files[0].1.clone().unwrap();
        assert!(conf.contains("d3d9.presentInterval = 0"), "{conf}");
    }

    #[test]
    fn each_look_card_reaches_the_game() {
        for (label, size, arg) in [
            ("Original", None, false),
            ("Widescreen", Some("WIDTH = dword:1920"), true),
            ("Steam Deck", Some("WIDTH = dword:1280"), true),
        ] {
            let mut ui = open("");
            ui.get_by_label(label).click();
            ui.run();
            let plan = game_gets(&ui);
            match size {
                Some(width) => assert!(registry(&plan).contains(&width.to_owned()), "{label}"),
                None => assert_eq!(registry(&plan).len(), 2, "{label}: only the paths"),
            }
            assert_eq!(
                plan.args.contains(&crate::plan::WIDESCREEN_ARG),
                arg,
                "{label}"
            );
        }
    }

    #[test]
    fn the_advanced_particles_switch_reaches_the_game() {
        let mut ui = open("");
        assert!(
            game_gets(&ui).args.contains(&crate::plan::FORCE_HT_ARG),
            "on by default"
        );
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        ui.get_by_role_and_label(Role::Switch, "Advanced particles")
            .click();
        ui.run();
        let saved = ui.state().form.settings();
        assert_eq!(saved.advanced_particles, Some(false));
        assert_eq!(
            saved.advanced_particles,
            flags(&["--advanced-particles", "off"]).advanced_particles
        );
        assert!(!game_gets(&ui).args.contains(&crate::plan::FORCE_HT_ARG));
    }

    #[test]
    fn the_vsync_switch_saves_what_its_flag_sets() {
        let mut ui = open("vsync = on\n");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        ui.get_by_role_and_label(Role::Switch, "VSync").click();
        ui.run();
        let saved = ui.state().form.settings();
        assert_eq!(saved.vsync, Some(false));
        assert_eq!(saved.vsync, flags(&["--vsync", "off"]).vsync);
    }

    #[test]
    fn a_size_the_option_refuses_says_what_it_needs() {
        let mut ui = open("");
        ui.state_mut().form = Form::from_text("resolution = huge\n");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        assert!(ui.query_by_label_contains("Resolution: needs").is_some());
    }

    #[test]
    fn play_waits_for_a_known_game_and_an_unknown_one_is_refused() {
        let mut ui = open("");
        assert!(testing::disabled(
            &ui.get_by_role_and_label(Role::Button, "Play")
        ));

        let dir = std::env::temp_dir().join(format!("cmr2005-ui-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(edition::GAME_EXE), b"not the game").unwrap();
        *ui.state_mut().form.value_mut("game") = dir.display().to_string();
        ui.run();
        let digest = rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"not the game"));
        assert!(
            ui.query_by_label_contains(&digest).is_some(),
            "the SHA-256 is shown"
        );
        assert!(
            ui.query_by_label_contains(edition::UNKNOWN_EDITION_ISSUE)
                .is_some()
        );
        let play = ui.get_by_role_and_label(Role::Button, "Play");
        assert!(testing::disabled(&play));
        play.click();
        ui.run();
        assert!(!ui.state().play);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The pad's LB/RB (and the window's tab cycling) reach the Wheel tab.
    #[test]
    fn tabs_step_and_wrap() {
        assert_eq!(Tab::Play.step(1), Tab::Picture);
        assert_eq!(Tab::Picture.step(1), Tab::Wheel);
        assert_eq!(Tab::Wheel.step(1), Tab::Play);
        assert_eq!(Tab::Play.step(-1), Tab::Wheel);
        let names: Vec<&str> = Tab::NAMES.iter().map(|(_, name)| *name).collect();
        assert_eq!(names, ["Play", "Picture", "Wheel"]);
    }
}
