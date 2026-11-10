//! The private runtime copy of the game folder (`rea_launcher::runtime`).
//!
//! The player's folder is only read. The launcher brings its files into a
//! runtime folder of its own and changes only that copy: every write there
//! is a whole-file swap (`rea_launcher::fsx`). The registry sends the game
//! here (`INSTALL_PATH`) and its saves to a folder of their own
//! (`SAVEGAME_PATH`, see [`crate::saves`]), so the game writes nothing in
//! the runtime (docs/RUNTIME_NOTES.md, run r11: only `SG\OPTIONS` and
//! `SG\BESTTIMES_GHOSTS` were written, and those go to the saves folder).

use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::runtime::{self, Policy, Stamp, SyncReport};
use rea_launcher::{fsx, paths};

use crate::edition::GAME_EXE;

/// The runtime's record of what it brought over.
pub const MANIFEST: &str = ".cmr2005-manifest";

/// The launcher's generated DXVK settings ([`crate::components`]).
pub const DXVK_CONF: &str = "dxvk.conf";

/// What Colin McRae Rally 2005 does with its files.
pub struct GamePolicy;

fn top(rel: &str) -> String {
    rel.split('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

impl Policy for GamePolicy {
    /// The 3.4 GB of tracks, cars, videos and fonts the game only reads:
    /// hard-linked when the disk allows, so the copy takes no extra space.
    fn link(&self, rel: &str) -> bool {
        matches!(top(rel).as_str(), "data" | "fonts")
    }

    /// The save folder of an installed game (`SG`), and DLLs a player
    /// added (a DSOAL `dsound.dll`): copied once, then kept as they are.
    fn game_written(&self, rel: &str, _source: Stamp) -> bool {
        top(rel) == "sg" || rel.to_ascii_lowercase().ends_with(".dll") && !rel.contains('/')
    }

    /// The launcher's own `dxvk.conf`: never taken from the game folder.
    fn owned(&self, rel: &str, _source: Stamp) -> bool {
        rel.eq_ignore_ascii_case(DXVK_CONF)
    }
}

/// Brings the game folder into `runtime`, which may never overlap it.
pub fn prepare(game: &Path, runtime: &Path) -> Result<SyncReport, String> {
    paths::check_separate(game, runtime)?;
    sync(game, runtime).map_err(|error| format!("Cannot prepare {}: {error}", runtime.display()))
}

fn sync(game: &Path, runtime: &Path) -> io::Result<SyncReport> {
    runtime::sync(game, runtime, &GamePolicy, MANIFEST)
}

/// The runtime's `cmr5.exe`, made from the player's `bytes`: the close
/// hook's DLL (`rea_close_hook.dll`) added to its import table, so Windows
/// loads it with the game and the launcher never touches the running game
/// (antivirus heuristics take a hook installed from outside for a
/// keylogger). The DLL also gives the game its registry key from HKCU
/// (`registry::REDIRECT`), so an exe that can't take the import is an
/// error: the game would not find its folders. `--check` asks the same.
pub fn game_exe(bytes: &[u8]) -> Result<Vec<u8>, String> {
    rea_close_hook::import_into(bytes)
        .map_err(|error| format!("{GAME_EXE} can't import {} ({error})", rea_close_hook::DLL))
}

/// The player's `cmr5.exe` in `game`, and its bytes.
pub fn read_game_exe(game: &Path) -> Result<(PathBuf, Vec<u8>), String> {
    let source = paths::find_ignoring_case(game, GAME_EXE)
        .ok_or_else(|| format!("There is no {GAME_EXE} in {}.", game.display()))?;
    let bytes = std::fs::read(&source)
        .map_err(|error| format!("Cannot read {}: {error}", source.display()))?;
    Ok((source, bytes))
}

/// Writes `runtime`'s `cmr5.exe` from the one in `game` on every start
/// ([`game_exe`]): always from the player's bytes, so the import is added
/// once, whatever an earlier start left there. The player's file is only
/// read.
pub fn write_game_exe(game: &Path, runtime: &Path) -> Result<(), String> {
    let (source, bytes) = read_game_exe(game)?;
    let exe = game_exe(&bytes)?;
    let name = source.file_name().unwrap_or(GAME_EXE.as_ref());
    let target = runtime.join(name);
    fsx::install(&target, exe)
        .map_err(|error| format!("Cannot write {}: {error}", target.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn data_is_linked_and_player_files_are_kept() {
        let stamp = Stamp { size: 1, mtime: 1 };
        assert!(GamePolicy.link("data/Tracks/aus/track.big"));
        assert!(GamePolicy.link("Fonts/HelNu_10.dds"));
        assert!(!GamePolicy.link("cmr5.exe"));
        assert!(!GamePolicy.link("MathCPU.dll"));
        assert!(GamePolicy.game_written("SG/OPTIONS", stamp));
        assert!(GamePolicy.game_written("dsound.dll", stamp));
        assert!(!GamePolicy.game_written("data/x.dll", stamp));
        assert!(!GamePolicy.game_written("cmr5.exe", stamp));
        assert!(GamePolicy.owned("DXVK.conf", stamp));
        assert!(!GamePolicy.owned("readme.txt", stamp));
    }

    #[test]
    fn the_game_is_copied_and_never_changed() {
        let root =
            std::env::temp_dir().join(format!("cmr2005-runtime-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (game, rt) = (root.join("Jogos d'Ação"), root.join("runtime"));
        fs::create_dir_all(game.join("data")).unwrap();
        fs::write(game.join("cmr5.exe"), b"exe").unwrap();
        fs::write(game.join("data/a.bin"), b"data").unwrap();

        let report = prepare(&game, &rt).unwrap();
        assert_eq!(report.brought, 2);
        assert_eq!(fs::read(rt.join("data/a.bin")).unwrap(), b"data");
        // A second run brings nothing new.
        assert_eq!(prepare(&game, &rt).unwrap().brought, 0);
        assert_eq!(fs::read(game.join("cmr5.exe")).unwrap(), b"exe");

        // A runtime inside the game folder is refused.
        assert!(prepare(&game, &game.join("runtime")).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    /// Not a PE (or one without room for the import): the game would not
    /// find its registry key without the hook, so nothing is written and
    /// the reason is given. The player's file is never written.
    #[test]
    fn an_exe_that_cant_take_the_hook_is_refused() {
        let root = std::env::temp_dir().join(format!("cmr2005-game-exe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (game, rt) = (root.join("Jogos d'Ação"), root.join("runtime"));
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&rt).unwrap();
        assert!(
            write_game_exe(&game, &rt)
                .unwrap_err()
                .contains("There is no cmr5.exe")
        );

        fs::write(game.join("CMR5.EXE"), b"MZ not a real exe").unwrap();
        let error = write_game_exe(&game, &rt).unwrap_err();
        assert!(
            error.starts_with("cmr5.exe can't import rea_close_hook.dll"),
            "{error}"
        );
        assert!(!rt.join("CMR5.EXE").exists());
        assert_eq!(
            fs::read(game.join("CMR5.EXE")).unwrap(),
            b"MZ not a real exe"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// The real game's exe, when `CMR2005_GAME_DIR` names a game folder
    /// (never committed, only read): the runtime copy imports the hook,
    /// once, however many starts write it, and the player's file stays as
    /// it was.
    #[test]
    fn the_real_cmr5_exe_takes_the_hook_once() {
        let Some(dir) = std::env::var_os("CMR2005_GAME_DIR") else {
            eprintln!("skipped: CMR2005_GAME_DIR is not set");
            return;
        };
        let game = std::path::PathBuf::from(dir);
        let player = fs::read(paths::find_ignoring_case(&game, GAME_EXE).unwrap()).unwrap();
        let root = std::env::temp_dir().join(format!("cmr2005-real-exe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        write_game_exe(&game, &root).unwrap();
        let first = fs::read(root.join(GAME_EXE)).unwrap();
        write_game_exe(&game, &root).unwrap();
        assert_eq!(fs::read(root.join(GAME_EXE)).unwrap(), first);
        assert_eq!(
            game_exe(&first).unwrap(),
            first,
            "an already hooked exe is unchanged"
        );
        let imports = rea_launcher::pe::edit::imports(&first).unwrap();
        assert_eq!(
            imports
                .iter()
                .filter(|i| i.dll.eq_ignore_ascii_case(rea_close_hook::DLL))
                .count(),
            1
        );
        assert_eq!(
            fs::read(paths::find_ignoring_case(&game, GAME_EXE).unwrap()).unwrap(),
            player
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// Under Wine (`sdk/scripts/wine-test`): a real PE, this test binary,
    /// takes the import and a second pass leaves it as it is.
    #[cfg(windows)]
    #[test]
    fn a_pe_takes_the_hook_once() {
        let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
        let hooked = game_exe(&bytes).unwrap();
        assert_ne!(hooked, bytes);
        assert_eq!(game_exe(&hooked).unwrap(), hooked);
    }
}
