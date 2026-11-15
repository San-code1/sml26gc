//! The editor's window (macroquad), split by panel. `Editor` holds what is on screen (the
//! level shown, camera, zoom, tool, selection, textures); what is being edited lives in
//! `EditorDocument`, and saving / graphics / starting the game go through the application
//! layer and the ports.
mod custom_view;
mod level_view;
mod objects_view;
mod palette;
mod status_bar;
mod widgets;
mod world_view;

pub use widgets::col;

use crate::application::art_service::{ArtService, Capture};
use crate::application::save::{self, SaveError};
use crate::domain::document::{EditorDocument, Saved};
use crate::domain::tools::{self, GridCanvas, Pointer, Tool, ToolKind};
use crate::ports::{EditedCopyStore, GameLauncher};
use cart_model::levels::{COLS, LEVELS, ROWS};
use cart_model::original_points::OnMap;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;
use std::collections::HashMap;
use widgets::*;

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Blocks,
    Objects,
}

/// Tabs of the world map editor.
pub const TAB_PICTURE: usize = 0;
pub const TAB_POINTS: usize = 1;
pub const TAB_CUSTOM: usize = 2;

pub struct Editor {
    es: bool,
    pub(crate) doc: EditorDocument,
    store: Box<dyn EditedCopyStore>,
    launcher: Box<dyn GameLauncher>,
    art: ArtService,
    atlases: Vec<Option<Texture2D>>,
    world_atlas: Option<Texture2D>,
    zone_atlases: Vec<Option<Texture2D>>,
    /// What each object type looks like in each level: texture and where its top-left sits
    /// relative to the object's position. None = draws nothing.
    looks: HashMap<(usize, u8), Option<(Texture2D, f32, f32)>>,
    pub(crate) cur: usize,
    block: u8,
    cam: Vec2,
    zoom: usize,
    tool: Box<dyn Tool>,
    grid: bool,
    status: String,
    pan: Option<(Vec2, Vec2)>,
    space_cache: (usize, usize),
    space_at: f64,
    obj_space: (usize, usize),
    overview: Option<Texture2D>,
    overview_stale: bool,
    show_overview: bool,
    pub(crate) mode: Mode,
    kind: u8,
    sel_obj: Option<usize>,
    drag_obj: Option<(usize, Vec2)>,
    in_world: bool,
    tile: u8,
    pub(crate) world_tab: usize,
    /// Which map picture the world map editor shows: the world map or a zone map.
    pub(crate) world_map: OnMap,
    point: usize,
    world_cam: Vec2,
    level_cam: Vec2,
    // the editor-made world map
    pub(crate) sel_point: Option<usize>,
    link_from: Option<usize>,
    drag_point: bool,
}

/// Painting blocks into the level shown.
struct LevelCanvas<'a> {
    doc: &'a mut EditorDocument,
    level: usize,
    block: u8,
    changed: &'a mut bool,
}

impl GridCanvas for LevelCanvas<'_> {
    fn begin_edit(&mut self) {
        self.doc.snapshot(self.level);
    }
    fn paint(&mut self, x: i32, y: i32) {
        if self.doc.set_block(self.level, x, y, self.block) {
            *self.changed = true;
        }
    }
}

/// Painting tiles into a map picture (the world map or a zone map).
struct WorldCanvas<'a> {
    doc: &'a mut EditorDocument,
    map: OnMap,
    tile: u8,
}

impl GridCanvas for WorldCanvas<'_> {
    fn begin_edit(&mut self) {
        self.doc.world_snapshot();
    }
    fn paint(&mut self, x: i32, y: i32) {
        self.doc
            .set_tile(self.map, x as usize, y as usize, self.tile);
    }
}

fn left_button() -> Pointer {
    Pointer {
        pressed: is_mouse_button_pressed(MouseButton::Left),
        down: is_mouse_button_down(MouseButton::Left),
        released: is_mouse_button_released(MouseButton::Left),
    }
}

impl Editor {
    pub fn new(
        doc: EditorDocument,
        store: Box<dyn EditedCopyStore>,
        launcher: Box<dyn GameLauncher>,
        had_copy: bool,
        es: bool,
    ) -> Editor {
        let status = if !had_copy {
            tr(es, "Original cartridge opened. Saving makes an edited copy; the original is never changed.", "Cartucho original abierto. Al guardar se crea una copia editada; el original no se toca.").to_string()
        } else {
            format!(
                "{} {}",
                tr(es, "Edited copy opened:", "Copia editada abierta:"),
                store.path().display()
            )
        };
        Editor {
            es,
            doc,
            store,
            launcher,
            art: ArtService::new(),
            atlases: (0..LEVELS).map(|_| None).collect(),
            world_atlas: None,
            zone_atlases: ZONES.iter().map(|_| None).collect(),
            looks: HashMap::new(),
            cur: 0,
            block: 0,
            cam: Vec2::ZERO,
            zoom: 3,
            tool: tools::make(ToolKind::Pencil),
            grid: true,
            status,
            pan: None,
            space_cache: (0, 1),
            space_at: -1.0,
            obj_space: (0, 1),
            overview: None,
            overview_stale: true,
            show_overview: true,
            mode: Mode::Blocks,
            kind: 0,
            sel_obj: None,
            drag_obj: None,
            in_world: false,
            tile: 0,
            world_tab: TAB_PICTURE,
            world_map: OnMap::World,
            point: 0,
            world_cam: Vec2::ZERO,
            level_cam: Vec2::ZERO,
            sel_point: None,
            link_from: None,
            drag_point: false,
        }
    }

    /// One frame: input, then drawing.
    pub fn frame(&mut self) {
        if self.in_world {
            self.update_world();
            self.draw_world();
        } else {
            self.update_level();
            self.draw_level();
        }
    }

    fn scale(&self) -> f32 {
        ZOOMS[self.zoom]
    }

    fn set_tool(&mut self, kind: ToolKind) {
        if self.tool.kind() != kind {
            self.tool = tools::make(kind);
        }
    }

    /// Where the mouse points, in the shown picture's pixels.
    fn world_pos(&self, m: Vec2) -> Vec2 {
        (m - vec2(PANEL, 0.0)) / self.scale() + self.cam
    }

    /// Keyboard shortcuts shared by both views: grid and tools (Ctrl ones are per view).
    fn tool_keys(&mut self, ctrl: bool) {
        if !ctrl {
            if is_key_pressed(KeyCode::G) {
                self.grid = !self.grid;
            }
            if is_key_pressed(KeyCode::B) {
                self.set_tool(ToolKind::Pencil);
            }
            if is_key_pressed(KeyCode::R) {
                self.set_tool(ToolKind::Box);
            }
        }
    }

    /// Pan with arrows / WASD, middle mouse or Space + left mouse; zoom at the mouse with
    /// the wheel; keep the camera inside a picture of `size` pixels. Returns whether Space
    /// is held, the visible size and the furthest camera position.
    fn navigate(&mut self, m: Vec2, ctrl: bool, size: Vec2) -> (bool, Vec2, Vec2) {
        let speed = 600.0 * get_frame_time() / self.scale();
        let mut d = Vec2::ZERO;
        if is_key_down(KeyCode::Left) || (!ctrl && is_key_down(KeyCode::A)) {
            d.x -= speed;
        }
        if is_key_down(KeyCode::Right) || (!ctrl && is_key_down(KeyCode::D)) {
            d.x += speed;
        }
        if is_key_down(KeyCode::Up) || (!ctrl && is_key_down(KeyCode::W)) {
            d.y -= speed;
        }
        if is_key_down(KeyCode::Down) || (!ctrl && is_key_down(KeyCode::S)) {
            d.y += speed;
        }
        self.cam += d;
        let space = is_key_down(KeyCode::Space);
        if m.x >= PANEL
            && (is_mouse_button_pressed(MouseButton::Middle)
                || (space && is_mouse_button_pressed(MouseButton::Left)))
        {
            self.pan = Some((m, self.cam));
        }
        if let Some((m0, c0)) = self.pan {
            if is_mouse_button_down(MouseButton::Middle)
                || (space && is_mouse_button_down(MouseButton::Left))
            {
                self.cam = c0 - (m - m0) / self.scale();
            } else {
                self.pan = None;
            }
        }
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && m.x >= PANEL {
            let before = self.world_pos(m);
            if wheel > 0.0 && self.zoom + 1 < ZOOMS.len() {
                self.zoom += 1;
            } else if wheel < 0.0 && self.zoom > 0 {
                self.zoom -= 1;
            }
            self.cam = before - (m - vec2(PANEL, 0.0)) / self.scale();
        }
        let view = vec2(screen_width() - PANEL, screen_height() - STATUS_H) / self.scale();
        let max = (size - view).max(Vec2::ZERO);
        self.cam = self.cam.clamp(Vec2::ZERO, max);
        (space, view, max)
    }

    /// Let the game load the level (from the world map) and keep its graphics.
    fn ensure_art(&mut self, l: usize) {
        match self.art.ensure_level(&self.doc.rom, l) {
            Capture::Ready => return,
            Capture::Failed(e) => {
                self.status = e;
                return;
            }
            Capture::NotLoaded => {
                self.status = tr(
                    self.es,
                    "The game didn't load this level; its graphics may be wrong.",
                    "El juego no cargo este nivel; sus graficos pueden no ser correctos.",
                )
                .into();
            }
            Capture::Captured => {}
        }
        self.atlases[l] = self.art.level(l).map(make_atlas);
    }

    /// The shown map's tiles, as the game shows them (captured on that map's screen).
    fn ensure_world_art(&mut self) {
        if self.map_atlas().is_some() {
            return;
        }
        if !self.art.booted() {
            self.ensure_art(self.cur);
        }
        match self.world_map {
            OnMap::World => self.world_atlas = self.art.world().map(make_world_atlas),
            OnMap::Zone(z) => self.zone_atlases[z] = self.art.zone(z).map(make_world_atlas),
        }
    }

    /// The tiles of the map picture shown.
    fn map_atlas(&self) -> Option<&Texture2D> {
        match self.world_map {
            OnMap::World => self.world_atlas.as_ref(),
            OnMap::Zone(z) => self.zone_atlases[z].as_ref(),
        }
    }

    /// Show another map picture (the world map or a zone map).
    pub fn show_map(&mut self, map: OnMap) {
        if map == self.world_map {
            return;
        }
        self.world_map = map;
        self.cam = Vec2::ZERO;
        self.sel_point = None;
        self.link_from = None;
        self.ensure_world_art();
    }

    pub fn select_level(&mut self, l: usize) {
        self.cur = l;
        self.ensure_art(l);
        // Start the view on the part of the map that isn't the background filler.
        let (c0, r0, r1) = self.doc.content_area(l);
        let view_h = (screen_height() - STATUS_H) / self.scale();
        let content_h = (r1 + 1 - r0) as f32 * BLOCK;
        let y = if content_h <= view_h {
            (r1 + 1) as f32 * BLOCK - view_h
        } else {
            r0 as f32 * BLOCK
        };
        self.cam = vec2(c0 as f32 * BLOCK, y.max(0.0));
        self.overview_stale = true;
        let shared = self.doc.sharing(l);
        self.status = if shared.is_empty() {
            format!("{} {:02}", tr(self.es, "Level", "Nivel"), l)
        } else {
            format!(
                "{} {:02}: {} {:?}",
                tr(self.es, "Level", "Nivel"),
                l,
                tr(
                    self.es,
                    "its map is also used by level(s)",
                    "su mapa tambien lo usa(n) el/los nivel(es)"
                ),
                shared
            )
        };
    }

    pub fn enter_world(&mut self, on: bool) {
        if on == self.in_world {
            return;
        }
        if on {
            self.level_cam = self.cam;
            self.ensure_world_art();
            self.cam = self.world_cam;
            self.status = tr(
                self.es,
                "World map: paint its tiles, or choose which level each point opens.",
                "Mapa de mundos: pinta sus tiles o elige que nivel abre cada punto.",
            )
            .into();
        } else {
            self.world_cam = self.cam;
            self.cam = self.level_cam;
        }
        self.in_world = on;
    }

    fn undo(&mut self) {
        if self.doc.undo(self.cur) {
            self.sel_obj = None;
            self.overview_stale = true;
        }
    }

    fn redo(&mut self) {
        if self.doc.redo(self.cur) {
            self.sel_obj = None;
            self.overview_stale = true;
        }
    }

    fn restore_original(&mut self) {
        self.doc.restore_level(self.cur);
        self.sel_obj = None;
        self.overview_stale = true;
        self.status = tr(
            self.es,
            "Level restored to the original (save to keep it).",
            "Nivel restaurado al original (guarda para conservarlo).",
        )
        .into();
    }

    /// Save the edited copy; the status bar says what happened.
    fn save(&mut self) -> bool {
        let es = self.es;
        match save::save(&mut self.doc, &*self.store) {
            Ok(parts) => {
                if parts.iter().any(|p| matches!(p, Saved::Objects(_))) {
                    self.sel_obj = None;
                }
                let notes: Vec<String> = parts
                    .iter()
                    .map(|p| match p {
                        Saved::Bank(n) => n.to_string(),
                        Saved::Objects(n) => n.to_string(),
                        Saved::WorldMap => tr(es, "world map", "mapa de mundos").to_string(),
                        Saved::LevelPoints => tr(es, "level points", "puntos de nivel").to_string(),
                        Saved::CustomMap => {
                            tr(es, "your world map", "tu mapa de mundos").to_string()
                        }
                    })
                    .collect();
                self.status = format!(
                    "{} {} {}",
                    tr(es, "Saved", "Guardado en"),
                    self.store.path().display(),
                    if notes.is_empty() {
                        String::new()
                    } else {
                        format!("({})", notes.join(", "))
                    }
                );
                true
            }
            Err(SaveError::Layout(e)) => {
                self.status = format!("{}: {e}", tr(es, "Not saved", "No guardado"));
                false
            }
            Err(SaveError::CustomMap(e)) => {
                self.status = format!(
                    "{}: {e}",
                    tr(es, "Could not save the map", "No se pudo guardar el mapa")
                );
                false
            }
            Err(SaveError::Write(e)) => {
                self.status = format!("{}: {e}", tr(es, "Could not save", "No se pudo guardar"));
                false
            }
        }
    }

    /// Save, then try the level shown in the game.
    fn play(&mut self) {
        if !self.save() {
            return;
        }
        self.status = match self.launcher.launch(self.store.path(), Some(self.cur)) {
            Ok(()) => format!(
                "{} {:02}",
                tr(self.es, "Playing level", "Jugando el nivel"),
                self.cur
            ),
            Err(e) => format!("{} {e}", tr(self.es, "Could not start", "No se pudo abrir")),
        };
    }

    /// Save, then start the game on its world map (to try the editor-made map).
    fn play_map(&mut self) {
        if !self.save() {
            return;
        }
        self.status = match self.launcher.launch(self.store.path(), None) {
            Ok(()) => tr(
                self.es,
                "Game started with your map.",
                "Juego abierto con tu mapa.",
            )
            .into(),
            Err(e) => e,
        };
    }

    /// For the check pictures: capture this level's object pictures first.
    pub fn capture_level_looks(&mut self) {
        for _ in 0..self.doc.levels[self.cur].content.objs.len() + 1 {
            self.pump_looks();
        }
    }

    /// For the check picture of the editor-made map: a sample route (not saved).
    pub fn sample_custom_map(&mut self) {
        use cart_model::custom_map::Point;
        if !self.doc.custom.points.is_empty() {
            return;
        }
        for (i, (x, y, l)) in [
            (120, 120, Some(0u8)),
            (180, 140, None),
            (240, 150, Some(1)),
            (300, 200, Some(2)),
        ]
        .iter()
        .enumerate()
        {
            self.doc.custom.points.push(Point {
                x: *x,
                y: *y,
                level: *l,
            });
            if i > 0 {
                self.doc.custom.paths.push((i - 1, i));
            }
        }
        self.sel_point = Some(2);
    }
}

// The level's size in pixels.
const LEVEL_SIZE: Vec2 = Vec2::new(COLS as f32 * BLOCK, ROWS as f32 * BLOCK);
