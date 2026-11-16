//! The game's own graphics, captured by running the console in the background: it boots
//! to the world map once (from the cartridge as it was when first needed), then each level
//! is entered on a copy of that state to see its blocks; object pictures and the world
//! map's tiles come from the same state, and each zone map's tiles from a copy taken there.
use cart_model::art::{capture_mario, LevelArt, Look, WorldArt};
use cart_model::levels::LEVELS;
use cart_model::zone_maps::ZONES;
use cart_model::GameDriver;
use gb_core::Gb;

pub enum Capture {
    /// Already there.
    Ready,
    Captured,
    /// The game didn't load the level; its graphics may be wrong.
    NotLoaded,
    /// The console couldn't start.
    Failed(String),
}

pub struct ArtService {
    boot: Option<Gb>,
    levels: Vec<Option<LevelArt>>,
    world: Option<WorldArt>,
    zones: Vec<Option<WorldArt>>,
}

impl ArtService {
    pub fn new() -> ArtService {
        ArtService { boot: None, levels: (0..LEVELS).map(|_| None).collect(), world: None, zones: ZONES.iter().map(|_| None).collect() }
    }

    pub fn booted(&self) -> bool {
        self.boot.is_some()
    }

    pub fn level(&self, l: usize) -> Option<&LevelArt> {
        self.levels[l].as_ref()
    }

    pub fn ensure_level(&mut self, rom: &[u8], l: usize) -> Capture {
        if self.levels[l].is_some() {
            return Capture::Ready;
        }
        if self.boot.is_none() {
            let mut g = match Gb::new(rom.to_vec(), 44100) {
                Ok(g) => g,
                Err(e) => return Capture::Failed(e),
            };
            g.boot_to_map();
            self.boot = Some(g);
        }
        let mut g = self.boot.clone().unwrap();
        let loaded = g.enter_level(l as u8);
        self.levels[l] = Some(LevelArt::capture(&g));
        if loaded {
            Capture::Captured
        } else {
            Capture::NotLoaded
        }
    }

    /// The world map's tiles as the game shows them (needs the console booted).
    pub fn world(&mut self) -> Option<&WorldArt> {
        if self.world.is_none() {
            self.world = Some(WorldArt::capture(self.boot.as_ref()?));
        }
        self.world.as_ref()
    }

    /// Zone map `z`'s tiles as the game shows them: the game is taken to one of the zone's
    /// points on a copy of the booted console (needs the console booted).
    pub fn zone(&mut self, z: usize) -> Option<&WorldArt> {
        if self.zones[z].is_none() {
            let mut g = self.boot.clone()?;
            g.show_map_at(ZONES[z].point);
            self.zones[z] = Some(WorldArt::capture(&g));
        }
        self.zones[z].as_ref()
    }

    /// What Mario looks like at a level's start. None while the console isn't booted.
    pub fn mario(&self) -> Option<Option<Look>> {
        Some(capture_mario(self.boot.as_ref()?))
    }
}
