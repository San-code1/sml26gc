//! Shared look and small pieces of the interface: colours, layout sizes, buttons, word
//! wrapping, and textures made from the game's captured graphics.
use cart_model::art::{LevelArt, Look, WorldArt};
use cart_model::levels::{COLS, ROWS};
use macroquad::prelude::*;

/// Ctrl+Z: undo; Ctrl+Y or Ctrl+Shift+Z: redo. Returns (undo, redo) pressed this frame.
pub fn undo_keys(ctrl: bool) -> (bool, bool) {
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let z = ctrl && is_key_pressed(KeyCode::Z);
    (z && !shift, (z && shift) || (ctrl && is_key_pressed(KeyCode::Y)))
}

pub const PAL: [u32; 4] = [0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1F];
pub const PANEL: f32 = 300.0;
pub const STATUS_H: f32 = 26.0;
pub const BLOCK: f32 = 16.0;
pub const ZOOMS: [f32; 7] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

pub fn col(i: usize) -> Color {
    let c = PAL[i];
    Color::from_rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255)
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

/// Draw a button; true when it was clicked this frame.
pub fn button(r: Rect, label: &str, active: bool, size: f32) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = r.contains(m);
    let (bg, fg) = if active { (col(3), col(0)) } else if hover { (col(1), col(3)) } else { (col(0), col(3)) };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(3));
    let dim = measure_text(label, None, size as u16, 1.0);
    draw_text(label, r.x + (r.w - dim.width) / 2.0, r.y + (r.h + dim.offset_y) / 2.0 - 1.0, size, fg);
    hover && is_mouse_button_pressed(MouseButton::Left)
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
