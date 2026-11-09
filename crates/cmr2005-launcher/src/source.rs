//! Where the game comes from: an installed folder (with `cmr5.exe`), or
//! GOG's installer files, which the launcher unpacks itself.
//!
//! GOG's installer hangs under Wine (docs/RESEARCH.md), so the launcher
//! never runs it: the SDK's `rea_launcher::installer` reads it with the
//! bundled innoextract (`components/innoextract.exe`, zlib licence) into a
//! folder of the launcher's own, once, and the launcher plays from that.
//! The installer files are only read. This module holds what is
//! Colin McRae Rally 2005's: the installer table, the "no game here"
//! message, the marker of launcher 0.2 and what to tell the player.

use std::fs;
use std::path::{Path, PathBuf};

use rea_launcher::identify::{self, Known};
use rea_launcher::installer::{self, MARKER};
pub use rea_launcher::installer::{Installer, InstallerBuild};
use rea_launcher::paths;

use crate::edition::{self, Found};

/// GOG's offline installer: the setup program beside its two data parts.
pub const SETUP_EXE: &str = "setup_colin2005.exe";

/// The installers this launcher unpacks (docs/RESEARCH.md, "GOG's
/// installer"): GOG's offline installer v1.1, Inno Setup 5.4.2. Its
/// `app\` is the installed game byte for byte; nothing collides and
/// nothing is added that the launcher bundles itself.
pub static INSTALLERS: &[Known<InstallerBuild>] = &[Known::new(
    "690a5e3f203e84a49fa0bad1a5e9bb628adce3f0b5a6e0349409484b737a8240",
    InstallerBuild {
        name: "GOG offline installer (v1.1)",
        setup: SETUP_EXE,
        parts: &[
            ("setup_colin2005-1.bin", 2_097_272_064),
            ("setup_colin2005-2.bin", 985_406_510),
        ],
        unpacked: 3_564_462_813,
        root: "app",
        prefer: &[],
        drop: &[],
    },
)];

/// The marker launcher 0.2 wrote into the folder it unpacked (the setup
/// program's SHA-256), before the SDK's [`MARKER`] (`.rea-unpacked`).
pub const OLD_MARKER: &str = ".cmr2005-unpacked";

/// A game folder the player chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An installed game: play from it.
    Installed { dir: PathBuf, found: Found },
    /// GOG's installer files: unpack them first.
    Installer(Installer),
}

impl Source {
    /// For the log and the window.
    pub fn describe(&self) -> String {
        match self {
            Source::Installed { found, .. } => found.edition.name.to_owned(),
            Source::Installer(installer) => installer.build.name.to_owned(),
        }
    }
}

/// What `dir` holds: an installed game (its edition), GOG's installer, or
/// a message for the player saying why it cannot be played. An installed
/// game wins when both are there.
pub fn detect(dir: &Path) -> Result<Source, String> {
    detect_in(dir, INSTALLERS)
}

fn detect_in(dir: &Path, installers: &'static [Known<InstallerBuild>]) -> Result<Source, String> {
    if !dir.is_dir() {
        return Err(format!("There is no folder {}.", dir.display()));
    }
    if paths::find_ignoring_case(dir, edition::GAME_EXE).is_some() {
        let found = edition::find(dir)?;
        return Ok(Source::Installed {
            dir: dir.to_path_buf(),
            found,
        });
    }
    match installer::detect(dir, installers, edition::UNKNOWN_EDITION_ISSUE)? {
        Some(found) => Ok(Source::Installer(found)),
        None => Err(format!(
            "There is no {} or {SETUP_EXE} in {}.",
            edition::GAME_EXE,
            dir.display()
        )),
    }
}

/// The launcher's game check on an unpacked folder: a known `cmr5.exe`.
pub fn check(dir: &Path) -> Result<Found, String> {
    edition::find(dir)
}

/// The marker text in `dir`, by name.
fn marker(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|text| text.trim().to_owned())
}

/// Whether `dest` already holds the game unpacked from `installer`, under
/// either marker. Only reads (for `--check`).
pub fn unpacked_already(dest: &Path, installer: &Installer) -> bool {
    let digest = identify::hex(&installer.sha256);
    [MARKER, OLD_MARKER]
        .into_iter()
        .any(|name| marker(dest, name).is_some_and(|text| text == digest))
}

/// Whether `dir` is a folder the launcher unpacked (either marker).
pub fn is_unpacked_folder(dir: &Path) -> bool {
    dir.join(MARKER).is_file() || dir.join(OLD_MARKER).is_file()
}

/// Before the SDK looks at `dest`: a folder launcher 0.2 unpacked carries
/// [`OLD_MARKER`], which the SDK does not know. It is renamed to
/// [`MARKER`], so the same installer's game is reused, not unpacked again
/// (and another installer build replaces the launcher's own folder, as it
/// did before). Returns a line for `launcher.log` when it renamed one.
pub fn adopt_old_marker(dest: &Path) -> Result<Option<String>, String> {
    let old = dest.join(OLD_MARKER);
    if !old.is_file() || dest.join(MARKER).exists() {
        return Ok(None);
    }
    fs::rename(&old, dest.join(MARKER))
        .map_err(|error| format!("Cannot update {}: {error}", old.display()))?;
    Ok(Some(format!(
        "unpacked game in {}: marker {OLD_MARKER} renamed to {MARKER}",
        dest.display()
    )))
}

/// Unpacks GOG's `installer` into `dest` with `innoextract` (the bundled
/// `components/innoextract.exe`), once: the SDK's
/// `rea_launcher_win32::innoextract::unpack` with this launcher's game
/// check, after [`adopt_old_marker`]. `watch` shows the progress (the
/// unpack window, `ui::show_unpack`); when it returns an error the unpack
/// is waited for without a window. `log` gets every step for
/// `launcher.log`.
#[cfg(windows)]
pub fn unpack(
    innoextract: &Path,
    installer: &Installer,
    dest: &Path,
    log: &mut dyn FnMut(&str),
    watch: impl FnOnce(&mut dyn installer::UnpackJob) -> Result<installer::UnpackOutcome, String>,
) -> Result<rea_launcher_win32::innoextract::Unpacked<Found>, String> {
    if let Some(line) = adopt_old_marker(dest)? {
        log(&line);
    }
    rea_launcher_win32::innoextract::unpack(innoextract, installer, dest, check, log, watch)
}

/// What becomes of the game setting once the game is unpacked into `dest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSetting {
    /// The new `game` value for `launcher.cfg`; `None` when the folder came
    /// from the command line (`--game`), which a run never saves over.
    pub save: Option<PathBuf>,
    /// For `launcher.log` and the player.
    pub message: String,
}

/// After an unpack into `dest`: point the game setting at it (the next
/// start finds an installed game, and the installer files are no longer
/// needed), unless `from_command_line`, when it is only said.
pub fn setting_after_unpack(dest: &Path, from_command_line: bool) -> GameSetting {
    let dir = dest.display();
    if from_command_line {
        GameSetting {
            save: None,
            message: format!(
                "The game was unpacked into {dir}. The game folder came from --game, so the \
                 setting was not changed; use --game \"{dir}\" from now on, and the installer \
                 files are no longer needed."
            ),
        }
    } else {
        GameSetting {
            save: Some(dest.to_path_buf()),
            message: format!(
                "The game was unpacked into {dir}. The game folder setting now points there; \
                 the installer files are no longer needed."
            ),
        }
    }
}

/// The `--check` and window line for GOG's installer: what the first Play
/// does, where (`dest`: the launcher's unpacked folder, when known), and
/// the space it needs.
pub fn installer_plan(installer: &Installer, dest: Option<&Path>) -> String {
    if let Some(dest) = dest.filter(|dest| unpacked_already(dest, installer)) {
        return format!(
            "{} was already unpacked into {}; Play uses that copy.",
            installer.build.name,
            dest.display()
        );
    }
    let into = dest.map_or_else(
        || "a folder of the launcher's own".to_owned(),
        |dest| dest.display().to_string(),
    );
    format!(
        "Found {}. The first Play unpacks the game from it once into {into} \
         (about {:.1} GB free space needed there). The installer is never run; \
         its files are only read.",
        installer.build.name,
        installer::space_needed(installer.build) as f64 / 1e9
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmr2005-source-{}-{name}-Jogos d'Ação",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    static FAKE: &[Known<InstallerBuild>] = &[Known {
        sha256: [0; 32],
        value: InstallerBuild {
            name: "fake",
            setup: SETUP_EXE,
            parts: &[("setup_colin2005-1.bin", 10), ("setup_colin2005-2.bin", 3)],
            unpacked: 8,
            root: "app",
            prefer: &[],
            drop: &[],
        },
    }];

    /// [`FAKE`] with the digest of `b"setup"`, leaked for the 'static table.
    fn fake_table() -> &'static [Known<InstallerBuild>] {
        let mut table = FAKE.to_vec();
        table[0].sha256 = identify::sha256(b"setup");
        Box::leak(table.into_boxed_slice())
    }

    /// A fake installer in `dir`: a setup program whose digest
    /// [`fake_table`] knows, and parts of the right sizes (sparse).
    fn fake_installer(dir: &Path) -> Installer {
        fs::write(dir.join("Setup_Colin2005.exe"), b"setup").unwrap();
        for (name, size) in FAKE[0].value.parts {
            fs::File::create(dir.join(name))
                .unwrap()
                .set_len(*size)
                .unwrap();
        }
        match detect_in(dir, fake_table()).unwrap() {
            Source::Installer(installer) => installer,
            other => panic!("{other:?}"),
        }
    }

    fn fake_check(dir: &Path) -> Result<Found, String> {
        let exe =
            paths::find_ignoring_case(dir, edition::GAME_EXE).ok_or_else(|| "no exe".to_owned())?;
        (fs::read(&exe).unwrap() == b"good")
            .then_some(())
            .ok_or("bad exe")?;
        Ok(Found {
            exe,
            edition: &edition::EDITIONS[0].value,
        })
    }

    /// innoextract's stand-in: the game under `app\` of its output folder.
    fn extract(args: &[OsString]) -> Result<(), String> {
        let at = args.iter().position(|a| a == "--output-dir").unwrap();
        let out = PathBuf::from(&args[at + 1]);
        fs::create_dir_all(out.join("app/data")).unwrap();
        fs::write(out.join("app/cmr5.exe"), b"good").unwrap();
        fs::write(out.join("app/data/a.big"), b"data").unwrap();
        Ok(())
    }

    #[test]
    fn the_owners_installer_is_known() {
        assert_eq!(INSTALLERS.len(), 1);
        let gog = &INSTALLERS[0];
        assert_eq!(
            identify::hex(&gog.sha256),
            "690a5e3f203e84a49fa0bad1a5e9bb628adce3f0b5a6e0349409484b737a8240"
        );
        let build = &gog.value;
        assert_eq!(build.setup, SETUP_EXE);
        assert_eq!(
            build.parts,
            [
                ("setup_colin2005-1.bin", 2_097_272_064),
                ("setup_colin2005-2.bin", 985_406_510),
            ]
        );
        assert_eq!(build.unpacked, 3_564_462_813);
        assert_eq!(build.root, "app");
        // GOG's app\ is the installed game as is (docs/RESEARCH.md).
        assert!(build.prefer.is_empty() && build.drop.is_empty());
    }

    #[test]
    fn a_folder_without_the_game_or_installer_is_explained() {
        let dir = temp("empty");
        let error = detect(&dir).unwrap_err();
        assert!(
            error.contains("cmr5.exe") && error.contains(SETUP_EXE),
            "{error}"
        );
        assert!(detect(&dir.join("nope")).unwrap_err().contains("no folder"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unknown_installer_is_left_alone_with_its_digest() {
        let dir = temp("unknown");
        fs::write(dir.join(SETUP_EXE), b"some other setup").unwrap();
        let error = detect(&dir).unwrap_err();
        let digest = identify::hex(&identify::sha256(b"some other setup"));
        assert!(error.contains("left alone"), "{error}");
        assert!(error.contains(&digest), "{error}");
        assert!(error.contains(edition::UNKNOWN_EDITION_ISSUE), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn another_inno_setup_installer_is_left_alone_too() {
        let dir = temp("other-inno");
        let mut exe = b"MZ".to_vec();
        exe.extend_from_slice(installer::INNO_SIGNATURE);
        exe.extend_from_slice(b"5.5.0)");
        fs::write(dir.join("setup_other_game.exe"), &exe).unwrap();
        let error = detect(&dir).unwrap_err();
        assert!(error.contains("setup_other_game.exe"), "{error}");
        assert!(error.contains("left alone"), "{error}");
        assert!(error.contains(edition::UNKNOWN_EDITION_ISSUE), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_installed_game_is_identified_by_its_exe() {
        let dir = temp("installed");
        fs::write(dir.join("CMR5.EXE"), b"not the game").unwrap();
        // An unknown exe is refused by the edition check, not taken for an installer.
        let error = detect(&dir).unwrap_err();
        assert!(error.contains("not a build this launcher knows"), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn detect_finds_the_installer_and_checks_its_parts() {
        let root = temp("parts");
        let installer = fake_installer(&root);
        assert_eq!(installer.build.name, "fake");
        assert_eq!(installer.sha256, identify::sha256(b"setup"));
        fs::File::create(root.join("setup_colin2005-2.bin"))
            .unwrap()
            .set_len(2)
            .unwrap();
        let error = detect_in(&root, fake_table()).unwrap_err();
        assert!(error.contains("has 2 bytes instead of 3"), "{error}");
        fs::remove_file(root.join("setup_colin2005-1.bin")).unwrap();
        let error = detect_in(&root, fake_table()).unwrap_err();
        assert!(
            error.contains("setup_colin2005-1.bin is missing"),
            "{error}"
        );
        // Not the owner's setup program: refused before the parts.
        assert!(detect(&root).unwrap_err().contains("left alone"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_player_unpacks_once_and_later_starts_reuse_it() {
        let root = temp("unpack");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        let runs = std::cell::Cell::new(0);
        let run = |args: &[OsString]| {
            runs.set(runs.get() + 1);
            extract(args)
        };
        let done = installer::unpack(&installer, &dest, run, fake_check).unwrap();
        assert_eq!(done.game.exe, dest.join("cmr5.exe"));
        assert!(unpacked_already(&dest, &installer));
        assert!(is_unpacked_folder(&dest));
        installer::unpack(&installer, &dest, run, fake_check).unwrap();
        assert_eq!(runs.get(), 1, "the second start reuses the unpacked game");
        fs::remove_dir_all(&root).unwrap();
    }

    /// Launcher 0.2's folder (`.cmr2005-unpacked`): kept, never unpacked again.
    #[test]
    fn a_folder_launcher_0_2_unpacked_is_reused() {
        let root = temp("old-marker");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("cmr5.exe"), b"good").unwrap();
        fs::write(
            dest.join(OLD_MARKER),
            format!("{}\n", identify::hex(&installer.sha256)),
        )
        .unwrap();
        assert!(unpacked_already(&dest, &installer), "--check sees it too");
        assert!(is_unpacked_folder(&dest));

        let line = adopt_old_marker(&dest).unwrap().unwrap();
        assert!(line.contains(OLD_MARKER) && line.contains(MARKER), "{line}");
        assert!(!dest.join(OLD_MARKER).exists());
        assert!(installer::is_unpacked(&dest, &installer));
        let done =
            installer::unpack(&installer, &dest, |_| panic!("must not run"), fake_check).unwrap();
        assert_eq!(done.game.exe, dest.join("cmr5.exe"));
        // Once renamed there is nothing left to do.
        assert_eq!(adopt_old_marker(&dest).unwrap(), None);
        fs::remove_dir_all(&root).unwrap();
    }

    /// Launcher 0.2 replaced its own folder for another installer build;
    /// so does this one.
    #[test]
    fn a_launcher_0_2_folder_from_another_installer_is_replaced() {
        let root = temp("old-other");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("cmr5.exe"), b"older").unwrap();
        fs::write(dest.join(OLD_MARKER), identify::hex(&[7; 32])).unwrap();
        assert!(!unpacked_already(&dest, &installer));
        adopt_old_marker(&dest).unwrap().unwrap();
        installer::unpack(&installer, &dest, extract, fake_check).unwrap();
        assert_eq!(fs::read(dest.join("cmr5.exe")).unwrap(), b"good");
        assert!(installer::is_unpacked(&dest, &installer));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_folder_the_player_made_is_never_replaced() {
        let root = temp("players");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("mine.txt"), b"keep").unwrap();
        assert_eq!(adopt_old_marker(&dest).unwrap(), None);
        let error = installer::unpack(&installer, &dest, |_| panic!("must not run"), fake_check)
            .unwrap_err();
        assert!(
            error.contains("not a folder this launcher unpacked"),
            "{error}"
        );
        assert_eq!(fs::read(dest.join("mine.txt")).unwrap(), b"keep");
        assert!(!is_unpacked_folder(&dest));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_unknown_exe_from_the_installer_is_not_kept() {
        let root = temp("badexe");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        let run = |args: &[OsString]| {
            extract(args)?;
            let at = args.iter().position(|a| a == "--output-dir").unwrap();
            fs::write(PathBuf::from(&args[at + 1]).join("app/cmr5.exe"), b"x").unwrap();
            Ok(())
        };
        let error = installer::unpack(&installer, &dest, run, check).unwrap_err();
        assert!(error.contains("not a build this launcher knows"), "{error}");
        assert!(!dest.exists());
        assert!(!installer::work_dir(&dest).exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_game_setting_points_at_the_unpacked_folder_unless_it_came_from_the_command_line() {
        let dest = Path::new(r"C:\users\player\AppData\Local\CMR2005Launcher\game");
        let saved = setting_after_unpack(dest, false);
        assert_eq!(saved.save.as_deref(), Some(dest));
        assert_eq!(
            saved.message,
            format!(
                "The game was unpacked into {}. The game folder setting now points there; \
                 the installer files are no longer needed.",
                dest.display()
            )
        );
        let said = setting_after_unpack(dest, true);
        assert_eq!(said.save, None);
        assert!(said.message.contains(&dest.display().to_string()));
        assert!(said.message.contains("--game"), "{}", said.message);
        assert!(
            said.message.contains("no longer needed"),
            "{}",
            said.message
        );
    }

    #[test]
    fn the_installer_plan_says_what_happens_and_the_space_it_needs() {
        let root = temp("plan");
        let gog = Installer {
            setup: root.join(SETUP_EXE),
            sha256: INSTALLERS[0].sha256,
            build: &INSTALLERS[0].value,
        };
        let dest = root.join("game");
        let plan = installer_plan(&gog, Some(&dest));
        assert!(
            plan.starts_with("Found GOG offline installer (v1.1)."),
            "{plan}"
        );
        assert!(plan.contains("unpacks the game from it once"), "{plan}");
        assert!(plan.contains(&dest.display().to_string()), "{plan}");
        assert!(plan.contains("about 3.9 GB free space"), "{plan}");
        assert!(plan.contains("never run"), "{plan}");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join(OLD_MARKER), identify::hex(&gog.sha256)).unwrap();
        let again = installer_plan(&gog, Some(&dest));
        assert!(installer_plan(&gog, None).contains("a folder of the launcher's own"));
        assert!(again.contains("already unpacked"), "{again}");
        assert!(dest.join(OLD_MARKER).is_file(), "--check only reads");
        fs::remove_dir_all(&root).unwrap();
    }
}
