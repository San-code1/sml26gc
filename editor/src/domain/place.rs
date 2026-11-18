//! Where the editor was when the project was last saved, to open there again: a level, or a
//! map of the zone editor. One line of text.
use cart_model::original_points::OnMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Level(usize),
    Map(OnMap),
}

impl Place {
    pub fn to_text(self) -> String {
        match self {
            Place::Level(l) => format!("level {l}\n"),
            Place::Map(OnMap::World) => "world\n".into(),
            Place::Map(OnMap::Zone(z)) => format!("zone {z}\n"),
            Place::Map(OnMap::NewZone(k)) => format!("new_zone {k}\n"),
        }
    }

    /// None for text that names no place.
    pub fn from_text(text: &str) -> Option<Place> {
        let mut words = text.split_whitespace();
        let kind = words.next()?;
        let n: Option<usize> = words.next().and_then(|n| n.parse().ok());
        match (kind, n) {
            ("world", None) => Some(Place::Map(OnMap::World)),
            ("level", Some(l)) => Some(Place::Level(l)),
            ("zone", Some(z)) => Some(Place::Map(OnMap::Zone(z))),
            ("new_zone", Some(k)) => Some(Place::Map(OnMap::NewZone(k))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_place_round_trips_and_odd_text_names_none() {
        for p in [Place::Level(7), Place::Map(OnMap::World), Place::Map(OnMap::Zone(3)), Place::Map(OnMap::NewZone(1))] {
            assert_eq!(Place::from_text(&p.to_text()), Some(p));
        }
        for odd in ["", "level", "level x", "zone -1", "world 2", "sky 1"] {
            assert_eq!(Place::from_text(odd), None, "{odd:?}");
        }
    }
}
