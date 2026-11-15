//! Pause menu drawn at Game Boy resolution in the screen's four shades, with a
//! pixel font (Latin and katakana): resume, key bindings, display, sound, language,
//! the game's own pause, quit. Texts in English (default), Spanish and Japanese.
use crate::ppu::{H, W};
use macroquad::prelude::*;
use std::path::PathBuf;

pub const DEFAULT_KEYS: [KeyCode; 8] = [
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::X,
    KeyCode::Z,
    KeyCode::Enter,
    KeyCode::Backspace,
];

// ---------------------------------------------------------------- texts

pub const LANG_EN: usize = 0;
pub const LANG_ES: usize = 1;
pub const LANG_JA: usize = 2;
/// Each language's name written in itself.
pub const LANG_NAMES: [&str; 3] = ["ENGLISH", "ESPAÑOL", "ニホンゴ"];
const LANG_IDS: [&str; 3] = ["en", "es", "ja"];

pub struct Texts {
    pause: &'static str,
    /// Continue, Controls, Display, Sound, Language, Original pause, Quit, Back to map, File select.
    main: [&'static str; 9],
    sure: &'static str,
    hint2: &'static str,
    map_custom: &'static str,
    map_original: &'static str,
    footer_main: &'static str,
    controls: &'static str,
    buttons: [&'static str; 8],
    restore: &'static str,
    back: &'static str,
    footer_controls: &'static str,
    press_key: &'static str,
    now: &'static str,
    cancel: &'static str,
    display: &'static str,
    colours: &'static str,
    palettes: [&'static str; 3],
    fullscreen: &'static str,
    yes: &'static str,
    no: &'static str,
    scale: &'static str,
    sharp: &'static str,
    fitted: &'static str,
    footer_change: &'static str,
    sound: &'static str,
    volume: &'static str,
    language: &'static str,
    footer_language: &'static str,
    hint: &'static str,
}

const EN: Texts = Texts {
    pause: "PAUSE",
    main: ["CONTINUE", "CONTROLS", "DISPLAY", "SOUND", "LANGUAGE", "ORIGINAL PAUSE", "QUIT GAME", "BACK TO MAP", "FILE SELECT"],
    sure: "ARE YOU SURE?",
    hint2: "ENTER: PAUSE",
    map_custom: "MAP: YOURS",
    map_original: "MAP: ORIGINAL",
    footer_main: "ENTER:OK  ESC:RESUME",
    controls: "CONTROLS",
    buttons: ["UP", "DOWN", "LEFT", "RIGHT", "A BUTTON", "B BUTTON", "START", "SELECT"],
    restore: "RESTORE DEFAULTS",
    back: "BACK",
    footer_controls: "ENTER:CHANGE ESC:BACK",
    press_key: "PRESS A KEY",
    now: "NOW:",
    cancel: "ESC: CANCEL",
    display: "DISPLAY",
    colours: "COLOURS",
    palettes: ["CLASSIC", "GREEN", "GREY"],
    fullscreen: "FULL SCREEN",
    yes: "YES",
    no: "NO",
    scale: "SCALE",
    sharp: "SHARP",
    fitted: "FITTED",
    footer_change: "</>:CHANGE ESC:BACK",
    sound: "SOUND",
    volume: "VOLUME",
    language: "LANGUAGE",
    footer_language: "ENTER:OK ESC:BACK",
    hint: "ESC: MENU",
};

const ES: Texts = Texts {
    pause: "PAUSA",
    main: ["CONTINUAR", "CONTROLES", "PANTALLA", "SONIDO", "IDIOMA", "PAUSA ORIGINAL", "SALIR DEL JUEGO", "VOLVER AL MAPA", "ELEGIR PARTIDA"],
    sure: "SEGURO?",
    hint2: "INTRO: PAUSA",
    map_custom: "MAPA: PROPIO",
    map_original: "MAPA: ORIGINAL",
    footer_main: "INTRO:OK  ESC:SEGUIR",
    controls: "CONTROLES",
    buttons: ["ARRIBA", "ABAJO", "IZQUIERDA", "DERECHA", "BOTON A", "BOTON B", "START", "SELECT"],
    restore: "RESTAURAR",
    back: "VOLVER",
    footer_controls: "INTRO:CAMBIAR ESC:VOLVER",
    press_key: "PULSA UNA TECLA",
    now: "AHORA:",
    cancel: "ESC: CANCELAR",
    display: "PANTALLA",
    colours: "COLORES",
    palettes: ["CLASICA", "VERDE", "GRIS"],
    fullscreen: "COMPLETA",
    yes: "SI",
    no: "NO",
    scale: "ESCALA",
    sharp: "NITIDA",
    fitted: "AJUSTADA",
    footer_change: "</>:CAMBIAR ESC:VOLVER",
    sound: "SONIDO",
    volume: "VOLUMEN",
    language: "IDIOMA",
    footer_language: "INTRO:OK ESC:VOLVER",
    hint: "ESC: MENU",
};

const JA: Texts = Texts {
    pause: "ポーズ",
    main: ["ツヅケル", "キーセッテイ", "ガメン", "サウンド", "ゲンゴ", "モトノポーズ", "ゲームヲヤメル", "チズニモドル", "データセンタク"],
    sure: "ホントニ?",
    hint2: "ENTER:ポーズ",
    map_custom: "チズ:ジブン",
    map_original: "チズ:オリジナル",
    footer_main: "ENTER:OK ESC:ツヅケル",
    controls: "キーセッテイ",
    buttons: ["ウエ", "シタ", "ヒダリ", "ミギ", "Aボタン", "Bボタン", "START", "SELECT"],
    restore: "モトニモドス",
    back: "モドル",
    footer_controls: "ENTER:ヘンコウ ESC:モドル",
    press_key: "キーヲオシテ",
    now: "イマ:",
    cancel: "ESC:キャンセル",
    display: "ガメン",
    colours: "イロ",
    palettes: ["クラシック", "ミドリ", "グレー"],
    fullscreen: "ゼンガメン",
    yes: "ハイ",
    no: "イイエ",
    scale: "スケール",
    sharp: "シャープ",
    fitted: "フィット",
    footer_change: "</>:ヘンコウ ESC:モドル",
    sound: "サウンド",
    volume: "オンリョウ",
    language: "ゲンゴ",
    footer_language: "ENTER:OK ESC:モドル",
    hint: "ESC:メニュー",
};

pub fn texts(lang: usize) -> &'static Texts {
    match lang {
        LANG_ES => &ES,
        LANG_JA => &JA,
        _ => &EN,
    }
}

// ---------------------------------------------------------------- settings

pub struct Settings {
    /// Up, Down, Left, Right, A, B, Start, Select.
    pub keys: [KeyCode; 8],
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

fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("settings.cfg")))
        .unwrap_or_else(|| PathBuf::from("settings.cfg"))
}

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(settings_path()) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            if let Some(i) = k.strip_prefix("key").and_then(|n| n.parse::<usize>().ok()) {
                if i < 8 {
                    if let Some(code) = KEYS.iter().find(|e| e.1 == v).map(|e| e.0) {
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

    pub fn save(&self) {
        let mut out = String::new();
        for (i, k) in self.keys.iter().enumerate() {
            out += &format!("key{i}={}\n", KEYS.iter().find(|e| e.0 == *k).map(|e| e.1).unwrap_or("Unknown"));
        }
        out += &format!(
            "palette={}\nvolume={}\nfullscreen={}\ninteger_scale={}\nlanguage={}\n",
            self.palette,
            self.volume,
            self.fullscreen as u8,
            self.integer_scale as u8,
            LANG_IDS[self.lang.min(2)]
        );
        let _ = std::fs::write(settings_path(), out);
    }
}

/// Keys that can be bound: code, file id, Spanish label, English label (also used in Japanese).
pub const KEYS: &[(KeyCode, &str, &str, &str)] = &[
    (KeyCode::Up, "Up", "FLECHA ARR", "UP ARROW"),
    (KeyCode::Down, "Down", "FLECHA ABA", "DOWN ARROW"),
    (KeyCode::Left, "Left", "FLECHA IZQ", "LEFT ARROW"),
    (KeyCode::Right, "Right", "FLECHA DER", "RIGHT ARROW"),
    (KeyCode::Space, "Space", "ESPACIO", "SPACE"),
    (KeyCode::Enter, "Enter", "INTRO", "ENTER"),
    (KeyCode::KpEnter, "KpEnter", "INTRO NUM", "NUM ENTER"),
    (KeyCode::Backspace, "Backspace", "RETROCESO", "BACKSPACE"),
    (KeyCode::Tab, "Tab", "TAB", "TAB"),
    (KeyCode::LeftShift, "LeftShift", "MAYUS IZQ", "L SHIFT"),
    (KeyCode::RightShift, "RightShift", "MAYUS DER", "R SHIFT"),
    (KeyCode::LeftControl, "LeftControl", "CTRL IZQ", "L CTRL"),
    (KeyCode::RightControl, "RightControl", "CTRL DER", "R CTRL"),
    (KeyCode::LeftAlt, "LeftAlt", "ALT", "ALT"),
    (KeyCode::A, "A", "A", "A"),
    (KeyCode::B, "B", "B", "B"),
    (KeyCode::C, "C", "C", "C"),
    (KeyCode::D, "D", "D", "D"),
    (KeyCode::E, "E", "E", "E"),
    (KeyCode::F, "F", "F", "F"),
    (KeyCode::G, "G", "G", "G"),
    (KeyCode::H, "H", "H", "H"),
    (KeyCode::I, "I", "I", "I"),
    (KeyCode::J, "J", "J", "J"),
    (KeyCode::K, "K", "K", "K"),
    (KeyCode::L, "L", "L", "L"),
    (KeyCode::M, "M", "M", "M"),
    (KeyCode::N, "N", "N", "N"),
    (KeyCode::O, "O", "O", "O"),
    (KeyCode::P, "P", "P", "P"),
    (KeyCode::Q, "Q", "Q", "Q"),
    (KeyCode::R, "R", "R", "R"),
    (KeyCode::S, "S", "S", "S"),
    (KeyCode::T, "T", "T", "T"),
    (KeyCode::U, "U", "U", "U"),
    (KeyCode::V, "V", "V", "V"),
    (KeyCode::W, "W", "W", "W"),
    (KeyCode::X, "X", "X", "X"),
    (KeyCode::Y, "Y", "Y", "Y"),
    (KeyCode::Z, "Z", "Z", "Z"),
    (KeyCode::Key0, "Key0", "0", "0"),
    (KeyCode::Key1, "Key1", "1", "1"),
    (KeyCode::Key2, "Key2", "2", "2"),
    (KeyCode::Key3, "Key3", "3", "3"),
    (KeyCode::Key4, "Key4", "4", "4"),
    (KeyCode::Key5, "Key5", "5", "5"),
    (KeyCode::Key6, "Key6", "6", "6"),
    (KeyCode::Key7, "Key7", "7", "7"),
    (KeyCode::Key8, "Key8", "8", "8"),
    (KeyCode::Key9, "Key9", "9", "9"),
    (KeyCode::Kp0, "Kp0", "NUM 0", "NUM 0"),
    (KeyCode::Kp1, "Kp1", "NUM 1", "NUM 1"),
    (KeyCode::Kp2, "Kp2", "NUM 2", "NUM 2"),
    (KeyCode::Kp3, "Kp3", "NUM 3", "NUM 3"),
    (KeyCode::Kp4, "Kp4", "NUM 4", "NUM 4"),
    (KeyCode::Kp5, "Kp5", "NUM 5", "NUM 5"),
    (KeyCode::Kp6, "Kp6", "NUM 6", "NUM 6"),
    (KeyCode::Kp7, "Kp7", "NUM 7", "NUM 7"),
    (KeyCode::Kp8, "Kp8", "NUM 8", "NUM 8"),
    (KeyCode::Kp9, "Kp9", "NUM 9", "NUM 9"),
    (KeyCode::Comma, "Comma", "COMA", "COMMA"),
    (KeyCode::Period, "Period", "PUNTO", "PERIOD"),
    (KeyCode::Minus, "Minus", "MENOS", "MINUS"),
    (KeyCode::Slash, "Slash", "BARRA", "SLASH"),
];

pub fn key_label(k: KeyCode, lang: usize) -> &'static str {
    KEYS.iter().find(|e| e.0 == k).map(|e| if lang == LANG_ES { e.2 } else { e.3 }).unwrap_or("?")
}

// ---------------------------------------------------------------- font

fn latin(c: char) -> Option<[u8; 7]> {
    Some(match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 17, 25, 21, 19, 17, 17],
        'Ñ' => [13, 22, 0, 25, 21, 19, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [31, 2, 4, 2, 1, 17, 14],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 30, 1, 1, 17, 14],
        '6' => [6, 8, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 2, 12],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '/' => [1, 1, 2, 4, 8, 16, 16],
        '<' => [2, 4, 8, 16, 8, 4, 2],
        '>' => [8, 4, 2, 1, 2, 4, 8],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        ' ' => [0; 7],
        _ => return None,
    })
}

/// Katakana: (base glyph, mark) where mark 1 = dakuten, 2 = handakuten.
fn kana(c: char) -> Option<([u8; 7], u8)> {
    let base = |c: char| -> [u8; 7] {
        match c {
            'ホ' => [4, 31, 4, 21, 21, 4, 12],
            'ー' => [0, 0, 0, 31, 0, 0, 0],
            'ス' => [31, 1, 2, 4, 10, 17, 0],
            'ツ' => [21, 21, 1, 2, 2, 4, 8],
            'ケ' => [8, 15, 18, 2, 2, 4, 8],
            'ル' => [10, 10, 10, 10, 11, 18, 20],
            'キ' => [4, 31, 4, 31, 4, 4, 4],
            'セ' => [8, 8, 31, 9, 10, 8, 7],
            'ッ' => [0, 0, 21, 21, 1, 2, 4],
            'テ' => [14, 0, 31, 4, 4, 4, 8],
            'イ' => [1, 2, 6, 10, 18, 2, 2],
            'カ' => [4, 4, 31, 5, 9, 9, 19],
            'メ' => [1, 1, 18, 12, 4, 10, 16],
            'ン' => [16, 8, 1, 1, 2, 4, 24],
            'サ' => [10, 31, 10, 10, 2, 4, 8],
            'ウ' => [4, 31, 17, 1, 2, 4, 8],
            'ト' => [8, 8, 12, 10, 8, 8, 8],
            'ム' => [4, 4, 8, 8, 18, 31, 1],
            'コ' => [31, 1, 1, 1, 1, 1, 31],
            'モ' => [31, 4, 31, 4, 4, 4, 3],
            'ノ' => [1, 1, 2, 2, 4, 8, 16],
            'ヲ' => [31, 1, 31, 1, 2, 4, 8],
            'ヤ' => [8, 31, 9, 10, 4, 4, 4],
            'エ' => [0, 31, 4, 4, 4, 31, 0],
            'シ' => [24, 1, 25, 2, 2, 4, 24],
            'タ' => [8, 15, 18, 10, 6, 4, 8],
            'ヒ' => [16, 16, 19, 28, 16, 16, 15],
            'リ' => [17, 17, 17, 17, 1, 2, 12],
            'ミ' => [24, 6, 0, 24, 6, 0, 30],
            'ニ' => [0, 14, 0, 0, 0, 31, 0],
            'ヘ' => [0, 8, 20, 2, 1, 0, 0],
            'オ' => [2, 31, 6, 10, 18, 2, 6],
            'ャ' => [0, 0, 8, 30, 10, 4, 4],
            'ョ' => [0, 0, 30, 2, 14, 2, 30],
            'ュ' => [0, 0, 0, 14, 2, 2, 31],
            'ィ' => [0, 0, 1, 2, 6, 10, 2],
            'ク' => [8, 15, 17, 1, 2, 4, 8],
            'ラ' => [14, 0, 31, 1, 2, 4, 8],
            'ロ' => [0, 31, 17, 17, 17, 31, 0],
            'ハ' => [0, 10, 10, 17, 17, 17, 0],
            'レ' => [16, 16, 16, 16, 17, 18, 28],
            'フ' => [31, 1, 1, 2, 2, 4, 8],
            'ア' => [31, 1, 10, 12, 8, 8, 16],
            'チ' => [3, 28, 4, 31, 4, 4, 8],
            'ナ' => [4, 4, 31, 4, 4, 8, 16],
            _ => [0; 7],
        }
    };
    let (b, mark) = match c {
        'ポ' => ('ホ', 2),
        'プ' => ('フ', 2),
        'ズ' => ('ス', 1),
        'ヅ' => ('ツ', 1),
        'ガ' => ('カ', 1),
        'ド' => ('ト', 1),
        'ゲ' => ('ケ', 1),
        'ゴ' => ('コ', 1),
        'ダ' => ('タ', 1),
        'ギ' => ('キ', 1),
        'ボ' => ('ホ', 1),
        'ゼ' => ('セ', 1),
        'グ' => ('ク', 1),
        'デ' => ('テ', 1),
        'ジ' => ('シ', 1),
        'ブ' => ('フ', 1),
        c if ('\u{30A0}'..='\u{30FF}').contains(&c) => (c, 0),
        _ => return None,
    };
    Some((base(b), mark))
}

fn advance(c: char) -> i32 {
    if kana(c).is_some() {
        8
    } else {
        6
    }
}

pub fn text_width(t: &str) -> i32 {
    t.chars().map(advance).sum()
}

// ---------------------------------------------------------------- drawing

pub struct Canvas<'a> {
    pub px: &'a mut [u8],
}

impl Canvas<'_> {
    fn set(&mut self, x: i32, y: i32, s: u8) {
        if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
            self.px[y as usize * W + x as usize] = s;
        }
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, s: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, s);
            }
        }
    }
    fn bits(&mut self, x: i32, y: i32, g: &[u8; 7], s: u8) {
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    self.set(x + col, y + row as i32, s);
                }
            }
        }
    }
    pub fn text(&mut self, x: i32, y: i32, t: &str, s: u8) {
        let mut cx = x;
        for c in t.chars() {
            if let Some((g, mark)) = kana(c) {
                self.bits(cx, y, &g, s);
                match mark {
                    1 => {
                        for (dx, dy) in [(5, 0), (5, 1), (7, 0), (7, 1)] {
                            self.set(cx + dx, y + dy, s);
                        }
                    }
                    2 => {
                        for (dx, dy) in [(6, -1), (5, 0), (7, 0), (6, 1)] {
                            self.set(cx + dx, y + dy, s);
                        }
                    }
                    _ => {}
                }
            } else if let Some(g) = latin(c.to_ascii_uppercase()).or_else(|| latin(c)) {
                self.bits(cx, y, &g, s);
            }
            cx += advance(c);
        }
    }
    /// Checkerboard of the lightest shade over the game picture (the "paused" veil).
    fn veil(&mut self) {
        for y in 0..H {
            for x in 0..W {
                if (x + y) % 2 == 0 {
                    self.px[y * W + x] = 0;
                }
            }
        }
    }
    /// A Game Boy style window: dark frame, light gap, mid-shade line, light fill.
    fn frame(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect(x, y, w, h, 3);
        self.rect(x + 2, y + 2, w - 4, h - 4, 0);
        self.rect(x + 3, y + 3, w - 6, h - 6, 2);
        self.rect(x + 4, y + 4, w - 8, h - 8, 0);
        for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
            self.set(cx, cy, 0);
        }
    }
    fn title(&mut self, x: i32, y: i32, w: i32, t: &str) {
        let tw = text_width(t) + 7;
        let tx = x + (w - tw) / 2;
        self.rect(tx, y - 4, tw, 11, 3);
        self.text(tx + 4, y - 2, t, 0);
    }
    fn cursor(&mut self, x: i32, y: i32, s: u8) {
        for r in 0..7 {
            let len = 4 - (r as i32 - 3).abs();
            for c in 0..len {
                self.set(x + c, y + r, s);
            }
        }
    }
    fn footer(&mut self, x: i32, bottom: i32, w: i32, t: &str) {
        let tw = text_width(t);
        self.rect(x + (w - tw) / 2 - 3, bottom - 4, tw + 5, 9, 3);
        self.text(x + (w - tw) / 2, bottom - 3, t, 0);
    }
}

// ---------------------------------------------------------------- menu logic

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Controls,
    Display,
    Sound,
    Language,
    Waiting(usize),
    /// Asking before leaving the level or the game: holds the main item.
    Confirm(usize),
}

pub enum Action {
    None,
    Close,
    PressStart,
    Quit,
    BackToMap,
    FileSelect,
    /// Switch between the editor-made world map and the cartridge's own.
    ToggleWorld,
}

// Main menu items (indexes into Texts::main).
const IT_CONTINUE: usize = 0;
const IT_CONTROLS: usize = 1;
const IT_DISPLAY: usize = 2;
const IT_SOUND: usize = 3;
const IT_LANGUAGE: usize = 4;
const IT_PAUSE: usize = 5;
const IT_QUIT: usize = 6;
const IT_MAP: usize = 7;
const IT_FILES: usize = 8;
const IT_WORLD: usize = 9;

pub struct Menu {
    screen: Screen,
    sel: usize,
    blink: u32,
    items: Vec<usize>,
    /// Some(on) when an editor-made world map exists for this cartridge.
    world: Option<bool>,
}

impl Menu {
    fn label<'a>(&self, t: &'a Texts, item: usize) -> &'a str {
        if item == IT_WORLD {
            if self.world == Some(true) {
                t.map_custom
            } else {
                t.map_original
            }
        } else {
            t.main[item]
        }
    }

    /// `mode` is the screen the game is on (a level, the map...).
    pub fn new(mode: cart_model::GameMode, world: Option<bool>) -> Menu {
        let in_level = mode == cart_model::GameMode::Playing;
        let on_map = mode == cart_model::GameMode::Map;
        // No "continue" item: Esc (or B) closes the menu. In a level the game's own pause comes first.
        let mut items = Vec::new();
        if in_level {
            items.push(IT_PAUSE);
        }
        items.extend([IT_CONTROLS, IT_DISPLAY, IT_SOUND, IT_LANGUAGE]);
        if in_level {
            items.push(IT_MAP);
        }
        if in_level || on_map {
            items.push(IT_FILES);
        }
        if world.is_some() {
            items.push(IT_WORLD);
        }
        items.push(IT_QUIT);
        Menu { screen: Screen::Main, sel: 0, blink: 0, items, world }
    }

    fn pos(&self, item: usize) -> usize {
        self.items.iter().position(|&i| i == item).unwrap_or(0)
    }

    /// For previews: show one screen ("main", "controls", "wait", "display", "sound", "language").
    pub fn preview(name: &str) -> Menu {
        let screen = match name {
            "controls" => Screen::Controls,
            "wait" => Screen::Waiting(4),
            "display" => Screen::Display,
            "sound" => Screen::Sound,
            "language" => Screen::Language,
            "confirm" => Screen::Confirm(IT_MAP),
            _ => Screen::Main,
        };
        let mut m = Menu::new(cart_model::GameMode::Playing, Some(true));
        m.screen = screen;
        m
    }

    fn count(&self) -> usize {
        match self.screen {
            Screen::Main => self.items.len(),
            Screen::Confirm(_) => 2,
            Screen::Controls => 10,
            Screen::Display => 4,
            Screen::Sound => 2,
            Screen::Language => 4,
            Screen::Waiting(_) => 0,
        }
    }

    fn go(&mut self, s: Screen, sel: usize) {
        self.screen = s;
        self.sel = sel;
    }

    pub fn update(&mut self, st: &mut Settings) -> Action {
        self.blink = self.blink.wrapping_add(1);
        if let Screen::Waiting(b) = self.screen {
            if is_key_pressed(KeyCode::Escape) {
                self.go(Screen::Controls, b);
                return Action::None;
            }
            if let Some(k) = get_last_key_pressed() {
                if KEYS.iter().any(|e| e.0 == k) {
                    // A key already used elsewhere swaps places, so no button is lost.
                    if let Some(other) = st.keys.iter().position(|&x| x == k) {
                        st.keys[other] = st.keys[b];
                    }
                    st.keys[b] = k;
                    st.save();
                    self.go(Screen::Controls, b);
                }
            }
            return Action::None;
        }
        let any = |ks: &[KeyCode]| ks.iter().any(|&k| is_key_pressed(k));
        let up = any(&[KeyCode::Up, st.keys[0]]);
        let down = any(&[KeyCode::Down, st.keys[1]]);
        let left = any(&[KeyCode::Left, st.keys[2]]);
        let right = any(&[KeyCode::Right, st.keys[3]]);
        let ok = any(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space, st.keys[4]]);
        let back = any(&[KeyCode::Escape, st.keys[5]]);
        let n = self.count();
        // Yes/No sit side by side, so they are chosen with Left/Right; lists use Up/Down.
        let (prev, next) = if matches!(self.screen, Screen::Confirm(_)) { (left, right) } else { (up, down) };
        if prev {
            self.sel = (self.sel + n - 1) % n;
        }
        if next {
            self.sel = (self.sel + 1) % n;
        }
        if back {
            let back_to = match self.screen {
                Screen::Main => return Action::Close,
                Screen::Controls => self.pos(IT_CONTROLS),
                Screen::Display => self.pos(IT_DISPLAY),
                Screen::Sound => self.pos(IT_SOUND),
                Screen::Confirm(item) => self.pos(item),
                _ => self.pos(IT_LANGUAGE),
            };
            self.go(Screen::Main, back_to);
            return Action::None;
        }
        let step: i32 = if right { 1 } else if left { -1 } else { 0 };
        match self.screen {
            Screen::Main => {
                if ok {
                    match self.items[self.sel] {
                        IT_CONTINUE => return Action::Close,
                        IT_CONTROLS => self.go(Screen::Controls, 0),
                        IT_DISPLAY => self.go(Screen::Display, 0),
                        IT_SOUND => self.go(Screen::Sound, 0),
                        IT_LANGUAGE => self.go(Screen::Language, st.lang),
                        IT_PAUSE => return Action::PressStart,
                        IT_WORLD => {
                            self.world = self.world.map(|on| !on);
                            return Action::ToggleWorld;
                        }
                        // "No" is selected first, so a stray key press never leaves the level.
                        item => self.go(Screen::Confirm(item), 1),
                    }
                }
            }
            Screen::Confirm(item) => {
                if ok {
                    if self.sel == 0 {
                        return match item {
                            IT_MAP => Action::BackToMap,
                            IT_FILES => Action::FileSelect,
                            _ => Action::Quit,
                        };
                    }
                    self.go(Screen::Main, self.pos(item));
                }
            }
            Screen::Controls => {
                if ok {
                    match self.sel {
                        0..=7 => self.screen = Screen::Waiting(self.sel),
                        8 => {
                            st.keys = DEFAULT_KEYS;
                            st.save();
                        }
                        _ => self.go(Screen::Main, self.pos(IT_CONTROLS)),
                    }
                }
            }
            Screen::Display => {
                let change = step != 0 || ok;
                match self.sel {
                    0 if change => {
                        st.palette = ((st.palette as i32 + if step == 0 { 1 } else { step } + 3) % 3) as usize;
                        st.save();
                    }
                    1 if change => {
                        st.fullscreen = !st.fullscreen;
                        set_fullscreen(st.fullscreen);
                        st.save();
                    }
                    2 if change => {
                        st.integer_scale = !st.integer_scale;
                        st.save();
                    }
                    3 if ok => self.go(Screen::Main, self.pos(IT_DISPLAY)),
                    _ => {}
                }
            }
            Screen::Sound => match self.sel {
                0 if step != 0 => {
                    st.volume = (st.volume as i32 + step).clamp(0, 10) as u8;
                    st.save();
                }
                1 if ok => self.go(Screen::Main, self.pos(IT_SOUND)),
                _ => {}
            },
            Screen::Language => {
                if ok {
                    if self.sel < 3 {
                        st.lang = self.sel;
                        st.save();
                    } else {
                        self.go(Screen::Main, self.pos(IT_LANGUAGE));
                    }
                }
            }
            Screen::Waiting(_) => {}
        }
        Action::None
    }

    pub fn draw(&self, px: &mut [u8], st: &Settings) {
        let t = texts(st.lang);
        let mut c = Canvas { px };
        c.veil();
        let blink_on = (self.blink / 16) % 2 == 0;
        let shade = |i: usize| if i == self.sel { 3 } else { 2 };
        match self.screen {
            Screen::Main => {
                let n = self.items.len() as i32;
                let gap = if n > 8 { 11 } else { 12 };
                let h = 28 + n * gap;
                let (x, w) = (14, 132);
                let y = ((H as i32 - h) / 2).max(6);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.pause);
                for (i, &item) in self.items.iter().enumerate() {
                    let ty = y + 12 + i as i32 * gap;
                    c.text(x + 18, ty, self.label(t, item), shade(i));
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_main);
            }
            Screen::Confirm(item) => {
                let (x, y, w, h) = (14, 40, 132, 64);
                c.frame(x, y, w, h);
                c.title(x, y, w, self.label(t, item));
                c.text(x + (w - text_width(t.sure)) / 2, y + 14, t.sure, 3);
                for (i, label) in [t.yes, t.no].iter().enumerate() {
                    let tx = x + 30 + i as i32 * 50;
                    c.text(tx, y + 32, label, shade(i));
                    if i == self.sel {
                        c.cursor(tx - 9, y + 32, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_language);
            }
            Screen::Controls => {
                let (x, y, w, h) = (2, 8, 156, 132);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.controls);
                for i in 0..10 {
                    let ty = y + 11 + i as i32 * 11;
                    if i < 8 {
                        c.text(x + 16, ty, t.buttons[i], shade(i));
                        let label = key_label(st.keys[i], st.lang);
                        c.text(x + w - 8 - text_width(label), ty, label, 3);
                    } else {
                        c.text(x + 16, ty, if i == 8 { t.restore } else { t.back }, shade(i));
                    }
                    if i == self.sel {
                        c.cursor(x + 8, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_controls);
            }
            Screen::Waiting(b) => {
                let (x, y, w, h) = (14, 44, 132, 56);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.buttons[b]);
                if blink_on {
                    c.text(x + (w - text_width(t.press_key)) / 2, y + 14, t.press_key, 3);
                }
                c.text(x + 18, y + 28, t.now, 2);
                c.text(x + 22 + text_width(t.now), y + 28, key_label(st.keys[b], st.lang), 3);
                c.text(x + (w - text_width(t.cancel)) / 2, y + 40, t.cancel, 2);
            }
            Screen::Display => {
                let (x, y, w, h) = (6, 30, 148, 84);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.display);
                let rows = [
                    (t.colours, t.palettes[st.palette % 3]),
                    (t.fullscreen, if st.fullscreen { t.yes } else { t.no }),
                    (t.scale, if st.integer_scale { t.sharp } else { t.fitted }),
                    (t.back, ""),
                ];
                for (i, (l, v)) in rows.iter().enumerate() {
                    let ty = y + 14 + i as i32 * 13;
                    c.text(x + 18, ty, l, shade(i));
                    if !v.is_empty() {
                        let vx = x + w - 14 - text_width(v);
                        c.text(vx, ty, v, 3);
                        if i == self.sel {
                            c.text(vx - 8, ty, "<", 2);
                            c.text(x + w - 12, ty, ">", 2);
                        }
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                    if i == 0 {
                        // palette swatch: the four shades of the chosen colours
                        let sx = x + 18 + text_width(l) + 4;
                        c.rect(sx, ty - 1, 18, 9, 3);
                        for k in 0..4u8 {
                            c.rect(sx + 1 + k as i32 * 4, ty, 4, 7, k);
                        }
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_change);
            }
            Screen::Sound => {
                let (x, y, w, h) = (14, 44, 132, 56);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.sound);
                c.text(x + 18, y + 13, t.volume, shade(0));
                let bx = x + 18;
                let by = y + 24;
                c.rect(bx - 1, by - 1, 10 * 9 + 1, 9, 3);
                c.rect(bx, by, 10 * 9 - 1, 7, 0);
                for i in 0..10 {
                    if i < st.volume as i32 {
                        c.rect(bx + 1 + i * 9, by + 1, 7, 5, 3);
                    } else {
                        c.rect(bx + 1 + i * 9, by + 3, 7, 1, 1);
                    }
                }
                c.text(bx + 94, by, &format!("{}", st.volume), 3);
                c.text(x + 18, y + 38, t.back, shade(1));
                c.cursor(x + 9, if self.sel == 0 { y + 13 } else { y + 38 }, 3);
            }
            Screen::Language => {
                let (x, y, w, h) = (22, 34, 116, 76);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.language);
                for i in 0..4 {
                    let ty = y + 13 + i as i32 * 12;
                    let label = if i < 3 { LANG_NAMES[i] } else { t.back };
                    c.text(x + 18, ty, label, shade(i));
                    if i < 3 && i == st.lang {
                        // the language in use is marked with a filled box, like a checked option
                        c.rect(x + w - 16, ty, 7, 7, 3);
                        c.rect(x + w - 15, ty + 1, 5, 5, 0);
                        c.rect(x + w - 14, ty + 2, 3, 3, 3);
                    } else if i < 3 {
                        c.rect(x + w - 16, ty, 7, 7, 3);
                        c.rect(x + w - 15, ty + 1, 5, 5, 0);
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_language);
            }
        }
    }
}

/// Small tag shown for a few seconds at start-up and when a level begins:
/// "ESC: MENU" and "ENTER: PAUSE".
pub fn draw_hint(px: &mut [u8], lang: usize) {
    let mut c = Canvas { px };
    let t = texts(lang);
    let w = text_width(t.hint).max(text_width(t.hint2)) + 7;
    let x = W as i32 - w - 2;
    c.rect(x, 2, w, 21, 3);
    c.text(x + 4, 4, t.hint, 0);
    c.text(x + 4, 14, t.hint2, 0);
}
