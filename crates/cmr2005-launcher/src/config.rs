//! `launcher.cfg`, the command line and `--help`, from one option table
//! (`rea_launcher::cfg::spec`, docs/STANDARDS.md "Settings window and CLI").
//!
//! [`OPTIONS`] names every setting once: its key, its flag (`--vsync` for
//! `vsync`), the values it takes, and the label and description the window
//! and `--help` show. The parser, `--help`, the first-run template and the
//! window rows all come from it, and the window's parity test fails when a
//! row has no control or a control has no row.
//!
//! Every option here was seen working in the real game
//! (docs/RUNTIME_NOTES.md); `plan` turns them into registry values, the
//! game's arguments and DXVK's `dxvk.conf`, and `e2e` tests each one from
//! window control to what the game gets.

use std::path::PathBuf;

use rea_launcher::cfg::{
    self, ArgError, KeyStyle, Kind, OptionSpec, Rejected, Resolution, Value, spec,
};
use rea_launcher::display;

/// The settings file, next to the exe (portable) or in the launcher's
/// AppData folder.
pub const FILE_NAME: &str = "launcher.cfg";

/// Flags spell keys with hyphens: `--display-mode` is `display_mode`.
pub const STYLE: KeyStyle = KeyStyle::Underscore;

/// The launcher's own switches: they take no value and never reach
/// `launcher.cfg`.
pub const SWITCHES: &[&str] = &["--settings", "--no-window", "--check", "--linux"];

/// Looks, as `(value, label)` for the option table.
pub const LOOKS: &[(&str, &str)] = &[
    ("original", "Original"),
    ("widescreen", "Widescreen"),
    ("steam_deck", "Steam Deck"),
];

/// The Steam Deck look's size: the 16:9 size of the Deck's width, so the
/// game's widescreen view is exact (it is only correct at 16:9; on the
/// Deck's 16:10 screen this leaves thin bars above and below).
pub const STEAM_DECK_SIZE: (u32, u32) = (1280, 720);

/// The Widescreen look's size when the screen is unknown.
pub const FALLBACK_SIZE: (u32, u32) = (1920, 1080);

/// The smallest and largest sizes the game is offered.
pub const MIN_SIZE: (u32, u32) = (640, 480);
pub const MAX_SIZE: (u32, u32) = (7680, 4320);

/// Every option: `launcher.cfg` key, flag and window control.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "game",
        Kind::Path,
        "Game folder",
        "Your installed game (the folder with cmr5.exe), or the folder with GOG's \
         installer files (setup_colin2005.exe and its two .bin files). An installer is never \
         run: the first Play unpacks the game from it once into the launcher's own folder \
         (about 3.9 GB free space needed) and this setting then points there. The folder you \
         choose is only read, never changed.",
    ),
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "Sets the picture in one go: Original as the game shipped, Widescreen fitted to \
         your screen, or Steam Deck (1280x720 at 60 fps). Options set after it win.",
    ),
    OptionSpec::new(
        "resolution",
        Kind::Size {
            min: MIN_SIZE,
            max: MAX_SIZE,
            aliases: cfg::ORIGINAL,
        },
        "Resolution",
        "The picture size. A 16:9 size shows the game's widescreen view; other shapes \
         keep the 4:3 view. Original keeps the size set in the game.",
    )
    .with_example("1920x1080"),
    OptionSpec::new(
        "fps_limit",
        Kind::Range { min: 0, max: 1000 },
        "Frame-rate cap",
        "The most frames per second the game draws; 0 or off for no cap. 60 keeps the \
         game's frame-rate-timed animations steady. Under Wine and Proton only.",
    )
    .with_value_aliases(&[("off", "0")])
    .with_example("60"),
    OptionSpec::new(
        "vsync",
        Kind::Toggle,
        "VSync",
        "Waits for the screen before showing each frame: no tearing, slightly more input \
         delay. Under Wine and Proton only.",
    ),
    OptionSpec::new(
        "runtime",
        Kind::Path,
        "Private copy folder",
        "Where the launcher keeps its own copy of the game and your saves. Empty: a folder \
         beside launcher.log. It must be outside the game folder.",
    )
    .launcher_only(),
];

/// Comment lines at the top of a new `launcher.cfg`.
const HEADING: &str = "Settings for CMR2005Launcher.exe (Colin McRae Rally 2005).\n\
    Every key is also a command-line flag: resolution = 1920x1080 is --resolution 1920x1080.\n\
    A key left commented out keeps the game's own value.";

/// A look: a named set of option values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    /// The game as it shipped.
    Original,
    /// The largest 16:9 size that fits the player's screen.
    Widescreen,
    SteamDeck,
}

impl Look {
    pub const ALL: [Look; 3] = [Look::Original, Look::Widescreen, Look::SteamDeck];

    /// The `launcher.cfg` value.
    pub fn key(self) -> &'static str {
        match self {
            Look::Original => "original",
            Look::Widescreen => "widescreen",
            Look::SteamDeck => "steam_deck",
        }
    }

    pub fn from_key(key: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|look| look.key() == key)
    }

    /// The window's card title (the option table's label for the value).
    pub fn label(self) -> &'static str {
        LOOKS
            .iter()
            .find(|(key, _)| *key == self.key())
            .map_or("", |(_, label)| label)
    }

    /// The look recommended on `screen`: Steam Deck on a Deck's screen,
    /// otherwise Widescreen.
    pub fn recommended(screen: Option<(u32, u32)>) -> Look {
        if display::is_steam_deck(screen) {
            Look::SteamDeck
        } else {
            Look::Widescreen
        }
    }

    /// The look's values on `screen` (each seen in the game,
    /// docs/RUNTIME_NOTES.md). Original keeps the game's own size and
    /// leaves the frame rate alone; Widescreen and Steam Deck cap at 60,
    /// which keeps the game's frame-rate-timed animations steady.
    pub fn picture(self, screen: Option<(u32, u32)>) -> Picture {
        match self {
            Look::Original => Picture {
                resolution: Some(Resolution::Original),
                fps_limit: Some(0),
                vsync: Some(false),
            },
            Look::Widescreen => {
                let (w, h) = display::largest_16_9(screen, MAX_SIZE, FALLBACK_SIZE);
                Picture {
                    resolution: Some(Resolution::Size(w, h)),
                    fps_limit: Some(60),
                    vsync: Some(true),
                }
            }
            Look::SteamDeck => Picture {
                resolution: Some(Resolution::Size(STEAM_DECK_SIZE.0, STEAM_DECK_SIZE.1)),
                fps_limit: Some(60),
                vsync: Some(true),
            },
        }
    }
}

/// The picture options for one run. `None` keeps the game's own value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<Resolution>,
    /// 0 is no cap.
    pub fps_limit: Option<u32>,
    pub vsync: Option<bool>,
}

impl Picture {
    /// The values as `(key, launcher.cfg text)`, for the window and the log.
    pub fn values(&self) -> [(&'static str, Option<String>); 3] {
        [
            ("resolution", self.resolution.map(|r| r.to_string())),
            ("fps_limit", self.fps_limit.map(|fps| fps.to_string())),
            ("vsync", self.vsync.map(|on| cfg::on_off(on).to_owned())),
        ]
    }

    /// The size the game renders, when the launcher sets one.
    pub fn size(&self) -> Option<(u32, u32)> {
        match self.resolution {
            Some(Resolution::Size(w, h)) => Some((w, h)),
            _ => None,
        }
    }

    /// Whether the game gets its widescreen view (`WIDESCREENDISPLAY`):
    /// exactly at a 16:9 size, the only shape it draws correctly.
    pub fn widescreen(&self) -> bool {
        self.size().is_some_and(display::is_16_9)
    }

    /// One line for `launcher.log`.
    pub fn describe(&self) -> String {
        self.values()
            .iter()
            .map(|(key, value)| format!("{key} {}", value.as_deref().unwrap_or("(game's own)")))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Common 16:9 sizes, largest first, for [`fit_mode`].
const SIZES_16_9: [(u32, u32); 8] = [
    (3840, 2160),
    (3200, 1800),
    (2560, 1440),
    (1920, 1080),
    (1600, 900),
    (1366, 768),
    (1280, 720),
    (1024, 576),
];

/// The mode the game can use for `wanted`: the game quits at once when the
/// display lists no such mode (docs/RUNTIME_NOTES.md, run e05). `wanted`
/// when `listed`; else, for a 16:9 size, the largest listed 16:9 size no
/// bigger; else `None`, keeping the game's own mode.
pub fn fit_mode(wanted: (u32, u32), listed: impl Fn(u32, u32) -> bool) -> Option<(u32, u32)> {
    if listed(wanted.0, wanted.1) {
        return Some(wanted);
    }
    if !display::is_16_9(wanted) {
        return None;
    }
    SIZES_16_9
        .into_iter()
        .filter(|&(w, h)| w <= wanted.0 && h <= wanted.1)
        .find(|&(w, h)| listed(w, h))
}

/// The settings from `launcher.cfg` and the command line. `None` is unset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub look: Option<Look>,
    pub resolution: Option<Resolution>,
    pub fps_limit: Option<u32>,
    pub vsync: Option<bool>,
    pub runtime: Option<PathBuf>,
}

impl Settings {
    /// One checked value from the file or a flag.
    pub fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        let path = |text: &str| (!text.is_empty()).then(|| PathBuf::from(text));
        match (key, value) {
            ("game", Value::Text(text)) => self.game = path(text),
            ("runtime", Value::Text(text)) => self.runtime = path(text),
            ("look", Value::Choice(choice)) => {
                let look = Look::from_key(choice).ok_or(Rejected::Unknown)?;
                // A look sets all of its values; options after it win.
                self.look = Some(look);
                self.resolution = None;
                self.fps_limit = None;
                self.vsync = None;
            }
            ("resolution", Value::Size(resolution)) => self.resolution = Some(resolution),
            ("fps_limit", Value::Number(fps)) => self.fps_limit = Some(fps),
            ("vsync", Value::Toggle(on)) => self.vsync = Some(on),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// These settings with the command line's on top: a flag wins over the
    /// file, and `--look` replaces the file's picture options.
    pub fn merged(mut self, flags: Settings) -> Settings {
        if flags.look.is_some() {
            self.look = flags.look;
            self.resolution = None;
            self.fps_limit = None;
            self.vsync = None;
        }
        self.game = flags.game.or(self.game);
        self.runtime = flags.runtime.or(self.runtime);
        self.resolution = flags.resolution.or(self.resolution);
        self.fps_limit = flags.fps_limit.or(self.fps_limit);
        self.vsync = flags.vsync.or(self.vsync);
        self
    }

    /// The picture for this run: the look's values, then the options set
    /// on their own.
    pub fn picture(&self, screen: Option<(u32, u32)>) -> Picture {
        let base = self
            .look
            .map(|look| look.picture(screen))
            .unwrap_or_default();
        Picture {
            resolution: self.resolution.or(base.resolution),
            fps_limit: self.fps_limit.or(base.fps_limit),
            vsync: self.vsync.or(base.vsync),
        }
    }
}

/// Reads `launcher.cfg` text; the warnings name each line it could not use.
pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
    let mut settings = Settings::default();
    let warnings = spec::parse_file(text, FILE_NAME, OPTIONS, |key, value| {
        settings.apply(key, value)
    });
    (settings, warnings)
}

/// Reads the command line: the settings its flags set, and the switches
/// (`--no-window`, …) in the order given.
pub fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> Result<(Settings, Vec<String>), ArgError> {
    let mut settings = Settings::default();
    let switches = spec::parse_args(args, STYLE, OPTIONS, SWITCHES, |key, value| {
        settings.apply(key, value)
    })?;
    Ok((settings, switches))
}

/// A new `launcher.cfg`: every option commented out, with its description.
pub fn template() -> String {
    spec::template(HEADING, OPTIONS)
}

/// The first run's `launcher.cfg`: the template with the recommended look's
/// values saved explicitly, which is how the default becomes the
/// recommended experience (STANDARDS "The game's own settings stay the
/// game's").
pub fn first_run_text(screen: Option<(u32, u32)>) -> String {
    let look = Look::recommended(screen);
    let mut text = set(&template(), "look", look.key());
    for (key, value) in look.picture(screen).values() {
        if let Some(value) = value {
            text = set(&text, key, &value);
        }
    }
    text
}

/// [`spec::set_key`] for the row of `key`.
pub fn set(text: &str, key: &str, value: &str) -> String {
    match spec::find(OPTIONS, key) {
        Some(row) => spec::set_key(text, row, value),
        None => cfg::set_key(text, key, value),
    }
}

/// `--help`.
pub fn usage() -> String {
    format!(
        "CMR2005Launcher.exe {version}: plays your own copy of Colin McRae Rally 2005 on modern Windows and Linux.\n\
         \n\
         Usage: CMR2005Launcher.exe [options]\n\
         \n\
         Without options the settings window opens; Play saves them and starts the game.\n\
         \n\
         Options (each is also a {FILE_NAME} key):\n\
         {options}\n\
         Launcher switches:\n  \
           --settings   only the settings window (exit 0 after Play, 3 when closed)\n  \
           --no-window  play with the saved settings and these options, no window\n  \
           --check      check the game folder and its edition, then exit (see launcher.log)\n  \
           --linux      started by the Linux helper (linux/cmr2005-play)\n  \
           --help       this text\n",
        version = env!("CARGO_PKG_VERSION"),
        options = spec::usage(OPTIONS, STYLE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_parses_to_nothing_and_lists_every_option() {
        let text = template();
        assert_eq!(parse_file(&text), (Settings::default(), Vec::new()));
        for row in OPTIONS {
            assert!(text.contains(&format!("#{} =", row.key)), "{}", row.key);
        }
    }

    #[test]
    fn the_first_run_saves_the_recommended_look_explicitly() {
        let (settings, warnings) = parse_file(&first_run_text(Some((2560, 1440))));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.look, Some(Look::Widescreen));
        assert_eq!(settings.resolution, Some(Resolution::Size(2560, 1440)));
        assert_eq!(settings.fps_limit, Some(60));
        assert_eq!(settings.vsync, Some(true));
        let deck = parse_file(&first_run_text(Some((800, 1280)))).0;
        assert_eq!(deck.look, Some(Look::SteamDeck));
        assert_eq!(deck.resolution, Some(Resolution::Size(1280, 720)));
        assert_eq!(deck.fps_limit, Some(60));
    }

    #[test]
    fn widescreen_fits_the_largest_16_by_9_size_into_the_screen() {
        let size = |screen| Look::Widescreen.picture(screen).resolution;
        assert_eq!(size(Some((3840, 2160))), Some(Resolution::Size(3840, 2160)));
        assert_eq!(size(Some((2560, 1080))), Some(Resolution::Size(1920, 1080)));
        assert_eq!(size(Some((1920, 1200))), Some(Resolution::Size(1920, 1080)));
        assert_eq!(size(Some((1080, 1920))), Some(Resolution::Size(1920, 1080)));
        assert_eq!(size(None), Some(Resolution::Size(1920, 1080)));
    }

    #[test]
    fn the_widescreen_look_caps_at_60_frames_per_second() {
        assert_eq!(Look::Widescreen.picture(None).fps_limit, Some(60));
        assert_eq!(Look::Original.picture(None).fps_limit, Some(0));
    }

    #[test]
    fn a_mode_the_display_lacks_falls_back_to_a_listed_one() {
        let listed = |w, h| matches!((w, h), (1920, 1080) | (1280, 720) | (1024, 768));
        assert_eq!(fit_mode((1920, 1080), listed), Some((1920, 1080)));
        assert_eq!(fit_mode((2560, 1440), listed), Some((1920, 1080)));
        assert_eq!(fit_mode((1600, 900), listed), Some((1280, 720)));
        assert_eq!(
            fit_mode((1024, 576), listed),
            None,
            "nothing smaller is listed"
        );
        assert_eq!(
            fit_mode((1280, 800), listed),
            None,
            "not 16:9: the game's own"
        );
        assert_eq!(fit_mode((1024, 768), listed), Some((1024, 768)));
    }

    #[test]
    fn the_widescreen_view_is_only_for_16_by_9_sizes() {
        let picture = |resolution| Picture {
            resolution: Some(resolution),
            ..Picture::default()
        };
        assert!(picture(Resolution::Size(1920, 1080)).widescreen());
        assert!(picture(Resolution::Size(1280, 720)).widescreen());
        assert!(!picture(Resolution::Size(1280, 800)).widescreen());
        assert!(!picture(Resolution::Size(2560, 1080)).widescreen());
        assert!(!picture(Resolution::Size(1024, 768)).widescreen());
        assert!(!picture(Resolution::Original).widescreen());
        assert!(!Picture::default().widescreen());
    }

    #[test]
    fn a_look_sets_its_values_and_later_options_win() {
        let (settings, _) =
            parse_file("resolution = 800x600\nlook = original\nvsync = on\nfps_limit = off\n");
        assert_eq!(
            settings.picture(None),
            Picture {
                resolution: Some(Resolution::Original),
                fps_limit: Some(0),
                vsync: Some(true),
            }
        );
        let unset = Settings::default().picture(None);
        assert_eq!(unset, Picture::default(), "unset keeps the game's values");
    }

    #[test]
    fn flags_win_over_the_file_and_a_look_flag_replaces_its_values() {
        let (file, _) = parse_file("game = C:\\Games\\Old\nresolution = 800x600\nvsync = on\n");
        let (flags, _) = parse_args(["--look".into(), "steam_deck".into()]).unwrap();
        let merged = file.clone().merged(flags);
        assert_eq!(merged.game, file.game);
        assert_eq!(
            merged.picture(None).resolution,
            Some(Resolution::Size(1280, 720))
        );
        let (flags, _) = parse_args(["--fps-limit".into(), "30".into()]).unwrap();
        let merged = file.merged(flags);
        assert_eq!(merged.fps_limit, Some(30));
        assert_eq!(merged.vsync, Some(true));
        assert_eq!(merged.resolution, Some(Resolution::Size(800, 600)));
    }

    #[test]
    fn bad_lines_are_reported_and_kept() {
        let (settings, warnings) = parse_file("vsync = maybe\nfps = 60\nresolution = 1280x720\n");
        assert_eq!(settings.resolution, Some(Resolution::Size(1280, 720)));
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("on or off"), "{warnings:?}");
        assert!(warnings[1].contains("unknown setting"), "{warnings:?}");
    }
}
