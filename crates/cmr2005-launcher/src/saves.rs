//! The saves folder: where the game keeps `OPTIONS`, `BESTTIMES_GHOSTS`,
//! `CAREER_SLOT_*` and `CHAMPIONSHIP_SLOT_*`.
//!
//! The registry's `SAVEGAME_PATH` sends the game's saves straight to this
//! folder, outside the runtime copy, so nothing has to be mirrored and a
//! runtime rebuild never touches them. An installed game may already have
//! saves in its own `SG` folder: those are copied in when the saves folder
//! has none of its own yet, never over a file that is there.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::{fsx, paths};

/// The installed game's save folder.
pub const GAME_SAVES: &str = "SG";

/// Written into the saves folder once the game's saves were looked at, so
/// saves a player deleted later never come back.
pub const IMPORTED: &str = ".cmr2005-imported";

/// The folders the launcher keeps under its data folder (`home`): saves,
/// the runtime copy, and the game unpacked from GOG's installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folders {
    pub runtime: PathBuf,
    pub saves: PathBuf,
    pub unpacked: PathBuf,
}

impl Folders {
    /// The folders for the `runtime` setting (`None`: `home\runtime`). The
    /// unpacked game sits beside the runtime (it is as big as the game, so
    /// it follows a runtime moved to another drive); the saves always stay
    /// in `home`.
    pub fn new(home: &Path, runtime: Option<&Path>) -> Folders {
        let runtime = runtime.map_or_else(|| home.join("runtime"), Path::to_path_buf);
        let name = runtime.file_name().unwrap_or_default().to_string_lossy();
        let unpacked = if name.eq_ignore_ascii_case("runtime") {
            runtime.with_file_name("game")
        } else {
            runtime.with_file_name(format!("{name}-game"))
        };
        Folders {
            runtime,
            saves: home.join("saves"),
            unpacked,
        }
    }
}

/// Copies the saves of an installed game (`game\SG`) into `saves` the
/// first time, when `saves` holds no file yet; returns the names copied.
/// The game folder is only read.
pub fn import(game: &Path, saves: &Path) -> io::Result<Vec<String>> {
    fs::create_dir_all(saves)?;
    if has_files(saves)? {
        return Ok(Vec::new());
    }
    let copied = copy_game_saves(game, saves)?;
    fsx::write_atomic(&saves.join(IMPORTED), copied.join("\n"))?;
    Ok(copied)
}

fn copy_game_saves(game: &Path, saves: &Path) -> io::Result<Vec<String>> {
    let Some(sg) = paths::find_ignoring_case(game, GAME_SAVES).filter(|p| p.is_dir()) else {
        return Ok(Vec::new());
    };
    let mut copied = Vec::new();
    for entry in fs::read_dir(sg)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let dest = saves.join(&name);
        if dest.exists() {
            continue;
        }
        fsx::install_copy(&entry.path(), &dest)?;
        copied.push(name.to_string_lossy().into_owned());
    }
    copied.sort();
    Ok(copied)
}

fn has_files(dir: &Path) -> io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        if entry?.file_type()?.is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmr2005-saves-{}-{name}-José's",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_folders_follow_the_runtime_setting() {
        let home = Path::new("/h");
        assert_eq!(
            Folders::new(home, None),
            Folders {
                runtime: "/h/runtime".into(),
                saves: "/h/saves".into(),
                unpacked: "/h/game".into(),
            }
        );
        let moved = Folders::new(home, Some(Path::new("/d/CMR 2005")));
        assert_eq!(moved.runtime, PathBuf::from("/d/CMR 2005"));
        assert_eq!(moved.unpacked, PathBuf::from("/d/CMR 2005-game"));
        assert_eq!(moved.saves, PathBuf::from("/h/saves"));
    }

    #[test]
    fn a_player_with_saves_in_the_game_folder_keeps_them() {
        let root = temp("import");
        let (game, saves) = (root.join("game"), root.join("saves"));
        fs::create_dir_all(game.join("sg")).unwrap();
        fs::write(game.join("sg/OPTIONS"), b"opts").unwrap();
        fs::write(game.join("sg/CAREER_SLOT_1"), b"career").unwrap();
        assert_eq!(import(&game, &saves).unwrap(), ["CAREER_SLOT_1", "OPTIONS"]);
        assert_eq!(fs::read(saves.join("CAREER_SLOT_1")).unwrap(), b"career");
        assert!(
            game.join("sg/OPTIONS").is_file(),
            "the game folder is only read"
        );

        // Later the game writes newer saves; a second start never brings the old ones back.
        fs::write(saves.join("OPTIONS"), b"newer").unwrap();
        fs::remove_file(saves.join("CAREER_SLOT_1")).unwrap();
        assert!(import(&game, &saves).unwrap().is_empty());
        assert_eq!(fs::read(saves.join("OPTIONS")).unwrap(), b"newer");
        assert!(!saves.join("CAREER_SLOT_1").exists());
        // Even with every save deleted, the old ones stay gone.
        fs::remove_file(saves.join("OPTIONS")).unwrap();
        assert!(import(&game, &saves).unwrap().is_empty());
        assert!(!saves.join("OPTIONS").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_game_saves_just_makes_the_folder() {
        let root = temp("none");
        let saves = root.join("deep/saves");
        assert!(import(&root, &saves).unwrap().is_empty());
        assert!(saves.is_dir());
        fs::remove_dir_all(&root).unwrap();
    }
}
