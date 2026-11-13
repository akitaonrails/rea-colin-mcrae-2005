//! Golden snapshots (docs/STANDARDS.md, "Golden snapshots"): `--help`, the
//! `launcher.cfg` template, and what a launch does for each look and a few
//! option mixes (`plan::plan`). A difference fails; review it like code.
//! `CMR2005_UPDATE_SNAPSHOTS=1 cargo test --test snapshots` (or
//! `REA_UPDATE_SNAPSHOTS=1`) rewrites them.

use std::path::Path;

use cmr2005_launcher::config::{self, Look, Settings};
use cmr2005_launcher::{plan, wheel};

/// `rea_launcher::snapshot!` with this launcher's own update variable kept
/// (`REA_UPDATE_SNAPSHOTS=1` works too). The crate's version is stored as
/// `<version>`, so a release bump never breaks a snapshot.
fn check(name: &str, actual: &str) {
    rea_launcher::snapshots!()
        .update_var("CMR2005_UPDATE_SNAPSHOTS")
        .assert(name, actual);
}

const RUNTIME: &str = r"C:\users\player\AppData\Local\CMR2005Launcher\runtime";
const SAVES: &str = r"C:\users\player\AppData\Local\CMR2005Launcher\saves";

fn launch(name: &str, flags: &[&str], screen: (u32, u32), wine: bool) {
    let args = flags.iter().map(|f| (*f).to_owned());
    let (settings, _) = config::parse_args(args).unwrap();
    let settings = Settings::default().merged(settings);
    let picture = settings.picture(Some(screen));
    let plan = plan::plan(
        &picture,
        settings.advanced_particles(),
        Path::new(RUNTIME),
        Path::new(SAVES),
        wine,
    );
    // No controller is attached here: Auto shows no wheel.
    let plan = plan.with_wheel(
        wheel::shown(&settings.wheel, Vec::new),
        wheel::runtime_ini(&settings.wheel, ""),
    );
    let text = format!(
        "flags: {}\nscreen: {}x{}, Wine: {wine}\npicture: {}\n\n{}",
        flags.join(" "),
        screen.0,
        screen.1,
        picture.describe(),
        plan.render()
    );
    check(name, &text);
}

#[test]
fn help_and_template() {
    check("help.txt", &config::usage());
    check("template.cfg", &config::template());
    check(
        "first-run-1080p.cfg",
        &config::first_run_text(Some((1920, 1080))),
    );
}

#[test]
fn each_look() {
    for look in Look::ALL {
        let name = format!("look-{}-wine.txt", look.key());
        launch(&name, &["--look", look.key()], (1920, 1080), true);
    }
    launch(
        "look-widescreen-windows.txt",
        &["--look", "widescreen"],
        (2560, 1440),
        false,
    );
    launch(
        "look-widescreen-ultrawide.txt",
        &["--look", "widescreen"],
        (3440, 1440),
        true,
    );
}

#[test]
fn option_mixes() {
    launch(
        "custom-16x10-144fps-no-vsync.txt",
        &[
            "--resolution",
            "1920x1200",
            "--fps-limit",
            "144",
            "--vsync",
            "off",
        ],
        (1920, 1200),
        true,
    );
    launch("unset-everything.txt", &[], (1920, 1080), true);
    launch(
        "wheel-on-900-ffb-60.txt",
        &["--wheel", "on", "--wheel-lock", "900", "--wheel-ffb", "60"],
        (1920, 1080),
        true,
    );
    launch(
        "no-advanced-particles.txt",
        &["--advanced-particles", "off"],
        (1920, 1080),
        false,
    );
}
