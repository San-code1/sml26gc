//! What the player chose: key bindings, colours, volume, screen and language, and their
//! `key=value` text form. Where the text is kept is a port (`SettingsStore`).
use super::i18n::{LANG_EN, LANG_IDS};
use super::keys::{Key, DEFAULT_KEYS};

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    /// Up, Down, Left, Right, A, B, Start, Select.
    pub keys: [Key; 8],
    pub palette: usize,
    pub volume: u8,
    pub fullscreen: bool,
    pub integer_scale: bool,
    pub lang: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { keys: DEFAULT_KEYS, palette: 0, volume: 8, fullscreen: false, integer_scale: true, lang: LANG_EN }
    }
}

impl Settings {
    /// Read `key=value` lines; anything missing or unknown keeps its default.
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            if let Some(i) = k.strip_prefix("key").and_then(|n| n.parse::<usize>().ok()) {
                if i < 8 {
                    if let Some(code) = Key::from_id(v) {
                        s.keys[i] = code;
                    }
                }
            }
            match k {
                "palette" => s.palette = v.parse().unwrap_or(0) % 3,
                "volume" => s.volume = v.parse::<u8>().unwrap_or(8).min(10),
                "fullscreen" => s.fullscreen = v == "1",
                "integer_scale" => s.integer_scale = v != "0",
                "language" => s.lang = LANG_IDS.iter().position(|&id| id == v).unwrap_or(LANG_EN),
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, k) in self.keys.iter().enumerate() {
            out += &format!("key{i}={}\n", k.id().unwrap_or("Unknown"));
        }
        out += &format!(
            "palette={}\nvolume={}\nfullscreen={}\ninteger_scale={}\nlanguage={}\n",
            self.palette,
            self.volume,
            self.fullscreen as u8,
            self.integer_scale as u8,
            LANG_IDS[self.lang.min(2)]
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        let s = Settings::from_text("key4=Space\nkey5=LeftShift\npalette=1\nfullscreen=1\nlanguage=es\nvolume=99\n");
        assert_eq!(s.keys[4], Key::Space);
        assert_eq!(s.keys[5], Key::LeftShift);
        assert_eq!((s.palette, s.fullscreen, s.lang, s.volume), (1, true, 1, 10));
        assert_eq!(Settings::from_text(&s.to_text()), s);
        assert_eq!(Settings::from_text(""), Settings::default());
    }
}
