//! The side panel's palettes: the level's 128 blocks, or the 128 object types.
use super::widgets::*;
use super::Editor;
use macroquad::prelude::*;

impl Editor {
    /// Object types 00-7F; the ones this level uses are dark.
    pub(super) fn object_palette(&mut self, mut y: f32) {
        draw_text(&format!("{} #{:02X}", tr(self.es, "OBJECT TYPES  chosen", "TIPOS DE OBJETO  elegido"), self.kind), 10.0, y + 12.0, 18.0, col(3));
        y += 18.0;
        let used: Vec<u8> = self.doc.levels[self.cur].content.objs.iter().map(|o| o.kind).collect();
        let cell = (34.0f32).min((screen_height() - y - 30.0) / 16.0);
        let mut chosen = None;
        for k in 0..128u8 {
            let r = Rect::new(10.0 + (k % 8) as f32 * cell, y + (k / 8) as f32 * cell, cell - 2.0, cell - 2.0);
            let in_level = used.contains(&k);
            let (bg, fg) = if k == self.kind { (col(3), col(0)) } else if in_level { (col(2), col(0)) } else { (col(1), col(3)) };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            if let Some((tex, _, _)) = self.look(k) {
                let f = ((r.w - 4.0) / tex.width()).min((r.h - 4.0) / tex.height()).min(2.0);
                let (tw, th) = (tex.width() * f, tex.height() * f);
                draw_texture_ex(tex, r.x + (r.w - tw) / 2.0, r.y + (r.h - th) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() });
                draw_text(&format!("{k:02X}"), r.x + 1.0, r.y + 9.0, 11.0, fg);
            } else {
                let t = format!("{k:02X}");
                let dim = measure_text(&t, None, 16, 1.0);
                let known = self.looks.contains_key(&(self.cur, k));
                draw_text(&t, r.x + (r.w - dim.width) / 2.0, r.y + (r.h + dim.offset_y) / 2.0, 16.0, if known { fg } else { Color::new(fg.r, fg.g, fg.b, 0.5) });
            }
            let mp: Vec2 = mouse_position().into();
            if r.contains(mp) && is_mouse_button_pressed(MouseButton::Left) {
                chosen = Some(k);
            }
        }
        if let Some(k) = chosen {
            self.choose_kind(k);
        }
        let tip = tr(self.es, "Dark: used in this level. Del: delete selected", "Oscuros: usados en este nivel. Supr: borrar");
        draw_text(tip, 10.0, (y + 16.0 * cell + 14.0).min(screen_height() - 4.0), 15.0, col(2));
    }

    /// The level's blocks as the game draws them.
    pub(super) fn block_palette(&mut self, mut y: f32) {
        draw_text(&format!("{} #{:02X}", tr(self.es, "BLOCKS  selected", "BLOQUES  elegido"), self.block), 10.0, y + 12.0, 18.0, col(3));
        y += 18.0;
        let Some(atlas) = &self.atlases[self.cur] else { return };
        let cell: f32 = 34.0;
        let avail = screen_height() - y - 8.0;
        let cell = cell.min(avail / 16.0);
        for id in 0..128u8 {
            let x = 10.0 + (id % 8) as f32 * cell;
            let yy = y + (id / 8) as f32 * cell;
            draw_texture_ex(
                atlas,
                x,
                yy,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect::new((id % 16) as f32 * 16.0, (id / 16) as f32 * 16.0, 16.0, 16.0)),
                    dest_size: Some(vec2(cell - 2.0, cell - 2.0)),
                    ..Default::default()
                },
            );
            if id == self.block {
                draw_rectangle_lines(x - 2.0, yy - 2.0, cell + 2.0, cell + 2.0, 3.0, col(3));
            }
            let mp: Vec2 = mouse_position().into();
            if Rect::new(x, yy, cell - 2.0, cell - 2.0).contains(mp) && is_mouse_button_pressed(MouseButton::Left) {
                self.block = id;
            }
        }
        let tip = tr(self.es, "Right click on the level: pick a block", "Clic derecho en el nivel: coger bloque");
        draw_text(tip, 10.0, (y + 16.0 * cell + 14.0).min(screen_height() - 4.0), 15.0, col(2));
    }
}
