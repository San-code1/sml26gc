//! Points of the world map and the game's zones: how many a map has and can have, and
//! moving one (its place, and the paths to and from it redrawn straight).
use super::{EditorDocument, ZONE_POINTS};
use cart_model::map_routes::{self, Place, Way, FREE_POINTS};
use cart_model::added_points;
use cart_model::original_points::{self, OnMap};

impl EditorDocument {
    /// Where point `p` stands on its map (`map`), from its place: picture pixels.
    fn stand(map: OnMap, pl: &Place) -> (i32, i32) {
        match map {
            OnMap::World => pl.on_world(),
            _ => (pl.x as i32, pl.y as i32),
        }
    }

    /// How far point `p` of `map` (the world map or a game zone) has been moved from where
    /// the game has it: picture pixels.
    pub fn point_moved(&self, map: OnMap, p: u8) -> (i32, i32) {
        let i = p as usize;
        if i >= map_routes::POINTS || self.routes.added.contains(&p) {
            return (0, 0);
        }
        let (a, b) = (Self::stand(map, &self.original_routes.places[i]), Self::stand(map, &self.routes.places[i]));
        (b.0 - a.0, b.1 - a.1)
    }

    /// Put point `p` of `map` (the world map or a game zone) `(dx, dy)` picture pixels from
    /// place `from`, in steps of 4 (as the map's walks go); the paths to and from it become
    /// straight. A zone's entrance on the world map moves with the point Mario walks out of
    /// the zone on. The caller takes the undo snapshot once per drag.
    pub fn move_point(&mut self, map: OnMap, p: u8, from: Place, dx: i32, dy: i32) {
        let snap = |v: i32| (v as f32 / 4.0).round() as i32 * 4;
        let (dx, dy) = (snap(dx), snap(dy));
        let i = p as usize;
        if i >= map_routes::ALL || matches!(map, OnMap::NewZone(_)) {
            return;
        }
        let place = match map {
            OnMap::World => {
                let (x, y) = from.on_world();
                Place { rest: from.rest, ..Place::at_world((x + dx).clamp(0, 511), (y + dy).clamp(0, 511)) }
            }
            _ => Place { x: (from.x as i32 + dx).clamp(0, 159) as u8, y: (from.y as i32 + dy).clamp(8, 143) as u8, ..from },
        };
        if place == self.routes.places[i] {
            return;
        }
        let moved: Vec<u8> = std::iter::once(p).chain(map_routes::twin(p).filter(|_| map == OnMap::World)).collect();
        for &m in &moved {
            self.routes.places[m as usize] = place;
        }
        for q in 0..map_routes::ALL {
            let from = Self::stand(map, &self.routes.places[q]);
            for d in 0..4 {
                let Some(w) = &self.routes.ways[q][d] else { continue };
                if !moved.contains(&(q as u8)) && !moved.contains(&w.to) {
                    continue;
                }
                let to = Self::stand(map, &self.routes.places[w.to as usize]);
                let way = Way::through(&[(to.0 - from.0, to.1 - from.1)], w.to);
                self.routes.ways[q][d] = Some(way);
            }
        }
        self.routes.sync_exits();
        self.routes_dirty = true;
    }

    /// How many points map `map` has and how many it can have.
    pub fn point_count(&self, map: OnMap) -> (usize, usize) {
        let added = |m: OnMap| if self.added_on() == m { self.routes.added.iter().filter(|&&p| added_points::index(p).is_some()).count() } else { self.added_kept_on(m) };
        match map {
            OnMap::NewZone(k) => self.new_zones.get(k).map_or((0, 0), |z| (z.points.len(), z.room().min(ZONE_POINTS))),
            OnMap::World => {
                let used = self.world_points().len() - self.routes.added.iter().filter(|&&p| added_points::index(p).is_some()).count() + added(map);
                let free = self.routes.added.iter().filter(|&&p| FREE_POINTS.contains(&p)).count();
                (used, used - free - added(map) + FREE_POINTS.len() + added_points::MAX)
            }
            OnMap::Zone(z) => {
                let n = original_points::on(map).count();
                let max = if z < added_points::ZONES { ZONE_POINTS.max(n) } else { n };
                (n + added(map), max)
            }
        }
    }
}
