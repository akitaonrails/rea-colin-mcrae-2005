//! The owner's real GOG installer, unpacked through the launcher's own
//! code (`source::detect`, then `source::unpack`: the SDK's innoextract
//! runner with the launcher's game check, no window) under
//! `sdk/scripts/wine-test`. Real installers stay outside the repository,
//! so this runs only when all three are set (Windows paths, `Z:\…`):
//!
//! - `CMR2005_REAL_INSTALLER`: the folder with `setup_colin2005.exe` and
//!   its two `.bin` files (only read);
//! - `CMR2005_INNOEXTRACT`: the bundled `innoextract.exe`;
//! - `CMR2005_UNPACK_DEST`: a folder under the repository's `.work/` that
//!   does not exist yet.
//!
//! It also checks that a folder launcher 0.2 unpacked (its
//! `.cmr2005-unpacked` marker) is reused without unpacking again.

#![cfg(windows)]

use std::path::PathBuf;
use std::time::Instant;

use cmr2005_launcher::source::{self, Source};
use rea_launcher::installer::MARKER;
use rea_launcher_win32::innoextract::Unpacked;

fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

#[test]
fn the_owners_installer_unpacks_into_a_known_game_once() {
    let (Some(folder), Some(innoextract), Some(dest)) = (
        var("CMR2005_REAL_INSTALLER"),
        var("CMR2005_INNOEXTRACT"),
        var("CMR2005_UNPACK_DEST"),
    ) else {
        eprintln!("skipped: CMR2005_REAL_INSTALLER, CMR2005_INNOEXTRACT, CMR2005_UNPACK_DEST");
        return;
    };
    let Source::Installer(installer) = source::detect(&folder).unwrap() else {
        panic!("{} holds no installer", folder.display());
    };
    assert_eq!(installer.build.name, "GOG offline installer (v1.1)");

    let mut log = |line: &str| println!("log: {line}");
    let no_window = |_: &mut dyn rea_launcher::installer::UnpackJob| {
        Err::<rea_launcher::installer::UnpackOutcome, _>("no window in this test".to_owned())
    };
    let started = Instant::now();
    let Unpacked::Ready(done) =
        source::unpack(&innoextract, &installer, &dest, &mut log, no_window).unwrap()
    else {
        panic!("cancelled");
    };
    println!("unpacked in {:.0} s", started.elapsed().as_secs_f64());
    assert_eq!(done.game.edition.name, "PC v1.1 (GOG)");
    assert!(done.preferred.is_empty() && done.removed.is_empty() && done.dropped.is_empty());
    assert!(rea_launcher::installer::is_unpacked(&dest, &installer));
    assert!(!rea_launcher::installer::work_dir(&dest).exists());

    // Launcher 0.2's marker: renamed, and the game reused without innoextract.
    std::fs::rename(dest.join(MARKER), dest.join(source::OLD_MARKER)).unwrap();
    let missing = dest.with_file_name("no-innoextract.exe");
    let started = Instant::now();
    let Unpacked::Ready(again) = source::unpack(&missing, &installer, &dest, &mut log, |_| {
        panic!("no unpack expected")
    })
    .unwrap() else {
        panic!("cancelled");
    };
    assert_eq!(again.game.edition.name, "PC v1.1 (GOG)");
    assert!(started.elapsed().as_secs() < 30, "reused, not unpacked");
    assert!(dest.join(MARKER).is_file() && !dest.join(source::OLD_MARKER).exists());

    // The unpacked folder is an installed game for the next start.
    assert!(matches!(
        source::detect(&dest).unwrap(),
        Source::Installed { .. }
    ));
}
