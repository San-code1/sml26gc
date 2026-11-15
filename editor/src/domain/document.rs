//! Everything being edited: the cartridge being built (the edited copy), the untouched
//! original it is checked against, the 32 levels (blocks and objects) and the new ones
//! after them (`extra_levels`) with their undo history, the world map picture, the level
//! each map point opens and the paths between points (`map_routes`). What has changed since
//! the last save is tracked here; the files are not.
use super::history::History;
use cart_model::error::{BankUsage, ObjectsUsage};
use cart_model::extra_levels::{self, Extra};
use cart_model::levels::{self, COLS, LEVELS, ROWS};
use cart_model::map_routes::{self, MapRoutes, Place, Way};
use cart_model::objects::{self, Obj};
use cart_model::original_points::OnMap;
use cart_model::{layout, patch, world, zone_maps, CartError};

/// A level as it can be undone: its blocks and its objects.
#[derive(Clone, PartialEq, Debug)]
pub struct LevelContent {
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
}

pub struct LevelDoc {
    pub content: LevelContent,
    pub blocks_dirty: bool,
    pub objects_dirty: bool,
    history: History<LevelContent>,
}

impl LevelDoc {
    pub fn dirty(&self) -> bool {
        self.blocks_dirty || self.objects_dirty
    }
}

/// What a save wrote into the cartridge (shown to the person).
pub enum Saved {
    Bank(BankUsage),
    WorldMap,
    LevelPoints,
    /// The map's paths and points: bytes the changed paths take.
    Routes(usize),
    Objects(ObjectsUsage),
    /// The new levels: how many, bytes used.
    NewLevels(usize, usize),
}

pub struct EditorDocument {
    pub original: Vec<u8>,
    pub rom: Vec<u8>,
    /// The 32 levels, then the new ones (number 32 and up).
    pub levels: Vec<LevelDoc>,
    /// Per new level: the level it is played in place of, and its header.
    pub extras: Vec<(u8, [u8; 20])>,
    /// A new level was added or removed.
    pub extras_dirty: bool,
    /// The world map picture (64 x 64 tiles) and the zone maps' (20 x 18 each).
    pub world: Vec<u8>,
    pub zones: Vec<Vec<u8>>,
    /// Any map picture changed.
    pub world_dirty: bool,
    world_history: History<(Vec<u8>, Vec<Vec<u8>>)>,
    /// Which level each of the game's own map points opens.
    pub points: Vec<u8>,
    pub points_dirty: bool,
    /// The paths between map points, where Mario stands on each, and the new points.
    pub routes: MapRoutes,
    pub routes_dirty: bool,
    /// Undo of the Points tab: the levels the points open and the paths, together.
    points_history: History<(Vec<u8>, MapRoutes)>,
}

fn level_from(rom: &[u8], l: usize) -> LevelContent {
    let (_, start) = levels::map_location(rom, levels::header(rom, l));
    LevelContent { map: levels::decode(rom, start).0, objs: objects::read(rom, l).0 }
}

impl EditorDocument {
    /// Start from the edited copy if there is a usable one; otherwise from a new copy of the
    /// original, made 1 MB so every zone has room for bigger levels (`patch::expand`).
    pub fn open(original: Vec<u8>, edited: Option<Vec<u8>>) -> EditorDocument {
        let mut rom = edited
            .filter(|r| r.len() == original.len() || patch::is_expanded(r))
            .unwrap_or_else(|| patch::expand(&original).unwrap_or_else(|_| original.clone()));
        patch::upgrade(&mut rom); // copies made before the object patch get it now
        let doc = |content| LevelDoc { content, blocks_dirty: false, objects_dirty: false, history: History::default() };
        let mut levels: Vec<LevelDoc> = (0..LEVELS).map(|l| doc(level_from(&rom, l))).collect();
        let stored = extra_levels::read(&rom);
        levels.extend(stored.iter().map(|e| doc(LevelContent { map: e.map.clone(), objs: e.objs.clone() })));
        EditorDocument {
            extras: stored.iter().map(|e| (e.base, e.header)).collect(),
            extras_dirty: false,
            world: world::read(&rom),
            zones: zone_maps::ZONES.iter().enumerate().map(|(z, _)| zone_maps::read(&rom, z)).collect(),
            world_dirty: false,
            world_history: History::default(),
            points: world::point_levels(&rom),
            points_dirty: false,
            routes: map_routes::read(&rom),
            routes_dirty: false,
            points_history: History::default(),
            original,
            rom,
            levels,
        }
    }

    // ------------------------------------------------------------ levels

    /// Remember level `l` before a change, for undo.
    pub fn snapshot(&mut self, l: usize) {
        let lv = &mut self.levels[l];
        lv.history.record(lv.content.clone());
    }

    pub fn undo(&mut self, l: usize) -> bool {
        let lv = &mut self.levels[l];
        let done = lv.history.undo(&mut lv.content);
        if done {
            lv.blocks_dirty = true;
            lv.objects_dirty = true;
        }
        done
    }

    pub fn redo(&mut self, l: usize) -> bool {
        let lv = &mut self.levels[l];
        let done = lv.history.redo(&mut lv.content);
        if done {
            lv.blocks_dirty = true;
            lv.objects_dirty = true;
        }
        done
    }

    /// Put block `b` at column `c`, row `r`. True if something changed.
    pub fn set_block(&mut self, l: usize, c: i32, r: i32, b: u8) -> bool {
        let i = r as usize * COLS + c as usize;
        let lv = &mut self.levels[l];
        if lv.content.map[i] == b {
            return false;
        }
        lv.content.map[i] = b;
        lv.blocks_dirty = true;
        true
    }

    pub fn block(&self, l: usize, c: i32, r: i32) -> u8 {
        self.levels[l].content.map[r as usize * COLS + c as usize]
    }

    /// The level as the original cartridge has it (undoable); a new level, as its base level
    /// is there.
    pub fn restore_level(&mut self, l: usize) {
        self.snapshot(l);
        let content = level_from(&self.original, self.base_of(l));
        let lv = &mut self.levels[l];
        lv.content = content;
        lv.blocks_dirty = true;
        lv.objects_dirty = true;
    }

    /// Levels whose map is the same one as level `l`'s (a new level has its own).
    pub fn sharing(&self, l: usize) -> Vec<usize> {
        if l >= LEVELS {
            return Vec::new();
        }
        levels::sharing(&self.rom, l)
    }

    // ------------------------------------------------------------ new levels

    /// The level of the 32 that level `l` is played as (itself for those): its graphics,
    /// blocks, music and time.
    pub fn base_of(&self, l: usize) -> usize {
        if l < LEVELS {
            l
        } else {
            self.extras.get(l - LEVELS).map(|e| e.0 as usize).unwrap_or(0)
        }
    }

    /// A new level after the others, starting as a copy of level `from` as it is being
    /// edited (a new level copies its base). Returns its number, or None when there are as
    /// many as there can be.
    pub fn add_level(&mut self, from: usize) -> Option<usize> {
        if self.extras.len() >= extra_levels::MAX {
            return None;
        }
        let base = self.base_of(from);
        let header = if from < LEVELS { extra_levels::based_on(&self.rom, base as u8).header } else { self.extras[from - LEVELS].1 };
        self.extras.push((base as u8, header));
        let content = self.levels[from].content.clone();
        self.levels.push(LevelDoc { content, blocks_dirty: true, objects_dirty: true, history: History::default() });
        self.extras_dirty = true;
        Some(self.levels.len() - 1)
    }

    /// Delete new level `l`; the ones after it move down a number, and the map points follow
    /// (those that opened it open nothing).
    pub fn remove_level(&mut self, l: usize) -> bool {
        if l < LEVELS || l >= self.levels.len() {
            return false;
        }
        self.levels.remove(l);
        self.extras.remove(l - LEVELS);
        self.extras_dirty = true;
        for p in &mut self.points {
            match *p as usize {
                v if v == l => *p = world::NO_LEVEL,
                v if v > l && *p != world::NO_LEVEL => *p -= 1,
                _ => {}
            }
        }
        self.points_dirty = true;
        true
    }

    fn extras_now(&self) -> Vec<Extra> {
        self.extras.iter().zip(&self.levels[LEVELS..]).map(|(&(base, header), lv)| Extra { base, header, map: lv.content.map.clone(), objs: lv.content.objs.clone() }).collect()
    }

    /// Where a level's view should start: the first column that isn't background filler
    /// and the rows used near it. Returns (column, first row, last row).
    pub fn content_area(&self, l: usize) -> (usize, usize, usize) {
        let map = &self.levels[l].content.map;
        let mut count = [0usize; 128];
        for &b in map {
            count[b as usize & 0x7F] += 1;
        }
        let bg = (0..128).max_by_key(|&i| count[i]).unwrap_or(0) as u8;
        let c0 = (0..COLS).find(|&c| (0..ROWS).any(|r| map[r * COLS + c] != bg)).unwrap_or(0);
        let near: Vec<usize> = (0..ROWS).filter(|&r| (c0..(c0 + 30).min(COLS)).any(|c| map[r * COLS + c] != bg)).collect();
        (c0, near.first().copied().unwrap_or(0), near.last().copied().unwrap_or(12))
    }

    // ------------------------------------------------------------ objects

    /// Add an object; returns its index.
    pub fn add_object(&mut self, l: usize, o: Obj) -> usize {
        let lv = &mut self.levels[l];
        lv.content.objs.push(o);
        lv.objects_dirty = true;
        lv.content.objs.len() - 1
    }

    pub fn move_object(&mut self, l: usize, i: usize, x: u16, y: u16) {
        let lv = &mut self.levels[l];
        if let Some(o) = lv.content.objs.get_mut(i) {
            if (x, y) != (o.x, o.y) {
                o.x = x;
                o.y = y;
                lv.objects_dirty = true;
            }
        }
    }

    pub fn remove_object(&mut self, l: usize, i: usize) {
        let lv = &mut self.levels[l];
        if i < lv.content.objs.len() {
            lv.content.objs.remove(i);
            lv.objects_dirty = true;
        }
    }

    /// Give object `i` another type (undoable). False if it already had it.
    pub fn set_object_kind(&mut self, l: usize, i: usize, kind: u8) -> bool {
        if !self.levels[l].content.objs.get(i).map(|o| o.kind != kind).unwrap_or(false) {
            return false;
        }
        self.snapshot(l);
        let lv = &mut self.levels[l];
        lv.content.objs[i].kind = kind;
        lv.objects_dirty = true;
        true
    }

    // ------------------------------------------------------------ world map

    /// Remember every map picture before a change, for undo.
    pub fn world_snapshot(&mut self) {
        self.world_history.record((self.world.clone(), self.zones.clone()));
    }

    fn swap_pictures(&mut self, undo: bool) {
        let mut cur = (std::mem::take(&mut self.world), std::mem::take(&mut self.zones));
        let done = if undo { self.world_history.undo(&mut cur) } else { self.world_history.redo(&mut cur) };
        (self.world, self.zones) = cur;
        if done {
            self.world_dirty = true;
        }
    }

    pub fn world_undo(&mut self) {
        self.swap_pictures(true);
    }

    pub fn world_redo(&mut self) {
        self.swap_pictures(false);
    }

    /// Size of a map picture in tiles.
    pub fn map_size(map: OnMap) -> (usize, usize) {
        match map {
            OnMap::World => (world::W, world::H),
            OnMap::Zone(_) => (zone_maps::W, zone_maps::H),
        }
    }

    fn picture(&self, map: OnMap) -> &Vec<u8> {
        match map {
            OnMap::World => &self.world,
            OnMap::Zone(z) => &self.zones[z],
        }
    }

    pub fn set_tile(&mut self, map: OnMap, x: usize, y: usize, t: u8) {
        let i = y * Self::map_size(map).0 + x;
        let pic = match map {
            OnMap::World => &mut self.world,
            OnMap::Zone(z) => &mut self.zones[z],
        };
        if pic[i] != t {
            pic[i] = t;
            self.world_dirty = true;
        }
    }

    pub fn tile(&self, map: OnMap, x: usize, y: usize) -> u8 {
        self.picture(map)[y * Self::map_size(map).0 + x]
    }

    /// Every map picture and the level points as the original has them (the pictures
    /// undoable).
    pub fn restore_world(&mut self) {
        self.world_snapshot();
        self.world = world::read(&self.original);
        self.zones = (0..zone_maps::ZONES.len()).map(|z| zone_maps::read(&self.original, z)).collect();
        self.points_history.record((self.points.clone(), self.routes.clone()));
        self.points = world::point_levels(&self.original);
        self.routes = map_routes::read(&self.original);
        self.world_dirty = true;
        self.points_dirty = true;
        self.routes_dirty = true;
    }

    /// Does map point `point` open a level (one of the 32 or a new one)?
    pub fn point_level(&self, point: usize) -> Option<usize> {
        let l = self.points[point] as usize;
        (l < self.levels.len()).then_some(l)
    }

    /// Step the level a game map point opens: no level, 0, 1, ... the last level (the new
    /// ones too), no level (undoable). Points the game uses for something else ($FF...) are
    /// left alone.
    pub fn step_point_level(&mut self, point: usize, step: i32) {
        let cur = self.points[point];
        let count = self.levels.len() as i32;
        if step == 0 || !((cur as i32) < count || cur == world::NO_LEVEL) {
            return;
        }
        self.points_record();
        let i = if cur == world::NO_LEVEL { count } else { cur as i32 };
        let n = (i + step).rem_euclid(count + 1);
        self.points[point] = if n == count { world::NO_LEVEL } else { n as u8 };
        self.points_dirty = true;
    }

    fn points_record(&mut self) {
        self.points_history.record((self.points.clone(), self.routes.clone()));
    }

    fn points_swap(&mut self, undo: bool) -> bool {
        let mut now = (std::mem::take(&mut self.points), std::mem::take(&mut self.routes));
        let done = if undo { self.points_history.undo(&mut now) } else { self.points_history.redo(&mut now) };
        (self.points, self.routes) = now;
        self.points_dirty |= done;
        self.routes_dirty |= done;
        done
    }

    pub fn points_undo(&mut self) -> bool {
        self.points_swap(true)
    }

    pub fn points_redo(&mut self) -> bool {
        self.points_swap(false)
    }

    // ------------------------------------------------------------ paths and new points

    /// Set or take out the way from `point` in direction `dir` (0 right, 1 left, 2 up,
    /// 3 down), undoable.
    pub fn set_way(&mut self, point: usize, dir: usize, way: Option<Way>) {
        if point >= map_routes::POINTS || self.routes.ways[point][dir] == way {
            return;
        }
        self.points_record();
        self.routes.ways[point][dir] = way;
        self.routes_dirty = true;
    }

    /// A path drawn from `from` through `corners` (pixels from `from`) to point `to`: the
    /// way out of `from` in the direction it starts in, and the way back the same road if
    /// `to` has none in the direction it would start in. Returns the directions used.
    pub fn draw_path(&mut self, from: usize, to: u8, corners: &[(i32, i32)]) -> Option<(usize, Option<usize>)> {
        let way = Way::through(corners, to);
        let dir = first_dir(&way)?;
        self.points_record();
        let end = *way.track().last()?;
        let back_corners: Vec<(i32, i32)> = corners.iter().rev().skip(1).map(|c| (c.0 - end.0, c.1 - end.1)).chain(std::iter::once((-end.0, -end.1))).collect();
        let back = Way::through(&back_corners, from as u8);
        let mut back_dir = None;
        if let Some(bd) = first_dir(&back) {
            let t = to as usize;
            if t < map_routes::POINTS && self.routes.ways[t][bd].is_none() {
                self.routes.ways[t][bd] = Some(back);
                back_dir = Some(bd);
            }
        }
        self.routes.ways[from][dir] = Some(way);
        self.routes_dirty = true;
        Some((dir, back_dir))
    }

    /// A new point on the world map at `(x, y)` (pixels of the picture), opening no level
    /// yet. None when every free number is taken.
    pub fn add_point(&mut self, x: i32, y: i32) -> Option<u8> {
        let p = self.routes.free_point()?;
        self.points_record();
        self.routes.added.push(p);
        self.routes.ways[p as usize] = Default::default();
        self.routes.places[p as usize] = Place::at_world(x, y);
        self.points[p as usize] = world::NO_LEVEL;
        self.routes_dirty = true;
        self.points_dirty = true;
        Some(p)
    }

    /// Take out a new point and every path to it.
    pub fn remove_point(&mut self, p: u8) -> bool {
        if !self.routes.added.contains(&p) {
            return false;
        }
        self.points_record();
        self.routes.remove_point(p);
        self.routes.places[p as usize] = map_routes::read(&self.original).places[p as usize];
        self.points[p as usize] = world::NO_LEVEL;
        self.routes_dirty = true;
        self.points_dirty = true;
        true
    }

    // ------------------------------------------------------------ space and saving

    /// The edited maps of one bank: (map number, blocks), one per map.
    pub fn bank_edits(&self, bank: u8) -> Vec<(u8, &[u8])> {
        let mut v: Vec<(u8, &[u8])> = Vec::new();
        for l in 0..LEVELS {
            let h = levels::header(&self.rom, l);
            if h.bank == bank && self.levels[l].blocks_dirty && !v.iter().any(|e| e.0 == h.map) {
                v.push((h.map, self.levels[l].content.map.as_slice()));
            }
        }
        v
    }

    /// Bytes level `l`'s zone needs and has (a new level: its own map and the room it gets).
    pub fn zone_space(&self, l: usize) -> (usize, usize) {
        if l >= LEVELS {
            return (levels::encode(&self.levels[l].content.map).len(), extra_levels::MAP_ROOM);
        }
        let bank = levels::header(&self.rom, l).bank;
        layout::bank_space(&self.rom, &self.original, bank, &self.bank_edits(bank))
    }

    fn object_lists(&self) -> Vec<Vec<Obj>> {
        self.levels[..LEVELS].iter().map(|l| l.content.objs.clone()).collect()
    }

    /// Bytes the object lists need and have: all those of the 32 levels together, or a new
    /// level's own (`l`).
    pub fn object_space(&self, l: usize) -> (usize, usize) {
        if l >= LEVELS {
            return (self.levels[l].content.objs.len() * 3 + 1, extra_levels::OBJECTS_ROOM);
        }
        objects::space(&self.rom, &self.original, &self.object_lists())
    }

    /// Write every change into the cartridge image (`rom`). Zones already written stay
    /// written if a later one doesn't fit.
    pub fn commit(&mut self) -> Result<Vec<Saved>, CartError> {
        let mut notes = Vec::new();
        let mut banks: Vec<u8> = (0..LEVELS).filter(|&l| self.levels[l].blocks_dirty).map(|l| levels::header(&self.rom, l).bank).collect();
        banks.sort();
        banks.dedup();
        for bank in banks {
            let edits: Vec<(u8, Vec<u8>)> = self.bank_edits(bank).into_iter().map(|(m, d)| (m, d.to_vec())).collect();
            let refs: Vec<(u8, &[u8])> = edits.iter().map(|(m, d)| (*m, d.as_slice())).collect();
            let mut rom = self.rom.clone();
            notes.push(Saved::Bank(layout::store_bank(&mut rom, &self.original, bank, &refs)?));
            self.rom = rom;
            for l in 0..LEVELS {
                let h = levels::header(&self.rom, l);
                if h.bank != bank {
                    continue;
                }
                // Levels that share an edited map see the change too.
                if let Some((_, d)) = edits.iter().find(|e| e.0 == h.map) {
                    self.levels[l].content.map = d.clone();
                }
                self.levels[l].blocks_dirty = false;
            }
        }
        if self.world_dirty {
            world::write(&mut self.rom, &self.world);
            for (z, pic) in self.zones.iter().enumerate() {
                zone_maps::write(&mut self.rom, z, pic);
            }
            self.world_dirty = false;
            notes.push(Saved::WorldMap);
        }
        if self.points_dirty {
            world::write_point_levels(&mut self.rom, &self.points);
            self.points_dirty = false;
            notes.push(Saved::LevelPoints);
        }
        if self.routes_dirty {
            let mut rom = self.rom.clone();
            notes.push(Saved::Routes(map_routes::write(&mut rom, &self.routes)?));
            self.rom = rom;
            self.routes_dirty = false;
        }
        if self.levels[..LEVELS].iter().any(|l| l.objects_dirty) {
            let mut rom = self.rom.clone();
            notes.push(Saved::Objects(objects::store(&mut rom, &self.original, &self.object_lists())?));
            self.rom = rom;
            for l in 0..LEVELS {
                // what the game will read: page by page, as stored
                self.levels[l].content.objs = objects::read(&self.rom, l).0;
                self.levels[l].objects_dirty = false;
            }
        }
        if self.extras_dirty || self.levels[LEVELS..].iter().any(|l| l.dirty()) {
            let mut rom = self.rom.clone();
            let used = extra_levels::store(&mut rom, &self.extras_now())?;
            self.rom = rom;
            for (k, e) in extra_levels::read(&self.rom).into_iter().enumerate() {
                let lv = &mut self.levels[LEVELS + k];
                lv.content.objs = e.objs; // stored left to right, as the game keeps them
                lv.blocks_dirty = false;
                lv.objects_dirty = false;
            }
            self.extras_dirty = false;
            notes.push(Saved::NewLevels(self.extras.len(), used));
        }
        Ok(notes)
    }
}

/// The pad direction a way starts in (0 right, 1 left, 2 up, 3 down): its first step's;
/// a diagonal one goes by where the way ends up most.
fn first_dir(w: &Way) -> Option<usize> {
    use map_routes::{DOWN, LEFT, RIGHT, UP};
    let s = *w.steps.first()?;
    let end = *w.track().last()?;
    let h = if s & RIGHT != 0 { Some(0) } else if s & LEFT != 0 { Some(1) } else { None };
    let v = if s & UP != 0 { Some(2) } else if s & DOWN != 0 { Some(3) } else { None };
    match (h, v) {
        (Some(h), Some(v)) => Some(if end.0.abs() >= end.1.abs() { h } else { v }),
        (h, v) => h.or(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> Option<EditorDocument> {
        let rom = host::find_rom(None)?;
        Some(EditorDocument::open(std::fs::read(rom).ok()?, None))
    }

    #[test]
    fn paint_undo_redo_and_save_into_the_cartridge() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let before = doc.levels[3].content.clone();
        let old = doc.block(3, 10, 5);
        let new = (old + 1) & 0x7F;
        doc.snapshot(3);
        assert!(doc.set_block(3, 10, 5, new));
        assert!(!doc.set_block(3, 10, 5, new), "same block twice is no change");
        assert!(doc.levels[3].dirty());
        assert!(doc.undo(3));
        assert_eq!(doc.levels[3].content, before);
        assert!(doc.redo(3));
        assert_eq!(doc.block(3, 10, 5), new);

        let i = doc.add_object(3, Obj { x: 64, y: 32, kind: 5, flag: false });
        doc.move_object(3, i, 72, 40);
        doc.set_tile(OnMap::World, 1, 1, doc.tile(OnMap::World, 1, 1) ^ 1);
        let tile = doc.tile(OnMap::World, 1, 1);
        let zone = OnMap::Zone(2);
        doc.world_snapshot();
        doc.set_tile(zone, 19, 17, doc.tile(zone, 19, 17) ^ 1);
        let zone_tile = doc.tile(zone, 19, 17);
        let parts = doc.commit().unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(parts[0], Saved::Bank(_)));
        assert!(parts.iter().any(|p| matches!(p, Saved::WorldMap)));
        assert!(parts.iter().any(|p| matches!(p, Saved::Objects(_))));
        assert!(!doc.levels.iter().any(|l| l.dirty()) && !doc.world_dirty);

        // Read back from the cartridge image, as the game will.
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.block(3, 10, 5), new);
        assert!(again.levels[3].content.objs.iter().any(|o| (o.x, o.y, o.kind) == (72, 40, 5)));
        assert_eq!(again.tile(OnMap::World, 1, 1), tile);
        assert_eq!(again.tile(zone, 19, 17), zone_tile);
        assert_eq!(again.zones[0], doc.zones[0], "other zone maps untouched");
        assert_eq!(again.levels[4].content, doc.levels[4].content, "other levels untouched");
    }

    #[test]
    fn restoring_brings_back_the_original_and_can_be_undone() {
        let Some(mut doc) = open() else { return };
        let original = doc.levels[0].content.clone();
        doc.snapshot(0);
        doc.remove_object(0, 0);
        let edited = doc.levels[0].content.clone();
        doc.restore_level(0);
        assert_eq!(doc.levels[0].content, original);
        assert!(doc.undo(0));
        assert_eq!(doc.levels[0].content, edited);

        doc.step_point_level(0, 1);
        assert!(doc.points_dirty);
        let zone = OnMap::Zone(5);
        let before = doc.tile(zone, 3, 3);
        doc.world_snapshot();
        doc.set_tile(zone, 3, 3, before ^ 1);
        doc.world_undo();
        assert_eq!(doc.tile(zone, 3, 3), before, "zone maps are undone with the world map");
        doc.world_redo();
        doc.restore_world();
        assert_eq!(doc.points, world::point_levels(&doc.original));
        assert_eq!(doc.tile(zone, 3, 3), before);
    }

    #[test]
    fn point_levels_undo() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let before = doc.points.clone();
        doc.step_point_level(0, 2);
        assert_eq!(doc.points[0], before[0] + 2);
        assert!(doc.points_undo());
        assert_eq!(doc.points, before);
        assert!(doc.points_redo() && doc.points[0] == before[0] + 2);
    }

    #[test]
    fn new_levels_are_added_saved_and_removed() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(5, 10, 5, 3);
        let n = doc.add_level(5).unwrap();
        assert_eq!((n, doc.base_of(n)), (LEVELS, 5));
        assert_eq!(doc.block(n, 10, 5), 3, "a copy of the level as it is being edited");
        let m = doc.add_level(n).unwrap();
        assert_eq!(doc.base_of(m), 5, "a copy of a new level has the same base");
        doc.set_block(m, 20, 5, 4);
        doc.add_object(m, Obj { x: 80, y: 40, kind: 5, flag: false });
        doc.points[0] = m as u8; // the game's first point opens the second new level
        doc.points[1] = n as u8;
        doc.points_dirty = true;
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.points[0], again.point_level(1)), (m as u8, Some(n)));
        assert_eq!(again.levels.len(), LEVELS + 2);
        assert_eq!(again.levels[m].content, doc.levels[m].content);
        assert_eq!(again.base_of(m), 5);
        assert!(doc.remove_level(n) && !doc.remove_level(3));
        assert_eq!(doc.points[0], n as u8, "the point follows its level");
        assert_eq!(doc.points[1], world::NO_LEVEL, "a point to a deleted level opens nothing");
        assert_eq!(doc.levels[n].content, again.levels[m].content);
    }

    #[test]
    fn paths_and_new_points_are_drawn_saved_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let original = doc.routes.clone();
        // a new point 32 pixels above point 04, a path up to it (and back down), level 7
        let (x, y) = doc.routes.places[4].on_world();
        let p = doc.add_point(x, y - 32).unwrap();
        assert_eq!(doc.draw_path(4, p, &[(0, -32)]), Some((2, Some(3))));
        doc.step_point_level(p as usize, 8); // no level -> 7
        assert_eq!(doc.point_level(p as usize), Some(7));
        assert_eq!(doc.routes.ways[p as usize][3].as_ref().map(|w| (w.to, w.track().last().copied())), Some((4, Some((0, 32)))));
        let parts = doc.commit().unwrap();
        assert!(parts.iter().any(|s| matches!(s, Saved::Routes(_))));
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.routes, doc.routes);
        assert_eq!(again.point_level(p as usize), Some(7));
        // undo the level, the path and the point
        assert!(doc.points_undo() && doc.points_undo() && doc.points_undo());
        assert_eq!(doc.routes, original);
        assert!(doc.points_redo() && doc.remove_point(p));
        assert_eq!(doc.routes, original, "taking the point out takes its paths");
    }
}
