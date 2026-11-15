//! Everything being edited: the cartridge being built (the edited copy), the untouched
//! original it is checked against, the 32 levels (blocks and objects) with their undo
//! history, the world map picture and level points, and the editor-made world map. What
//! has changed since the last save is tracked here; the files are not.
use super::history::History;
use cart_model::custom_map::CustomMap;
use cart_model::error::{BankUsage, ObjectsUsage};
use cart_model::levels::{self, COLS, LEVELS, ROWS};
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
    Objects(ObjectsUsage),
    CustomMap,
}

pub struct EditorDocument {
    pub original: Vec<u8>,
    pub rom: Vec<u8>,
    pub levels: Vec<LevelDoc>,
    /// The world map picture (64 x 64 tiles) and the zone maps' (20 x 18 each).
    pub world: Vec<u8>,
    pub zones: Vec<Vec<u8>>,
    /// Any map picture changed.
    pub world_dirty: bool,
    world_history: History<(Vec<u8>, Vec<Vec<u8>>)>,
    /// Which level each of the game's own map points opens.
    pub points: Vec<u8>,
    pub points_dirty: bool,
    pub custom: CustomMap,
    pub custom_dirty: bool,
}

fn level_from(rom: &[u8], l: usize) -> LevelContent {
    let (_, start) = levels::map_location(rom, levels::header(rom, l));
    LevelContent { map: levels::decode(rom, start).0, objs: objects::read(rom, l).0 }
}

impl EditorDocument {
    /// Start from the edited copy if there is a usable one; otherwise from a new copy of the
    /// original, made 1 MB so every zone has room for bigger levels (`patch::expand`).
    pub fn open(original: Vec<u8>, edited: Option<Vec<u8>>, custom: CustomMap) -> EditorDocument {
        let mut rom = edited
            .filter(|r| r.len() == original.len() || patch::is_expanded(r))
            .unwrap_or_else(|| patch::expand(&original).unwrap_or_else(|_| original.clone()));
        patch::upgrade(&mut rom); // copies made before the object patch get it now
        let levels = (0..LEVELS)
            .map(|l| LevelDoc { content: level_from(&rom, l), blocks_dirty: false, objects_dirty: false, history: History::default() })
            .collect();
        EditorDocument {
            world: world::read(&rom),
            zones: zone_maps::ZONES.iter().enumerate().map(|(z, _)| zone_maps::read(&rom, z)).collect(),
            world_dirty: false,
            world_history: History::default(),
            points: world::point_levels(&rom),
            points_dirty: false,
            custom,
            custom_dirty: false,
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

    /// The level as the original cartridge has it (undoable).
    pub fn restore_level(&mut self, l: usize) {
        self.snapshot(l);
        let lv = &mut self.levels[l];
        lv.content = level_from(&self.original, l);
        lv.blocks_dirty = true;
        lv.objects_dirty = true;
    }

    /// Levels whose map is the same one as level `l`'s.
    pub fn sharing(&self, l: usize) -> Vec<usize> {
        levels::sharing(&self.rom, l)
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
        self.points = world::point_levels(&self.original);
        self.world_dirty = true;
        self.points_dirty = true;
    }

    /// Step the level a game map point opens: no level, 0, 1, ... 31, no level.
    pub fn step_point_level(&mut self, point: usize, step: i32) {
        let cur = self.points[point];
        if step == 0 || !(cur < 32 || cur == world::NO_LEVEL) {
            return;
        }
        let i = if cur == world::NO_LEVEL { 32 } else { cur as i32 };
        let n = (i + step).rem_euclid(33);
        self.points[point] = if n == 32 { world::NO_LEVEL } else { n as u8 };
        self.points_dirty = true;
    }

    /// Add the game's own points (where it draws them, with the levels they open now) to the
    /// editor-made map, leaving out any already there. Returns how many were added.
    pub fn add_original_points(&mut self) -> usize {
        let original = CustomMap::from_original(&self.points);
        let was_empty = self.custom.points.is_empty();
        let mut added = 0;
        for p in original.points {
            if !self.custom.points.iter().any(|q| (q.map, q.x, q.y) == (p.map, p.x, p.y)) {
                self.custom.points.push(p);
                added += 1;
            }
        }
        if was_empty {
            self.custom.start = original.start;
        }
        self.custom_dirty |= added > 0;
        added
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

    /// Bytes level `l`'s zone needs and has.
    pub fn zone_space(&self, l: usize) -> (usize, usize) {
        let bank = levels::header(&self.rom, l).bank;
        layout::bank_space(&self.rom, &self.original, bank, &self.bank_edits(bank))
    }

    fn object_lists(&self) -> Vec<Vec<Obj>> {
        self.levels.iter().map(|l| l.content.objs.clone()).collect()
    }

    /// Bytes all the object lists need and have.
    pub fn object_space(&self) -> (usize, usize) {
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
        if self.levels.iter().any(|l| l.objects_dirty) {
            let mut rom = self.rom.clone();
            notes.push(Saved::Objects(objects::store(&mut rom, &self.original, &self.object_lists())?));
            self.rom = rom;
            for l in 0..LEVELS {
                // what the game will read: page by page, as stored
                self.levels[l].content.objs = objects::read(&self.rom, l).0;
                self.levels[l].objects_dirty = false;
            }
        }
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> Option<EditorDocument> {
        let rom = host::find_rom(None)?;
        Some(EditorDocument::open(std::fs::read(rom).ok()?, None, CustomMap::default()))
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
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()), CustomMap::default());
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

        let n = doc.add_original_points();
        assert!(n > 30 && doc.custom_dirty);
        assert_eq!(doc.custom.points[doc.custom.start].level, Some(0), "Mario starts on the first level");
        assert_eq!(doc.add_original_points(), 0, "points already there are not added twice");
    }
}
