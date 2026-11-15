//! The editor's new zones while the game is played (`cart_model::new_zones`): when Mario
//! stands on a new zone's door on the world map, the zone is written over its base zone in
//! the console's ROM and its cleared flags take the base zone's place in the file being
//! played; it stays while he is on its points, and the base zone comes back once he is on
//! the world map away from the door. Which zone is in, and the flags not in the game's
//! memory, are kept by a `ZoneProgressStore`, so a game saved inside a new zone resumes there.
//! An exit into a new zone leads to its door (`new_zones::resolve`): arriving there from
//! another zone, the port presses A for Mario.
use crate::domain::zone_progress::{Inside, ZoneProgress};
use crate::ports::ZoneProgressStore;
use cart_model::new_zones::{self, NewZone, CURRENT_FILE};
use cart_model::ram_map::{GAME_MODE, MAP_POINT};
use gb_core::{Buttons, Gb};

/// The map shown ($00: the world map).
const CURRENT_MAP: u16 = 0xA68B;
const MAP_MODE: u8 = 0x0C;
/// Window frames to wait on a door before pressing A, and for how long.
const ENTER_WAIT: u32 = 40;
const ENTER_PRESS: u32 = 6;

pub struct ZoneSwap {
    zones: Vec<NewZone>,
    progress: ZoneProgress,
    store: Box<dyn ZoneProgressStore>,
    undo: Vec<(usize, Vec<u8>)>,
    /// The map shown when last on a map.
    last_map: u8,
    /// Frames left until A is let go after arriving on a door from another zone.
    enter_in: u32,
}

impl ZoneSwap {
    /// The new zones of the console's cartridge; the one the progress was saved in is put
    /// back in place (its flags are already in the battery save).
    pub fn new(g: &mut Gb, store: Box<dyn ZoneProgressStore>) -> ZoneSwap {
        let zones = new_zones::read(g.cart().rom());
        let progress = store.load().unwrap_or_default();
        let mut s = ZoneSwap { zones, progress, store, undo: Vec::new(), last_map: 0, enter_in: 0 };
        if let Some(i) = s.progress.inside.clone() {
            match new_zones::writes_for(g.cart().rom(), &s.zones, i.zone).ok() {
                Some(w) => s.undo = new_zones::apply(g.cart_mut().rom_mut(), &w),
                None => {
                    // the zone is gone from the copy: give the base zone its flags back
                    let (flags, values): (Vec<u8>, Vec<u8>) = i.base.into_iter().unzip();
                    new_zones::set_progress(g, i.file, &flags, &values);
                    s.progress.inside = None;
                    s.store.save(&s.progress);
                }
            }
        }
        s
    }

    /// The new zone in place, if any.
    pub fn inside(&self) -> Option<usize> {
        self.progress.inside.as_ref().map(|i| i.zone)
    }

    /// After the game's frames: put a new zone in or out as Mario's place asks. True if it
    /// changed (the battery save should be written now, to match the kept progress).
    pub fn update(&mut self, g: &mut Gb) -> bool {
        if self.zones.is_empty() || g.read(GAME_MODE) != MAP_MODE {
            return false;
        }
        let point = g.read(MAP_POINT);
        let map = g.read(CURRENT_MAP);
        let from_zone = std::mem::replace(&mut self.last_map, map) != 0 && map == 0;
        let now = self.inside();
        // the zone in place stays on its door and its points (entering, the point changes
        // while the world map is still up)
        let stays = now.filter(|&k| point == self.zones[k].door || new_zones::base_of(self.zones[k].base).is_some_and(|b| b.holds(point)));
        let want = stays.or_else(|| (map == 0).then(|| self.zones.iter().position(|z| z.door == point)).flatten());
        if want == now {
            return false;
        }
        self.leave(g);
        if let Some(k) = want {
            self.enter(g, k);
            if from_zone {
                self.enter_in = ENTER_WAIT + ENTER_PRESS;
            }
        }
        self.store.save(&self.progress);
        true
    }

    /// Buttons the port presses for Mario this window frame (A, to enter a new zone an exit
    /// led into).
    pub fn buttons(&mut self) -> Buttons {
        self.enter_in = self.enter_in.saturating_sub(1);
        if self.enter_in > 0 && self.enter_in <= ENTER_PRESS {
            Buttons::A
        } else {
            Buttons::NONE
        }
    }

    fn enter(&mut self, g: &mut Gb, k: usize) {
        let Some(b) = new_zones::base_of(self.zones[k].base) else { return };
        let Ok(w) = new_zones::writes_for(g.cart().rom(), &self.zones, k) else { return };
        let flags = new_zones::flags(g.cart().rom(), b);
        self.undo = new_zones::apply(g.cart_mut().rom_mut(), &w);
        let file = g.read(CURRENT_FILE);
        let base = new_zones::progress(g, file, &flags);
        let own = self.progress.kept.remove(&(k, file)).unwrap_or_else(|| vec![0; flags.len()]);
        new_zones::set_progress(g, file, &flags, &own);
        self.progress.inside = Some(Inside { zone: k, file, base: flags.into_iter().zip(base).collect() });
    }

    fn leave(&mut self, g: &mut Gb) {
        let Some(i) = self.progress.inside.take() else { return };
        new_zones::apply(g.cart_mut().rom_mut(), &std::mem::take(&mut self.undo));
        let (flags, values): (Vec<u8>, Vec<u8>) = i.base.into_iter().unzip();
        let own = new_zones::progress(g, i.file, &flags);
        self.progress.kept.insert((i.zone, i.file), own);
        new_zones::set_progress(g, i.file, &flags, &values);
    }
}
