//! Objects mode: each level's objects drawn with the game's own pictures, placed, dragged,
//! retyped and deleted with the mouse.
use super::widgets::*;
use super::{Editor, Mode};
use cart_model::objects::Obj;
use macroquad::prelude::*;

impl Editor {
    /// Capture one more object picture (types used in this level first, then the chosen
    /// type, then the rest), one per frame so the window stays responsive.
    pub(super) fn pump_looks(&mut self) {
        if self.mode != Mode::Objects || self.in_world || !self.art.booted() {
            return;
        }
        let l = self.cur;
        let mut want: Vec<u8> = self.doc.levels[l]
            .content
            .objs
            .iter()
            .map(|o| o.kind)
            .collect();
        want.push(self.kind);
        want.extend(0..128u8);
        let Some(k) = want
            .into_iter()
            .find(|k| !self.looks.contains_key(&(l, *k)))
        else {
            return;
        };
        let look = self
            .art
            .look(&self.doc.rom, &self.doc.original, l, k)
            .flatten();
        let v = look.map(|lk| (look_texture(&lk), lk.dx as f32, lk.dy as f32));
        self.looks.insert((l, k), v);
    }

    pub(super) fn look(&self, k: u8) -> Option<&(Texture2D, f32, f32)> {
        self.looks.get(&(self.cur, k)).and_then(|v| v.as_ref())
    }

    /// The object under a point of the level (level pixels), topmost first.
    fn object_at(&self, w: Vec2) -> Option<usize> {
        self.doc.levels[self.cur]
            .content
            .objs
            .iter()
            .rposition(|o| Rect::new(o.x as f32, o.y as f32, 16.0, 16.0).contains(w))
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
                self.kind = o.kind;
                self.drag_obj = Some((i, w - vec2(o.x as f32, o.y as f32)));
            } else {
                let i = self.doc.add_object(
                    l,
                    Obj {
                        x: snap(w.x, 4088.0),
                        y: snap(w.y, 760.0),
                        kind: self.kind,
                        flag: false,
                    },
                );
                self.sel_obj = Some(i);
                self.drag_obj = Some((i, vec2(4.0, 4.0)));
            }
        }
        if let Some((i, off)) = self.drag_obj {
            if is_mouse_button_down(MouseButton::Left) {
                let p = w - off;
                self.doc
                    .move_object(l, i, snap(p.x, 4088.0), snap(p.y, 760.0));
            } else {
                self.drag_obj = None;
            }
        }
        let delete_here = is_mouse_button_pressed(MouseButton::Right) && in_view;
        let target = if delete_here {
            self.object_at(w)
        } else if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
            self.sel_obj
        } else {
            None
        };
        if let Some(i) = target {
            self.doc.snapshot(l);
            self.doc.remove_object(l, i);
            self.sel_obj = None;
            self.drag_obj = None;
        }
    }

    /// Pick an object type in the palette: the selected object takes it too.
    pub(super) fn choose_kind(&mut self, k: u8) {
        self.kind = k;
        if let Some(i) = self.sel_obj {
            self.doc.set_object_kind(self.cur, i, k);
        }
    }

    pub(super) fn draw_objects(&self) {
        let objs_mode = self.mode == Mode::Objects;
        for (i, o) in self.doc.levels[self.cur].content.objs.iter().enumerate() {
            let x = PANEL + (o.x as f32 - self.cam.x) * self.scale();
            let y = (o.y as f32 - self.cam.y) * self.scale();
            let s = 16.0 * self.scale();
            if x + s < PANEL || x > screen_width() || y + s < 0.0 || y > screen_height() {
                continue;
            }
            let a = if objs_mode { 0.75 } else { 0.4 };
            let sel = objs_mode && self.sel_obj == Some(i);
            if let Some((tex, dx, dy)) = self.look(o.kind) {
                // the object's own picture, as the game draws it
                let tint = if objs_mode {
                    WHITE
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.7)
                };
                draw_texture_ex(
                    tex,
                    x + dx * self.scale(),
                    y + dy * self.scale(),
                    tint,
                    DrawTextureParams {
                        dest_size: Some(vec2(
                            tex.width() * self.scale(),
                            tex.height() * self.scale(),
                        )),
                        ..Default::default()
                    },
                );
                draw_rectangle_lines(
                    x,
                    y,
                    s,
                    s,
                    if sel { 3.0 } else { 1.0 },
                    if sel {
                        WHITE
                    } else {
                        Color::new(1.0, 1.0, 1.0, if objs_mode { 0.35 } else { 0.0 })
                    },
                );
                if objs_mode {
                    draw_text(
                        &format!("{:02X}", o.kind),
                        x + 2.0,
                        y + s - 2.0,
                        14.0,
                        Color::new(1.0, 1.0, 1.0, 0.8),
                    );
                }
                continue;
            }
            draw_rectangle(x, y, s, s, Color::new(0.12, 0.12, 0.12, a));
            draw_rectangle_lines(
                x,
                y,
                s,
                s,
                if sel { 3.0 } else { 1.5 },
                if sel {
                    WHITE
                } else {
                    Color::new(0.77, 0.81, 0.63, a + 0.2)
                },
            );
            let t = format!("{:02X}", o.kind);
            let fs = (s * 0.55).clamp(10.0, 26.0);
            let dim = measure_text(&t, None, fs as u16, 1.0);
            draw_text(
                &t,
                x + (s - dim.width) / 2.0,
                y + (s + dim.offset_y) / 2.0,
                fs,
                Color::new(0.9, 0.95, 0.8, a + 0.25),
            );
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
                format!(
                    "#{:02X}  x {} y {}  {}",
                    o.kind,
                    o.x,
                    o.y,
                    tr(
                        self.es,
                        "drag: move   right click: delete",
                        "arrastra: mover   clic dcho: borrar"
                    )
                )
            }
            None => format!(
                "{} #{:02X}",
                tr(self.es, "click: place object", "clic: poner objeto"),
                self.kind
            ),
        };
        draw_text(&tip, PANEL + 8.0, ch - 8.0, 20.0, WHITE);
    }
}
