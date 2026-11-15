//! Drawing over the Game Boy picture in its four shades: a pixel font (Latin and katakana),
//! boxes, Game Boy style windows, and the start-up hint tag.
use super::i18n::texts;
use gb_core::ppu::{H, W};

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
    pub fn veil(&mut self) {
        for y in 0..H {
            for x in 0..W {
                if (x + y) % 2 == 0 {
                    self.px[y * W + x] = 0;
                }
            }
        }
    }
    /// A Game Boy style window: dark frame, light gap, mid-shade line, light fill.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect(x, y, w, h, 3);
        self.rect(x + 2, y + 2, w - 4, h - 4, 0);
        self.rect(x + 3, y + 3, w - 6, h - 6, 2);
        self.rect(x + 4, y + 4, w - 8, h - 8, 0);
        for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
            self.set(cx, cy, 0);
        }
    }
    pub fn title(&mut self, x: i32, y: i32, w: i32, t: &str) {
        let tw = text_width(t) + 7;
        let tx = x + (w - tw) / 2;
        self.rect(tx, y - 4, tw, 11, 3);
        self.text(tx + 4, y - 2, t, 0);
    }
    pub fn cursor(&mut self, x: i32, y: i32, s: u8) {
        for r in 0..7 {
            let len = 4 - (r as i32 - 3).abs();
            for c in 0..len {
                self.set(x + c, y + r, s);
            }
        }
    }
    pub fn footer(&mut self, x: i32, bottom: i32, w: i32, t: &str) {
        let tw = text_width(t);
        self.rect(x + (w - tw) / 2 - 3, bottom - 4, tw + 5, 9, 3);
        self.text(x + (w - tw) / 2, bottom - 3, t, 0);
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
