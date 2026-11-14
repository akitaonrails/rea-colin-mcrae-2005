//! The game's controller records in `SG\OPTIONS`: the virtual wheel's
//! (`rea_launcher::wheel::dinput::VIRTUAL`, which the in-game layer lists
//! first) and the Xidi pad's, in slot 2 of player 1.
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
//! Each start wants one of three things in slot 2 ([`Want`]): the virtual
//! wheel (the layer shows it), the Xidi pad (the `controller` option, no
//! wheel), or what the game itself put there (neither). The launcher
//! changes slot 2 of player 1 only, and only to swap these:
//!
//! - The record already there is left as it is when it is the one wanted
//!   (with the player's in-game changes). Otherwise it is put aside in the
//!   [`Stash`]: the wheel's and the pad's as the launcher's, any other as
//!   the record they displaced, with player 1's slot.
//! - The wanted one goes in: the wheel's or the pad's stashed record, or
//!   its seed the first time ([`seed`], [`pad_record`]); player 1 drives
//!   the wheel, and a pad plugged in when player 1 is on the keyboard.
//!   For neither, the displaced record comes back (the game's reset would
//!   keep the wheel's calibration and apply it to a pad, and the pad's
//!   bindings name Xidi's axes), and player 1's slot unless the player
//!   chose another in the game. A launcher that never shows the wheel or
//!   the Xidi pad never changes `OPTIONS`.
//! - **After the game** ([`keep`]): the wheel's or the pad's record is
//!   stashed again, with what the player changed in the game.
//!
//! There is no `OPTIONS` until the game first saves; then nothing is
//! seeded until the next start.

use std::io;
use std::ops::Range;
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

impl std::fmt::Display for Ids {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vid, self.pid)
    }
}

/// The virtual wheel's ids.
pub const VIRTUAL_IDS: Ids = Ids {
    pid: VIRTUAL.pid as u32,
    vid: VIRTUAL.vid as u32,
};

/// Xidi's first controller with no XInput pad plugged in: its
/// `guidProduct` is `{0001ffff-…}`, so the game stores FFFF:0001 (read
/// in the game's memory, RUNTIME_NOTES 2026-11-14).
pub const XIDI_UNPLUGGED: Ids = Ids {
    pid: 0x0001,
    vid: 0xffff,
};

/// What a start wants in slot 2 of player 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// Whatever the game itself puts there: the launcher's records out.
    Native,
    /// The virtual wheel: the wheel layer shows it.
    Wheel,
    /// The Xidi pad with the ids XInput reports for its pad, or Xidi's
    /// first controller with none plugged in (`None`).
    Pad(Option<Ids>),
}

impl std::fmt::Display for Want {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Want::Native => write!(f, "the game's own controller"),
            Want::Wheel => write!(f, "the virtual wheel"),
            Want::Pad(Some(ids)) => write!(f, "the Xidi pad {ids}"),
            Want::Pad(None) => write!(f, "Xidi's first controller (no pad plugged in)"),
        }
    }
}

impl Want {
    /// What the game is shown in slot 2 for this start: the wheel when
    /// `wheel`, else the Xidi pad with `gamepad` on (`pad`: its ids,
    /// asked only then), else the game's own.
    pub fn for_start(wheel: bool, gamepad: bool, pad: impl FnOnce() -> Option<Ids>) -> Want {
        match (wheel, gamepad) {
            (true, _) => Want::Wheel,
            (false, true) => Want::Pad(pad()),
            (false, false) => Want::Native,
        }
    }
}

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

/// The Xidi pad's controls in the game's numbering: Xidi's StandardGamepad
/// (`components::XIDI_CONF`) has 12 buttons, then its hat's two objects,
/// then the axes X, Y, Z (right stick), RotX (RT), RotY (LT), RotZ.
const PAD_X: i32 = 14;
const PAD_Y: i32 = 15;
const PAD_RT: i32 = 17;
const PAD_LT: i32 = 18;

/// The Xidi pad's bindings: the game's own defaults for a pad
/// (`0x46ebd0`: X steers, Y for its other axis actions, buttons 1–10 in
/// order) with accelerate on RT's axis and brake on LT's. The game reads
/// accelerate below the centre and brake above it (`XIDI_CONF`).
const PAD_BINDINGS: [(usize, i32); 22] = [
    (0x04, PAD_Y),
    (0x08, PAD_Y),
    (0x14, PAD_X),
    (0x18, PAD_X),
    (0x1c, PAD_Y),
    (0x20, PAD_Y),
    (0x24, PAD_X), // steering
    (0x28, PAD_X),
    (0x2c, PAD_Y),
    (0x30, PAD_Y),
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
    (0x5c, PAD_RT), // accelerate
    (0x60, PAD_LT), // brake
];

/// Byte 0 for the pad: vibration on (bit 1) at strength 3 of 0–4 (bits
/// 2–4; 80 %, `0x4afc77`), which the player changes in Options.
const PAD_VIBRATION: u8 = 0x02 | (3 << 2);

/// Player 1's slot that means the keyboard.
const KEYBOARD_SLOT: u32 = 0;

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

/// Player 1's record for the Xidi pad with `ids`: analogue steering and
/// pedals, not inverted, the [`PAD_BINDINGS`] (every other control
/// unbound), vibration on, and no calibration (the game's defaults), as
/// the game itself writes a pad's record into an empty slot.
pub fn pad_record(ids: Ids) -> Vec<u8> {
    let mut record = vec![0u8; RECORD];
    for at in (0x04..PID).step_by(4) {
        put_u32(&mut record, at, u32::MAX);
    }
    for (at, control) in PAD_BINDINGS {
        record[at..at + 4].copy_from_slice(&control.to_le_bytes());
    }
    record[FLAGS] = PAD_VIBRATION;
    put_u32(&mut record, PID, ids.pid);
    put_u32(&mut record, VID, ids.vid);
    record
}

/// The record the launcher's displaced from slot 2, and player 1's slot
/// before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Displaced {
    pub selected: u32,
    pub record: Vec<u8>,
}

/// The records the launcher keeps aside: the virtual wheel's, each Xidi
/// pad's (by the ids inside it), and the one they displaced (the game's
/// own).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stash {
    pub wheel: Option<Vec<u8>>,
    pub displaced: Option<Displaced>,
    pub pads: Vec<Vec<u8>>,
    /// Slot 2 holds a pad's record the launcher put there.
    pub pad_in_slot: bool,
}

/// Launcher 0.5.0's stash: the wheel's and the displaced record.
const MAGIC_V1: &[u8; 8] = b"CMR5WHL1";
/// Since the Xidi pad: those, then the pads' records (a count byte first)
/// and whether one is in slot 2.
const MAGIC: &[u8; 8] = b"CMR5WHL2";

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
        let v2 = match take(8)? {
            magic if magic == MAGIC => true,
            magic if magic == MAGIC_V1 => false,
            _ => return None,
        };
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
        let (pads, pad_in_slot) = if v2 {
            let count = take(1)?[0];
            let pads = (0..count)
                .map(|_| take(RECORD).map(<[u8]>::to_vec))
                .collect::<Option<Vec<_>>>()?;
            (pads, take(1)?[0] == 1)
        } else {
            (Vec::new(), false)
        };
        (at == bytes.len()).then_some(Stash {
            wheel,
            displaced,
            pads,
            pad_in_slot,
        })
    }

    /// The stashed record of the pad with `wanted` ids.
    fn pad(&self, wanted: Ids) -> Option<&[u8]> {
        self.pads
            .iter()
            .map(Vec::as_slice)
            .find(|record| ids(record) == wanted)
    }

    /// Keeps `record` as its pad's, in place of an older one.
    fn keep_pad(&mut self, record: Vec<u8>) {
        let wanted = ids(&record);
        self.pads.retain(|pad| ids(pad) != wanted);
        self.pads.push(record);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        push_record(&mut out, self.wheel.as_deref());
        match &self.displaced {
            Some(d) => {
                out.push(1);
                out.extend(d.selected.to_le_bytes());
                out.extend(&d.record);
            }
            None => out.push(0),
        }
        // One count byte: past 255 pads the oldest records are dropped.
        let pads = &self.pads[self.pads.len().saturating_sub(255)..];
        out.push(pads.len() as u8);
        for pad in pads {
            out.extend(pad);
        }
        out.push(u8::from(self.pad_in_slot));
        out
    }
}

/// A stashed record: 1 and its bytes, or 0 for none.
fn push_record(out: &mut Vec<u8>, record: Option<&[u8]>) {
    match record {
        Some(record) => {
            out.push(1);
            out.extend(record);
        }
        None => out.push(0),
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
    /// The Xidi pad's record seeded in slot 2 (and player 1 on it when
    /// `true`).
    PadSeeded(Ids, bool),
    /// The Xidi pad's stashed record put back in slot 2.
    PadRestored(Ids),
    /// The record the launcher's had displaced put back.
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
            Change::PadSeeded(ids, driven) => format!(
                "OPTIONS: the Xidi pad {ids} seeded in slot 2 (triggers as pedals, vibration on){}",
                if driven {
                    "; player 1 drives with it"
                } else {
                    ""
                }
            ),
            Change::PadRestored(ids) => format!(
                "OPTIONS: the Xidi pad {ids}'s record (with your in-game changes) back in slot 2"
            ),
            Change::PutBack(ids) => {
                format!("OPTIONS: the game's own controller record {ids} back in slot 2")
            }
        }
    }
}

/// Player 1's slot 2 in `OPTIONS`.
fn wheel_slot() -> Range<usize> {
    record_at(WHEEL_SLOT)..record_at(WHEEL_SLOT) + RECORD
}

/// Whose record slot 2 holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Occupant {
    /// The virtual wheel's, as the launcher put it there.
    Wheel,
    /// The Xidi pad's, as the launcher put it there.
    Pad(Ids),
    /// The launcher put the pad's there, and the game has since reset it
    /// for another device: made from ours, so nobody's to keep.
    Replaced,
    /// The game's own (a controller the player set up, or none).
    Native,
}

fn occupant(record: &[u8], stash: &Stash) -> Occupant {
    let here = ids(record);
    if here == VIRTUAL_IDS && stash.wheel.is_some() {
        Occupant::Wheel
    } else if stash.pad_in_slot {
        match stash.pad(here) {
            Some(_) => Occupant::Pad(here),
            None => Occupant::Replaced,
        }
    } else {
        Occupant::Native
    }
}

/// Changes `options` (the whole file) for a start that wants `want` in
/// slot 2, keeping aside in `stash` what it replaces.
pub fn before_run(options: &mut [u8], stash: &mut Stash, want: Want) -> Change {
    if options.len() != SIZE {
        return Change::NoOptions;
    }
    let here = occupant(&options[wheel_slot()], stash);
    let wanted = match (here, want) {
        (Occupant::Wheel, Want::Wheel) | (Occupant::Native, Want::Native) => true,
        (Occupant::Pad(ids), Want::Pad(pad)) => ids == pad.unwrap_or(XIDI_UNPLUGGED),
        _ => false,
    };
    if wanted || (want == Want::Native && stash.displaced.is_none()) {
        return Change::Untouched;
    }
    take_out(options, stash, here);
    match want {
        Want::Wheel => show_wheel(options, stash),
        Want::Pad(pad) => show_pad(options, stash, pad),
        Want::Native => put_back(options, stash),
    }
}

/// Slot 2's record aside: the launcher's into its place in the stash, the
/// game's own as the displaced one with player 1's slot.
fn take_out(options: &[u8], stash: &mut Stash, here: Occupant) {
    let record = options[wheel_slot()].to_vec();
    match here {
        Occupant::Wheel => stash.wheel = Some(record),
        Occupant::Pad(_) => stash.keep_pad(record),
        Occupant::Native => {
            stash.displaced = Some(Displaced {
                selected: u32_at(options, SELECTED),
                record,
            });
        }
        Occupant::Replaced => {}
    }
    stash.pad_in_slot = false;
}

/// The virtual wheel's record in (stashed, else seeded), player 1 on it.
fn show_wheel(options: &mut [u8], stash: &mut Stash) -> Change {
    let change = match &stash.wheel {
        Some(record) => {
            options[wheel_slot()].copy_from_slice(record);
            Change::Restored
        }
        None => {
            seed(&mut options[wheel_slot()]);
            stash.wheel = Some(options[wheel_slot()].to_vec());
            Change::Seeded
        }
    };
    put_u32(options, SELECTED, WHEEL_SLOT);
    change
}

/// The Xidi pad's record in: its stashed one when it is the same pad,
/// else [`pad_record`], with player 1 moved from the keyboard to a pad
/// that is plugged in.
fn show_pad(options: &mut [u8], stash: &mut Stash, pad: Option<Ids>) -> Change {
    let wanted = pad.unwrap_or(XIDI_UNPLUGGED);
    stash.pad_in_slot = true;
    if let Some(record) = stash.pad(wanted) {
        options[wheel_slot()].copy_from_slice(record);
        return Change::PadRestored(wanted);
    }
    let record = pad_record(wanted);
    options[wheel_slot()].copy_from_slice(&record);
    stash.keep_pad(record);
    let drive = pad.is_some() && u32_at(options, SELECTED) == KEYBOARD_SLOT;
    if drive {
        put_u32(options, SELECTED, WHEEL_SLOT);
    }
    Change::PadSeeded(wanted, drive)
}

/// The displaced record back, and player 1's slot unless the player
/// chose another in the game.
fn put_back(options: &mut [u8], stash: &mut Stash) -> Change {
    let Some(displaced) = stash.displaced.take() else {
        return Change::Untouched;
    };
    options[wheel_slot()].copy_from_slice(&displaced.record);
    if u32_at(options, SELECTED) == WHEEL_SLOT {
        put_u32(options, SELECTED, displaced.selected);
    }
    Change::PutBack(ids(&displaced.record))
}

/// The launcher's record [`after_run`] stashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Wheel,
    Pad(Ids),
}

/// After the game: stashes the wheel's or the pad's record as the game
/// saved it, when slot 2 holds one of them.
pub fn after_run(options: &[u8], stash: &mut Stash) -> Option<Kept> {
    if options.len() != SIZE {
        return None;
    }
    let record = options[wheel_slot()].to_vec();
    match occupant(&record, stash) {
        Occupant::Wheel => {
            stash.wheel = Some(record);
            Some(Kept::Wheel)
        }
        Occupant::Pad(ids) => {
            stash.keep_pad(record);
            Some(Kept::Pad(ids))
        }
        Occupant::Replaced | Occupant::Native => None,
    }
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
pub fn prepare(saves: &Path, stash: &Path, want: Want) -> io::Result<Change> {
    let Some(file) = paths::find_ignoring_case(saves, OPTIONS).filter(|p| p.is_file()) else {
        return Ok(Change::NoOptions);
    };
    let mut options = std::fs::read(&file)?;
    let mut kept = read_stash(stash)?;
    let change = before_run(&mut options, &mut kept, want);
    if matches!(change, Change::NoOptions | Change::Untouched) {
        return Ok(change);
    }
    fsx::write_atomic(stash, kept.to_bytes())?;
    fsx::write_atomic(&file, options)?;
    Ok(change)
}

/// [`after_run`] on the files of [`prepare`].
pub fn keep(saves: &Path, stash: &Path) -> io::Result<Option<Kept>> {
    let Some(file) = paths::find_ignoring_case(saves, OPTIONS).filter(|p| p.is_file()) else {
        return Ok(None);
    };
    let options = std::fs::read(file)?;
    let mut stashed = read_stash(stash)?;
    let kept = after_run(&options, &mut stashed);
    if kept.is_some() {
        fsx::write_atomic(stash, stashed.to_bytes())?;
    }
    Ok(kept)
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
        &options[wheel_slot()]
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
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Seeded
        );
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
        before_run(&mut after, &mut Stash::default(), Want::Wheel);
        let slot = wheel_slot();
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
        assert_eq!(
            before_run(&mut after, &mut stash, Want::Native),
            Change::Untouched
        );
        assert_eq!(after, before);
        assert_eq!(stash, Stash::default());
    }

    #[test]
    fn a_file_that_is_not_the_games_is_not_touched() {
        let mut short = vec![0u8; 100];
        assert_eq!(
            before_run(&mut short, &mut Stash::default(), Want::Wheel),
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
        before_run(&mut bytes, &mut stash, Want::Wheel);
        let at = record_at(WHEEL_SLOT);
        bytes[at + MODES] = 0b100; // inverted pedals, in the game's menu
        put_u32(&mut bytes, SELECTED, 0);
        assert_eq!(after_run(&bytes, &mut stash), Some(Kept::Wheel));
        let saved = bytes.clone();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Untouched
        );
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
        before_run(&mut bytes, &mut stash, Want::Wheel);
        bytes[record_at(WHEEL_SLOT) + MODES] = 0b100;
        after_run(&bytes, &mut stash);
        let wheel = record(&bytes).to_vec();

        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, original, "the pad's record and player 1's slot");
        assert_eq!(
            after_run(&bytes, &mut stash),
            None,
            "the pad is not stashed"
        );
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::Untouched
        );

        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Restored
        );
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
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Seeded
        );
        assert_eq!(word(&bytes, 0x5c), 36);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Untouched
        );
        let game_made = stash.displaced.clone().unwrap().record;
        assert_eq!(ids(&game_made), VIRTUAL_IDS);
        // Without the wheel the game's own record comes back, so its reset
        // to the pad keeps none of the seed's calibration.
        before_run(&mut bytes, &mut stash, Want::Native);
        assert_eq!(record(&bytes), game_made);
    }

    /// The player picked another controller in the game while the wheel
    /// was in slot 2: putting the pad back leaves that choice.
    #[test]
    fn a_slot_chosen_in_the_game_stays() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Wheel);
        put_u32(&mut bytes, SELECTED, 1);
        before_run(&mut bytes, &mut stash, Want::Native);
        assert_eq!(u32_at(&bytes, SELECTED), 1);
    }

    #[test]
    fn the_stash_survives_its_file_and_a_bad_file_is_empty() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Wheel);
        assert!(stash.wheel.is_some() && stash.displaced.is_some());
        assert_eq!(Stash::parse(&stash.to_bytes()), stash);
        before_run(&mut bytes, &mut stash, Want::Pad(None));
        assert!(stash.pads.len() == 1 && stash.pad_in_slot);
        assert_eq!(Stash::parse(&stash.to_bytes()), stash);
        let empty = Stash::default();
        assert_eq!(Stash::parse(&empty.to_bytes()), empty);
        assert_eq!(Stash::parse(b"nonsense"), Stash::default());
        let cut = &stash.to_bytes()[..100];
        assert_eq!(Stash::parse(cut), Stash::default());
    }

    /// The pad's record as the launcher seeds it: the game's own pad
    /// defaults with accelerate on RT's axis and brake on LT's, analogue,
    /// vibration on at 80 %, its ids, no calibration.
    #[test]
    fn the_pad_record_binds_the_stick_and_the_triggers_with_vibration_on() {
        let record = pad_record(PAD);
        let word = |at: usize| u32_at(&record, at) as i32;
        assert_eq!(record.len(), RECORD);
        assert_eq!(ids(&record), PAD);
        assert_eq!((word(0x24), word(0x28)), (14, 14), "steering on X");
        assert_eq!(
            (word(0x5c), word(0x60)),
            (17, 18),
            "RT accelerates, LT brakes"
        );
        assert_eq!((word(0x34), word(0x58)), (0, 9), "buttons 1-10 in order");
        assert_eq!((word(0x04), word(0x30)), (15, 15), "Y as the game binds it");
        for at in [0x0c, 0x10, 0x64, 0x68, 0x6c, 0x70] {
            assert_eq!(word(at), -1, "0x{at:x} unbound");
        }
        assert_eq!(record[0], 0x0e, "vibration on, strength 3 of 4");
        assert_eq!(record[1], 0, "analogue steering and pedals, not inverted");
        assert!(
            record[CALIBRATION..].iter().all(|&b| b == 0),
            "no calibration"
        );
    }

    /// The Xidi pad's ids are the pad's own (Xidi 5 passes them on), so
    /// the record the game kept for the pad read without Xidi is
    /// displaced, and player 1 moves from the keyboard to the pad.
    #[test]
    fn a_pad_start_seeds_the_xidi_pad_and_player_1_drives_it() {
        let before = options();
        let mut bytes = before.clone();
        let mut stash = Stash::default();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::PadSeeded(PAD, true)
        );
        assert_eq!(record(&bytes), pad_record(PAD));
        assert_eq!(u32_at(&bytes, SELECTED), WHEEL_SLOT);
        assert_eq!(stash.displaced.as_ref().unwrap().record, record(&before));
        for at in changed(&before, &bytes) {
            assert!(wheel_slot().contains(&at) || (SELECTED..SELECTED + 4).contains(&at));
        }
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::Untouched
        );
    }

    /// With no pad plugged in Xidi still lists its first controller
    /// (FFFF:0001): its record goes in, so the game never resets the
    /// player's own record, but player 1 stays on the keyboard.
    #[test]
    fn without_a_pad_plugged_in_the_keyboard_stays_player_1s() {
        let mut bytes = options();
        let mut stash = Stash::default();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(None)),
            Change::PadSeeded(XIDI_UNPLUGGED, false)
        );
        assert_eq!(ids(record(&bytes)), XIDI_UNPLUGGED);
        assert_eq!(u32_at(&bytes, SELECTED), 0);
    }

    /// The player's own changes to the pad stay from start to start, and
    /// the gamepad turned off gives the file back byte for byte.
    #[test]
    fn the_pads_changes_stay_and_gamepad_off_puts_the_file_back() {
        let original = options();
        let mut bytes = original.clone();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD)));
        bytes[record_at(WHEEL_SLOT)] = 0x06; // vibration strength 1, in the game
        assert_eq!(after_run(&bytes, &mut stash), Some(Kept::Pad(PAD)));
        let tweaked = record(&bytes).to_vec();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::Untouched
        );
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, original);
        assert_eq!(after_run(&bytes, &mut stash), None);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::PadRestored(PAD)
        );
        assert_eq!(record(&bytes), tweaked);
    }

    /// The wheel takes slot 2 from the pad and gives it back: each keeps
    /// its own record, and the game's own comes back when both are gone.
    #[test]
    fn the_wheel_and_the_xidi_pad_take_turns_and_both_give_the_slot_back() {
        let original = options();
        let mut bytes = original.clone();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD)));
        let pad = record(&bytes).to_vec();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Seeded
        );
        assert_eq!(ids(record(&bytes)), VIRTUAL_IDS);
        assert_eq!(stash.pad(PAD), Some(&pad[..]));
        let wheel = record(&bytes).to_vec();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::PadRestored(PAD)
        );
        assert_eq!(record(&bytes), pad);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Wheel),
            Change::Restored
        );
        assert_eq!(record(&bytes), wheel);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, original);
    }

    /// Another pad, or the game resetting the launcher's pad record for
    /// another device: the pad gets the launcher's record again, and the
    /// game's own record the launcher displaced first is still the one
    /// that comes back.
    #[test]
    fn another_pad_is_seeded_and_the_games_own_record_is_kept() {
        let original = options();
        let mut bytes = original.clone();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD)));
        let other = Ids {
            pid: 0x6012,
            vid: 0x2dc8,
        };
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(other))),
            Change::PadSeeded(other, false)
        );
        // The game reset slot 2 for a device Xidi listed differently.
        let at = record_at(WHEEL_SLOT);
        put_u32(&mut bytes, at + PID, 0x1234);
        assert_eq!(after_run(&bytes, &mut stash), None);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(other))),
            Change::PadRestored(other)
        );
        assert_eq!(record(&bytes), pad_record(other));
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, original);
    }

    /// A start without the pad plugged in (Xidi's own controller in slot
    /// 2) keeps the pad's record for when it is back.
    #[test]
    fn each_pad_keeps_its_own_record() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD)));
        bytes[record_at(WHEEL_SLOT)] = 0x06;
        let tweaked = record(&bytes).to_vec();
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(None)),
            Change::PadSeeded(XIDI_UNPLUGGED, false)
        );
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Pad(Some(PAD))),
            Change::PadRestored(PAD)
        );
        assert_eq!(record(&bytes), tweaked);
        assert_eq!(stash.pads.len(), 2);
        assert_eq!(Stash::parse(&stash.to_bytes()), stash);
    }

    #[test]
    fn the_start_wants_the_wheel_then_the_pad_then_the_games_own() {
        let unasked = || -> Option<Ids> { panic!("asked for the pad") };
        assert_eq!(Want::for_start(true, true, unasked), Want::Wheel);
        assert_eq!(Want::for_start(false, false, unasked), Want::Native);
        assert_eq!(
            Want::for_start(false, true, || Some(PAD)),
            Want::Pad(Some(PAD))
        );
        assert_eq!(PAD.to_string(), "045e:028e");
        assert_eq!(Want::Pad(Some(PAD)).to_string(), "the Xidi pad 045e:028e");
    }

    /// Launcher 0.5.0's stash (no pad yet) still reads.
    #[test]
    fn a_stash_from_0_5_0_still_reads() {
        let mut bytes = options();
        let mut stash = Stash::default();
        before_run(&mut bytes, &mut stash, Want::Wheel);
        let mut old = MAGIC_V1.to_vec();
        push_record(&mut old, stash.wheel.as_deref());
        let displaced = stash.displaced.as_ref().unwrap();
        old.push(1);
        old.extend(displaced.selected.to_le_bytes());
        old.extend(&displaced.record);
        assert_eq!(Stash::parse(&old), stash);
        assert_eq!(
            before_run(&mut bytes, &mut stash, Want::Native),
            Change::PutBack(PAD)
        );
        assert_eq!(bytes, options());
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
        assert_eq!(
            prepare(&saves, &stash, Want::Wheel).unwrap(),
            Change::NoOptions
        );
        assert!(!stash.exists());

        std::fs::write(saves.join("options"), options()).unwrap();
        assert_eq!(
            prepare(&saves, &stash, Want::Native).unwrap(),
            Change::Untouched
        );
        assert!(!stash.exists(), "a start without the wheel writes nothing");
        assert_eq!(std::fs::read(saves.join("options")).unwrap(), options());

        assert_eq!(
            prepare(&saves, &stash, Want::Wheel).unwrap(),
            Change::Seeded
        );
        assert!(stash.is_file());
        assert_eq!(keep(&saves, &stash).unwrap(), Some(Kept::Wheel));
        assert_eq!(
            prepare(&saves, &stash, Want::Native).unwrap(),
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
        let change = before_run(&mut bytes, &mut stash, Want::Wheel);
        assert!(matches!(change, Change::Seeded), "{change:?}");
        assert!(changed(&original, &bytes).len() <= 48 + 4);
        assert_eq!(bytes[TRAILER..], original[TRAILER..]);
        if let Some(seeded) = std::env::var_os("CMR2005_OPTIONS_SEEDED") {
            assert!(std::fs::read(seeded).unwrap() == bytes, "seed.py's bytes");
        }
        after_run(&bytes, &mut stash);
        before_run(&mut bytes, &mut stash, Want::Native);
        assert_eq!(bytes, original);
    }
}
