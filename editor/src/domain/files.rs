//! Level, zone and project files: plain text, one item per line, numbers in hex. The first line
//! names the kind and version, the last is `end`, so a cut file is found. Blocks are kept
//! raw (not compressed), so a file does not depend on the cartridge's codec and a diff shows
//! what changed.
//!
//! A level: `base`, `name` (from version 2), `header` (20 bytes), `map` (48 rows of 256 blocks), `objects` (x y kind
//! flag), `secret` (from version 3: `-`, `point` n or `newzone` k). A new zone: `base`, `name`, `entry`, `door`, `picture` (18 rows of 20 tiles), `points`
//! (number, place, level, flag, shut, pass point code, the four ways) and `exits`. A project:
//! the cartridge's hash, every level as above, the world and zone pictures, the level each
//! map point opens, the paths, where Mario stands on each point, the new points, the doors,
//! (from version 2) the new zones and (from version 4) all 256 point numbers and the game
//! zones' added points (`zonepoints`: per zone, its number and its points as a new zone's),
//! and (from version 5) the pad directions the world map's added points shut (`shuts`).
use super::document::StandIn;
use cart_model::added_points::{self, ZoneAdded};
use cart_model::doors::{Doors, Leads};
use cart_model::levels::{LEVELS, MAP_BYTES};
use cart_model::map_routes::{self, MapRoutes, Place, Way};
use cart_model::new_zones::{self, NewZone, ZonePoint};
use cart_model::objects::Obj;
use cart_model::secret_exits::SecretTo;
use cart_model::{extra_levels, world, zone_maps};
use std::fmt::Write;

const LEVEL_KIND: &str = "NMB level";
const PROJECT_KIND: &str = "NMB project";
const ZONE_KIND: &str = "NMB zone";
/// Version 1 has no names, version 2 no secret exit target.
const LEVEL_VERSION: u32 = 3;
const ZONE_VERSION: u32 = 1;
/// Version 1 has no new zones, version 2 no secret exit targets.
const PROJECT_VERSION: u32 = 5;
/// Point levels a project file kept before version 4.
const OLD_LEVELS: usize = 0x82;
const MAP_ROW: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    NotLevel,
    NotProject,
    NotZone,
    /// Made by a later editor.
    Newer,
    /// Cut short or changed by hand where it can't be read: the line (from 1).
    Damaged(usize),
    /// A base level that is not one of the 32.
    BaseOutside(usize),
    /// A new zone made from a zone no door enters (an index into `zone_maps::ZONES`).
    NotABase(usize),
    /// A project made over another cartridge.
    OtherCartridge,
}

/// A level as files keep it: what it plays as, its blocks and its objects.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelFile {
    pub stand_in: StandIn,
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
    /// A new level's name (empty: none).
    pub name: String,
    /// Where its secret exit leads (None: the game's way).
    pub secret: Option<SecretTo>,
}

/// Everything a project file holds.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectFile {
    /// `cartridge_id` of the original cartridge.
    pub cartridge: u64,
    /// The 32 levels (their own number as base), then the new ones.
    pub levels: Vec<LevelFile>,
    pub world: Vec<u8>,
    pub zones: Vec<Vec<u8>>,
    pub points: Vec<u8>,
    pub routes: MapRoutes,
    pub doors: Doors,
    pub new_zones: Vec<NewZone>,
    /// The point each zone a door enters is entered on (empty in version 1).
    pub entries: Vec<u8>,
    /// The game zones' added points (from version 4).
    pub zone_points: Vec<ZoneAdded>,
    /// The pad directions the world map's added points shut, from `added_points::FIRST`
    /// (from version 5).
    pub shuts: Vec<u8>,
}

/// Tells cartridges apart (64-bit FNV-1a of the whole image).
pub fn cartridge_id(rom: &[u8]) -> u64 {
    rom.iter().fold(0xCBF2_9CE4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01B3))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn rows(out: &mut String, cells: &[u8], width: usize) {
    for row in cells.chunks(width) {
        out.push_str(&hex(row));
        out.push('\n');
    }
}

fn level_block(out: &mut String, l: &LevelFile) {
    let _ = writeln!(out, "base {:02x}\nname {}", l.stand_in.base, l.name);
    let _ = writeln!(out, "header {}", hex(&l.stand_in.header));
    out.push_str("map\n");
    rows(out, &l.map, MAP_ROW);
    let _ = writeln!(out, "objects {}", l.objs.len());
    for o in &l.objs {
        let _ = writeln!(out, "{:04x} {:04x} {:02x} {}", o.x, o.y, o.kind, o.flag as u8);
    }
    let _ = match l.secret {
        None => writeln!(out, "secret -"),
        Some(SecretTo::Point(p)) => writeln!(out, "secret point {p:02x}"),
        Some(SecretTo::NewZone(k)) => writeln!(out, "secret newzone {k:02x}"),
    };
}

pub fn level_text(l: &LevelFile) -> String {
    let mut out = format!("{LEVEL_KIND} {LEVEL_VERSION}\n");
    level_block(&mut out, l);
    out.push_str("end\n");
    out
}

pub fn read_level(text: &str) -> Result<LevelFile, FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(LEVEL_KIND, LEVEL_VERSION, FileError::NotLevel)?;
    let l = r.level(version)?;
    r.end()?;
    Ok(l)
}

fn way_field(w: &Option<Way>) -> String {
    match w {
        None => "-".into(),
        Some(w) => format!("{:02x}:{}", w.to, hex(&w.steps)),
    }
}

fn exit_line(out: &mut String, at: u8, leads: Leads) {
    let _ = match leads {
        Leads::World(to) => writeln!(out, "{at:02x} world {to:02x}"),
        Leads::Zone(z) => writeln!(out, "{at:02x} zone {z:02x}"),
        Leads::NewZone(k) => writeln!(out, "{at:02x} newzone {k:02x}"),
    };
}

fn zone_block(out: &mut String, z: &NewZone) {
    let _ = writeln!(out, "base {:02x}\nname {}\nentry {:02x}\ndoor {:02x}\npicture", z.base, z.name, z.entry, z.door);
    rows(out, &z.picture, zone_maps::W);
    point_lines(out, &z.points);
    let _ = writeln!(out, "exits {}", z.exits.len());
    for &(at, leads) in &z.exits {
        exit_line(out, at, leads);
    }
}

fn point_lines(out: &mut String, points: &[ZonePoint]) {
    let _ = writeln!(out, "points {}", points.len());
    for p in points {
        let pl = &p.place;
        let pass = p.pass.map_or("-".to_string(), |a| format!("{a:04x}"));
        let ways = p.ways.iter().map(way_field).collect::<Vec<_>>().join(" ");
        let _ = writeln!(out, "{:02x} {:04x} {:04x} {:02x} {:02x} {} {:02x} {:02x} {:02x} {pass} {ways}", p.number, pl.cam_x, pl.cam_y, pl.x, pl.y, hex(&pl.rest), p.level, p.flag, p.shut);
    }
}

pub fn zone_text(z: &NewZone) -> String {
    let mut out = format!("{ZONE_KIND} {ZONE_VERSION}\n");
    zone_block(&mut out, z);
    out.push_str("end\n");
    out
}

pub fn read_zone(text: &str) -> Result<NewZone, FileError> {
    let mut r = Reader::new(text);
    r.kind(ZONE_KIND, ZONE_VERSION, FileError::NotZone)?;
    let z = r.zone()?;
    r.end()?;
    Ok(z)
}

pub fn project_text(p: &ProjectFile) -> String {
    let mut out = format!("{PROJECT_KIND} {PROJECT_VERSION}\ncartridge {:016x}\nlevels {}\n", p.cartridge, p.levels.len());
    for l in &p.levels {
        level_block(&mut out, l);
    }
    out.push_str("world\n");
    rows(&mut out, &p.world, world::W);
    let _ = writeln!(out, "zones {}", p.zones.len());
    for z in &p.zones {
        rows(&mut out, z, zone_maps::W);
    }
    let _ = writeln!(out, "points {}", hex(&p.points));
    let _ = writeln!(out, "ways {}", p.routes.ways.len());
    for w in &p.routes.ways {
        out.push_str(&w.iter().map(way_field).collect::<Vec<_>>().join(" "));
        out.push('\n');
    }
    let _ = writeln!(out, "places {}", p.routes.places.len());
    for pl in &p.routes.places {
        let _ = writeln!(out, "{:04x} {:04x} {:02x} {:02x} {}", pl.cam_x, pl.cam_y, pl.x, pl.y, hex(&pl.rest));
    }
    let _ = writeln!(out, "added {}", hex(&p.routes.added));
    let _ = writeln!(out, "into {}", p.doors.into.len());
    for slots in &p.doors.into {
        let _ = writeln!(out, "{}", hex(slots));
    }
    let _ = writeln!(out, "exits {}", p.doors.exits.len());
    for &(at, leads) in &p.doors.exits {
        exit_line(&mut out, at, leads);
    }
    let _ = writeln!(out, "entries {}", hex(&p.entries));
    let _ = writeln!(out, "newzones {}", p.new_zones.len());
    for z in &p.new_zones {
        zone_block(&mut out, z);
    }
    let _ = writeln!(out, "zonepoints {}", p.zone_points.len());
    for z in &p.zone_points {
        let _ = writeln!(out, "zone {:02x}", z.zone);
        point_lines(&mut out, &z.points);
    }
    let _ = writeln!(out, "shuts {}", hex(&p.shuts));
    out.push_str("end\n");
    out
}

pub fn read_project(text: &str) -> Result<ProjectFile, FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(PROJECT_KIND, PROJECT_VERSION, FileError::NotProject)?;
    let cartridge = r.hex::<u64>("cartridge")?;
    let count = r.number("levels")?;
    if !(LEVELS..=LEVELS + extra_levels::MAX).contains(&count) {
        return Err(r.damaged());
    }
    let levels = (0..count).map(|_| r.level(version)).collect::<Result<Vec<_>, _>>()?;
    r.word("world")?;
    let world = r.rows(world::H, world::W)?;
    let zones = (0..r.count("zones", zone_maps::ZONES.len())?).map(|_| r.rows(zone_maps::H, zone_maps::W)).collect::<Result<Vec<_>, _>>()?;
    // before version 4: the game's numbers only ($82 levels, $80 ways and places)
    let (n_levels, n_points) = if version >= 4 { (world::POINT_COUNT, map_routes::ALL) } else { (OLD_LEVELS, map_routes::POINTS) };
    let mut points = r.bytes_field("points")?;
    if points.len() != n_levels {
        return Err(r.damaged());
    }
    points.resize(world::POINT_COUNT, world::NO_LEVEL);
    let mut ways = (0..r.count("ways", n_points)?).map(|_| r.ways()).collect::<Result<Vec<_>, _>>()?;
    let mut places = (0..r.count("places", n_points)?).map(|_| r.place()).collect::<Result<Vec<_>, _>>()?;
    ways.resize(map_routes::ALL, Default::default());
    places.resize(map_routes::ALL, Place { cam_x: 0, cam_y: 0, x: 0, y: 0, rest: [0, 0] });
    let added = r.bytes_field("added")?;
    let into_n = r.number("into")?;
    let into = (0..into_n).map(|_| r.bytes_line()).collect::<Result<Vec<_>, _>>()?;
    let exits_n = r.number("exits")?;
    let exits = (0..exits_n).map(|_| r.exit()).collect::<Result<Vec<_>, _>>()?;
    let entries = if version >= 2 { r.bytes_field("entries")? } else { Vec::new() };
    let new_zones = if version >= 2 {
        let n = r.number("newzones")?;
        if n > new_zones::MAX {
            return Err(r.damaged());
        }
        (0..n).map(|_| r.zone()).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let zone_points = if version >= 4 {
        let n = r.number("zonepoints")?;
        (0..n).map(|_| r.zone_added()).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let mut shuts = if version >= 5 { r.bytes_field("shuts")? } else { Vec::new() };
    if shuts.len() > added_points::MAX {
        return Err(r.damaged());
    }
    shuts.resize(added_points::MAX, 0);
    r.end()?;
    Ok(ProjectFile { cartridge, levels, world, zones, points, routes: MapRoutes { ways, places, added }, doors: Doors { into, exits }, new_zones, entries, zone_points, shuts })
}

struct Reader<'a> {
    lines: std::str::Lines<'a>,
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Reader<'a> {
        Reader { lines: text.lines(), at: 0 }
    }

    fn damaged(&self) -> FileError {
        FileError::Damaged(self.at)
    }

    fn line(&mut self) -> Result<&'a str, FileError> {
        self.at += 1;
        self.lines.next().map(str::trim).ok_or(FileError::Damaged(self.at))
    }

    /// The first line: this kind of file, at a version this editor reads (up to `newest`).
    fn kind(&mut self, kind: &str, newest: u32, wrong: FileError) -> Result<u32, FileError> {
        let line = self.line().map_err(|_| wrong.clone())?.trim_start_matches('\u{feff}');
        let version = line.strip_prefix(kind).and_then(|v| v.trim().parse::<u32>().ok()).ok_or(wrong)?;
        if version > newest {
            return Err(FileError::Newer);
        }
        Ok(version)
    }

    fn word(&mut self, key: &str) -> Result<(), FileError> {
        if self.line()? == key {
            Ok(())
        } else {
            Err(self.damaged())
        }
    }

    fn field(&mut self, key: &str) -> Result<&'a str, FileError> {
        let line = self.line()?;
        match line.split_once(' ') {
            Some((k, v)) if k == key => Ok(v.trim()),
            None if line == key => Ok(""),
            _ => Err(self.damaged()),
        }
    }

    fn hex<T: Hex>(&mut self, key: &str) -> Result<T, FileError> {
        let v = self.field(key)?;
        T::from_hex(v).ok_or(self.damaged())
    }

    /// A count (decimal).
    fn number(&mut self, key: &str) -> Result<usize, FileError> {
        let v = self.field(key)?;
        v.parse().map_err(|_| self.damaged())
    }

    /// A count that must be exactly `n`.
    fn count(&mut self, key: &str, n: usize) -> Result<usize, FileError> {
        let v = self.number(key)?;
        if v == n {
            Ok(v)
        } else {
            Err(self.damaged())
        }
    }

    fn bytes(&self, s: &str) -> Result<Vec<u8>, FileError> {
        if s.len() % 2 != 0 || !s.is_ascii() {
            return Err(self.damaged());
        }
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| self.damaged())).collect()
    }

    fn bytes_line(&mut self) -> Result<Vec<u8>, FileError> {
        let line = self.line()?;
        self.bytes(line)
    }

    fn bytes_field(&mut self, key: &str) -> Result<Vec<u8>, FileError> {
        let v = self.field(key)?;
        self.bytes(v)
    }

    fn rows(&mut self, h: usize, w: usize) -> Result<Vec<u8>, FileError> {
        let mut cells = Vec::with_capacity(w * h);
        for _ in 0..h {
            let row = self.bytes_line()?;
            if row.len() != w {
                return Err(self.damaged());
            }
            cells.extend(row);
        }
        Ok(cells)
    }

    fn level(&mut self, version: u32) -> Result<LevelFile, FileError> {
        let named = version >= 2;
        let base = self.hex::<u8>("base")?;
        if base as usize >= LEVELS {
            return Err(FileError::BaseOutside(base as usize));
        }
        let name = if named { self.name()? } else { String::new() };
        let header: [u8; 20] = self.bytes_field("header")?.try_into().map_err(|_| self.damaged())?;
        self.word("map")?;
        let map = self.rows(MAP_BYTES / MAP_ROW, MAP_ROW)?;
        let n = self.number("objects")?;
        let objs = (0..n).map(|_| self.object()).collect::<Result<Vec<_>, _>>()?;
        let secret = if version >= 3 { self.secret()? } else { None };
        Ok(LevelFile { stand_in: StandIn { base, header }, map, objs, name, secret })
    }

    fn secret(&mut self) -> Result<Option<SecretTo>, FileError> {
        let v = self.field("secret")?;
        let f: Vec<&str> = v.split_whitespace().collect();
        let to = match f[..] {
            ["-"] => Some(None),
            ["point", p] => u8::from_str_radix(p, 16).ok().map(|p| Some(SecretTo::Point(p))),
            ["newzone", k] => usize::from_str_radix(k, 16).ok().filter(|&k| k < new_zones::MAX).map(|k| Some(SecretTo::NewZone(k))),
            _ => None,
        };
        to.ok_or(self.damaged())
    }

    fn name(&mut self) -> Result<String, FileError> {
        let v = self.field("name")?;
        Ok(cart_model::extra_levels::clean_name(v))
    }

    fn object(&mut self) -> Result<Obj, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let obj = match f[..] {
            [x, y, kind, flag] => (|| Some(Obj { x: u16::from_str_radix(x, 16).ok()?, y: u16::from_str_radix(y, 16).ok()?, kind: u8::from_str_radix(kind, 16).ok()?, flag: matches!(flag, "1") }))(),
            _ => None,
        };
        obj.ok_or(self.damaged())
    }

    fn ways(&mut self) -> Result<[Option<Way>; 4], FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 4 {
            return Err(self.damaged());
        }
        self.way_fields(&f)
    }

    fn way_fields(&self, f: &[&str]) -> Result<[Option<Way>; 4], FileError> {
        let mut ways: [Option<Way>; 4] = Default::default();
        for (slot, &field) in ways.iter_mut().zip(f) {
            if field == "-" {
                continue;
            }
            let (to, steps) = field.split_once(':').ok_or(self.damaged())?;
            *slot = Some(Way { to: u8::from_str_radix(to, 16).map_err(|_| self.damaged())?, steps: self.bytes(steps)? });
        }
        Ok(ways)
    }

    fn place(&mut self) -> Result<Place, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let place = match f[..] {
            [cx, cy, x, y, rest] => (|| {
                let rest: [u8; 2] = self.bytes(rest).ok()?.try_into().ok()?;
                Some(Place { cam_x: u16::from_str_radix(cx, 16).ok()?, cam_y: u16::from_str_radix(cy, 16).ok()?, x: u8::from_str_radix(x, 16).ok()?, y: u8::from_str_radix(y, 16).ok()?, rest })
            })(),
            _ => None,
        };
        place.ok_or(self.damaged())
    }

    /// A new zone: its points within its base's numbers, one of them the entry, an exit for
    /// each of its base's exit sites.
    fn zone(&mut self) -> Result<NewZone, FileError> {
        let base = self.hex::<u8>("base")? as usize;
        let b = new_zones::base_of(base).ok_or(FileError::NotABase(base))?;
        let name = self.name()?;
        let entry = self.hex::<u8>("entry")?;
        let door = self.hex::<u8>("door")?;
        self.word("picture")?;
        let picture = self.rows(zone_maps::H, zone_maps::W)?;
        let n = self.number("points")?;
        let points = (0..n).map(|_| self.zone_point()).collect::<Result<Vec<_>, _>>()?;
        let mut numbers: Vec<u8> = points.iter().map(|p| p.number).collect();
        numbers.sort();
        numbers.dedup();
        if numbers.len() != n || !numbers.iter().all(|&p| b.holds(p)) || !numbers.contains(&entry) {
            return Err(self.damaged());
        }
        let exits = (0..self.count("exits", new_zones::exit_sites(b).len())?).map(|_| self.exit()).collect::<Result<Vec<_>, _>>()?;
        Ok(NewZone { base, picture, points, entry, exits, door, name })
    }

    /// A game zone's added points: numbered from $80, at most `added_points::MAX`.
    fn zone_added(&mut self) -> Result<ZoneAdded, FileError> {
        let zone = self.hex::<u8>("zone")? as usize;
        let n = self.number("points")?;
        let points = (0..n).map(|_| self.zone_point()).collect::<Result<Vec<_>, _>>()?;
        if zone >= added_points::ZONES || n > added_points::MAX || !points.iter().all(|p| added_points::index(p.number).is_some()) {
            return Err(self.damaged());
        }
        Ok(ZoneAdded { zone, points })
    }

    fn zone_point(&mut self) -> Result<ZonePoint, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 14 {
            return Err(self.damaged());
        }
        let ways = self.way_fields(&f[10..])?;
        let point = (|| {
            let b = |s: &str| u8::from_str_radix(s, 16).ok();
            let w = |s: &str| u16::from_str_radix(s, 16).ok();
            let rest: [u8; 2] = self.bytes(f[5]).ok()?.try_into().ok()?;
            let pass = if f[9] == "-" { None } else { Some(w(f[9])?) };
            let place = Place { cam_x: w(f[1])?, cam_y: w(f[2])?, x: b(f[3])?, y: b(f[4])?, rest };
            Some(ZonePoint { number: b(f[0])?, place, ways, level: b(f[6])?, flag: b(f[7])?, shut: b(f[8])?, pass })
        })();
        point.ok_or(self.damaged())
    }

    fn exit(&mut self) -> Result<(u8, Leads), FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let exit = match f[..] {
            [at, kind, to] => (|| {
                let at = u8::from_str_radix(at, 16).ok()?;
                let to = u8::from_str_radix(to, 16).ok()?;
                match kind {
                    "world" => Some((at, Leads::World(to))),
                    "zone" => Some((at, Leads::Zone(to as usize))),
                    "newzone" => Some((at, Leads::NewZone(to as usize))),
                    _ => None,
                }
            })(),
            _ => None,
        };
        exit.ok_or(self.damaged())
    }

    fn end(&mut self) -> Result<(), FileError> {
        self.word("end")
    }
}

trait Hex: Sized {
    fn from_hex(s: &str) -> Option<Self>;
}

impl Hex for u8 {
    fn from_hex(s: &str) -> Option<u8> {
        u8::from_str_radix(s, 16).ok()
    }
}

impl Hex for u64 {
    fn from_hex(s: &str) -> Option<u64> {
        u64::from_str_radix(s, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(base: u8) -> LevelFile {
        let map = (0..MAP_BYTES).map(|i| (i % 0x80) as u8).collect();
        let objs = vec![Obj { x: 0x40, y: 0x18, kind: 0x25, flag: true }, Obj { x: 0xFF8, y: 0x3F8, kind: 0x7F, flag: false }];
        let secret = [None, Some(SecretTo::Point(0x07)), Some(SecretTo::NewZone(1))][base as usize % 3];
        LevelFile { stand_in: StandIn { base, header: core::array::from_fn(|i| i as u8 * 3) }, map, objs, name: format!("Cave {base}"), secret }
    }

    fn project() -> ProjectFile {
        let way = Way { steps: vec![1, 1, 0x18], to: 7 };
        ProjectFile {
            cartridge: 0x0123_4567_89AB_CDEF,
            levels: (0..LEVELS as u8 + 2).map(|l| level(l % LEVELS as u8)).collect(),
            world: (0..world::W * world::H).map(|i| i as u8).collect(),
            zones: (0..zone_maps::ZONES.len()).map(|z| vec![z as u8; zone_maps::W * zone_maps::H]).collect(),
            points: (0..world::POINT_COUNT).map(|p| p as u8).collect(),
            routes: MapRoutes {
                ways: (0..map_routes::ALL).map(|p| if p % 3 == 0 { [Some(way.clone()), None, Some(Way { steps: vec![], to: 2 }), None] } else { Default::default() }).collect(),
                places: (0..map_routes::ALL).map(|p| Place { cam_x: p as u16 * 3, cam_y: 0x150, x: 0x4B, y: p as u8, rest: [1, 2] }).collect(),
                added: vec![0x6D, 0x7B, 0x80],
            },
            doors: Doors { into: vec![vec![0x02, 0x19], vec![0xFF]], exits: vec![(0x23, Leads::World(0x05)), (0x28, Leads::Zone(2))] },
            new_zones: vec![zone()],
            entries: vec![0x23, 0x28, 0x32, 0x3C, 0x50],
            zone_points: vec![ZoneAdded { zone: 1, points: vec![ZonePoint { number: 0x80, flag: 0x80, ..zone().points[0].clone() }] }],
            shuts: (0..added_points::MAX).map(|j| if j == 0 { 0x90 } else { 0 }).collect(),
        }
    }

    fn zone() -> NewZone {
        let place = |x: u8| Place { cam_x: 0, cam_y: 0, x, y: 0x60, rest: [1, 2] };
        let way = |to: u8| Some(Way { steps: vec![0x10, 0x02], to });
        let points = vec![
            ZonePoint { number: 0x23, place: place(0x40), ways: [way(0x25), None, None, None], level: 1, flag: 5, shut: 0x10, pass: None },
            ZonePoint { number: 0x25, place: place(0x60), ways: [None, way(0x23), None, None], level: world::NO_LEVEL, flag: 6, shut: 0, pass: Some(0x429A) },
        ];
        let sites = new_zones::exit_sites(&new_zones::BASES[0]).len();
        let exits = (0..sites).map(|i| (0x25, if i == 0 { Leads::World(0x19) } else { Leads::NewZone(0) })).collect();
        NewZone { base: 0, picture: (0..zone_maps::W * zone_maps::H).map(|i| i as u8).collect(), points, entry: 0x23, exits, door: 0x19, name: "Big tree".into() }
    }

    #[test]
    fn a_zone_comes_back_as_it_was_written() {
        let z = zone();
        let text = zone_text(&z);
        assert!(text.starts_with("NMB zone 1\nbase 00\nname Big tree\n") && text.ends_with("end\n"));
        assert_eq!(read_zone(&text), Ok(z));
    }

    #[test]
    fn unusable_zone_files_are_refused() {
        let text = zone_text(&zone());
        assert!(matches!(read_zone(&text[..text.len() / 2]), Err(FileError::Damaged(_))));
        assert_eq!(read_zone(&text.replace("base 00", "base 04")), Err(FileError::NotABase(4)));
        assert!(matches!(read_zone(&text.replace("entry 23", "entry 24")), Err(FileError::Damaged(_))), "the entry is one of its points");
        assert!(matches!(read_zone(&text.replace("\n25 ", "\n32 ")), Err(FileError::Damaged(_))), "points stay in the base's numbers");
        assert_eq!(read_zone(&text.replace("NMB zone 1", "NMB zone 2")), Err(FileError::Newer));
        assert_eq!(read_zone(&level_text(&level(0))), Err(FileError::NotZone));
    }

    #[test]
    fn a_level_comes_back_as_it_was_written() {
        let l = level(13);
        let text = level_text(&l);
        assert!(text.starts_with("NMB level 3\nbase 0d\nname Cave 13\n") && text.ends_with("secret point 07\nend\n"));
        assert_eq!(read_level(&text), Ok(l.clone()));
        for b in [12, 14] {
            assert_eq!(read_level(&level_text(&level(b))), Ok(level(b)));
        }
        let v2 = text.replacen("NMB level 3", "NMB level 2", 1).replacen("secret point 07\n", "", 1);
        assert_eq!(read_level(&v2), Ok(LevelFile { secret: None, ..l.clone() }), "version 2 has no secret exit target");
        let old = v2.replacen("NMB level 2", "NMB level 1", 1).replacen("name Cave 13\n", "", 1);
        assert_eq!(read_level(&old), Ok(LevelFile { name: String::new(), secret: None, ..l }), "version 1 has no names");
        assert!(matches!(read_level(&text.replace("secret point 07", "secret door 07")), Err(FileError::Damaged(_))));
    }

    #[test]
    fn unusable_level_files_are_refused() {
        let text = level_text(&level(3));
        assert!(matches!(read_level(&text[..text.len() / 2]), Err(FileError::Damaged(_))));
        assert_eq!(read_level(&text.replace("base 03", "base 20")), Err(FileError::BaseOutside(32)));
        assert_eq!(read_level(&text.replace("NMB level 3", "NMB level 4")), Err(FileError::Newer));
        assert_eq!(read_level("hello"), Err(FileError::NotLevel));
        assert_eq!(read_level(""), Err(FileError::NotLevel));
        let short_row = text.replacen("\n0001", "\n01", 1);
        assert!(matches!(read_level(&short_row), Err(FileError::Damaged(_))));
        assert_eq!(read_level(&project_text(&project())), Err(FileError::NotLevel));
    }

    #[test]
    fn a_project_comes_back_as_it_was_written() {
        let p = project();
        assert_eq!(read_project(&project_text(&p)), Ok(p));
    }

    #[test]
    fn a_version_1_project_has_no_new_zones() {
        // the game's point numbers only
        let mut p = project();
        p.points.truncate(OLD_LEVELS);
        p.routes.ways.truncate(map_routes::POINTS);
        p.routes.places.truncate(map_routes::POINTS);
        p.routes.added.retain(|&a| a < 0x80);
        let text = project_text(&p);
        let old = format!("{}end\n", &text[..text.find("entries").unwrap()]).replacen("NMB project 5", "NMB project 1", 1);
        let old: String = old.lines().filter(|l| !l.starts_with("name ") && !l.starts_with("secret ")).map(|l| format!("{l}\n")).collect();
        let levels = p.levels.iter().map(|l| LevelFile { name: String::new(), secret: None, ..l.clone() }).collect();
        p.points.resize(world::POINT_COUNT, world::NO_LEVEL);
        p.routes.ways.resize(map_routes::ALL, Default::default());
        p.routes.places.resize(map_routes::ALL, Place { cam_x: 0, cam_y: 0, x: 0, y: 0, rest: [0, 0] });
        assert_eq!(read_project(&old), Ok(ProjectFile { new_zones: Vec::new(), entries: Vec::new(), zone_points: Vec::new(), shuts: vec![0; added_points::MAX], levels, ..p }));
    }

    #[test]
    fn unusable_project_files_are_refused() {
        let text = project_text(&project());
        let cut = &text[..text.find("zones").unwrap()];
        assert!(matches!(read_project(cut), Err(FileError::Damaged(_))));
        assert_eq!(read_project(&level_text(&level(0))), Err(FileError::NotProject));
        assert!(matches!(read_project(&text.replace("levels 34", "levels 3")), Err(FileError::Damaged(_))));
    }

    #[test]
    fn cartridges_are_told_apart() {
        assert_ne!(cartridge_id(&[1, 2, 3]), cartridge_id(&[1, 2, 4]));
        assert_eq!(cartridge_id(&[9; 64]), cartridge_id(&[9; 64]));
    }
}
