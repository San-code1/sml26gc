//! What the original game does with some map points besides their level, paths and doors,
//! read from the map code (bank $0F) and checked in the emulator (`research mapprobe`,
//! `research spotcheck`):
//!
//! * 0F:7003: back on the map after clearing a level ($A224 = 0) on point $05 or $1A, Mario
//!   is put on $3F, the flight to the Space Zone, which lands on its point $3C. On $00 he is
//!   put on $14 (with the start's flag set) and walks on to $13.
//! * The door chain (0F:460B) enters the Space Zone from $12, a number the game also draws at
//!   the junction $13; no path stops on $12, so that door never runs.
//! * Each zone's entrance on the world map has a twin, the number Mario stands on after
//!   walking out of the zone (`map_routes::ZONE_EXITS`): same place and ways; the Space Zone's
//!   exit ($40) puts him on $1A, the twin of $05, which opens the same level.
//! * $0C walks a script to the hidden $0D into the Turtle Zone (`doors`).
use crate::{doors, map_routes, world};

/// A fact about a point of the original game's map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    /// Where a new game starts; clearing its level walks Mario on to $13.
    Start,
    /// Clearing the level opened here flies Mario to the Space Zone.
    FlightOut,
    /// Where the flight to the Space Zone lands.
    FlightIn,
    /// A zone's entrance on the world map (`zone`, a `zone_maps::ZONES` index): Mario stands
    /// on `twin` there after walking out of the zone.
    Entrance { zone: usize, twin: u8 },
    /// Where Mario stands after walking out of `zone`: the same place and paths as `twin`.
    ZoneReturn { zone: usize, twin: u8 },
    /// The game's door into the Space Zone is on $12, drawn here, which no path stops on.
    UnusedDoor,
    /// Leads into the Turtle Zone by a walk the game makes on its own.
    TurtleWalk,
    /// No path in or out and no level: Mario never stands on it.
    NeverStood,
}

/// The Space Zone (`zone_maps::ZONES` index).
pub const SPACE_ZONE: usize = 3;
/// The game's own number of its door into the Space Zone, and where it is drawn.
pub const SPACE_DOOR: u8 = 0x12;
const SPACE_DOOR_DRAWN: u8 = 0x13;
const SPACE_ENTRY: u8 = 0x3C;
const FLIGHT_FROM: [u8; 2] = [0x05, 0x1A];
const TURTLE_DOOR: u8 = 0x0C;
/// `map_routes::ZONE_EXITS`' zones (`zone_maps::ZONES` indices), in its order.
const EXIT_ZONES: [usize; 6] = [0, SPACE_ZONE, 2, 1, 5, 4];

/// What the original game (`original`, the cartridge as bought) does with point `p`.
pub fn notes(original: &[u8], p: u8) -> Vec<Note> {
    let mut v = Vec::new();
    if p == 0 {
        v.push(Note::Start);
    }
    if FLIGHT_FROM.contains(&p) {
        v.push(Note::FlightOut);
    }
    if p == SPACE_ENTRY {
        v.push(Note::FlightIn);
    }
    if let Some(i) = map_routes::ZONE_EXITS.iter().position(|&(a, b)| a == p || b == p) {
        let (a, b) = map_routes::ZONE_EXITS[i];
        let zone = EXIT_ZONES[i];
        v.push(if a == p { Note::Entrance { zone, twin: b } } else { Note::ZoneReturn { zone, twin: a } });
    }
    if p == SPACE_DOOR_DRAWN && doors::read(original).door_of(SPACE_DOOR).is_some() {
        v.push(Note::UnusedDoor);
    }
    if p == TURTLE_DOOR {
        v.push(Note::TurtleWalk);
    }
    if v.is_empty() && never_stood(original, p) {
        v.push(Note::NeverStood);
    }
    v
}

fn never_stood(original: &[u8], p: u8) -> bool {
    let routes = map_routes::read(original);
    let d = doors::read(original);
    let i = p as usize;
    i < map_routes::POINTS
        && routes.ways[i].iter().all(Option::is_none)
        && !routes.ways.iter().take(map_routes::POINTS).flatten().flatten().any(|w| w.to == p)
        && world::point_levels(original)[i] == world::NO_LEVEL
        && d.door_of(p).is_none()
        && d.exit_on(p).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone_maps::ZONES;

    #[test]
    fn exit_zones_are_the_zones_the_entrances_open() {
        assert_eq!(ZONES[EXIT_ZONES[0]].name, "Tree Zone");
        assert_eq!(ZONES[EXIT_ZONES[1]].name, "Space Zone");
        assert_eq!(ZONES[EXIT_ZONES[5]].name, "Turtle Zone");
    }
}
