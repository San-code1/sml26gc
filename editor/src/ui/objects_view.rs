//! Objects mode: each level's objects drawn with the game's own pictures, placed, dragged,
//! retyped and deleted with the mouse.
use super::widgets::*;
use super::{Editor, Mode};
use crate::domain::look_cache;
use cart_model::art::Look;
use cart_model::objects::Obj;
use macroquad::prelude::*;

/// New pictures between two writes of the cache.
const LOOKS_SAVE_EVERY: usize = 16;

impl Editor {
    /// Take the object pictures the background capture has made (asking for the shown
    /// level's first) and keep them in the cache now and then.
    pub(super) fn pump_looks(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.mode == Mode::Objects && !self.in_world {
            let mut want: Vec<(usize, u8)> = self.doc.levels[self.cur].content.objs.iter().map(|o| (base, o.kind)).collect();
            want.extend(self.kind.map(|k| (base, k)));
            want.retain(|key| !self.looks.contains_key(key));
            self.look_loader.prefer(&want);
        }
        for (l, k, look) in self.look_loader.take() {
            self.add_look(l, k, look);
        }
        let all = self.look_data.len();
        if all >= self.looks_saved + LOOKS_SAVE_EVERY || (all > self.looks_saved && self.look_loader.done()) {
            self.look_store.save(&look_cache::to_text(&self.look_data));
            self.looks_saved = all;
        }
    }

    pub(super) fn add_look(&mut self, l: usize, k: u8, look: Option<Look>) {
        let v = look.as_ref().map(|lk| (look_texture(lk), lk.dx as f32, lk.dy as f32));
        if let Some(lk) = &look {
            self.captured.insert((l, k), lk.clone());
        }
        self.looks.insert((l, k), v);
        self.look_data.push((l, k, look));
    }

    pub(super) fn look(&self, k: u8) -> Option<&(Texture2D, f32, f32)> {
        self.looks.get(&(self.doc.base_of(self.cur), k)).and_then(|v| v.as_ref())
    }

    /// For the check pictures: wait for the shown level's object pictures.
    pub(super) fn wait_for_level_looks(&mut self) {
        let base = self.doc.base_of(self.cur);
        let want: Vec<(usize, u8)> = self.doc.levels[self.cur].content.objs.iter().map(|o| (base, o.kind)).collect();
        self.look_loader.prefer(&want);
        while want.iter().any(|key| !self.looks.contains_key(key)) {
            let Some((l, k, look)) = self.look_loader.wait() else { break };
            self.add_look(l, k, look);
        }
    }

    /// The object under a point of the level (level pixels), topmost first.
    fn object_at(&self, w: Vec2) -> Option<usize> {
        self.doc.levels[self.cur].content.objs.iter().rposition(|o| Rect::new(o.x as f32, o.y as f32, 16.0, 16.0).contains(w))
    }

    pub(super) fn update_objects(&mut self, m: Vec2) {
        let in_view = self.cell_at(m).is_some();
        let w = self.world_pos(m);
        let snap = |v: f32, max: f32| ((v / 8.0).floor() * 8.0).clamp(0.0, max) as u16;
        let l = self.cur;
        if is_mouse_button_pressed(MouseButton::Left) && in_view {
            self.doc.snapshot(l);
            if let Some(i) = self.object_at(w) {
                let o = self.doc.levels[l].content.objs[i];
                self.sel_obj = Some(i);
                self.kind = Some(o.kind);
                self.drag_obj = Some((i, w - vec2(o.x as f32, o.y as f32)));
            } else if let Some(kind) = self.kind {
                let i = self.doc.add_object(l, Obj { x: snap(w.x, 4088.0), y: snap(w.y, 760.0), kind, flag: false });
                self.sel_obj = Some(i);
                self.drag_obj = Some((i, vec2(4.0, 4.0)));
            } else {
                self.sel_obj = None;
            }
        }
        if let Some((i, off)) = self.drag_obj {
            if is_mouse_button_down(MouseButton::Left) {
                let p = w - off;
                self.doc.move_object(l, i, snap(p.x, 4088.0), snap(p.y, 760.0));
            } else {
                self.drag_obj = None;
            }
        }
        // right click: on an object, delete it; elsewhere, put the palette's type down
        if is_mouse_button_pressed(MouseButton::Right) && in_view {
            match self.object_at(w) {
                Some(i) => self.delete_object(i),
                None => self.drop_kind(),
            }
        }
    }

    fn delete_object(&mut self, i: usize) {
        self.doc.snapshot(self.cur);
        self.doc.remove_object(self.cur, i);
        self.sel_obj = None;
        self.drag_obj = None;
    }

    pub(super) fn delete_selected_object(&mut self) {
        if let Some(i) = self.sel_obj {
            self.delete_object(i);
        }
    }

    /// Pick an object type in the palette: the selected object takes it too. The type already
    /// chosen is put down instead.
    pub(super) fn choose_kind(&mut self, k: u8) {
        if self.kind == Some(k) {
            self.drop_kind();
            return;
        }
        self.kind = Some(k);
        if let Some(i) = self.sel_obj {
            self.doc.set_object_kind(self.cur, i, k);
        }
    }

    /// No type chosen in the palette and no object selected: clicks only pick objects.
    fn drop_kind(&mut self) {
        self.kind = None;
        self.sel_obj = None;
    }

    pub(super) fn draw_objects(&self) {
        let objs_mode = self.mode == Mode::Objects;
        for (i, o) in self.doc.levels[self.cur].content.objs.iter().enumerate() {
            let x = self.panel + (o.x as f32 - self.cam.x) * self.scale();
            let y = (o.y as f32 - self.cam.y) * self.scale();
            let s = 16.0 * self.scale();
            if x + s < self.panel || x > screen_width() || y + s < 0.0 || y > screen_height() {
                continue;
            }
            let a = if objs_mode { 0.75 } else { 0.4 };
            let sel = objs_mode && self.sel_obj == Some(i);
            if let Some((tex, dx, dy)) = self.look(o.kind) {
                // the object's own picture, as the game draws it
                let tint = if objs_mode { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.7) };
                draw_texture_ex(tex, x + dx * self.scale(), y + dy * self.scale(), tint, DrawTextureParams {
                    dest_size: Some(vec2(tex.width() * self.scale(), tex.height() * self.scale())),
                    ..Default::default()
                });
                if objs_mode {
                    marker(Rect::new(x, y, s, s), if sel { 3.0 } else { 1.5 }, if sel { MARK_SELECTED } else { MARK });
                    marked_text(&format!("{:02X}", o.kind), x + 2.0, y + s - 2.0, 14.0, if sel { MARK_SELECTED } else { MARK });
                }
                continue;
            }
            draw_rectangle(x, y, s, s, Color::new(0.12, 0.12, 0.12, a));
            let c = if sel { MARK_SELECTED } else { Color { a: a + 0.25, ..MARK } };
            marker(Rect::new(x, y, s, s), if sel { 3.0 } else { 1.5 }, c);
            let t = format!("{:02X}", o.kind);
            let fs = (s * 0.55).clamp(10.0, 26.0);
            let dim = measure_text(&t, None, fs as u16, 1.0);
            marked_text(&t, x + (s - dim.width) / 2.0, y + (s + dim.offset_y) / 2.0, fs, c);
        }
    }

    /// What a click would do, at the bottom of the view.
    pub(super) fn object_tip(&self, ch: f32) {
        let m: Vec2 = mouse_position().into();
        if self.cell_at(m).is_none() {
            return;
        }
        let tip = match self.object_at(self.world_pos(m)) {
            Some(i) => {
                let o = self.doc.levels[self.cur].content.objs[i];
                format!("#{:02X}  x {} y {}  {}", o.kind, o.x, o.y, tr(self.es, "drag: move   right click: delete", "arrastra: mover   clic dcho: borrar"))
            }
            None => match self.kind {
                Some(k) => format!("{} #{k:02X}   {}", tr(self.es, "click: place object", "clic: poner objeto"), tr(self.es, "right click: none", "clic dcho: ninguno")),
                None => tr(self.es, "choose an object in the palette to place it", "elige un objeto en la paleta para ponerlo").to_string(),
            },
        };
        draw_text(&tip, self.panel + 8.0, ch - 8.0, 20.0, WHITE);
    }
}
