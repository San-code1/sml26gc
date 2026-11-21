//! New blank zone: one of the base zones is chosen while its map and its tiles are shown;
//! Create adds an empty new zone with its graphics and music (`EditorDocument::blank_zone`).
use super::shortcuts::{keys, Action};
use super::widgets::*;
use super::Editor;
use cart_model::new_zones::BASES;
use cart_model::original_points::OnMap;
use cart_model::zone_maps;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, is_key_pressed};
use crate::domain::document::ZoneRefusal;

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const MAP_SCALE: f32 = 3.0;
const TILE_SCALE: f32 = 2.0;

impl Editor {
    /// Open the chooser on the base of the map shown, or the first base.
    pub(super) fn open_blank_zone_chooser(&mut self) {
        let shown = match self.world_map {
            OnMap::Zone(z) => Some(z),
            OnMap::NewZone(k) => self.doc.new_zones.get(k).map(|z| z.base),
            OnMap::World => None,
        };
        let i = shown.and_then(|z| BASES.iter().position(|b| b.zone == z)).unwrap_or(0);
        self.blank_zone_choice = Some(i);
        self.status = tr(self.lang, "New blank zone: choose the zone whose graphics and music it has. Esc: cancel.").into();
    }

    /// The chooser open (for the check pictures).
    pub fn show_blank_zone_chooser(&mut self) {
        self.open_blank_zone_chooser();
    }

    /// The zone editor's map list unfolded (for the check pictures).
    pub fn open_map_list(&mut self) {
        self.maps_open = true;
    }

    fn base_atlas(&mut self, z: usize) -> Option<Texture2D> {
        if self.zone_atlases[z].is_none() {
            if !self.art.booted() {
                self.ensure_art(self.cur);
            }
            self.zone_atlases[z] = self.art.zone(z).map(make_world_atlas);
        }
        self.zone_atlases[z].clone()
    }

    /// The chooser's frame, in place of the zone editor's.
    pub(super) fn blank_zone_frame(&mut self) {
        let Some(i) = self.blank_zone_choice else { return };
        let z = BASES[i].zone;
        let atlas = self.base_atlas(z);
        clear_background(col(3));
        let lang = self.lang;
        let mut choice = Some(i);
        let mut create = false;
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let w = self.panel - 2.0 * PAD - 8.0;
        put_text(tr(lang, "NEW BLANK ZONE"), PAD, 25.0, 20.0, col(3));
        put_text(tr(lang, "Graphics and music of:"), PAD, 52.0, 15.0, INFO);
        let mut y = 60.0;
        for (j, b) in BASES.iter().enumerate() {
            let name = zone_name(lang, b.zone);
            if button(Rect::new(PAD, y, w, ROW), name, j == i, 15.0) {
                choice = Some(j);
            }
            y += ROW + 4.0;
        }
        if is_key_pressed(KeyCode::Down) {
            choice = Some((i + 1) % BASES.len());
        }
        if is_key_pressed(KeyCode::Up) {
            choice = Some((i + BASES.len() - 1) % BASES.len());
        }
        y += 10.0;
        let notes = [
            format!("{} {}.", tr(lang, "It plays with the tiles, map animations and music of the"), zone_name(lang, z)),
            tr(lang, "It starts as a plain picture and one point, its entry: no paths, levels, exits or door.").to_string(),
            format!("{}: {}", tr(lang, "Key"), keys(Action::BlankZone)),
        ];
        for line in notes.iter().flat_map(|l| wrap(l, 34)) {
            put_text(&line, PAD, y + 12.0, 15.0, col(3));
            y += 17.0;
        }
        y += 10.0;
        let half = (w - 4.0) / 2.0;
        if button(Rect::new(PAD, y, half, ROW), tr(lang, "Create"), false, 15.0) || is_key_pressed(KeyCode::Enter) {
            create = true;
        }
        if button(Rect::new(PAD + half + 4.0, y, half, ROW), tr(lang, "Cancel (Esc)"), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            choice = None;
        }
        // the view: the base zone's map as the project has it, and its tiles
        let x0 = self.panel + 16.0;
        let name = zone_name(lang, z);
        put_text(&format!("{} {}", tr(lang, "MAP OF"), name.to_uppercase()), x0, 26.0, 18.0, col(0));
        let x1 = x0 + zone_maps::W as f32 * 8.0 * MAP_SCALE + 24.0;
        put_text(tr(lang, "ITS TILES"), x1, 26.0, 18.0, col(0));
        match atlas {
            Some(atlas) => {
                let s = 8.0 * MAP_SCALE;
                for ty in 0..zone_maps::H {
                    for tx in 0..zone_maps::W {
                        let t = self.doc.tile(OnMap::Zone(z), tx, ty);
                        let src = Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0);
                        draw_texture_ex(&atlas, x0 + tx as f32 * s, 36.0 + ty as f32 * s, WHITE, DrawTextureParams { source: Some(src), dest_size: Some(vec2(s, s)), ..Default::default() });
                    }
                }
                let side = atlas.width() * TILE_SCALE;
                draw_texture_ex(&atlas, x1, 36.0, WHITE, DrawTextureParams { dest_size: Some(vec2(side, atlas.height() * TILE_SCALE)), ..Default::default() });
            }
            None => {
                put_text(tr(lang, "Capturing the zone's tiles..."), x0, 60.0, 16.0, col(0));
            }
        }
        let h = 22.0;
        draw_rectangle(self.panel, screen_height() - h, screen_width() - self.panel, h, col(1));
        status_message(&self.status, self.panel, screen_width(), screen_height() - h, h, 16.0, self.severity());
        if create {
            self.blank_zone_choice = None;
            self.make_blank_zone(z);
        } else {
            self.blank_zone_choice = choice;
            if choice.is_none() {
                self.status = tr(lang, "No zone made.").into();
            }
        }
    }

    fn make_blank_zone(&mut self, z: usize) {
        match self.doc.blank_zone(z) {
            Ok(k) => {
                self.clipboard = None;
                self.maps_open = false;
                self.show_map(OnMap::NewZone(k));
                self.point = self.doc.new_zones[k].entry as usize;
                self.status = format!("{} {}", self.map_name(OnMap::NewZone(k)), tr(self.lang, "made blank: draw it in Picture, add points in Points; it has no door yet."));
                self.mark(Severity::Done);
            }
            Err(ZoneRefusal::TooManyZones(max)) => {
                self.status = format!("{} {max}.", tr(self.lang, "No more new zones: there can be"));
                self.mark(Severity::Warning);
            }
            Err(_) => {}
        }
    }
}
