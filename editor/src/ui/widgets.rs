//! Shared look and small pieces of the interface: colours, layout sizes, buttons, word
//! wrapping, and textures made from the game's captured graphics.
use cart_model::art::{LevelArt, Look, WorldArt};
use cart_model::levels::{COLS, ROWS};
use macroquad::prelude::*;

pub const PAL: [u32; 4] = [0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1F];
/// The side panel's narrowest width; it can be dragged wider.
pub const PANEL_MIN: f32 = 300.0;
pub const STATUS_H: f32 = 26.0;
pub const BLOCK: f32 = 16.0;
pub const ZOOMS: [f32; 7] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

pub fn col(i: usize) -> Color {
    let c = PAL[i];
    Color::from_rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255)
}

/// Marks drawn over the game's pictures (objects, map points): colours none of the game's
/// four shades is near, each over a dark border.
pub const MARK: Color = Color::new(1.0, 0.25, 0.85, 1.0);
pub const MARK_SELECTED: Color = Color::new(1.0, 0.9, 0.1, 1.0);

/// A rectangle outline that shows on any picture: a dark border under a bright line.
pub fn marker(r: Rect, thick: f32, c: Color) {
    draw_rectangle_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, thick + 2.0, Color::new(0.0, 0.0, 0.0, c.a));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, thick, c);
}

/// What kind of message the status says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Info,
    /// Something was done that changed what is edited.
    Done,
    /// Something was not done, or needs a look, without harm.
    Warning,
    /// Something failed.
    Error,
}

impl Severity {
    pub fn colour(self) -> Color {
        match self {
            Severity::Info => col(3),
            Severity::Done => Color::new(0.08, 0.40, 0.14, 1.0),
            Severity::Warning => Color::new(0.80, 0.42, 0.0, 1.0),
            Severity::Error => Color::new(0.67, 0.16, 0.12, 1.0),
        }
    }

    /// A sign before the text, so the kind shows without the colour too.
    fn sign(self) -> &'static str {
        match self {
            Severity::Info => "",
            Severity::Done => "+ ",
            Severity::Warning => "! ",
            Severity::Error => "x ",
        }
    }
}

/// Text in a kind's colour and weight, without its sign (a message's further lines).
fn kind_text(text: &str, x: f32, y: f32, size: f32, kind: Severity) {
    let c = kind.colour();
    draw_text(&text, x, y, size, c);
    if kind != Severity::Info {
        draw_text(&text, x + 1.0, y, size, c);
    }
}

/// `t` cut into lines no wider than `width` pixels at `size` (a word wider than that has a
/// line of its own).
pub fn wrap_px(t: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines: Vec<String> = vec![String::new()];
    for w in t.split(' ') {
        let last = lines.last_mut().expect("never empty");
        let tried = if last.is_empty() { w.to_string() } else { format!("{last} {w}") };
        if !last.is_empty() && measure_text(&tried, None, size as u16, 1.0).width > width {
            lines.push(w.to_string());
        } else {
            *last = tried;
        }
    }
    lines
}

/// A message on as many lines as it needs, from `y` (the first line's base) down; returns the
/// height it took.
pub fn message_lines(t: &str, x: f32, y: f32, width: f32, size: f32, kind: Severity) -> f32 {
    let step = size + 2.0;
    let lines = wrap_px(&format!("{}{t}", kind.sign()), width - 12.0, size);
    for (i, line) in lines.iter().enumerate() {
        kind_text(line, x, y + i as f32 * step, size, kind);
    }
    lines.len() as f32 * step
}

/// A status bar's message: one line in the bar from `left` to `right`, the lines it does not
/// fit growing above the bar (over the view) on the same background.
pub fn status_message(t: &str, left: f32, right: f32, bar_top: f32, bar_h: f32, size: f32, kind: Severity) {
    let width = right - left - 4.0;
    let lines = wrap_px(&format!("{}{t}", kind.sign()), width - 12.0, size);
    let extra = (lines.len().max(1) - 1) as f32 * (size + 2.0);
    if extra > 0.0 {
        draw_rectangle(left, bar_top - extra, right - left, extra, col(1));
        draw_rectangle(left, bar_top - extra, right - left, 1.0, col(2));
    }
    let base = bar_top + bar_h / 2.0 + size / 2.0 - 4.0 - extra;
    message_lines(t, left + 8.0, base, width, size, kind);
}

/// Text with a dark shadow, readable on any picture.
pub fn marked_text(t: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_text(t, x + 1.0, y + 1.0, size, Color::new(0.0, 0.0, 0.0, c.a));
    draw_text(t, x, y, size, c);
}

/// English or Spanish text.
pub fn tr(es: bool, en: &'static str, es_text: &'static str) -> &'static str {
    if es {
        es_text
    } else {
        en
    }
}

/// Split text into lines of at most `n` characters at spaces.
pub fn wrap(t: &str, n: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for w in t.split(' ') {
        let last = lines.last_mut().unwrap();
        if !last.is_empty() && last.len() + 1 + w.len() > n {
            lines.push(w.to_string());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(w);
        }
    }
    lines
}

/// Draw a button, its label made smaller if it doesn't fit; true when it was clicked this
/// frame.
pub fn button(r: Rect, label: &str, active: bool, size: f32) -> bool {
    let hover = r.contains(mouse_position().into());
    let (bg, fg) = if active { (col(3), col(0)) } else if hover { (col(1), col(3)) } else { (col(0), col(3)) };
    framed_label(r, label, bg, fg, col(3), 2.0, size);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// A button set apart from the choices beside it (dark, with a thin frame): for the actions
/// under a list of things to choose.
pub fn action_button(r: Rect, label: &str, size: f32) -> bool {
    let hover = r.contains(mouse_position().into());
    let bg = if hover { col(3) } else { col(2) };
    framed_label(r, label, bg, col(0), col(3), 1.0, size);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// The text lands on whole pixels: the font blurs between them.
fn framed_label(r: Rect, label: &str, bg: Color, fg: Color, frame: Color, thick: f32, size: f32) {
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, thick, frame);
    let mut size = size;
    let mut dim = measure_text(label, None, size as u16, 1.0);
    while dim.width > r.w - 6.0 && size > 10.0 {
        size -= 1.0;
        dim = measure_text(label, None, size as u16, 1.0);
    }
    let x = (r.x + (r.w - dim.width) / 2.0).round();
    let y = (r.y + (r.h + dim.offset_y) / 2.0 - 1.0).round();
    draw_text(label, x, y, size, fg);
}

fn texture(img: &Image) -> Texture2D {
    let t = Texture2D::from_image(img);
    t.set_filter(FilterMode::Nearest);
    t
}

/// An object's picture (transparent where it draws nothing).
pub fn look_texture(l: &Look) -> Texture2D {
    let mut img = Image::gen_image_color(l.w as u16, l.h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..l.h {
        for x in 0..l.w {
            if let Some(s) = l.px[y * l.w + x] {
                img.set_pixel(x as u32, y as u32, col(s as usize));
            }
        }
    }
    texture(&img)
}

/// One pixel per block: the block's average shade, for the whole-level overview.
pub fn make_overview(art: &LevelArt, map: &[u8]) -> Texture2D {
    let mut mean = [0usize; 128];
    for id in 0..128u8 {
        let mut s = 0;
        for y in 0..16 {
            for x in 0..16 {
                s += art.block_pixel(id, x, y) as usize;
            }
        }
        mean[id as usize] = (s + 128) / 256;
    }
    let mut img = Image::gen_image_color(COLS as u16, ROWS as u16, BLACK);
    for (i, &b) in map.iter().enumerate() {
        img.set_pixel((i % COLS) as u32, (i / COLS) as u32, col(mean[b as usize & 0x7F].min(3)));
    }
    texture(&img)
}

/// The level's 128 blocks, 16 per row.
pub fn make_atlas(art: &LevelArt) -> Texture2D {
    let mut img = Image::gen_image_color(256, 128, BLACK);
    for id in 0..128u8 {
        for y in 0..16 {
            for x in 0..16 {
                let s = art.block_pixel(id, x, y) as usize;
                img.set_pixel((id as u32 % 16) * 16 + x as u32, (id as u32 / 16) * 16 + y as u32, col(s));
            }
        }
    }
    texture(&img)
}

/// The world map's 256 tiles, 16 per row.
pub fn make_world_atlas(art: &WorldArt) -> Texture2D {
    let mut img = Image::gen_image_color(128, 128, BLACK);
    for t in 0..=255u8 {
        for y in 0..8 {
            for x in 0..8 {
                img.set_pixel((t as u32 % 16) * 8 + x as u32, (t as u32 / 16) * 8 + y as u32, col(art.pixel(t, x, y) as usize));
            }
        }
    }
    texture(&img)
}
