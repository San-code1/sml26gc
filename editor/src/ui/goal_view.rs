//! Where the level ends: its finish and secret exit blocks drawn over the level, placed with
//! "Place finish" and "Place secret exit" (a click puts it on the clicked cell) and dragged,
//! the finish's bell going along; where the secret exit leads.
use super::start_view::Placing;
use super::widgets::*;
use super::{Editor, Mode};
use cart_model::level_goals::Goal;
use cart_model::new_zones;
use cart_model::original_points::{self, OnMap};
use cart_model::secret_exits::{self, SecretTo};
use macroquad::prelude::*;

impl Editor {
    fn goal_of(p: Placing) -> Option<Goal> {
        match p {
            Placing::Finish => Some(Goal::Finish),
            Placing::Secret => Some(Goal::Secret),
            Placing::Start => None,
        }
    }

    /// Mouse on the level for the goals. True when it took the click (or a drag goes on).
    pub(super) fn update_goal(&mut self, m: Vec2) -> bool {
        let l = self.cur;
        if let Some(g) = self.drag_goal {
            if is_mouse_button_down(MouseButton::Left) {
                if let Some((c, r)) = self.cell_at(m) {
                    self.doc.place_goal(l, g, (c as usize, r as usize));
                }
            } else {
                self.drag_goal = None;
            }
            return true;
        }
        let Some(cell) = self.cell_at(m).map(|(c, r)| (c as usize, r as usize)) else { return false };
        if !is_mouse_button_pressed(MouseButton::Left) {
            return false;
        }
        let placing = self.placing.and_then(Self::goal_of);
        let over = [Goal::Finish, Goal::Secret].into_iter().find(|&g| self.doc.goal_cells(l, g).contains(&cell));
        let goal = match (placing, over) {
            (Some(g), _) => g,
            (None, Some(g)) if self.mode == Mode::Objects => g,
            _ => return false,
        };
        if goal == Goal::Finish && self.doc.ends_with_boss(l) {
            return true;
        }
        self.doc.snapshot(l);
        self.doc.place_goal(l, goal, cell);
        self.drag_goal = Some(goal);
        true
    }

    /// The finish and the secret exit over the blocks and objects, outlined and named.
    pub(super) fn draw_goals(&self) {
        let k = self.scale();
        for (g, name, idle) in [(Goal::Finish, tr(self.es, "FINISH", "META"), Color::new(0.3, 1.0, 0.4, 0.8)), (Goal::Secret, tr(self.es, "SECRET EXIT", "SALIDA SECRETA"), Color::new(0.4, 0.8, 1.0, 0.85))] {
            let on = self.placing.and_then(Self::goal_of) == Some(g);
            let c = if on { Color::new(1.0, 0.85, 0.1, 1.0) } else { idle };
            let cells = self.doc.goal_cells(self.cur, g);
            for &(cx, cy) in &cells {
                let (x, y) = (self.panel + (cx as f32 * BLOCK - self.cam.x) * k, (cy as f32 * BLOCK - self.cam.y) * k);
                draw_rectangle_lines(x - 2.0, y - 2.0, BLOCK * k + 4.0, BLOCK * k + 4.0, 2.0, c);
            }
            if let Some(&(cx, cy)) = cells.first() {
                let (x, y) = (self.panel + (cx as f32 * BLOCK - self.cam.x) * k, (cy as f32 * BLOCK - self.cam.y) * k);
                draw_text(name, x + 1.0, y - 5.0, 16.0, BLACK);
                draw_text(name, x, y - 6.0, 16.0, c);
            }
        }
    }

    /// Where the finish is, beside its button in the panel (or that the level has none).
    pub(super) fn finish_note(&self, y: f32) {
        let t = match self.doc.goal_cells(self.cur, Goal::Finish).first() {
            Some(&(c, r)) => format!("{c}, {r}"),
            None if self.doc.ends_with_boss(self.cur) => tr(self.es, "ends with its boss", "termina con su jefe").to_string(),
            None => tr(self.es, "none", "ninguna").to_string(),
        };
        draw_text(&t, 178.0, y + 18.0, 15.0, col(2));
    }

    /// Where the secret exit is, beside its button.
    pub(super) fn secret_note(&self, y: f32) {
        let t = match self.doc.goal_cells(self.cur, Goal::Secret).first() {
            Some(&(c, r)) => format!("{c}, {r}"),
            None => tr(self.es, "none", "ninguna").to_string(),
        };
        draw_text(&t, 178.0, y + 18.0, 15.0, col(2));
    }

    /// Where the shown level's secret exit leads, as the panel says it.
    pub(super) fn secret_leads(&self) -> String {
        let l = self.cur;
        match self.doc.level_secret(l) {
            Some(SecretTo::Point(p)) => self.point_name(p),
            Some(SecretTo::NewZone(k)) => {
                let entry = self.doc.new_zones.get(k).map_or(0, |z| z.entry);
                format!("{}, {} {entry:02X}", self.map_name(OnMap::NewZone(k)), tr(self.es, "entering on", "entrando por"))
            }
            None => match secret_exits::GAME.iter().find(|g| g.0 == self.doc.base_of(l)) {
                Some(&(_, p)) => format!("{} ({})", self.point_name(p), tr(self.es, "the game's", "la del juego")),
                None if self.doc.goal_cells(l, Goal::Secret).is_empty() => "-".to_string(),
                None => tr(self.es, "choose where (U)", "elige adonde (U)").to_string(),
            },
        }
    }

    /// A map point by its number and map (where the game draws it, or else the zone whose
    /// numbers it is in).
    fn point_name(&self, p: u8) -> String {
        let drawn = original_points::SPOTS.iter().find(|s| s.point == p).map(|s| s.map);
        let zone = || new_zones::BASES.iter().find(|b| b.has(p)).map(|b| OnMap::Zone(b.zone));
        format!("{} {p:02X}", self.map_name(drawn.or_else(zone).unwrap_or(OnMap::World)))
    }
}
