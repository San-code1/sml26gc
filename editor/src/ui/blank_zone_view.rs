//! New blank zone: one of the base zones is chosen while its map and its tiles are shown;
//! Create adds an empty new zone with its graphics and music (`EditorDocument::blank_zone`).
use super::shortcuts::{keys, Action};
use super::widgets::*;
use super::Editor;
use cart_model::new_zones::BASES;
use cart_model::original_points::OnMap;
use cart_model::zone_maps::{self, ZONES};
use macroquad::prelude::*;
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
        self.status = tr(self.es, "New blank zone: choose the zone whose graphics and music it has. Esc: cancel.", "Zona en blanco: elige la zona de la que toma graficos y musica. Esc: cancelar.").into();
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
        let es = self.es;
        let mut choice = Some(i);
        let mut create = false;
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let w = self.panel - 2.0 * PAD - 8.0;
        draw_text(tr(es, "NEW BLANK ZONE", "ZONA EN BLANCO"), PAD, 25.0, 20.0, col(3));
        draw_text(tr(es, "Graphics and music of:", "Graficos y musica de:"), PAD, 52.0, 15.0, col(2));
        let mut y = 60.0;
        for (j, b) in BASES.iter().enumerate() {
            let name = if es { ZONES[b.zone].name_es } else { ZONES[b.zone].name };
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
            format!("{} {}.", tr(es, "It plays with the tiles, map animations and music of the", "Se juega con los tiles, animaciones del mapa y musica de la"), if es { ZONES[z].name_es } else { ZONES[z].name }),
            tr(es, "It starts as a plain picture and one point, its entry: no paths, levels, exits or door.", "Empieza con el dibujo liso y un solo punto, su entrada: sin caminos, niveles, salidas ni puerta.").to_string(),
            format!("{}: {}", tr(es, "Key", "Tecla"), keys(Action::BlankZone)),
        ];
        for line in notes.iter().flat_map(|l| wrap(l, 34)) {
            draw_text(&line, PAD, y + 12.0, 15.0, col(3));
            y += 17.0;
        }
        y += 10.0;
        let half = (w - 4.0) / 2.0;
        if button(Rect::new(PAD, y, half, ROW), tr(es, "Create", "Crear"), false, 15.0) || is_key_pressed(KeyCode::Enter) {
            create = true;
        }
        if button(Rect::new(PAD + half + 4.0, y, half, ROW), tr(es, "Cancel (Esc)", "Cancelar (Esc)"), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            choice = None;
        }
        // the view: the base zone's map as the project has it, and its tiles
        let x0 = self.panel + 16.0;
        let name = if es { ZONES[z].name_es } else { ZONES[z].name };
        draw_text(&format!("{} {}", tr(es, "MAP OF", "MAPA DE"), name.to_uppercase()), x0, 26.0, 18.0, col(0));
        let x1 = x0 + zone_maps::W as f32 * 8.0 * MAP_SCALE + 24.0;
        draw_text(tr(es, "ITS TILES", "SUS TILES"), x1, 26.0, 18.0, col(0));
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
                draw_text(tr(es, "Capturing the zone's tiles...", "Capturando los tiles de la zona..."), x0, 60.0, 16.0, col(0));
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
                self.status = tr(es, "No zone made.", "No se ha creado ninguna zona.").into();
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
                self.status = format!("{} {}", self.map_name(OnMap::NewZone(k)), tr(self.es, "made blank: draw it in Picture, add points in Points; it has no door yet.", "creada en blanco: dibujala en Dibujo y anade puntos en Puntos; aun no tiene puerta."));
                self.mark(Severity::Done);
            }
            Err(ZoneRefusal::TooManyZones(max)) => {
                self.status = format!("{} {max}.", tr(self.es, "No more new zones: there can be", "No caben mas zonas nuevas: puede haber"));
                self.mark(Severity::Warning);
            }
            Err(_) => {}
        }
    }
}
