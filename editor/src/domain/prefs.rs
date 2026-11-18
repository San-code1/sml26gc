//! The editor's own settings, kept as `key=value` lines.
use i18n::Lang;

/// Palette zoom steps: how many screen pixels per pixel of a block or tile.
pub const PALETTE_ZOOMS: std::ops::RangeInclusive<u32> = 1..=5;
const DEFAULT_ZOOM: u32 = 2;
/// Volume steps of a level's music played in the editor (not the game's).
pub const MUSIC_VOLUMES: std::ops::RangeInclusive<u32> = 0..=10;
/// What a test game from the editor starts with.
pub const TEST_LIVES: std::ops::RangeInclusive<u32> = 0..=99;
pub const TEST_COINS: std::ops::RangeInclusive<u32> = 0..=999;
const DEFAULT_VOLUME: u32 = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// The block and object type palettes.
    pub level_palette_zoom: u32,
    /// The world and zone maps' tile palette.
    pub map_palette_zoom: u32,
    /// Listening to a level's music in the editor.
    pub music_volume: u32,
    /// The project opened last (empty: none).
    pub last_project: String,
    /// Mario's lives and coins in a test game (Play, Here).
    pub test_lives: u32,
    pub test_coins: u32,
    /// The editor's language; None: the system's.
    pub language: Option<Lang>,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs { level_palette_zoom: DEFAULT_ZOOM, map_palette_zoom: DEFAULT_ZOOM, music_volume: DEFAULT_VOLUME, last_project: String::new(), test_lives: 99, test_coins: 999, language: None }
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
            music_volume: value("music_volume", &MUSIC_VOLUMES, d.music_volume),
            test_lives: value("test_lives", &TEST_LIVES, d.test_lives),
            test_coins: value("test_coins", &TEST_COINS, d.test_coins),
            last_project: text.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == "last_project").map(|(_, v)| v.trim().to_string()).unwrap_or_default(),
            language: text.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == "language").and_then(|(_, v)| Lang::from_id(v)),
        }
    }

    pub fn to_text(&self) -> String {
        format!(
            "level_palette_zoom={}\nmap_palette_zoom={}\nmusic_volume={}\nlast_project={}\ntest_lives={}\ntest_coins={}\nlanguage={}\n",
            self.level_palette_zoom,
            self.map_palette_zoom,
            self.music_volume,
            self.last_project,
            self.test_lives,
            self.test_coins,
            self.language.map_or("auto", Lang::id)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip_and_bad_values() {
        let p = Prefs { level_palette_zoom: 4, map_palette_zoom: 5, music_volume: 3, last_project: "Sky Islands".into(), test_lives: 5, test_coins: 120, language: Some(Lang::Ja) };
        assert_eq!(Prefs::from_text(&p.to_text()), p);
        assert_eq!(Prefs::from_text(&Prefs::default().to_text()).language, None, "auto");
        assert_eq!(Prefs::from_text(""), Prefs::default());
        let odd = Prefs::from_text("other=1\nlevel_palette_zoom = 9\nmap_palette_zoom=x\nmusic_volume=11\n");
        assert_eq!(odd, Prefs::default());
    }
}
