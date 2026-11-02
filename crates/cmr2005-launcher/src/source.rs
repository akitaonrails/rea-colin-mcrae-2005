//! Where the game comes from: an installed folder (with `cmr5.exe`), or
//! the GOG installer files, which the launcher unpacks itself.
//!
//! GOG's installer hangs under Wine (docs/RESEARCH.md), so the launcher
//! never runs it: it reads the installer with the bundled innoextract
//! (`components/innoextract.exe`, zlib licence) into a folder of its own,
//! once, and plays from that. The installer files are only read.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::identify::{self, Known};
use rea_launcher::paths;

use crate::edition::{self, Found};

/// GOG's offline installer: the setup program and its data parts.
pub const SETUP_EXE: &str = "setup_colin2005.exe";

/// One installer this launcher knows: the setup program's SHA-256 and
/// the exact size of each data part (hashing 3 GB on every start would be
/// slow; innoextract checks every file's checksum while unpacking).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallerBuild {
    pub name: &'static str,
    pub parts: &'static [(&'static str, u64)],
}

pub static INSTALLERS: &[Known<InstallerBuild>] = &[Known::new(
    "690a5e3f203e84a49fa0bad1a5e9bb628adce3f0b5a6e0349409484b737a8240",
    InstallerBuild {
        name: "GOG offline installer (v1.1)",
        parts: &[
            ("setup_colin2005-1.bin", 2_097_272_064),
            ("setup_colin2005-2.bin", 985_406_510),
        ],
    },
)];

/// The record in an unpacked folder: the SHA-256 of the setup program it
/// came from. Only a folder with this file is the launcher's to replace.
pub const MARKER: &str = ".cmr2005-unpacked";

/// The free space an unpack needs: the 3.4 GB game plus a margin.
pub const UNPACK_BYTES: u64 = 3_800_000_000;

/// A game folder the player chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An installed game: play from it.
    Installed { dir: PathBuf, found: Found },
    /// GOG's installer files: unpack them first.
    Installer(Installer),
}

/// GOG's installer, identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    pub setup: PathBuf,
    pub sha256: [u8; 32],
    pub build: &'static InstallerBuild,
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
/// a message for the player saying why it cannot be played.
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
    let Some(setup) = paths::find_ignoring_case(dir, SETUP_EXE).filter(|p| p.is_file()) else {
        return Err(format!(
            "There is no {} or {SETUP_EXE} in {}.",
            edition::GAME_EXE,
            dir.display()
        ));
    };
    let sha256 = identify::sha256_file(&setup)
        .map_err(|error| format!("Cannot read {}: {error}", setup.display()))?;
    let build = identify::identify_digest(sha256, installers).map_err(|digest| {
        format!(
            "This {SETUP_EXE} is not an installer this launcher knows, so it was left alone.\n\n\
             SHA-256: {}\n\n\
             Please report it, with that SHA-256 and where your copy came from:\n{}",
            identify::hex(&digest),
            edition::UNKNOWN_EDITION_ISSUE
        )
    })?;
    for (name, size) in build.parts {
        let part = paths::find_ignoring_case(dir, name).filter(|p| p.is_file());
        let actual = part
            .as_deref()
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len());
        if actual != Some(*size) {
            let what = match actual {
                None => "is missing".to_owned(),
                Some(actual) => format!("has {actual} bytes instead of {size}"),
            };
            return Err(format!(
                "{name} {what} in {}. It belongs with {SETUP_EXE}: download it again from GOG.",
                dir.display()
            ));
        }
    }
    Ok(Source::Installer(Installer {
        setup,
        sha256,
        build,
    }))
}

/// Whether `dest` already holds the game unpacked from `installer`.
pub fn is_unpacked(dest: &Path, installer: &Installer) -> bool {
    let marker = fs::read_to_string(dest.join(MARKER)).unwrap_or_default();
    marker.trim() == identify::hex(&installer.sha256)
        && paths::find_ignoring_case(dest, edition::GAME_EXE).is_some()
}

/// The folder innoextract writes into, beside `dest`.
pub fn work_dir(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".unpacking");
    dest.with_file_name(name)
}

/// innoextract's arguments: extract only the game (`app\`), no temporary
/// files, stop on a checksum error, errors only, no progress bar.
pub fn innoextract_args(setup: &Path, out: &Path) -> Vec<String> {
    [
        "--extract",
        "--test",
        "--exclude-temp",
        "--silent",
        "--progress=0",
        "--include",
        "app",
        "--output-dir",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([out.display().to_string(), setup.display().to_string()])
    .collect()
}

/// Unpacks `installer` into `dest` with `run` (innoextract, called with
/// [`innoextract_args`]), unless it is there already. The game is checked
/// (a known `cmr5.exe`) before it takes `dest`'s place; a `dest` without
/// the [`MARKER`] is never removed.
pub fn unpack(
    installer: &Installer,
    dest: &Path,
    run: impl FnOnce(&[String]) -> Result<(), String>,
) -> Result<Found, String> {
    unpack_checked(installer, dest, run, edition::find)
}

fn unpack_checked(
    installer: &Installer,
    dest: &Path,
    run: impl FnOnce(&[String]) -> Result<(), String>,
    check: impl Fn(&Path) -> Result<Found, String>,
) -> Result<Found, String> {
    if is_unpacked(dest, installer) {
        return check(dest);
    }
    if dest.exists() && !dest.join(MARKER).is_file() {
        return Err(format!(
            "{} already exists and is not a folder this launcher unpacked; \
             move it away or choose another private copy folder.",
            dest.display()
        ));
    }
    let work = work_dir(dest);
    remove_dir(&work).map_err(|error| cannot(&work, error))?;
    fs::create_dir_all(&work).map_err(|error| cannot(&work, error))?;
    run(&innoextract_args(&installer.setup, &work))?;
    let app = paths::find_ignoring_case(&work, "app")
        .filter(|p| p.is_dir())
        .ok_or_else(|| format!("The installer gave no game files in {}.", work.display()))?;
    check(&app)?;
    fs::write(app.join(MARKER), identify::hex(&installer.sha256))
        .map_err(|error| cannot(&app, error))?;
    remove_dir(dest).map_err(|error| cannot(dest, error))?;
    fs::rename(&app, dest).map_err(|error| cannot(dest, error))?;
    remove_dir(&work).map_err(|error| cannot(&work, error))?;
    check(dest)
}

fn remove_dir(dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(dir) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn cannot(path: &Path, error: io::Error) -> String {
    format!("Cannot unpack the game into {}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmr2005-source-{}-{name}-Jogos d'Ação",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fake installer: a setup program whose digest the test registers,
    /// and parts of the right sizes (sparse, so nothing big is written).
    fn fake_installer(dir: &Path) -> Installer {
        fs::write(dir.join("Setup_Colin2005.exe"), b"setup").unwrap();
        static BUILD: InstallerBuild = InstallerBuild {
            name: "test installer",
            parts: &[("setup_colin2005-1.bin", 10), ("setup_colin2005-2.bin", 3)],
        };
        for (name, size) in BUILD.parts {
            fs::File::create(dir.join(name))
                .unwrap()
                .set_len(*size)
                .unwrap();
        }
        Installer {
            setup: dir.join("Setup_Colin2005.exe"),
            sha256: identify::sha256(b"setup"),
            build: &BUILD,
        }
    }

    #[test]
    fn the_owners_installer_is_known() {
        assert_eq!(INSTALLERS.len(), 1);
        let parts = INSTALLERS[0].value.parts;
        assert_eq!(parts.len(), 2);
        assert!(
            parts
                .iter()
                .all(|(name, size)| name.ends_with(".bin") && *size > 0)
        );
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
    fn an_unknown_installer_is_refused_with_its_digest() {
        let dir = temp("unknown");
        fs::write(dir.join(SETUP_EXE), b"some other setup").unwrap();
        let error = detect(&dir).unwrap_err();
        let digest = identify::hex(&identify::sha256(b"some other setup"));
        assert!(error.contains(&digest), "{error}");
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
    fn innoextract_extracts_only_the_game_and_checks_it() {
        let args = innoextract_args(
            Path::new(r"D:\GOG\setup_colin2005.exe"),
            Path::new(r"C:\x y"),
        );
        assert_eq!(
            args,
            [
                "--extract",
                "--test",
                "--exclude-temp",
                "--silent",
                "--progress=0",
                "--include",
                "app",
                "--output-dir",
                r"C:\x y",
                r"D:\GOG\setup_colin2005.exe",
            ]
        );
        assert_eq!(
            work_dir(Path::new("/a/game")),
            PathBuf::from("/a/game.unpacking")
        );
    }

    #[test]
    fn a_failed_unpack_leaves_no_game_behind() {
        let root = temp("failed");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        let error = unpack(&installer, &dest, |_| Err("innoextract failed".into())).unwrap_err();
        assert_eq!(error, "innoextract failed");
        assert!(!dest.exists());
        // innoextract "succeeded" without the game: refused, nothing kept.
        let error = unpack(&installer, &dest, |_| Ok(())).unwrap_err();
        assert!(error.contains("no game files"), "{error}");
        assert!(!dest.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_unknown_exe_from_the_installer_is_not_kept() {
        let root = temp("badexe");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        let error = unpack(&installer, &dest, |args| {
            let out = PathBuf::from(&args[8]);
            fs::create_dir_all(out.join("app")).unwrap();
            fs::write(out.join("app/cmr5.exe"), b"not the game").unwrap();
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("not a build this launcher knows"), "{error}");
        assert!(!dest.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_folder_the_player_made_is_never_replaced() {
        let root = temp("players");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("mine.txt"), b"keep").unwrap();
        let error = unpack(&installer, &dest, |_| panic!("must not run")).unwrap_err();
        assert!(
            error.contains("not a folder this launcher unpacked"),
            "{error}"
        );
        assert_eq!(fs::read(dest.join("mine.txt")).unwrap(), b"keep");
        fs::remove_dir_all(&root).unwrap();
    }

    static FAKE: &[Known<InstallerBuild>] = &[Known {
        sha256: [0; 32],
        value: InstallerBuild {
            name: "fake",
            parts: &[("setup_colin2005-1.bin", 10), ("setup_colin2005-2.bin", 3)],
        },
    }];

    /// [`FAKE`] with the digest of `b"setup"`, leaked for the 'static table.
    fn fake_table() -> &'static [Known<InstallerBuild>] {
        let mut table = FAKE.to_vec();
        table[0].sha256 = identify::sha256(b"setup");
        Box::leak(table.into_boxed_slice())
    }

    #[test]
    fn detect_finds_the_installer_and_checks_its_parts() {
        let root = temp("parts");
        fake_installer(&root);
        let table = fake_table();
        match detect_in(&root, table).unwrap() {
            Source::Installer(installer) => {
                assert_eq!(installer.build.name, "fake");
                assert_eq!(installer.sha256, identify::sha256(b"setup"));
            }
            other => panic!("{other:?}"),
        }
        fs::File::create(root.join("setup_colin2005-2.bin"))
            .unwrap()
            .set_len(2)
            .unwrap();
        let error = detect_in(&root, table).unwrap_err();
        assert!(error.contains("has 2 bytes instead of 3"), "{error}");
        fs::remove_file(root.join("setup_colin2005-1.bin")).unwrap();
        let error = detect_in(&root, table).unwrap_err();
        assert!(
            error.contains("setup_colin2005-1.bin is missing"),
            "{error}"
        );
        // Not the owner's setup program: refused before the parts.
        assert!(
            detect(&root)
                .unwrap_err()
                .contains("not an installer this launcher knows")
        );
        fs::remove_dir_all(&root).unwrap();
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

    #[test]
    fn the_player_unpacks_once_and_later_starts_reuse_it() {
        let root = temp("unpack");
        let installer = fake_installer(&root);
        let dest = root.join("game");
        let runs = std::cell::Cell::new(0);
        let extract = |args: &[String]| {
            runs.set(runs.get() + 1);
            let out = PathBuf::from(&args[8]);
            assert_eq!(args[9], installer.setup.display().to_string());
            fs::create_dir_all(out.join("app/data")).unwrap();
            fs::write(out.join("app/cmr5.exe"), b"good").unwrap();
            fs::write(out.join("app/data/a.big"), b"data").unwrap();
            fs::create_dir_all(out.join("tmp")).unwrap();
            Ok(())
        };
        let found = unpack_checked(&installer, &dest, extract, fake_check).unwrap();
        assert_eq!(found.exe, dest.join("cmr5.exe"));
        assert_eq!(fs::read(dest.join("data/a.big")).unwrap(), b"data");
        assert!(is_unpacked(&dest, &installer));
        assert!(!work_dir(&dest).exists(), "the work folder is removed");
        unpack_checked(&installer, &dest, extract, fake_check).unwrap();
        assert_eq!(runs.get(), 1, "the second start reuses the unpacked game");

        // Another installer (a new GOG build): the launcher's own folder is replaced.
        let newer = Installer {
            sha256: identify::sha256(b"newer"),
            ..installer.clone()
        };
        assert!(!is_unpacked(&dest, &newer));
        unpack_checked(&newer, &dest, extract, fake_check).unwrap();
        assert_eq!(runs.get(), 2);
        assert!(is_unpacked(&dest, &newer));
        fs::remove_dir_all(&root).unwrap();
    }
}
