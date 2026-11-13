//! The game's controller records in `SG\OPTIONS`, for the virtual wheel
//! (`rea_launcher::wheel::dinput::VIRTUAL`, which the in-game layer
//! lists first, so the game puts it in slot 2).
//!
//! `OPTIONS` (docs/RESEARCH.md, "Wheels"; cmr5.exe GOG 1.1) is exactly
//! [`SIZE`] bytes with no checksum. Each player has 18 records of
//! [`RECORD`] bytes, one per slot: 0 keyboard, 1 mouse, 2–17 the
//! controllers in DirectInput's order. A record keeps its device's
//! product and vendor ids; when they don't match the device in that slot
//! the game resets the record's bindings and ids to that device's
//! defaults, but keeps byte 0, the calibration tables and the autocentre
//! word. The last 8 bytes (profile id, version) are never touched: a
//! changed version makes the game delete the other save slots.
//!
//! The launcher changes slot 2 of player 1 only:
//!
//! - **Wheel shown** ([`prepare`] with `wheel`): a record that is already
//!   the virtual wheel's, and one the launcher put there, is left as it is
//!   (the player's in-game changes). Otherwise the record there is put
//!   aside in the [`Stash`] under its ids, and the virtual wheel's stashed
//!   record goes in, or the [`seed`] the first time; player 1 then uses
//!   slot 2.
//! - **No wheel**: when slot 2 still holds the virtual wheel's record, it
//!   is stashed and the record it displaced goes back (the game's reset
//!   would keep the wheel's calibration and apply it to the pad). Any
//!   other record is left as it is: a launcher that never showed the
//!   wheel never changes `OPTIONS`.
//! - **After the game** ([`keep`]): the virtual wheel's record is stashed
//!   again, with what the player changed in the game.
//!
//! There is no `OPTIONS` until the game first saves; then nothing is
//! seeded until the next start.

use std::io;
use std::path::Path;

use rea_launcher::wheel::dinput::VIRTUAL;
use rea_launcher::{fsx, paths};

/// The game's settings file in the saves folder.
pub const OPTIONS: &str = "OPTIONS";

/// The launcher's stash of records, in its data folder.
pub const STASH: &str = "wheel-records";

/// `OPTIONS`' size; the game refuses any other.
pub const SIZE: usize = 0xe_34e8;
/// One controller record.
pub const RECORD: usize = 0x261c;
const FIRST_RECORD: usize = 0x60;
/// Player 1's slot (u32); players 2–4 follow.
const SELECTED: usize = 0xa_b888;
/// The trailer the launcher never writes: profile id, version, flags.
pub const TRAILER: usize = 0xe_34e0;

/// The virtual wheel's slot: the first controller.
pub const WHEEL_SLOT: u32 = 2;

/// The ids a record keeps (`+0x74` product, `+0x78` vendor), as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ids {
    pub pid: u32,
    pub vid: u32,
}

/// The virtual wheel's ids.
pub const VIRTUAL_IDS: Ids = Ids {
    pid: VIRTUAL.pid as u32,
    vid: VIRTUAL.vid as u32,
};

// Record offsets.
const FLAGS: usize = 0x00;
const MODES: usize = 0x01;
const PID: usize = 0x74;
const VID: usize = 0x78;
const CALIBRATION: usize = 0x7c;
const AUTOCENTRE: usize = 0x253c;

/// The virtual wheel's controls in the game's numbering (32 buttons, the
/// hat's two objects, then the axes in `dinput::Axis` order).
const X: i32 = 34;
const Y: i32 = 35;
const Z: i32 = 36;
const RZ: i32 = 37;

/// The seed's bindings, `(offset, control)`: the game's own wheel
/// defaults (`0x46ebd0`) with steering on X and the two pedals on Z and
/// Rz instead of the combined Y.
const BINDINGS: [(usize, i32); 30] = [
    (0x04, Y),
    (0x08, Y),
    (0x0c, -1),
    (0x10, -1),
    (0x14, X),
    (0x18, X),
    (0x1c, Y),
    (0x20, Y),
    (0x24, X), // steering
    (0x28, X),
    (0x2c, Y),
    (0x30, Y),
    (0x34, 0), // buttons 0–9
    (0x38, 1),
    (0x3c, 2),
    (0x40, 3),
    (0x44, 4),
    (0x48, 5),
    (0x4c, 6),
    (0x50, 7),
    (0x54, 8),
    (0x58, 9),
    (0x5c, Z),  // accelerate
    (0x60, RZ), // brake
    (0x64, -1),
    (0x68, -1),
    (0x6c, -1),
    (0x70, -1),
    (PID, VIRTUAL.pid as i32),
    (VID, VIRTUAL.vid as i32),
];

/// Calibration tables `(entry, 8 values)`: steering (7) over the whole
/// axis with no deadzone, accelerate (9) and brake (10) from the
/// resting minimum to full. Raw 0/127/128/255 steer -32767/-128/129/32767.
const TABLES: [(usize, [i32; 8]); 3] = [
    (7, [0x7fff, 0, 0, -0x7fff, 0x7fff, 0, 100, 0]),
    (9, [0, 0, 0, 0, 0xffff, 0, 100, 0]),
    (10, [0, 0, 0, 0, 0xffff, 0, 100, 0]),
];

/// Byte 0: force feedback on (bit 1) at strength 4 of 0–4, full (bits
/// 2–4); the layer applies the player's strength.
const FFB_FULL: u8 = 0x02 | (4 << 2);

fn record_at(slot: u32) -> usize {
    FIRST_RECORD + slot as usize * RECORD
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// A record's ids.
pub fn ids(record: &[u8]) -> Ids {
    Ids {
        pid: u32_at(record, PID),
        vid: u32_at(record, VID),
    }
}

/// Makes `record` the virtual wheel's: its bindings, analogue steering
/// and pedals, not inverted, force feedback on at full, autocentre off,
/// and the three calibration tables. Everything else stays.
pub fn seed(record: &mut [u8]) {
    for (at, control) in BINDINGS {
        record[at..at + 4].copy_from_slice(&control.to_le_bytes());
    }
    record[FLAGS] = FFB_FULL;
    record[MODES] = 0;
    for (entry, values) in TABLES {
        let at = CALIBRATION + entry * 0x20;
        for (i, value) in values.into_iter().enumerate() {
            record[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    put_u32(record, AUTOCENTRE, 1);
}

/// The record the virtual wheel's displaced from slot 2, and player 1's
/// slot before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Displaced {
    pub selected: u32,
    pub record: Vec<u8>,
}

/// The records the launcher keeps aside: the virtual wheel's, and the one
/// it displaced (a pad's, keyed by its ids inside the record).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stash {
    pub wheel: Option<Vec<u8>>,
    pub displaced: Option<Displaced>,
}

const MAGIC: &[u8; 8] = b"CMR5WHL1";

impl Stash {
    /// The stash file's bytes; anything unreadable is an empty stash.
    pub fn parse(bytes: &[u8]) -> Stash {
        Stash::read(bytes).unwrap_or_default()
    }

    fn read(bytes: &[u8]) -> Option<Stash> {
        let mut at = 0;
        let mut take = |n: usize| {
            let part = bytes.get(at..at + n)?;
            at += n;
            Some(part)
        };
        if take(8)? != MAGIC {
            return None;
        }
        let wheel = match take(1)?[0] {
            1 => Some(take(RECORD)?.to_vec()),
            _ => None,
        };
        let displaced = match take(1)?[0] {
            1 => Some(Displaced {
                selected: u32_at(take(4)?, 0),
                record: take(RECORD)?.to_vec(),
            }),
            _ => None,
        };
        (at == bytes.len()).then_some(Stash { wheel, displaced })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        match &self.wheel {
            Some(record) => {
                out.push(1);
                out.extend(record);
            }
            None => out.push(0),
        }
        match &self.displaced {
            Some(d) => {
                out.push(1);
                out.extend(d.selected.to_le_bytes());
                out.extend(&d.record);
            }
            None => out.push(0),
        }
        out
    }
}

/// What a start did to `OPTIONS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// No `OPTIONS` yet, or not the game's size: nothing to change.
    NoOptions,
    Untouched,
    /// The virtual wheel's record seeded in slot 2.
    Seeded,
    /// The virtual wheel's stashed record put back in slot 2.
    Restored,
    /// The record the virtual wheel had displaced put back (no wheel).
    PutBack(Ids),
}

impl Change {
    /// For `launcher.log`.
    pub fn describe(self) -> String {
        match self {
            Change::NoOptions => {
                "no OPTIONS yet (the game writes it when it first saves); controls not seeded"
                    .to_owned()
            }
            Change::Untouched => "OPTIONS controls unchanged".to_owned(),
            Change::Seeded => "OPTIONS: the virtual wheel seeded in slot 2 for player 1".to_owned(),
            Change::Restored => {
                "OPTIONS: the virtual wheel's record (with your in-game changes) back in slot 2"
                    .to_owned()
            }
            Change::PutBack(ids) => format!(
                "OPTIONS: no wheel this time; the controller record {:04x}:{:04x} back in slot 2",
                ids.vid, ids.pid
            ),
        }
    }
}

/// Changes `options` (the whole file) for a start with the virtual wheel
/// shown or not, keeping aside in `stash` what it replaces.
pub fn before_run(options: &mut [u8], stash: &mut Stash, wheel: bool) -> Change {
    if options.len() != SIZE {
        return Change::NoOptions;
    }
    let slot = record_at(WHEEL_SLOT)..record_at(WHEEL_SLOT) + RECORD;
    let ours = ids(&options[slot.clone()]) == VIRTUAL_IDS;
    if wheel {
        if ours && stash.wheel.is_some() {
            return Change::Untouched;
        }
        stash.displaced = Some(Displaced {
            selected: u32_at(options, SELECTED),
            record: options[slot.clone()].to_vec(),
        });
        let change = match &stash.wheel {
            Some(record) => {
                options[slot].copy_from_slice(record);
                Change::Restored
            }
            None => {
                seed(&mut options[slot.clone()]);
                stash.wheel = Some(options[slot].to_vec());
                Change::Seeded
            }
        };
        put_u32(options, SELECTED, WHEEL_SLOT);
        return change;
    }
    if !ours {
        return Change::Untouched;
    }
    let Some(displaced) = stash.displaced.take() else {
        return Change::Untouched;
    };
    stash.wheel = Some(options[slot.clone()].to_vec());
    options[slot].copy_from_slice(&displaced.record);
    if u32_at(options, SELECTED) == WHEEL_SLOT {
        put_u32(options, SELECTED, displaced.selected);
    }
    Change::PutBack(ids(&displaced.record))
}

/// After the game: stashes the virtual wheel's record as the game saved
/// it. `true` when there was one.
pub fn after_run(options: &[u8], stash: &mut Stash) -> bool {
    if options.len() != SIZE || stash.wheel.is_none() {
        return false;
    }
    let record = &options[record_at(WHEEL_SLOT)..record_at(WHEEL_SLOT) + RECORD];
    if ids(record) != VIRTUAL_IDS {
        return false;
    }
    stash.wheel = Some(record.to_vec());
    true
}

fn read_stash(stash: &Path) -> io::Result<Stash> {
    match std::fs::read(stash) {
        Ok(bytes) => Ok(Stash::parse(&bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Stash::default()),
        Err(error) => Err(error),
    }
}

/// [`before_run`] on the `OPTIONS` in `saves`, with the stash file at
/// `stash`; writes both only when something changed.
pub fn prepare(saves: &Path, stash: &Path, wheel: bool) -> io::Result<Change> {
    let Some(file) = paths::find_ignoring_case(saves, OPTIONS).filter(|p| p.is_file()) else {
        return Ok(Change::NoOptions);
    };
    let mut options = std::fs::read(&file)?;
    let mut kept = read_stash(stash)?;
    let change = before_run(&mut options, &mut kept, wheel);
    if matches!(change, Change::NoOptions | Change::Untouched) {
        return Ok(change);
    }
    fsx::write_atomic(stash, kept.to_bytes())?;
    fsx::write_atomic(&file, options)?;
    Ok(change)
}

/// [`after_run`] on the files of [`prepare`].
pub fn keep(saves: &Path, stash: &Path) -> io::Result<bool> {
    let Some(file) = paths::find_ignoring_case(saves, OPTIONS).filter(|p| p.is_file()) else {
        return Ok(false);
    };
    let options = std::fs::read(file)?;
    let mut kept = read_stash(stash)?;
    if !after_run(&options, &mut kept) {
        return Ok(false);
    }
    fsx::write_atomic(stash, kept.to_bytes())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: Ids = Ids {
        pid: 0x028e,
        vid: 0x045e,
    };

    /// An `OPTIONS` with a pad in slot 2 (its own steering calibration,
    /// as the game writes for a pad), player 1 on slot 0, and a trailer.
    fn options() -> Vec<u8> {
        let mut bytes = vec![0u8; SIZE];
        for slot in 0..18 {
            let at = record_at(slot);
            put_u32(&mut bytes, at + PID, u32::MAX);
            put_u32(&mut bytes, at + VID, u32::MAX);
            bytes[at + MODES] = 2;
        }
        let at = record_at(WHEEL_SLOT);
        put_u32(&mut bytes, at + PID, PAD.pid);
        put_u32(&mut bytes, at + VID, PAD.vid);
        bytes[at + MODES] = 0;
        put_u32(&mut bytes, at + 0x24, 34);
        put_u32(&mut bytes, at + CALIBRATION + 7 * 0x20 + 8, 8000);
        bytes[TRAILER..].copy_from_slice(&[0x39, 0x30, 0, 0, 0x1b, 1, 0, 0]);
        bytes
    }

    fn record(options: &[u8]) -> &[u8] {
        &options[record_at(WHEEL_SLOT)..record_at(WHEEL_SLOT) + RECORD]
    }

    fn word(options: &[u8], at: usize) -> i32 {
        u32_at(options, record_at(WHEEL_SLOT) + at) as i32
    }

    /// The bytes that differ between two files.
    fn changed(a: &[u8], b: &[u8]) -> Vec<usize> {
        (0..a.len()).filter(|&i| a[i] != b[i]).collect()
    }

    #[test]
    fn the_seed_binds_steering_and_both_pedals_of_the_virtual_wheel() {
        let mut bytes = options();
        let mut stash = Stash::default();
        assert_eq!(before_run(&mut bytes, &mut stash, true), Change::Seeded);
        assert_eq!(ids(record(&bytes)), VIRTUAL_IDS);
        assert_eq!((word(&bytes, 0x74), word(&bytes, 0x78)), (0x5245, 0x1209));
        assert_eq!((word(&bytes, 0x24), word(&bytes, 0x28)), (34, 34), "X");
        assert_eq!(word(&bytes, 0x5c), 36, "accelerate on Z");
        assert_eq!(word(&bytes, 0x60), 37, "brake on Rz");
        assert_eq!(word(&bytes, 0x34), 0);
        assert_eq!(word(&bytes, 0x58), 9);
        assert_eq!(word(&bytes, 0x0c), -1);
        assert_eq!(record(&bytes)[0], 0x12, "force feedback on, full");
        assert_eq!(record(&bytes)[1], 0, "analogue, not inverted");
        assert_eq!(word(&bytes, 0x253c), 1, "autocentre off");
        let steer = CALIBRATION + 7 * 0x20;
        assert_eq!(word(&bytes, steer), 0x7fff);
        assert_eq!(word(&bytes, steer + 8), 0, "no deadzone");
        assert_eq!(word(&bytes, steer + 12), -0x7fff);
        assert_eq!(word(&bytes, CALIBRATION + 10 * 0x20 + 16), 0xffff);
        assert_eq!(u32_at(&bytes, SELECTED), WHEEL_SLOT, "player 1 drives it");
    }

    /// The research's own seed (`seed.py`) changes 48 bytes of a real
    /// file; here the bytes stay inside slot 2 and player 1's slot, and
    /// the trailer is never written.
    #[test]
    fn seeding_changes_only_slot_2_and_the_selection() {
        let before = options();
        let mut after = before.clone();
        before_run(&mut after, &mut Stash::default(), true);
        let slot = record_at(WHEEL_SLOT)..record_at(WHEEL_SLOT) + RECORD;
        for at in changed(&before, &after) {
            assert!(
                slot.contains(&at) || (SELECTED..SELECTED + 4).contains(&at),
                "0x{at:x}"
            );
        }
        assert_eq!(after[TRAILER..], before[TRAILER..]);
    }

    #[test]
    fn a_launch_without_the_wheel_leaves_options_as_it_is() {
        let before = options();
        let mut after = before.clone();
        let mut stash = Stash::default();
        assert_eq!(before_run(&mut after, &mut stash, false), Change::Untouched);
        assert_eq!(after, before);
        assert_eq!(stash, Stash::default());
    }

    #[test]
    fn a_file_that_is_not_the_games_is_not_touched() {
        let mut short = vec![0u8; 100];
        assert_eq!(
            before_run(&mut short, &mut Stash::default(), true),
            Change::NoOptions
        );
        assert_eq!(short, vec![0u8; 100]);
    }

    /// The game kept the virtual wheel's record and the player changed it
    /// in the game: the next wheel start leaves it alone.
    #[test]
    fn the_players_changes_to_the_wheel_stay() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, true);
        let at = record_at(WHEEL_SLOT);
        bytes[at + MODES] = 0b100; // inverted pedals, in the game's menu
        put_u32(&mut bytes, SELECTED, 0);
        assert!(after_run(&bytes, &mut stash));
        let saved = bytes.clone();
        assert_eq!(before_run(&mut bytes, &mut stash, true), Change::Untouched);
        assert_eq!(bytes, saved);
    }

    /// Wheel, then pad: the pad's record comes back as it was, and the
    /// wheel's calibration doesn't reach the pad (the game's reset would
    /// keep it). Then the wheel again: its record comes back with the
    /// player's changes.
    #[test]
    fn wheel_and_pad_starts_take_turns_in_slot_2() {
        let original = options();
        let mut bytes = original.clone();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, true);
        bytes[record_at(WHEEL_SLOT) + MODES] = 0b100;
        after_run(&bytes, &mut stash);
        let wheel = record(&bytes).to_vec();

        assert_eq!(
            before_run(&mut bytes, &mut stash, false),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, original, "the pad's record and player 1's slot");
        assert!(!after_run(&bytes, &mut stash), "the pad is not stashed");
        assert_eq!(before_run(&mut bytes, &mut stash, false), Change::Untouched);

        assert_eq!(before_run(&mut bytes, &mut stash, true), Change::Restored);
        assert_eq!(record(&bytes), wheel);
        assert_eq!(u32_at(&bytes, SELECTED), WHEEL_SLOT);
    }

    /// The game wrote the virtual wheel's record itself (its defaults,
    /// the combined pedal axis) on a start before `OPTIONS` existed: the
    /// launcher seeds it once.
    #[test]
    fn a_wheel_record_the_game_made_is_seeded_once() {
        let mut bytes = options();
        let at = record_at(WHEEL_SLOT);
        put_u32(&mut bytes, at + PID, VIRTUAL_IDS.pid);
        put_u32(&mut bytes, at + VID, VIRTUAL_IDS.vid);
        put_u32(&mut bytes, at + 0x5c, 35);
        let mut stash = Stash::default();
        assert_eq!(before_run(&mut bytes, &mut stash, true), Change::Seeded);
        assert_eq!(word(&bytes, 0x5c), 36);
        assert_eq!(before_run(&mut bytes, &mut stash, true), Change::Untouched);
        let game_made = stash.displaced.clone().unwrap().record;
        assert_eq!(ids(&game_made), VIRTUAL_IDS);
        // Without the wheel the game's own record comes back, so its reset
        // to the pad keeps none of the seed's calibration.
        before_run(&mut bytes, &mut stash, false);
        assert_eq!(record(&bytes), game_made);
    }

    /// The player picked another controller in the game while the wheel
    /// was in slot 2: putting the pad back leaves that choice.
    #[test]
    fn a_slot_chosen_in_the_game_stays() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, true);
        put_u32(&mut bytes, SELECTED, 1);
        before_run(&mut bytes, &mut stash, false);
        assert_eq!(u32_at(&bytes, SELECTED), 1);
    }

    #[test]
    fn the_stash_survives_its_file_and_a_bad_file_is_empty() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, true);
        assert!(stash.wheel.is_some() && stash.displaced.is_some());
        assert_eq!(Stash::parse(&stash.to_bytes()), stash);
        let empty = Stash::default();
        assert_eq!(Stash::parse(&empty.to_bytes()), empty);
        assert_eq!(Stash::parse(b"nonsense"), Stash::default());
        let cut = &stash.to_bytes()[..100];
        assert_eq!(Stash::parse(cut), Stash::default());
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmr2005-controls-{}-{name}-José's",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("saves")).unwrap();
        dir
    }

    #[test]
    fn the_files_change_only_when_something_does() {
        let root = temp("files");
        let (saves, stash) = (root.join("saves"), root.join(STASH));
        assert_eq!(prepare(&saves, &stash, true).unwrap(), Change::NoOptions);
        assert!(!stash.exists());

        std::fs::write(saves.join("options"), options()).unwrap();
        assert_eq!(prepare(&saves, &stash, false).unwrap(), Change::Untouched);
        assert!(!stash.exists(), "a start without the wheel writes nothing");
        assert_eq!(std::fs::read(saves.join("options")).unwrap(), options());

        assert_eq!(prepare(&saves, &stash, true).unwrap(), Change::Seeded);
        assert!(stash.is_file());
        assert!(keep(&saves, &stash).unwrap());
        assert_eq!(
            prepare(&saves, &stash, false).unwrap(),
            Change::PutBack(PAD)
        );
        assert_eq!(std::fs::read(saves.join("options")).unwrap(), options());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A real `OPTIONS` the game wrote, when `CMR2005_OPTIONS` names one
    /// (only read; never committed): a wheel start changes at most the 48
    /// bytes the research's seed did (the same bytes as its output, when
    /// `CMR2005_OPTIONS_SEEDED` names it), and the pad start after it
    /// puts the file back byte for byte.
    #[test]
    fn a_real_options_file_round_trips() {
        let Some(path) = std::env::var_os("CMR2005_OPTIONS") else {
            eprintln!("skipped: CMR2005_OPTIONS is not set");
            return;
        };
        let original = std::fs::read(path).unwrap();
        assert_eq!(original.len(), SIZE);
        let mut bytes = original.clone();
        let mut stash = Stash::default();
        let change = before_run(&mut bytes, &mut stash, true);
        assert!(matches!(change, Change::Seeded), "{change:?}");
        assert!(changed(&original, &bytes).len() <= 48 + 4);
        assert_eq!(bytes[TRAILER..], original[TRAILER..]);
        if let Some(seeded) = std::env::var_os("CMR2005_OPTIONS_SEEDED") {
            assert!(std::fs::read(seeded).unwrap() == bytes, "seed.py's bytes");
        }
        after_run(&bytes, &mut stash);
        before_run(&mut bytes, &mut stash, false);
        assert_eq!(bytes, original);
    }
}
