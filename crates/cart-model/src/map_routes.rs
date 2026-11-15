//! The paths Mario walks on the map, as the game keeps them (read from its map code, bank $0F):
//!
//! * Ways out of each point: bank $18 $563A, 8 bytes per point ($00-$7F): four pointers,
//!   for right, left, up and down ($FFFF = no way). 0F:453F picks one with the pad.
//! * A way is a walk script (bank $18 $5A3A-$5EE5): one byte per step, ended by $00 and
//!   then the point Mario arrives at (0F:47EC writes it to $A840). A step's low nibble is
//!   where it goes (1 right, 2 left, 4 up, 8 down; up or down with right or left goes
//!   diagonally); it moves 8 pixels, 4 with bit 4 set; bits 5-7 pick Mario's animation.
//! * Where Mario stands on each point when the map is shown again (after a level): bank $18
//!   $5EE6, 8 bytes per point: camera x (high, low), camera y (high, low), Mario's x and y
//!   on the screen, two unused bytes (0F:6EA4).
//! * A point's level is in `world` (0F:4218); its "cleared" flag is $A848 + 0F:650E[point],
//!   and while it is clear the pad directions in 0F:6591[flag] are shut (0F:61D7).
//!
//! Bank $18 is full, so an edited copy keeps all this in bank $36 (the twin of $16, which
//! nothing reads): the code that reads it does so through two little stubs in bank 0 that
//! set the upper bank bit around the read (the same trick as the level decoder):
//!   0:39D5  di / push af / ld a,1 / ld ($4000),a / pop af / call $3E7B / jr $39ED
//!   0:39E2  di / push af / ld a,1 / ld ($4000),a / pop af / call $3E8F
//!   0:39ED  push af / xor a / ld ($4000),a / pop af / ret
//! and every `ld a,$18 / call $3E7B|$3E8F` of the map code becomes `ld a,$16 / call $39D5|$39E2`.
//! Bank $36: "RTE1", the number of new points and their numbers ($4004), the ways table
//! ($4010), the places table ($4410), the game's scripts as they are ($4810) and scripts for
//! changed ways after them.
//!
//! New points take numbers the game leaves unused (`FREE_POINTS`); each gets a "cleared"
//! flag no point uses (`FREE_FLAG`, at 0F:650E) so finishing its level marks nothing else.
use crate::error::CartError;
use crate::patch::{is_expanded, TWIN};
use crate::rom::{offset, set_word, word};

/// Points that have ways and places ($00-$7F).
pub const POINTS: usize = 0x80;
/// Ways out of a point, in this order.
pub const DIRS: [char; 4] = ['R', 'L', 'U', 'D'];
pub const RIGHT: u8 = 0x01;
pub const LEFT: u8 = 0x02;
pub const UP: u8 = 0x04;
pub const DOWN: u8 = 0x08;
/// A step of 4 pixels instead of 8.
pub const HALF: u8 = 0x10;

const OLD_BANK: usize = 0x18;
const OLD_WAYS: u16 = 0x563A;
const OLD_SCRIPTS: u16 = 0x5A3A;
const OLD_PLACES: u16 = 0x5EE6;

/// Where an edited copy keeps them.
pub const BANK: usize = 0x36;
const MAGIC: &[u8; 4] = b"RTE1";
const ADDED: u16 = 0x4004;
const FLAGS: u16 = 0x650E;
const NO_FLAG: u8 = 0xFE;
const NEW_WAYS: u16 = 0x4010;
const NEW_PLACES: u16 = 0x4410;
const NEW_SCRIPTS: u16 = 0x4810;
/// The game's own scripts, moved as they are.
const KEPT: usize = (OLD_PLACES - OLD_SCRIPTS) as usize;
/// Room for scripts of changed ways, after the game's.
pub const ROOM: usize = 0x8000 - NEW_SCRIPTS as usize - KEPT;

const STUB_AT: usize = 0x39D5;
const STUB: [u8; 31] = [
    0xF3, 0xF5, 0x3E, 0x01, 0xEA, 0x00, 0x40, 0xF1, 0xCD, 0x7B, 0x3E, 0x18, 0x0B, // read (hl)
    0xF3, 0xF5, 0x3E, 0x01, 0xEA, 0x00, 0x40, 0xF1, 0xCD, 0x8F, 0x3E, // read (bc)
    0xF5, 0xAF, 0xEA, 0x00, 0x40, 0xF1, 0xC9,
];
const READ_HL: u16 = 0x39D5;
const READ_BC: u16 = 0x39E2;
/// Map code reading bank $18 through 0:3E7B (hl) ...
const READS_HL: [u16; 8] = [0x456F, 0x4577, 0x6EBA, 0x6EC3, 0x6ECC, 0x6ED5, 0x6EDE, 0x6EE7];
/// ... and through 0:3E8F (bc).
const READS_BC: [u16; 3] = [0x458E, 0x4810, 0x481E];
/// `ld hl,$563A` (ways) and `ld hl,$5EE6` (places): their operands.
const WAYS_REF: u16 = 0x4543;
const PLACES_REF: u16 = 0x6EA8;
/// `ld bc,$5BF6`: a script the code walks by itself (points $0C and $1E).
const SCRIPT_REF: u16 = 0x4500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Way {
    /// Steps, as the game keeps them (see the module notes).
    pub steps: Vec<u8>,
    /// The point Mario arrives at.
    pub to: u8,
}

/// Where Mario stands on a point when the map is shown again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub cam_x: u16,
    pub cam_y: u16,
    pub x: u8,
    pub y: u8,
    pub rest: [u8; 2],
}

/// Mario is drawn this far right and below the camera when the game has room to centre it.
const MARIO_X: i32 = 0x4B;
const MARIO_Y: i32 = 0x4C;
/// Feet are 4 pixels above Mario's y.
const FEET: i32 = 4;

impl Place {
    fn from(b: &[u8]) -> Place {
        Place { cam_x: (b[0] as u16) << 8 | b[1] as u16, cam_y: (b[2] as u16) << 8 | b[3] as u16, x: b[4], y: b[5], rest: [b[6], b[7]] }
    }

    fn bytes(&self) -> [u8; 8] {
        [(self.cam_x >> 8) as u8, self.cam_x as u8, (self.cam_y >> 8) as u8, self.cam_y as u8, self.x, self.y, self.rest[0], self.rest[1]]
    }

    /// On the world map: the middle of Mario's feet on the 512 x 512 picture.
    pub fn on_world(&self) -> (i32, i32) {
        (self.cam_x as i32 + self.x as i32, self.cam_y as i32 + self.y as i32 - FEET)
    }

    /// Standing at `(x, y)` of the world picture, with the camera on him as far as the map
    /// lets it go.
    pub fn at_world(x: i32, y: i32) -> Place {
        let cx = (x - MARIO_X).clamp(0, 512 - 160);
        let cy = (y + FEET - MARIO_Y).clamp(0, 512 - 144);
        Place { cam_x: cx as u16, cam_y: cy as u16, x: (x - cx) as u8, y: (y + FEET - cy) as u8, rest: [0, 0] }
    }
}

impl Way {
    /// Where each step ends, from the start (pixels; x right, y down).
    pub fn track(&self) -> Vec<(i32, i32)> {
        let mut p = (0, 0);
        let mut v = vec![p];
        for &s in &self.steps {
            let n = if s & HALF != 0 { 4 } else { 8 };
            if s & RIGHT != 0 {
                p.0 += n;
            }
            if s & LEFT != 0 {
                p.0 -= n;
            }
            if s & UP != 0 {
                p.1 -= n;
            }
            if s & DOWN != 0 {
                p.1 += n;
            }
            v.push(p);
        }
        v
    }

    /// Steps that walk through `corners` (pixels, from the start), each leg straight or at
    /// 45 degrees; leftovers under 4 pixels are dropped. A leg that is neither is walked
    /// diagonally first, then straight.
    pub fn through(corners: &[(i32, i32)], to: u8) -> Way {
        let mut steps = Vec::new();
        let mut at = (0, 0);
        for &c in corners {
            let (dx, dy) = (c.0 - at.0, c.1 - at.1);
            let diag = dx.abs().min(dy.abs());
            let dir = |x: i32, y: i32| (if x > 0 { RIGHT } else if x < 0 { LEFT } else { 0 }) | (if y > 0 { DOWN } else if y < 0 { UP } else { 0 });
            let push = |steps: &mut Vec<u8>, d: u8, len: i32| {
                for _ in 0..len / 8 {
                    steps.push(d);
                }
                if len % 8 >= 4 {
                    steps.push(d | HALF);
                }
            };
            push(&mut steps, dir(dx, dy), diag);
            push(&mut steps, dir(dx - dx.signum() * diag, dy - dy.signum() * diag), dx.abs().max(dy.abs()) - diag);
            at = c;
        }
        steps.retain(|&s| s & 0x0F != 0);
        Way { steps, to }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut v: Vec<u8> = self.steps.iter().map(|&s| if s == 0 { RIGHT } else { s }).collect();
        v.push(0);
        v.push(self.to);
        v
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapRoutes {
    /// Per point, its ways right, left, up, down.
    pub ways: Vec<[Option<Way>; 4]>,
    pub places: Vec<Place>,
    /// New points (from `FREE_POINTS`), in the order they were made.
    pub added: Vec<u8>,
}

impl MapRoutes {
    /// A free number for a new point, if any is left.
    pub fn free_point(&self) -> Option<u8> {
        FREE_POINTS.iter().copied().find(|p| !self.added.contains(p))
    }

    /// Give each zone exit the ways of its entrance. True if any changed.
    pub fn sync_exits(&mut self) -> bool {
        let mut changed = false;
        for (a, b) in ZONE_EXITS {
            if self.ways.len() > b as usize && self.ways[b as usize] != self.ways[a as usize] {
                self.ways[b as usize] = self.ways[a as usize].clone();
                changed = true;
            }
        }
        changed
    }

    /// Take out a new point and every way to it.
    pub fn remove_point(&mut self, p: u8) {
        self.added.retain(|&a| a != p);
        self.ways[p as usize] = Default::default();
        for w in &mut self.ways {
            for d in w.iter_mut() {
                if d.as_ref().is_some_and(|x| x.to == p) {
                    *d = None;
                }
            }
        }
    }
}

/// Has this copy got the routes in bank $36?
pub fn moved(rom: &[u8]) -> bool {
    is_expanded(rom) && rom[offset(BANK, 0x4000)..offset(BANK, 0x4004)] == MAGIC[..] && rom[offset(0, STUB_AT as u16)..STUB_AT + STUB.len()] == STUB
}

fn script(rom: &[u8], bank: usize, at: u16) -> Way {
    let mut steps = Vec::new();
    let mut a = at;
    while a < 0x7FFF && rom[offset(bank, a)] != 0 {
        steps.push(rom[offset(bank, a)]);
        a += 1;
    }
    Way { steps, to: rom[offset(bank, a + 1)] }
}

pub fn read(rom: &[u8]) -> MapRoutes {
    let (bank, ways, places) = if moved(rom) { (BANK, NEW_WAYS, NEW_PLACES) } else { (OLD_BANK, OLD_WAYS, OLD_PLACES) };
    let mut r = MapRoutes { ways: Vec::with_capacity(POINTS), places: Vec::with_capacity(POINTS), added: Vec::new() };
    if bank == BANK {
        let n = (rom[offset(BANK, ADDED)] as usize).min(FREE_POINTS.len());
        r.added = rom[offset(BANK, ADDED + 1)..offset(BANK, ADDED + 1) + n].iter().copied().filter(|p| FREE_POINTS.contains(p)).collect();
    }
    for p in 0..POINTS {
        let mut w: [Option<Way>; 4] = Default::default();
        for (d, slot) in w.iter_mut().enumerate() {
            let at = word(rom, offset(bank, ways + (p * 8 + d * 2) as u16));
            if at != 0xFFFF {
                *slot = Some(script(rom, bank, at));
            }
        }
        r.ways.push(w);
        let o = offset(bank, places + (p * 8) as u16);
        r.places.push(Place::from(&rom[o..o + 8]));
    }
    r
}

/// The map code patch, in a half of the copy (`base` 0 or `TWIN`).
fn patch_code(rom: &mut [u8], base: usize) {
    rom[base + STUB_AT..base + STUB_AT + STUB.len()].copy_from_slice(&STUB);
    for (sites, to) in [(&READS_HL[..], READ_HL), (&READS_BC[..], READ_BC)] {
        for &s in sites {
            let o = base + offset(0x0F, s);
            rom[o + 1] = (BANK & 0x1F) as u8;
            set_word(rom, o + 3, to);
        }
    }
    set_word(rom, base + offset(0x0F, WAYS_REF), NEW_WAYS);
    set_word(rom, base + offset(0x0F, PLACES_REF), NEW_PLACES);
    let s = word(rom, offset(0x0F, SCRIPT_REF));
    let s = if (OLD_SCRIPTS..OLD_PLACES).contains(&s) { s - OLD_SCRIPTS + NEW_SCRIPTS } else { s };
    set_word(rom, base + offset(0x0F, SCRIPT_REF), s);
}

/// Can these routes go in this copy? It must be the 1 MB edited copy of the version the
/// patch was made for.
pub fn can_write(rom: &[u8]) -> bool {
    if !is_expanded(rom) {
        return false;
    }
    if moved(rom) {
        return true;
    }
    let read = |s: u16, call: u16| rom[offset(0x0F, s)..offset(0x0F, s) + 5] == [0x3E, 0x18, 0xCD, call as u8, (call >> 8) as u8];
    READS_HL.iter().all(|&s| read(s, 0x3E7B))
        && READS_BC.iter().all(|&s| read(s, 0x3E8F))
        && word(rom, offset(0x0F, WAYS_REF)) == OLD_WAYS
        && word(rom, offset(0x0F, PLACES_REF)) == OLD_PLACES
        && rom[STUB_AT..STUB_AT + STUB.len()].iter().all(|&b| b == 0xFF)
}

/// Write the routes into the edited copy (moving them to bank $36 the first time).
/// Returns the bytes the added scripts take.
pub fn write(rom: &mut [u8], r: &MapRoutes) -> Result<usize, CartError> {
    if !can_write(rom) {
        return Err(CartError::UnknownVersion);
    }
    // the game's scripts, as they are, and where each started
    let game: Vec<u8> = if moved(rom) {
        rom[offset(BANK, NEW_SCRIPTS)..offset(BANK, NEW_SCRIPTS) + KEPT].to_vec()
    } else {
        rom[offset(OLD_BANK, OLD_SCRIPTS)..offset(OLD_BANK, OLD_SCRIPTS) + KEPT].to_vec()
    };
    let mut added: Vec<u8> = Vec::new();
    let mut table = vec![0xFFu8; POINTS * 8];
    for (p, w) in r.ways.iter().enumerate().take(POINTS) {
        for (d, way) in w.iter().enumerate() {
            let Some(way) = way else { continue };
            let b = way.bytes();
            // the same bytes anywhere make the same walk
            let at = if let Some(s) = game.windows(b.len()).position(|x| x == &b[..]) {
                NEW_SCRIPTS as usize + s
            } else if let Some(s) = added.windows(b.len()).position(|x| x == &b[..]) {
                NEW_SCRIPTS as usize + KEPT + s
            } else {
                added.extend_from_slice(&b);
                NEW_SCRIPTS as usize + KEPT + added.len() - b.len()
            };
            table[p * 8 + d * 2] = at as u8;
            table[p * 8 + d * 2 + 1] = (at >> 8) as u8;
        }
    }
    if added.len() > ROOM {
        return Err(CartError::RoutesFull { need: added.len(), capacity: ROOM });
    }
    let bank = offset(BANK, 0x4000);
    rom[bank..bank + 0x4000].fill(0xFF);
    rom[bank..bank + 4].copy_from_slice(MAGIC);
    let n = r.added.len().min(FREE_POINTS.len());
    rom[offset(BANK, ADDED)] = n as u8;
    rom[offset(BANK, ADDED + 1)..offset(BANK, ADDED + 1) + n].copy_from_slice(&r.added[..n]);
    for p in FREE_POINTS {
        let f = if r.added.contains(&p) { FREE_FLAG } else { NO_FLAG };
        for base in [0, TWIN] {
            rom[base + offset(0x0F, FLAGS) + p as usize] = f;
        }
    }
    let o = offset(BANK, NEW_SCRIPTS);
    rom[o..o + KEPT].copy_from_slice(&game);
    rom[o + KEPT..o + KEPT + added.len()].copy_from_slice(&added);
    let o = offset(BANK, NEW_WAYS);
    rom[o..o + table.len()].copy_from_slice(&table);
    for (p, pl) in r.places.iter().enumerate().take(POINTS) {
        let o = offset(BANK, NEW_PLACES + (p * 8) as u16);
        rom[o..o + 8].copy_from_slice(&pl.bytes());
    }
    if !moved_code(rom) {
        for base in [0, TWIN] {
            patch_code(rom, base);
        }
    }
    Ok(added.len())
}

fn moved_code(rom: &[u8]) -> bool {
    word(rom, offset(0x0F, WAYS_REF)) == NEW_WAYS
}

/// A zone's entrance on the world map and the point Mario stands on when he walks out of
/// the zone: the same place, and the same ways out (the game keeps them twice).
pub const ZONE_EXITS: [(u8, u8); 6] = [(0x02, 0x19), (0x05, 0x1A), (0x06, 0x1B), (0x09, 0x1C), (0x0B, 0x1D), (0x0C, 0x1E)];

/// The other point of an entrance / zone exit pair.
pub fn twin(p: u8) -> Option<u8> {
    ZONE_EXITS.iter().find_map(|&(a, b)| if a == p { Some(b) } else if b == p { Some(a) } else { None })
}

/// Point numbers the game never uses (no level, no flag, no ways in or out, no code of its
/// own) that it shows on the world map when it shows the map again: room for new points.
/// (The game picks the map from the point's number: the other unused numbers, $3A, $41,
/// $4C-$4F, $54-$59, $5D-$63, $76-$77, fall in a zone's numbers and show that zone.)
pub const FREE_POINTS: [u8; 6] = [0x6D, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F];
/// The flag ($A848 + this) a new point gets: one no point uses, that shuts no way.
pub const FREE_FLAG: u8 = 0x0A;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_corners() {
        let w = Way::through(&[(16, 0), (16, -12), (28, -24)], 5);
        assert_eq!(w.steps, vec![RIGHT, RIGHT, UP, UP | HALF, RIGHT | UP, (RIGHT | UP) | HALF]);
        assert_eq!(*w.track().last().unwrap(), (28, -24));
        assert_eq!(w.bytes()[w.steps.len()..], [0, 5]);
        // not at 45 degrees: diagonal first
        let w = Way::through(&[(24, 8)], 1);
        assert_eq!(w.steps, vec![RIGHT | DOWN, RIGHT, RIGHT]);
    }

    #[test]
    fn places_on_the_world_picture() {
        // point 0 of the game: camera ($81, $142), Mario at ($4B, $4C)
        let p = Place::from(&[0x00, 0x81, 0x01, 0x42, 0x4B, 0x4C, 0, 0]);
        assert_eq!(p.on_world(), (204, 394));
        assert_eq!(Place::at_world(204, 394), p);
        // near a corner the camera stops and Mario moves
        let q = Place::at_world(20, 500);
        assert_eq!((q.cam_x, q.cam_y), (0, 512 - 144));
        assert_eq!(q.on_world(), (20, 500));
    }
}
