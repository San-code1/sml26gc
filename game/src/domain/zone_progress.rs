//! Progress in the editor's new zones and added points, kept apart from the game's battery
//! save: a new zone uses its base zone's cleared flags while it is played, so the port keeps
//! whichever set is not in the game's memory, per file (A, B, C); the added points' flag bytes
//! are not in the game's save at all, so the port keeps them per map and file, and the map
//! each file was last on.
use std::collections::BTreeMap;

const HEADER: &str = "NMB zones 1";

/// The zone Mario is in (or at the door of) when the progress was written: its flags are
/// in the game's memory and the base zone's are kept here, as (flag, byte).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inside {
    pub zone: usize,
    pub file: u8,
    pub base: Vec<(u8, u8)>,
}

/// A map with added points of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    World,
    /// A game zone (`zone_maps::ZONES` index).
    Zone(usize),
    NewZone(usize),
}

impl MapKey {
    fn text(self) -> String {
        match self {
            MapKey::World => "w".into(),
            MapKey::Zone(z) => format!("z{z}"),
            MapKey::NewZone(k) => format!("n{k}"),
        }
    }

    fn parse(t: &str) -> Option<MapKey> {
        match (t.get(..1)?, t.get(1..)?) {
            ("w", "") => Some(MapKey::World),
            ("z", n) => Some(MapKey::Zone(n.parse().ok()?)),
            ("n", n) => Some(MapKey::NewZone(n.parse().ok()?)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZoneProgress {
    /// (new zone, file) to its cleared flag bytes, while it is not the one swapped in.
    pub kept: BTreeMap<(usize, u8), Vec<u8>>,
    pub inside: Option<Inside>,
    /// (map, file) to its added points' flag bytes.
    pub added: BTreeMap<(MapKey, u8), Vec<u8>>,
    /// The map each file was last on.
    pub shown: BTreeMap<u8, MapKey>,
}

fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!(" {b:02x}")).collect()
}

fn bytes<'a>(words: impl Iterator<Item = &'a str>) -> Option<Vec<u8>> {
    words.map(|w| u8::from_str_radix(w, 16).ok()).collect()
}

impl ZoneProgress {
    pub fn to_text(&self) -> String {
        let mut t = format!("{HEADER}\n");
        for ((zone, file), v) in &self.kept {
            t += &format!("zone {zone} file {file}{}\n", hex(v));
        }
        if let Some(i) = &self.inside {
            let base: String = i.base.iter().map(|(f, v)| format!(" {f:02x}:{v:02x}")).collect();
            t += &format!("inside {} file {}{base}\n", i.zone, i.file);
        }
        for ((map, file), v) in &self.added {
            t += &format!("added {} file {file}{}\n", map.text(), hex(v));
        }
        for (file, map) in &self.shown {
            t += &format!("shown {} file {file}\n", map.text());
        }
        t
    }

    /// None if the text is not a progress file.
    pub fn from_text(text: &str) -> Option<ZoneProgress> {
        let mut lines = text.lines();
        if lines.next()?.trim() != HEADER {
            return None;
        }
        let mut p = ZoneProgress::default();
        for line in lines {
            let mut w = line.split_whitespace();
            let (kind, what, _, file) = (w.next(), w.next(), w.next(), w.next().and_then(|f| f.parse::<u8>().ok()));
            if let (Some(k @ ("added" | "shown")), Some(map), Some(file)) = (kind, what.and_then(MapKey::parse), file) {
                if k == "shown" {
                    p.shown.insert(file, map);
                } else if let Some(v) = bytes(w) {
                    p.added.insert((map, file), v);
                }
                continue;
            }
            let (Some(zone), Some(file)) = (what.and_then(|z| z.parse().ok()), file) else { continue };
            match kind {
                Some("zone") => {
                    if let Some(v) = bytes(w) {
                        p.kept.insert((zone, file), v);
                    }
                }
                Some("inside") => {
                    let pair = |x: &str| {
                        let (f, v) = x.split_once(':')?;
                        Some((u8::from_str_radix(f, 16).ok()?, u8::from_str_radix(v, 16).ok()?))
                    };
                    if let Some(base) = w.map(pair).collect::<Option<Vec<_>>>() {
                        p.inside = Some(Inside { zone, file, base });
                    }
                }
                _ => {}
            }
        }
        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_round_trips_as_text() {
        let mut p = ZoneProgress::default();
        p.kept.insert((0, 2), vec![0x80, 0, 0x80]);
        p.kept.insert((3, 0), vec![]);
        p.inside = Some(Inside { zone: 1, file: 0, base: vec![(1, 0x80), (0x24, 0)] });
        p.added.insert((MapKey::World, 0), vec![0x80, 0]);
        p.added.insert((MapKey::Zone(3), 2), vec![0, 0, 0x80]);
        p.added.insert((MapKey::NewZone(1), 1), vec![]);
        p.shown.insert(0, MapKey::Zone(3));
        p.shown.insert(2, MapKey::World);
        let t = p.to_text();
        assert!(t.starts_with(HEADER));
        assert_eq!(ZoneProgress::from_text(&t), Some(p));
        assert_eq!(ZoneProgress::from_text("something else"), None);
    }
}
