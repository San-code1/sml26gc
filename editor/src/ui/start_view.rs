//! Where Mario begins the level: drawn with the game's own picture of him in both modes,
//! placed with "Place start" (a click puts his feet on the clicked block) and dragged.
use super::testing_view::warn_col;
use super::widgets::*;
use super::{Editor, Mode};
use cart_model::level_start::{Start, SPRITE_H, SPRITE_TOP, SPRITE_W};
use macroquad::prelude::*;

/// What a click on the level places.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Placing {
    Start,
    Finish,
    Secret,
    Checkpoint,
    /// Not placed: a click starts the game there.
    PlayHere,
}

impl Editor {
    /// Capture Mario's picture once the console has booted.
    pub(super) fn pump_mario(&mut self) {
        if self.mario.is_none() && self.art.booted() {
            self.mario = Some(self.art.mario().flatten().map(|l| (look_texture(&l), l.dx as f32, l.dy as f32)));
        }
    }

    /// Mario's picture at the start, in level pixels: top-left and size.
    fn mario_rect(&self) -> Rect {
        let s = self.doc.level_start(self.cur);
        match self.mario.as_ref().and_then(|m| m.as_ref()) {
            Some((t, dx, dy)) => Rect::new(s.mario_x as f32 + dx, s.mario_y as f32 + dy, t.width(), t.height()),
            None => Rect::new(s.mario_x as f32, (s.mario_y + SPRITE_TOP) as f32, SPRITE_W as f32, (SPRITE_H - 2) as f32),
        }
    }

    /// "Place start" on or off (the key or the button again, or Esc, leaves it).
    pub(super) fn toggle_placing(&mut self, what: Placing) {
        self.placing = if self.placing == Some(what) { None } else { Some(what) };
        // the finish may be far from the view: show it
        let goal = match self.placing {
            Some(Placing::Finish) => Some(cart_model::level_goals::Goal::Finish),
            Some(Placing::Secret) => Some(cart_model::level_goals::Goal::Secret),
            _ => None,
        };
        if let Some(&(c, r)) = goal.and_then(|g| self.doc.goal_cells(self.cur, g).first().copied()).as_ref() {
            self.look = Some((vec2(c as f32, r as f32) * BLOCK, false));
        }
        self.status = match self.placing {
            Some(Placing::Start) => tr(self.es, "Click the block Mario should stand on when the level begins. Esc: done.", "Haz clic en el bloque sobre el que Mario empieza el nivel. Esc: terminar.").into(),
            Some(Placing::Secret) => tr(self.es, "Click the cell of the secret exit. Esc: done.", "Haz clic en la celda de la salida secreta. Esc: terminar.").into(),
            Some(Placing::Finish) => tr(self.es, "Click the cell where the level ends (its bell goes along). Esc: done.", "Haz clic en la celda donde termina el nivel (su campana va con ella). Esc: terminar.").into(),
            Some(Placing::PlayHere) => tr(self.es, "Click where Mario should start: the copy is saved and the game starts there. Esc: cancel.", "Haz clic donde debe empezar Mario: se guarda la copia y el juego empieza ahi. Esc: cancelar.").into(),
            Some(Placing::Checkpoint) => tr(self.es, "Click the cell for a checkpoint bell; Mario starts again just left of it, on the ground below. Esc: done.", "Haz clic en la celda de una campana de control; Mario vuelve a salir a su izquierda, sobre el suelo. Esc: terminar.").into(),
            None => String::new(),
        };
    }

    /// The start at level pixel `w`: Mario's feet on the top of the block there.
    pub(super) fn start_at(w: Vec2) -> Start {
        let x = ((w.x - SPRITE_W as f32 / 2.0).max(0.0) / 8.0).floor() as u16 * 8;
        let feet = (w.y.max(0.0) / BLOCK).floor() as u16 * BLOCK as u16;
        Start::standing(x, feet)
    }

    /// Mouse on the level for the start. True when it took the click (or a drag goes on).
    pub(super) fn update_start(&mut self, m: Vec2) -> bool {
        let w = self.world_pos(m);
        let l = self.cur;
        if self.drag_start {
            if is_mouse_button_down(MouseButton::Left) {
                let feet = w + vec2(0.0, (SPRITE_H - SPRITE_TOP) as f32 / 2.0);
                self.doc.set_level_start(l, Self::start_at(feet));
            } else {
                self.drag_start = false;
            }
            return true;
        }
        let over = self.mario_rect().contains(w);
        let may_drag = self.placing == Some(Placing::Start) || self.mode == Mode::Objects;
        if is_mouse_button_pressed(MouseButton::Left) && self.cell_at(m).is_some() && (self.placing == Some(Placing::Start) || (over && may_drag)) {
            self.doc.snapshot(l);
            if !over {
                self.doc.set_level_start(l, Self::start_at(w));
            }
            self.drag_start = true;
            return true;
        }
        if let Some(p) = self.placing.filter(|_| is_mouse_button_pressed(MouseButton::Right)) {
            self.toggle_placing(p);
            return true;
        }
        self.placing.is_some()
    }

    /// Mario at the start, over the blocks and objects, and (with objects or in Objects mode)
    /// the screen the level begins on, where objects never appear.
    pub(super) fn draw_start(&self) {
        let r = self.mario_rect();
        let k = self.scale();
        let c = &self.doc.levels[self.cur].content;
        let crowded = c.objs.iter().any(|o| c.start.on_first_screen(o.x, o.y));
        if self.mode == Mode::Objects || crowded {
            let (fx, fy, fw, fh) = c.start.first_screen();
            let at = vec2(self.panel + (fx as f32 - self.cam.x) * k, (fy as f32 - self.cam.y) * k);
            let col = if crowded { warn_col() } else { MARK };
            marker(Rect::new(at.x, at.y, fw as f32 * k, fh as f32 * k), 2.0, col);
            let t = tr(self.es, "first screen: objects here never appear", "primera pantalla: aqui los objetos no aparecen");
            marked_text(t, at.x + 4.0, at.y + 14.0, 14.0, col);
        }
        let (x, y) = (self.panel + (r.x - self.cam.x) * k, (r.y - self.cam.y) * k);
        match self.mario.as_ref().and_then(|m| m.as_ref()) {
            Some((t, _, _)) => draw_texture_ex(t, x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h) * k), ..Default::default() }),
            None => draw_rectangle_lines(x, y, r.w * k, r.h * k, 2.0, col(3)),
        }
        let c = if self.placing == Some(Placing::Start) { Color::new(1.0, 0.85, 0.1, 1.0) } else { Color::new(1.0, 1.0, 1.0, 0.6) };
        marker(Rect::new(x - 2.0, y - 2.0, r.w * k + 4.0, r.h * k + 4.0), 2.0, c);
        let t = tr(self.es, "START", "INICIO");
        draw_text(t, x + 1.0, y - 5.0, 16.0, BLACK);
        draw_text(t, x, y - 6.0, 16.0, c);
    }
}
