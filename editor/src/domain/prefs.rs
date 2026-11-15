//! The editor's own settings, kept as `key=value` lines.

/// Palette zoom steps: how many screen pixels per pixel of a block or tile.
pub const PALETTE_ZOOMS: std::ops::RangeInclusive<u32> = 1..=5;
const DEFAULT_ZOOM: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// The block and object type palettes.
    pub level_palette_zoom: u32,
    /// The world and zone maps' tile palette.
    pub map_palette_zoom: u32,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs { level_palette_zoom: DEFAULT_ZOOM, map_palette_zoom: DEFAULT_ZOOM }
    }
}

impl Prefs {
    /// Unknown keys are ignored; missing or bad values keep the default.
    pub fn from_text(text: &str) -> Prefs {
        let value = |key: &str, ok: &std::ops::RangeInclusive<u32>, default: u32| {
            text.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == key).and_then(|(_, v)| v.trim().parse().ok()).filter(|v| ok.contains(v)).unwrap_or(default)
        };
        let d = Prefs::default();
        Prefs {
            level_palette_zoom: value("level_palette_zoom", &PALETTE_ZOOMS, d.level_palette_zoom),
            map_palette_zoom: value("map_palette_zoom", &PALETTE_ZOOMS, d.map_palette_zoom),
        }
    }

    pub fn to_text(&self) -> String {
        format!("level_palette_zoom={}\nmap_palette_zoom={}\n", self.level_palette_zoom, self.map_palette_zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip_and_bad_values() {
        let p = Prefs { level_palette_zoom: 4, map_palette_zoom: 5 };
        assert_eq!(Prefs::from_text(&p.to_text()), p);
        assert_eq!(Prefs::from_text(""), Prefs::default());
        let odd = Prefs::from_text("other=1\nlevel_palette_zoom = 9\nmap_palette_zoom=x\n");
        assert_eq!(odd, Prefs::default());
    }
}
