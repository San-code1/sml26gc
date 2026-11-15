//! Everything being edited: the cartridge being built (the edited copy), the untouched
//! original it is checked against, the 32 levels (blocks and objects) and the new ones
//! after them (`extra_levels`) with their undo history, the world map picture, the level
//! each map point opens and the paths between points (`map_routes`), and the new zones
//! (`zones`). What has changed since the last save is tracked here; the files are not.
mod restore;
mod zones;

pub use zones::{ZonePoints, ZoneRefusal};
use super::files::{self, FileError, LevelFile, ProjectFile};
use super::history::History;
use super::stamp::{Area, Stamp};
use cart_model::error::{BankUsage, ObjectsUsage};
use cart_model::extra_levels::{self, Extra};
use cart_model::levels::{self, COLS, LEVELS, ROWS};
use cart_model::doors::{self, Doors, Leads};
use cart_model::map_routes::{self, MapRoutes, Place, Way};
use cart_model::objects::{self, Obj};
use cart_model::original_points::OnMap;
use cart_model::{layout, new_zones, patch, world, zone_maps, CartError};

/// What a painting tool works on: a level's blocks or a map picture's tiles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grid {
    Level(usize),
    Map(OnMap),
}

/// What a new level is played as: one of the 32 (its graphics, music and time) with the new
/// level's own header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StandIn {
    pub base: u8,
    pub header: [u8; 20],
}

/// A level as it can be undone: its blocks, its objects and, for a new level, its stand-in.
#[derive(Clone, PartialEq, Debug)]
pub struct LevelContent {
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
    /// None for the 32 levels of the cartridge.
    pub stand_in: Option<StandIn>,
    /// A new level's name (empty: none).
    pub name: String,
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
    /// Which points enter which zone and where the zones' exits lead.
    Doors,
    Objects(ObjectsUsage),
    /// The new levels: how many, bytes used.
    NewLevels(usize, usize),
    /// The new zones: how many, bytes used.
    NewZones(usize, usize),
}

pub struct EditorDocument {
    pub original: Vec<u8>,
    pub rom: Vec<u8>,
    /// The 32 levels, then the new ones (number 32 and up).
    pub levels: Vec<LevelDoc>,
    /// A new level was added or removed.
    pub extras_dirty: bool,
    /// The world map picture (64 x 64 tiles) and the zone maps' (20 x 18 each): the game's
    /// eight, then the new zones'.
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
    /// Doors between the world map and the zones.
    pub doors: Doors,
    pub doors_dirty: bool,
    /// The new zones' points, paths, exits and doors (`zones`).
    pub new_zones: Vec<ZonePoints>,
    pub new_zones_dirty: bool,
    /// The point each zone a door enters is entered on (`doors::INTO` order).
    pub entries: Vec<u8>,
    /// Undo of the Points tab: the levels the points open, the paths, the doors, the new
    /// zones' points and the zones' entry points, together.
    points_history: History<PointsState>,
}

type PointsState = (Vec<u8>, MapRoutes, Doors, Vec<ZonePoints>, Vec<u8>);

/// The point each zone a door enters is entered on in `rom`.
fn entries_of(rom: &[u8]) -> Vec<u8> {
    doors::INTO.iter().map(|d| new_zones::entry_point(rom, d.zone).unwrap_or(doors::NONE)).collect()
}

fn level_from(rom: &[u8], l: usize) -> LevelContent {
    let (_, start) = levels::map_location(rom, levels::header(rom, l));
    LevelContent { map: levels::decode(rom, start).0, objs: objects::read(rom, l).0, stand_in: None, name: String::new() }
}

impl EditorDocument {
    /// Start from the edited copy if there is a usable one; otherwise from a new copy of the
    /// original, made 1 MB so every zone has room for bigger levels (`patch::expand`).
    pub fn open(original: Vec<u8>, edited: Option<Vec<u8>>) -> EditorDocument {
        let mut rom = edited
            .filter(|r| r.len() == original.len() || patch::is_expanded(r))
            .unwrap_or_else(|| patch::expand(&original).unwrap_or_else(|_| original.clone()));
        patch::upgrade(&mut rom); // copies made before the object patch get it now
        let mut routes = map_routes::read(&rom);
        // copies saved before zone exits followed their entrance get it now
        let routes_dirty = routes.sync_exits();
        let doc = |content| LevelDoc { content, blocks_dirty: false, objects_dirty: false, history: History::default() };
        let mut levels: Vec<LevelDoc> = (0..LEVELS).map(|l| doc(level_from(&rom, l))).collect();
        let stored = extra_levels::read(&rom);
        levels.extend(stored.into_iter().map(|e| doc(LevelContent { map: e.map, objs: e.objs, stand_in: Some(StandIn { base: e.base, header: e.header }), name: e.name })));
        let mut zones: Vec<Vec<u8>> = (0..zone_maps::ZONES.len()).map(|z| zone_maps::read(&rom, z)).collect();
        let (new_zones, pictures): (Vec<ZonePoints>, Vec<Vec<u8>>) = new_zones::read(&rom).into_iter().map(ZonePoints::from).unzip();
        zones.extend(pictures);
        // the copy keeps an exit into a new zone as one to its door
        let mut doors = doors::read(&rom);
        for e in &mut doors.exits {
            if let Leads::World(p) = e.1 {
                if let Some(k) = new_zones.iter().position(|z| z.door == p && p != doors::NONE) {
                    e.1 = Leads::NewZone(k);
                }
            }
        }
        EditorDocument {
            extras_dirty: false,
            world: world::read(&rom),
            zones,
            world_dirty: false,
            world_history: History::default(),
            points: world::point_levels(&rom),
            points_dirty: false,
            routes,
            routes_dirty,
            doors,
            doors_dirty: false,
            new_zones,
            new_zones_dirty: false,
            entries: entries_of(&rom),
            points_history: History::default(),
            original,
            rom,
            levels,
        }
    }

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
    /// is there (header too), keeping its name.
    pub fn restore_level(&mut self, l: usize) {
        self.snapshot(l);
        let base = self.base_of(l);
        let mut content = level_from(&self.original, base);
        let at = levels::header(&self.original, base).at;
        let lv = &mut self.levels[l];
        content.stand_in = lv.content.stand_in.map(|s| StandIn { header: self.original[at..at + 20].try_into().unwrap_or(s.header), ..s });
        content.name = std::mem::take(&mut lv.content.name);
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

    /// The level of the 32 that level `l` is played as (itself for those): its graphics,
    /// blocks, music and time.
    pub fn base_of(&self, l: usize) -> usize {
        self.levels.get(l).and_then(|lv| lv.content.stand_in).map(|s| s.base as usize).unwrap_or(l.min(LEVELS - 1))
    }

    /// How many new levels there are.
    pub fn new_levels(&self) -> usize {
        self.levels.len() - LEVELS
    }

    /// A new level after the others, starting as a copy of level `from` as it is being
    /// edited (a new level copies its base). Returns its number, or None when there are as
    /// many as there can be.
    pub fn add_level(&mut self, from: usize) -> Option<usize> {
        let mut content = self.levels[from].content.clone();
        let base = self.base_of(from) as u8;
        content.stand_in = content.stand_in.or_else(|| Some(StandIn { base, header: extra_levels::based_on(&self.rom, base).header }));
        self.push_level(content)
    }

    /// Add `content` (a new level's) after the others, or None when there is no room.
    fn push_level(&mut self, content: LevelContent) -> Option<usize> {
        if self.new_levels() >= extra_levels::MAX {
            return None;
        }
        self.levels.push(LevelDoc { content, blocks_dirty: true, objects_dirty: true, history: History::default() });
        self.extras_dirty = true;
        Some(self.levels.len() - 1)
    }

    /// Level `l` as a file keeps it; one of the 32 is its own base, with its header.
    pub fn level_file(&self, l: usize) -> LevelFile {
        let c = &self.levels[l].content;
        let stand_in = c.stand_in.unwrap_or_else(|| {
            let at = levels::header(&self.rom, l).at;
            StandIn { base: l as u8, header: self.rom[at..at + 20].try_into().unwrap_or([0; 20]) }
        });
        LevelFile { stand_in, map: c.map.clone(), objs: c.objs.clone(), name: c.name.clone() }
    }

    /// A new level after the others from a level file; None when there is no room.
    pub fn import_level(&mut self, f: LevelFile) -> Option<usize> {
        self.push_level(LevelContent { map: f.map, objs: f.objs, stand_in: Some(f.stand_in), name: f.name })
    }

    /// Replace new level `l` whole with a level file (undoable). The 32 are never replaced.
    pub fn replace_level(&mut self, l: usize, f: LevelFile) -> bool {
        if l < LEVELS || l >= self.levels.len() {
            return false;
        }
        self.snapshot(l);
        let lv = &mut self.levels[l];
        lv.content = LevelContent { map: f.map, objs: f.objs, stand_in: Some(f.stand_in), name: f.name };
        lv.blocks_dirty = true;
        lv.objects_dirty = true;
        true
    }

    /// Name new level `l` (undoable); the 32 have the game's numbers only.
    pub fn rename_level(&mut self, l: usize, name: &str) {
        let name = cart_model::extra_levels::clean_name(name);
        if l < LEVELS || l >= self.levels.len() || self.levels[l].content.name == name {
            return;
        }
        self.snapshot(l);
        self.levels[l].content.name = name;
        self.extras_dirty = true;
    }

    /// The name of level `l` (empty: none).
    pub fn level_name(&self, l: usize) -> &str {
        self.levels.get(l).map_or("", |lv| &lv.content.name)
    }

    /// Everything being edited, as a project file keeps it.
    pub fn project_file(&self) -> ProjectFile {
        ProjectFile {
            cartridge: files::cartridge_id(&self.original),
            levels: (0..self.levels.len()).map(|l| self.level_file(l)).collect(),
            world: self.world.clone(),
            zones: self.zones[..zone_maps::ZONES.len()].to_vec(),
            points: self.points.clone(),
            routes: self.routes.clone(),
            doors: self.doors.clone(),
            new_zones: (0..self.new_zones.len()).map(|k| self.new_zone(k)).collect(),
            entries: self.entries.clone(),
        }
    }

    /// Put a project file in place of everything being edited: all of it marked changed,
    /// nothing written until the next save, every undo history emptied. Refused when the
    /// file was made over another cartridge.
    pub fn replace_project(&mut self, p: ProjectFile) -> Result<(), FileError> {
        if p.cartridge != files::cartridge_id(&self.original) {
            return Err(FileError::OtherCartridge);
        }
        self.levels = p
            .levels
            .into_iter()
            .enumerate()
            .map(|(l, f)| LevelDoc { content: LevelContent { map: f.map, objs: f.objs, stand_in: (l >= LEVELS).then_some(f.stand_in), name: if l >= LEVELS { f.name } else { String::new() } }, blocks_dirty: true, objects_dirty: true, history: History::default() })
            .collect();
        self.extras_dirty = true;
        (self.world, self.zones) = (p.world, p.zones);
        self.world_dirty = true;
        self.world_history = History::default();
        (self.points, self.routes, self.doors) = (p.points, p.routes, p.doors);
        self.entries = if p.entries.len() == doors::INTO.len() { p.entries } else { entries_of(&self.original) };
        self.new_zones_dirty = !self.new_zones.is_empty() || !p.new_zones.is_empty();
        let count = p.new_zones.len();
        let (zones, pictures): (Vec<ZonePoints>, Vec<Vec<u8>>) = p.new_zones.into_iter().map(|z| ZonePoints::from(zones::settled(z, count))).unzip();
        self.new_zones = zones;
        self.zones.extend(pictures);
        for e in &mut self.doors.exits {
            if matches!(e.1, Leads::NewZone(j) if j >= count) {
                e.1 = Leads::World(doors::NONE);
            }
        }
        self.points_dirty = true;
        self.routes_dirty = true;
        self.doors_dirty = true;
        self.points_history = History::default();
        Ok(())
    }

    /// Delete new level `l`; the ones after it move down a number, and the map points follow
    /// (those that opened it open nothing).
    pub fn remove_level(&mut self, l: usize) -> bool {
        if l < LEVELS || l >= self.levels.len() {
            return false;
        }
        self.levels.remove(l);
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
        self.levels[LEVELS..]
            .iter()
            .filter_map(|lv| lv.content.stand_in.map(|s| Extra { base: s.base, header: s.header, map: lv.content.map.clone(), objs: lv.content.objs.clone(), name: lv.content.name.clone() }))
            .collect()
    }

    /// Where a level's view should start: the first column that isn't background filler
    /// and the rows used near it. Returns (column, first row, last row).
    pub fn content_area(&self, l: usize) -> (usize, usize, usize) {
        let map = &self.levels[l].content.map;
        let bg = self.background(Grid::Level(l));
        let c0 = (0..COLS).find(|&c| (0..ROWS).any(|r| map[r * COLS + c] != bg)).unwrap_or(0);
        let near: Vec<usize> = (0..ROWS).filter(|&r| (c0..(c0 + 30).min(COLS)).any(|c| map[r * COLS + c] != bg)).collect();
        (c0, near.first().copied().unwrap_or(0), near.last().copied().unwrap_or(12))
    }

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
            OnMap::Zone(_) | OnMap::NewZone(_) => (zone_maps::W, zone_maps::H),
        }
    }

    fn picture(&self, map: OnMap) -> &Vec<u8> {
        match map {
            OnMap::World => &self.world,
            OnMap::Zone(z) => &self.zones[z],
            OnMap::NewZone(k) => &self.zones[zone_maps::ZONES.len() + k],
        }
    }

    pub fn set_tile(&mut self, map: OnMap, x: usize, y: usize, t: u8) {
        let i = y * Self::map_size(map).0 + x;
        let pic = match map {
            OnMap::World => &mut self.world,
            OnMap::Zone(z) => &mut self.zones[z],
            OnMap::NewZone(k) => &mut self.zones[zone_maps::ZONES.len() + k],
        };
        if pic[i] != t {
            pic[i] = t;
            self.world_dirty = true;
        }
    }

    pub fn tile(&self, map: OnMap, x: usize, y: usize) -> u8 {
        self.picture(map)[y * Self::map_size(map).0 + x]
    }

    pub fn grid_size(&self, g: Grid) -> (usize, usize) {
        match g {
            Grid::Level(_) => (COLS, ROWS),
            Grid::Map(m) => Self::map_size(m),
        }
    }

    fn inside(&self, g: Grid, x: i32, y: i32) -> bool {
        let (w, h) = self.grid_size(g);
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h
    }

    /// The block or tile at `(x, y)`, if it is inside.
    pub fn cell(&self, g: Grid, x: i32, y: i32) -> Option<u8> {
        self.inside(g, x, y).then(|| match g {
            Grid::Level(l) => self.block(l, x, y),
            Grid::Map(m) => self.tile(m, x as usize, y as usize),
        })
    }

    /// Put a block or tile; outside the grid nothing happens. True if something changed.
    pub fn set_cell(&mut self, g: Grid, x: i32, y: i32, v: u8) -> bool {
        if !self.inside(g, x, y) {
            return false;
        }
        match g {
            Grid::Level(l) => self.set_block(l, x, y, v),
            Grid::Map(m) => {
                let changed = self.tile(m, x as usize, y as usize) != v;
                self.set_tile(m, x as usize, y as usize, v);
                changed
            }
        }
    }

    /// Remember the grid before a change, for undo.
    pub fn grid_snapshot(&mut self, g: Grid) {
        match g {
            Grid::Level(l) => self.snapshot(l),
            Grid::Map(_) => self.world_snapshot(),
        }
    }

    /// The block the level uses most, or the tile the map uses most.
    pub fn background(&self, g: Grid) -> u8 {
        let mut count = [0usize; 256];
        match g {
            Grid::Level(l) => self.levels[l].content.map.iter().for_each(|&b| count[b as usize & 0x7F] += 1),
            Grid::Map(m) => self.picture(m).iter().for_each(|&t| count[t as usize] += 1),
        }
        (0..256).max_by_key(|&i| count[i]).unwrap_or(0) as u8
    }

    /// The part of `a` inside the grid, if any.
    pub fn clip(&self, g: Grid, a: Area) -> Option<Area> {
        let (w, h) = self.grid_size(g);
        let c = Area { x0: a.x0.max(0), y0: a.y0.max(0), x1: a.x1.min(w as i32 - 1), y1: a.y1.min(h as i32 - 1) };
        (c.x0 <= c.x1 && c.y0 <= c.y1).then_some(c)
    }

    /// The cells of `a` (the part inside the grid).
    pub fn copy_rect(&self, g: Grid, a: Area) -> Option<Stamp> {
        let a = self.clip(g, a)?;
        let cells = a.cells().map(|(x, y)| self.cell(g, x, y).unwrap_or(0)).collect();
        Some(Stamp { w: a.w(), h: a.h(), cells })
    }

    /// Fill `a` with the background, undoable.
    pub fn clear_rect(&mut self, g: Grid, a: Area) {
        let Some(a) = self.clip(g, a) else { return };
        let bg = self.background(g);
        self.grid_snapshot(g);
        for (x, y) in a.cells() {
            self.set_cell(g, x, y, bg);
        }
    }

    /// Does map point `point` open a level (one of the 32 or a new one)?
    #[cfg(test)]
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
        self.points_history.record((self.points.clone(), self.routes.clone(), self.doors.clone(), self.new_zones.clone(), self.entries.clone()));
    }

    fn points_swap(&mut self, undo: bool) -> bool {
        let mut now = (std::mem::take(&mut self.points), std::mem::take(&mut self.routes), std::mem::take(&mut self.doors), std::mem::take(&mut self.new_zones), std::mem::take(&mut self.entries));
        let done = if undo { self.points_history.undo(&mut now) } else { self.points_history.redo(&mut now) };
        (self.points, self.routes, self.doors, self.new_zones, self.entries) = now;
        self.points_dirty |= done;
        self.routes_dirty |= done;
        self.doors_dirty |= done;
        self.new_zones_dirty |= done;
        done
    }

    /// Make world point `p` enter zone `zone` (`doors::INTO` index) or nothing, undoable; a
    /// door into a new zone there is taken out.
    pub fn set_door(&mut self, p: u8, zone: Option<usize>) {
        if self.doors.into.is_empty() || self.doors.door_of(p) == zone {
            return;
        }
        self.points_record();
        self.doors.set_door(p, zone);
        self.doors_dirty = true;
        if zone.is_some() {
            for z in self.new_zones.iter_mut().filter(|z| z.door == p) {
                z.door = doors::NONE;
                self.new_zones_dirty = true;
            }
        }
    }

    /// Set where exit `i` (`doors::EXITS` index) is and leads, undoable.
    pub fn set_exit(&mut self, i: usize, at: u8, leads: Leads) {
        if self.doors.exits.get(i) == Some(&(at, leads)) || i >= self.doors.exits.len() {
            return;
        }
        self.points_record();
        self.doors.exits[i] = (at, leads);
        self.doors_dirty = true;
    }

    pub fn points_undo(&mut self) -> bool {
        self.points_swap(true)
    }

    pub fn points_redo(&mut self) -> bool {
        self.points_swap(false)
    }

    /// Set or take out the way from `point` in direction `dir` (0 right, 1 left, 2 up,
    /// 3 down), undoable. A zone's entrance and the point Mario leaves the zone on change
    /// together.
    pub fn set_way(&mut self, point: usize, dir: usize, way: Option<Way>) {
        if point >= map_routes::POINTS || self.routes.ways[point][dir] == way {
            return;
        }
        self.points_record();
        self.put_way(point, dir, way);
    }

    fn put_way(&mut self, point: usize, dir: usize, way: Option<Way>) {
        if let Some(t) = map_routes::twin(point as u8) {
            self.routes.ways[t as usize][dir] = way.clone();
        }
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
        let mut back_dir = None;
        if let Some((back, bd)) = way_back(&way, corners, from as u8) {
            let t = to as usize;
            if t < map_routes::POINTS && self.routes.ways[t][bd].is_none() {
                self.put_way(t, bd, Some(back));
                back_dir = Some(bd);
            }
        }
        self.put_way(from, dir, Some(way));
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
        let pictures_changed = self.world_dirty;
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
            for (z, pic) in self.zones.iter().enumerate().take(zone_maps::ZONES.len()) {
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
        let stored_zones: Vec<new_zones::NewZone> = (0..self.new_zones.len()).map(|k| self.new_zone(k)).collect();
        if self.doors_dirty {
            let mut d = self.doors.clone();
            for e in &mut d.exits {
                e.1 = new_zones::resolve(e.1, &stored_zones);
            }
            if doors::write(&mut self.rom, &d) {
                notes.push(Saved::Doors);
            }
            for (i, &p) in self.entries.iter().enumerate() {
                if p != doors::NONE && new_zones::entry_point(&self.rom, doors::INTO[i].zone) != Some(p) {
                    new_zones::set_entry(&mut self.rom, doors::INTO[i].zone, p, self.routes.places[p as usize])?;
                }
            }
            self.doors_dirty = false;
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
            notes.push(Saved::NewLevels(self.new_levels(), used));
        }
        if self.new_zones_dirty || (pictures_changed && !stored_zones.is_empty()) {
            let mut rom = self.rom.clone();
            let used = new_zones::store(&mut rom, &stored_zones)?;
            self.rom = rom;
            self.new_zones_dirty = false;
            notes.push(Saved::NewZones(stored_zones.len(), used));
        }
        Ok(notes)
    }
}

/// The way back from the end of `way` (drawn from `from` through `corners`) the same road,
/// and the direction it starts in.
fn way_back(way: &Way, corners: &[(i32, i32)], from: u8) -> Option<(Way, usize)> {
    let end = *way.track().last()?;
    let back_corners: Vec<(i32, i32)> = corners.iter().rev().skip(1).map(|c| (c.0 - end.0, c.1 - end.1)).chain(std::iter::once((-end.0, -end.1))).collect();
    let back = Way::through(&back_corners, from);
    let d = first_dir(&back)?;
    Some((back, d))
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
    fn copy_cut_and_undo_on_levels_and_maps() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        for g in [Grid::Level(3), Grid::Map(OnMap::World), Grid::Map(OnMap::Zone(1))] {
            let a = Area::between((2, 3), (5, 4));
            let before = doc.copy_rect(g, a).unwrap();
            assert_eq!((before.w, before.h, before.at(1, 0)), (4, 2, doc.cell(g, 3, 3).unwrap()));
            let bg = doc.background(g);
            doc.clear_rect(g, a);
            assert!(a.cells().all(|(x, y)| doc.cell(g, x, y) == Some(bg)));
            match g {
                Grid::Level(l) => assert!(doc.undo(l)),
                Grid::Map(_) => doc.world_undo(),
            }
            assert_eq!(doc.copy_rect(g, a), Some(before));
            let (w, h) = doc.grid_size(g);
            let edge = doc.copy_rect(g, Area::between((w as i32 - 2, h as i32 - 1), (w as i32 + 3, h as i32 + 3))).unwrap();
            assert_eq!((edge.w, edge.h), (2, 1), "clipped to the grid");
            assert_eq!(doc.copy_rect(g, Area::between((-5, -5), (-1, -1))), None);
            assert!(!doc.set_cell(g, -1, 0, 1) && doc.cell(g, w as i32, 0).is_none());
        }
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
        let world_tile = doc.tile(OnMap::World, 5, 5);
        doc.world_snapshot();
        doc.set_tile(OnMap::World, 5, 5, world_tile ^ 1);
        assert!(doc.restore_map(zone));
        assert_eq!(doc.tile(zone, 3, 3), before);
        assert_eq!(doc.points[0], world::point_levels(&doc.original)[0] + 1, "a zone's restore leaves the world map's points");
        assert_eq!(doc.tile(OnMap::World, 5, 5), world_tile ^ 1, "and its picture");
        assert!(doc.restore_map(OnMap::World));
        assert_eq!(doc.points, world::point_levels(&doc.original));
        assert_eq!(doc.tile(OnMap::World, 5, 5), world_tile);
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
    fn a_new_levels_stand_in_is_undone_with_it() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let n = doc.add_level(2).unwrap();
        let before = doc.levels[n].content.clone();
        let other = doc.levels[20].content.clone();
        doc.snapshot(n);
        doc.levels[n].content = LevelContent { stand_in: Some(StandIn { base: 20, header: [7; 20] }), ..other };
        assert_eq!(doc.base_of(n), 20);
        assert!(doc.undo(n));
        assert_eq!((doc.base_of(n), &doc.levels[n].content), (2, &before));
        assert!(doc.redo(n));
        assert_eq!(doc.levels[n].content.stand_in.map(|s| s.header), Some([7; 20]));
        doc.restore_level(n);
        assert_eq!(doc.base_of(n), 20, "restoring a new level keeps what it plays as");
    }

    #[test]
    fn levels_go_out_to_files_and_come_back() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(9, 4, 4, 0x11);
        let f = files::read_level(&files::level_text(&doc.level_file(9))).unwrap();
        assert_eq!(f.stand_in.base, 9);
        let n = doc.import_level(f.clone()).unwrap();
        assert_eq!((doc.base_of(n), doc.block(n, 4, 4)), (9, 0x11));
        assert_eq!(doc.levels[n].content.objs, doc.levels[9].content.objs);
        assert!(!doc.replace_level(9, f.clone()), "the 32 levels are never replaced");
        let m = doc.add_level(2).unwrap();
        let before = doc.levels[m].content.clone();
        assert!(doc.replace_level(m, f));
        assert_eq!((doc.base_of(m), doc.block(m, 4, 4)), (9, 0x11));
        assert!(doc.undo(m));
        assert_eq!(doc.levels[m].content, before);
        while doc.add_level(0).is_some() {}
        assert_eq!(doc.new_levels(), extra_levels::MAX);
        assert_eq!(doc.import_level(doc.level_file(0)), None, "no room");
    }

    #[test]
    fn a_project_goes_out_to_a_file_and_comes_back() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(1, 6, 6, 0x22);
        let n = doc.add_level(4).unwrap();
        doc.set_tile(OnMap::Zone(3), 2, 2, 0x44);
        doc.step_point_level(0, 1);
        let text = files::project_text(&doc.project_file());
        let mut other = open().unwrap();
        other.snapshot(1);
        other.replace_project(files::read_project(&text).unwrap()).unwrap();
        assert_eq!(other.levels.len(), n + 1);
        assert_eq!((other.block(1, 6, 6), other.base_of(n), other.tile(OnMap::Zone(3), 2, 2)), (0x22, 4, 0x44));
        assert_eq!((&other.points, &other.routes, &other.doors), (&doc.points, &doc.routes, &doc.doors));
        assert!(other.levels.iter().all(|l| l.dirty()) && other.world_dirty && other.points_dirty);
        assert!(!other.undo(1), "no undo across a project import");
        other.commit().unwrap();
        let again = EditorDocument::open(other.original.clone(), Some(other.rom.clone()));
        assert_eq!((again.block(1, 6, 6), again.base_of(n)), (0x22, 4));
        let mut p = doc.project_file();
        p.cartridge ^= 1;
        assert_eq!(other.replace_project(p), Err(FileError::OtherCartridge));
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
        // a path out of a zone's entrance is also the path out of where Mario leaves the zone
        let (x, y) = doc.routes.places[2].on_world();
        let q = doc.add_point(x, y + 16).unwrap();
        doc.set_way(2, 3, None);
        assert!(doc.draw_path(2, q, &[(0, 16)]).is_some());
        assert_eq!(doc.routes.ways[0x19], doc.routes.ways[2]);
        assert_eq!(doc.routes.ways[0x19][3].as_ref().map(|w| w.to), Some(q));
        // doors: the new point opens the Pumpkin Zone, the Tree Zone's exit leads into the
        // Macro Zone; saved, read back, undone
        doc.set_door(q, Some(1));
        doc.set_exit(0, 0x23, Leads::Zone(2));
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.doors.door_of(q), again.doors.exits[0]), (Some(1), (0x23, Leads::Zone(2))));
        assert!(doc.points_undo() && doc.points_undo());
        assert_eq!(doc.doors, doors::read(&doc.original));
    }
}
