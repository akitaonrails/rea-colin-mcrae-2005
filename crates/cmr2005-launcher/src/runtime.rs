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
use std::path::Path;

use rea_launcher::paths;
use rea_launcher::runtime::{self, Policy, Stamp, SyncReport};

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
}
