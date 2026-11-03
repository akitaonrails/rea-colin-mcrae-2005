//! The settings window, on `rea_launcher_ui`.
//!
//! [`draw`] is the whole window as a function of [`Window`], so the
//! headless tests below drive the real widgets (egui_kittest) on the host
//! and under `sdk/scripts/wine-test`; `show` wraps it in eframe on Windows.
//!
//! Layout: the hero image strip on the left, the title and status, the
//! Play tab (game folder and look cards) and the Picture tab (every other
//! option row of `config::OPTIONS`), the footer with the Play plate.
//! Any change to the window's look or layout goes through a design skill
//! (docs/STANDARDS.md "Settings window and CLI").

use std::path::PathBuf;

use rea_launcher::cfg::{OptionSpec, spec};
use rea_launcher_ui::download;
use rea_launcher_ui::egui::{self, Align2, Rect, RichText, TextureHandle, pos2, vec2};
use rea_launcher_ui::spec::{mark_rendered, spec_path_row, spec_row};
use rea_launcher_ui::theme::{body, condensed};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::config::{Look, OPTIONS};
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
}

impl Tab {
    pub const NAMES: [(Tab, &'static str); 2] = [(Tab::Play, "Play"), (Tab::Picture, "Picture")];

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
    /// A known edition, by name.
    Found(&'static str),
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
        Ok(Source::Installed { found, .. }) => Check::Found(found.edition.name),
        Ok(Source::Installer(installer)) => Check::Found(installer.build.name),
        Err(problem) => Check::Problem(problem),
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
}

impl Window {
    /// The window over a `launcher.cfg` text.
    pub fn new(text: &str, screen: Option<(u32, u32)>) -> Window {
        let form = Form::from_text(text);
        let checked = form.get("game").to_owned();
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
                RichText::new("Colin McRae Rally 2005")
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
                "Choose the folder with {} or GOG's {}. Under Wine, Z:\\ is your Linux root folder.",
                edition::GAME_EXE,
                source::SETUP_EXE
            ),
            MUTED,
        ),
        Check::Empty => (
            format!(
                "Choose the folder with {} or GOG's {}.",
                edition::GAME_EXE,
                source::SETUP_EXE
            ),
            MUTED,
        ),
        Check::Found(name) => (format!("Found {name}."), SEPIA),
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

/// Every option row that is not on the Play tab and has a window control.
fn picture_tab(ui: &mut egui::Ui, w: &mut Window) {
    let mut changed = false;
    for row in OPTIONS
        .iter()
        .filter(|row| !row.launcher_only && !matches!(row.key, "game" | "look"))
    {
        changed |= spec_row(ui, row, w.form.value_mut(row.key)).changed();
        ui.add_space(10.0);
    }
    if changed {
        w.form.picture_edited(w.screen);
    }
}

/// The unpack window's state: GOG's installer being unpacked on the first
/// start (`source::begin`/`finish`), so a player never faces a silent
/// screen for minutes.
pub struct Unpacking {
    /// Bytes written so far, and the unpacked game's size.
    pub done: u64,
    pub total: u64,
    /// Seconds since the unpack began, for the time left.
    pub elapsed: f64,
    pub hero: Option<TextureHandle>,
    /// Cancel was pressed (the caller stops innoextract).
    pub cancel: bool,
}

impl Unpacking {
    pub fn new(total: u64) -> Unpacking {
        Unpacking {
            done: 0,
            total,
            elapsed: 0.0,
            hero: None,
            cancel: false,
        }
    }

    /// The bar's fraction, never past full.
    pub fn fraction(&self) -> f32 {
        (self.done as f64 / self.total.max(1) as f64).min(1.0) as f32
    }

    /// "1.2 GB of 3.6 GB, about 2 minutes left" (the time once there is a rate).
    pub fn status(&self) -> String {
        let sizes = format!(
            "{} of {}",
            download::size_text(self.done.min(self.total)),
            download::size_text(self.total)
        );
        if self.elapsed < 3.0 || self.done == 0 {
            return sizes;
        }
        let rate = self.done as f64 / self.elapsed;
        let left = self.total.saturating_sub(self.done) as f64 / rate;
        format!("{sizes}, {}", download::time_left(left))
    }
}

/// The unpack window, filling `ui`: the photo, what is happening and why,
/// the progress, and Cancel.
pub fn draw_unpack(ui: &mut egui::Ui, u: &mut Unpacking) {
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, BASE);
    let layout = Layout::new(full, PHOTO_W);
    draw_hero(ui, layout.photo, u.hero.as_ref());
    ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
        ui.add(
            egui::Label::new(
                RichText::new("Colin McRae Rally 2005")
                    .font(condensed(38.0))
                    .color(INK),
            )
            .wrap(),
        );
        ui.label(
            RichText::new("Unpacking the game. This happens only on the first start.")
                .font(body(14.0))
                .color(MUTED),
        );
        ui.add_space(24.0);
        widgets::heading(ui, "Unpacking from GOG's installer");
        ui.add_space(6.0);
        widgets::progress_bar(ui, u.fraction());
        ui.add_space(6.0);
        widgets::note(ui, &u.status());
        ui.add_space(14.0);
        widgets::note(
            ui,
            "The launcher copies the game out of your installer files into its own \
             folder, about 3.6 GB. Your installer files are only read. The game starts \
             as soon as this is done.",
        );
    });
    let button = Rect::from_min_size(
        pos2(
            layout.footer.right() - 150.0,
            layout.footer.center().y - 20.0,
        ),
        vec2(130.0, 40.0),
    );
    let cancel = ui
        .scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
            widgets::outline_button(ui, "Cancel")
        })
        .inner;
    if cancel.clicked() {
        u.cancel = true;
    }
}

#[cfg(windows)]
pub use app::{Outcome, UnpackJob, UnpackOutcome, show, show_unpack};

#[cfg(windows)]
mod app {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use rea_launcher::fsx;
    use rea_launcher_ui::eframe;
    use rea_launcher_ui::egui;
    use rea_launcher_ui::pad::{self, Nav, PadNav};
    use rea_launcher_ui::theme;
    use rea_launcher_ui::widgets;
    use rea_launcher_ui::window::{SIZE, WindowFit};
    use rea_launcher_win32::{dialog, display, wine, xinput};

    use super::{HERO_JPEG, Window, draw};
    use crate::config;
    use crate::palette::{METRICS, PALETTE};

    /// How the window ended.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Outcome {
        /// Play was pressed; the settings are saved.
        Play,
        /// Closed without Play.
        Closed,
    }

    /// The unpack the window watches (implemented in `main.rs` over
    /// innoextract's process).
    pub trait UnpackJob {
        /// Bytes written so far.
        fn done(&mut self) -> u64;
        /// The result once innoextract has ended.
        fn finished(&mut self) -> Option<Result<(), String>>;
        /// Stops it (Cancel, or the window closed).
        fn cancel(&mut self);
    }

    /// How the unpack window ended.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum UnpackOutcome {
        Finished(Result<(), String>),
        Cancelled,
    }

    struct UnpackApp<'a> {
        state: super::Unpacking,
        fit: WindowFit,
        job: &'a mut dyn UnpackJob,
        started: std::time::Instant,
        outcome: &'a mut Option<UnpackOutcome>,
    }

    /// Shows the unpack window until `job` ends or the player cancels.
    pub fn show_unpack(total: u64, job: &mut dyn UnpackJob) -> Result<UnpackOutcome, String> {
        let scale = display::wine_dpi_scale();
        let screen = display::screen_size();
        let fit = WindowFit::new(SIZE, wine::is_wine(), screen, scale).with_native_fit(true);
        let options = fit.native_options("Colin McRae Rally 2005");
        let mut outcome = None;
        eframe::run_native(
            "Colin McRae Rally 2005",
            options,
            Box::new(|cc| {
                theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
                let mut state = super::Unpacking::new(total);
                state.hero =
                    HERO_JPEG.and_then(|jpeg| widgets::load_jpeg(&cc.egui_ctx, "hero", jpeg));
                Ok(Box::new(UnpackApp {
                    state,
                    fit,
                    job,
                    started: std::time::Instant::now(),
                    outcome: &mut outcome,
                }))
            }),
        )
        .map_err(|error| error.to_string())?;
        // Closed some other way: stop the unpack too.
        Ok(outcome.unwrap_or(UnpackOutcome::Cancelled))
    }

    impl eframe::App for UnpackApp<'_> {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            self.fit.first_frame(&ctx);
            if self.outcome.is_none() {
                if let Some(result) = self.job.finished() {
                    *self.outcome = Some(UnpackOutcome::Finished(result));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.state.done = self.job.done();
                    self.state.elapsed = self.started.elapsed().as_secs_f64();
                }
            }
            let closing = ctx.input(|input| input.viewport().close_requested());
            super::draw_unpack(ui, &mut self.state);
            if self.outcome.is_none() && (self.state.cancel || closing) {
                self.job.cancel();
                *self.outcome = Some(UnpackOutcome::Cancelled);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    struct App<'a> {
        window: Window,
        fit: WindowFit,
        path: PathBuf,
        /// The `launcher.cfg` text the window's values are saved into.
        text: String,
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
        let options = fit.native_options("Colin McRae Rally 2005");
        let text = saved.map_or_else(|| config::first_run_text(screen), str::to_owned);
        let mut outcome = Outcome::Closed;
        eframe::run_native(
            "Colin McRae Rally 2005",
            options,
            Box::new(|cc| {
                theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
                let mut window = Window::new(&text, screen);
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
            match fsx::write_atomic(&self.path, text) {
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
            Window::new(text, Some((1920, 1080))),
            draw,
        )
    }

    fn flags(args: &[&str]) -> Settings {
        let args = args.iter().map(|arg| (*arg).to_owned());
        config::parse_args(args).unwrap().0
    }

    #[test]
    fn the_unpack_window_shows_progress_and_time_left() {
        let mut u = Unpacking::new(3_600_000_000);
        assert_eq!(u.status(), "0 MB of 3.6 GB");
        u.done = 1_200_000_000;
        u.elapsed = 60.0;
        assert_eq!(u.status(), "1.2 GB of 3.6 GB, about 2 min left");
        assert!((u.fraction() - 1.0 / 3.0).abs() < 1e-6);
        u.done = 4_000_000_000;
        assert_eq!(u.fraction(), 1.0, "never past full");
        assert!(u.status().starts_with("3.6 GB of 3.6 GB"));
    }

    #[test]
    fn the_unpack_window_can_be_cancelled() {
        let mut ui = testing::harness(&PALETTE, &METRICS, Unpacking::new(100), draw_unpack);
        ui.run();
        assert!(ui.query_by_label_contains("first start").is_some());
        assert!(ui.query_by_label_contains("of 100 B").is_none());
        ui.get_by_role_and_label(Role::Button, "Cancel").click();
        ui.run();
        assert!(ui.state().cancel);
    }

    #[test]
    fn the_window_has_a_control_for_every_option_and_no_other() {
        let mut ui = open("");
        ui.get_by_role_and_label(Role::Tab, "Picture").click();
        ui.run();
        spec::assert_parity(OPTIONS, &rendered_keys(&ui.ctx));
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
    fn game_gets(ui: &Harness<'static, Window>) -> crate::plan::Plan {
        let settings = ui.state().form.settings();
        let picture = settings.picture(ui.state().screen);
        crate::plan::plan(
            &picture,
            std::path::Path::new(r"C:\r"),
            std::path::Path::new(r"C:\s"),
            true,
        )
    }

    fn registry(plan: &crate::plan::Plan) -> Vec<String> {
        plan.registry.iter().map(ToString::to_string).collect()
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
        assert_eq!(plan.args, [crate::plan::WIDESCREEN_ARG]);
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
            assert_eq!(!plan.args.is_empty(), arg, "{label}");
        }
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

    #[test]
    fn tabs_step_and_wrap() {
        assert_eq!(Tab::Play.step(1), Tab::Picture);
        assert_eq!(Tab::Play.step(-1), Tab::Picture);
        assert_eq!(Tab::Picture.step(1), Tab::Play);
    }
}
