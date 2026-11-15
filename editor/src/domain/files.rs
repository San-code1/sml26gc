//! Level and project files: plain text, one item per line, numbers in hex. The first line
//! names the kind and version, the last is `end`, so a cut file is found. Blocks are kept
//! raw (not compressed), so a file does not depend on the cartridge's codec and a diff shows
//! what changed.
//!
//! A level: `base`, `header` (20 bytes), `map` (48 rows of 256 blocks), `objects` (x y kind
//! flag). A project: the cartridge's hash, every level as above, the world and zone
//! pictures, the level each map point opens, the paths, where Mario stands on each point,
//! the new points and the doors.
use super::document::StandIn;
use cart_model::doors::{Doors, Leads};
use cart_model::levels::{LEVELS, MAP_BYTES};
use cart_model::map_routes::{MapRoutes, Place, Way};
use cart_model::objects::Obj;
use cart_model::{extra_levels, world, zone_maps};
use std::fmt::Write;

const LEVEL_KIND: &str = "NMB level";
const PROJECT_KIND: &str = "NMB project";
const VERSION: u32 = 1;
const MAP_ROW: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    NotLevel,
    NotProject,
    /// Made by a later editor.
    Newer,
    /// Cut short or changed by hand where it can't be read: the line (from 1).
    Damaged(usize),
    /// A base level that is not one of the 32.
    BaseOutside(usize),
    /// A project made over another cartridge.
    OtherCartridge,
}

/// A level as files keep it: what it plays as, its blocks and its objects.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelFile {
    pub stand_in: StandIn,
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
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
    let _ = writeln!(out, "base {:02x}", l.stand_in.base);
    let _ = writeln!(out, "header {}", hex(&l.stand_in.header));
    out.push_str("map\n");
    rows(out, &l.map, MAP_ROW);
    let _ = writeln!(out, "objects {}", l.objs.len());
    for o in &l.objs {
        let _ = writeln!(out, "{:04x} {:04x} {:02x} {}", o.x, o.y, o.kind, o.flag as u8);
    }
}

pub fn level_text(l: &LevelFile) -> String {
    let mut out = format!("{LEVEL_KIND} {VERSION}\n");
    level_block(&mut out, l);
    out.push_str("end\n");
    out
}

pub fn read_level(text: &str) -> Result<LevelFile, FileError> {
    let mut r = Reader::new(text);
    r.kind(LEVEL_KIND, FileError::NotLevel)?;
    let l = r.level()?;
    r.end()?;
    Ok(l)
}

fn way_field(w: &Option<Way>) -> String {
    match w {
        None => "-".into(),
        Some(w) => format!("{:02x}:{}", w.to, hex(&w.steps)),
    }
}

pub fn project_text(p: &ProjectFile) -> String {
    let mut out = format!("{PROJECT_KIND} {VERSION}\ncartridge {:016x}\nlevels {}\n", p.cartridge, p.levels.len());
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
    for (at, leads) in &p.doors.exits {
        let _ = match leads {
            Leads::World(to) => writeln!(out, "{at:02x} world {to:02x}"),
            Leads::Zone(z) => writeln!(out, "{at:02x} zone {z:02x}"),
        };
    }
    out.push_str("end\n");
    out
}

pub fn read_project(text: &str) -> Result<ProjectFile, FileError> {
    let mut r = Reader::new(text);
    r.kind(PROJECT_KIND, FileError::NotProject)?;
    let cartridge = r.hex::<u64>("cartridge")?;
    let count = r.number("levels")?;
    if !(LEVELS..=LEVELS + extra_levels::MAX).contains(&count) {
        return Err(r.damaged());
    }
    let levels = (0..count).map(|_| r.level()).collect::<Result<Vec<_>, _>>()?;
    r.word("world")?;
    let world = r.rows(world::H, world::W)?;
    let zones = (0..r.count("zones", zone_maps::ZONES.len())?).map(|_| r.rows(zone_maps::H, zone_maps::W)).collect::<Result<Vec<_>, _>>()?;
    let points = r.bytes_field("points")?;
    if points.len() != world::POINT_COUNT {
        return Err(r.damaged());
    }
    let ways = (0..r.count("ways", cart_model::map_routes::POINTS)?).map(|_| r.ways()).collect::<Result<Vec<_>, _>>()?;
    let places = (0..r.count("places", cart_model::map_routes::POINTS)?).map(|_| r.place()).collect::<Result<Vec<_>, _>>()?;
    let added = r.bytes_field("added")?;
    let into_n = r.number("into")?;
    let into = (0..into_n).map(|_| r.bytes_line()).collect::<Result<Vec<_>, _>>()?;
    let exits_n = r.number("exits")?;
    let exits = (0..exits_n).map(|_| r.exit()).collect::<Result<Vec<_>, _>>()?;
    r.end()?;
    Ok(ProjectFile { cartridge, levels, world, zones, points, routes: MapRoutes { ways, places, added }, doors: Doors { into, exits } })
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

    /// The first line: this kind of file, at a version this editor reads.
    fn kind(&mut self, kind: &str, wrong: FileError) -> Result<(), FileError> {
        let line = self.line().map_err(|_| wrong.clone())?.trim_start_matches('\u{feff}');
        let version = line.strip_prefix(kind).and_then(|v| v.trim().parse::<u32>().ok()).ok_or(wrong)?;
        if version > VERSION {
            return Err(FileError::Newer);
        }
        Ok(())
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

    fn level(&mut self) -> Result<LevelFile, FileError> {
        let base = self.hex::<u8>("base")?;
        if base as usize >= LEVELS {
            return Err(FileError::BaseOutside(base as usize));
        }
        let header: [u8; 20] = self.bytes_field("header")?.try_into().map_err(|_| self.damaged())?;
        self.word("map")?;
        let map = self.rows(MAP_BYTES / MAP_ROW, MAP_ROW)?;
        let n = self.number("objects")?;
        let objs = (0..n).map(|_| self.object()).collect::<Result<Vec<_>, _>>()?;
        Ok(LevelFile { stand_in: StandIn { base, header }, map, objs })
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
        let mut ways: [Option<Way>; 4] = Default::default();
        for (slot, field) in ways.iter_mut().zip(f) {
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
        LevelFile { stand_in: StandIn { base, header: core::array::from_fn(|i| i as u8 * 3) }, map, objs }
    }

    fn project() -> ProjectFile {
        let way = Way { steps: vec![1, 1, 0x18], to: 7 };
        ProjectFile {
            cartridge: 0x0123_4567_89AB_CDEF,
            levels: (0..LEVELS as u8 + 2).map(|l| level(l % LEVELS as u8)).collect(),
            world: (0..world::W * world::H).map(|i| i as u8).collect(),
            zones: (0..zone_maps::ZONES.len()).map(|z| vec![z as u8; zone_maps::W * zone_maps::H]).collect(),
            points: (0..world::POINT_COUNT as u8).collect(),
            routes: MapRoutes {
                ways: (0..cart_model::map_routes::POINTS).map(|p| if p % 3 == 0 { [Some(way.clone()), None, Some(Way { steps: vec![], to: 2 }), None] } else { Default::default() }).collect(),
                places: (0..cart_model::map_routes::POINTS).map(|p| Place { cam_x: p as u16 * 3, cam_y: 0x150, x: 0x4B, y: p as u8, rest: [1, 2] }).collect(),
                added: vec![0x6D, 0x7B],
            },
            doors: Doors { into: vec![vec![0x02, 0x19], vec![0xFF]], exits: vec![(0x23, Leads::World(0x05)), (0x28, Leads::Zone(2))] },
        }
    }

    #[test]
    fn a_level_comes_back_as_it_was_written() {
        let l = level(12);
        let text = level_text(&l);
        assert!(text.starts_with("NMB level 1\nbase 0c\n") && text.ends_with("end\n"));
        assert_eq!(read_level(&text), Ok(l));
    }

    #[test]
    fn unusable_level_files_are_refused() {
        let text = level_text(&level(3));
        assert!(matches!(read_level(&text[..text.len() / 2]), Err(FileError::Damaged(_))));
        assert_eq!(read_level(&text.replace("base 03", "base 20")), Err(FileError::BaseOutside(32)));
        assert_eq!(read_level(&text.replace("NMB level 1", "NMB level 2")), Err(FileError::Newer));
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
