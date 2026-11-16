//! The editor's window (macroquad), split by panel. `Editor` holds what is on screen (the
//! level shown, camera, zoom, tool, selection, textures); what is being edited lives in
//! `EditorDocument`, and saving / graphics / starting the game go through the application
//! layer and the ports.
mod blank_view;
mod files_view;
mod grid_edit;
mod level_panel;
mod level_view;
mod menu;
mod names_view;
mod objects_view;
mod palette;
mod shortcuts;
mod goal_view;
mod start_view;
mod status_bar;
mod target_picker;
mod testing_view;
mod widgets;
mod world_view;

pub use widgets::col;

use crate::application::art_service::{ArtService, Capture};
use crate::application::look_loader::LookLoader;
use crate::application::music::MusicPlayer;
use crate::domain::look_cache;
use crate::application::save::{self, SaveError};
use crate::domain::document::{EditorDocument, Grid, Saved};
use crate::domain::prefs::{Prefs, PALETTE_ZOOMS};
use crate::domain::stamp::Stamp;
use crate::domain::tools::{self, GridCanvas, Pointer, Tool, ToolKind};
use crate::ports::{EditedCopyStore, FileDialogs, GameLauncher, LookStore, MenuBar, PrefsStore, SoundOut, TextFiles};
use grid_edit::Brush;
use palette::PaletteView;
use shortcuts::{Action, Ctx};
use cart_model::levels::{COLS, LEVELS, ROWS};
use cart_model::map_routes::Place;
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
    blocks: Brush,
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
    /// The object type chosen in the palette to place (None: clicks only pick objects).
    kind: Option<u8>,
    sel_obj: Option<usize>,
    drag_obj: Option<(usize, Vec2)>,
    in_world: bool,
    tiles: Brush,
    /// A palette selection being dragged: where it started (column, row).
    palette_drag: Option<(i32, i32)>,
    /// A right-button drag on the grid picking a brush: where it started.
    pick_from: Option<(i32, i32)>,
    /// Copied cells and the level or map they came from (pasted only there).
    clipboard: Option<(Grid, Stamp)>,
    /// The tool to go back to after pasting.
    prev_tool: ToolKind,
    prefs: Prefs,
    prefs_store: Box<dyn PrefsStore>,
    level_palette: PaletteView,
    map_palette: PaletteView,
    /// The side panel's width: as wide as its palette (see `fitted_panel`).
    panel: f32,
    pub(crate) world_tab: usize,
    /// Which map picture the world map editor shows: the world map or a zone map.
    pub(crate) world_map: OnMap,
    point: usize,
    world_cam: Vec2,
    level_cam: Vec2,
    /// A path being drawn from the chosen point: its corners so far (pixels of the picture).
    route_draw: Option<Vec<(i32, i32)>>,
    /// The next click on the world map puts a new point there.
    placing_point: bool,
    /// A new zone's point being dragged with the mouse.
    drag_point: Option<u8>,
    /// Where a drag of a world map or game zone point began: the point's place, the mouse.
    drag_from: Option<(Place, Vec2)>,
    dialogs: Box<dyn FileDialogs>,
    text_files: Box<dyn TextFiles>,
    /// A file action waiting for the keys and buttons to be let go (see `run_pending_file`).
    pending_file: Option<Action>,
    /// Mario's picture at a level's start (None: not captured yet; Some(None): no picture).
    mario: Option<Option<(Texture2D, f32, f32)>>,
    /// What a click on the level places, when not painting or moving objects.
    placing: Option<start_view::Placing>,
    /// Mario's start being dragged.
    drag_start: bool,
    /// The finish or secret exit being dragged.
    drag_goal: Option<cart_model::level_goals::Goal>,
    /// A level pixel to bring into view on the next frame (true: even when it is in view).
    look: Option<(Vec2, bool)>,
    /// A new level's or zone's name being typed.
    naming: Option<names_view::Naming>,
    /// Where a door or exit leads, being chosen on the maps.
    picker: Option<target_picker::Picker>,
    menu: Box<dyn MenuBar>,
    /// Each File menu entry's state as last set (None: not set yet).
    menu_enabled: [Option<bool>; menu::FILE_MENU_LEN],
    /// A warning's place marked on the level until a time.
    mark: Option<(Vec2, f64)>,
    /// The level picker's grid is unfolded.
    levels_open: bool,
    side_tab: level_panel::SideTab,
    /// The level inspector scrolled by, and how tall it was last drawn.
    side_scroll: f32,
    side_content: f32,
    /// The New blank level chooser is open on this game level.
    blank_choice: Option<usize>,
    /// Per base level, the object types it draws right (`offered_types`).
    offered: HashMap<usize, Vec<(u8, Vec<usize>)>>,
    /// Each captured (level, type) picture that draws something.
    captured: HashMap<(usize, u8), cart_model::art::Look>,
    /// The object palette shows every type, not only those the level draws right.
    show_all_objects: bool,
    /// The world map editor's Points panel: how far it is scrolled, and its height.
    points_scroll: f32,
    points_content: f32,
    /// The tooltip of the button under the mouse this frame.
    tip: Option<String>,
    /// The left button went down for a click that is done (play here, a checkpoint): the
    /// level takes nothing more from it until it is let go.
    hold_click: bool,
    music: MusicPlayer,
    sound: Box<dyn SoundOut>,
    look_loader: LookLoader,
    look_store: Box<dyn LookStore>,
    /// Every object picture known, as the cache keeps them, and how many it has.
    look_data: Vec<look_cache::Entry>,
    looks_saved: usize,
}

/// What the editor works through, besides the document.
pub struct Ports {
    pub store: Box<dyn EditedCopyStore>,
    pub launcher: Box<dyn GameLauncher>,
    pub prefs: Box<dyn PrefsStore>,
    pub dialogs: Box<dyn FileDialogs>,
    pub text_files: Box<dyn TextFiles>,
    pub menu: Box<dyn MenuBar>,
    pub sound: Box<dyn SoundOut>,
    pub looks: Box<dyn LookStore>,
}

/// Painting into the level or map picture shown.
struct DocCanvas<'a> {
    doc: &'a mut EditorDocument,
    grid: Grid,
    changed: &'a mut bool,
}

impl GridCanvas for DocCanvas<'_> {
    fn begin_edit(&mut self) {
        self.doc.grid_snapshot(self.grid);
    }
    fn paint(&mut self, x: i32, y: i32, v: u8) {
        if self.doc.set_cell(self.grid, x, y, v) {
            *self.changed = true;
        }
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
    pub fn new(doc: EditorDocument, ports: Ports, had_copy: bool, es: bool) -> Editor {
        let Ports { store, launcher, prefs: prefs_store, dialogs, text_files, menu, sound, looks: look_store } = ports;
        let cached = look_store.load().map(|t| look_cache::from_text(&t)).unwrap_or_default();
        let known = cached.iter().map(|e| (e.0, e.1)).collect();
        let look_loader = LookLoader::start(doc.rom.clone(), doc.original.clone(), &known);
        let status = if !had_copy {
            tr(es, "Original cartridge opened. Saving makes an edited copy; the original is never changed.", "Cartucho original abierto. Al guardar se crea una copia editada; el original no se toca.").to_string()
        } else {
            format!("{} {}", tr(es, "Edited copy opened:", "Copia editada abierta:"), store.path().display())
        };
        let mut ed = Editor {
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
            blocks: Brush::new(),
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
            kind: None,
            sel_obj: None,
            drag_obj: None,
            in_world: false,
            tiles: Brush::new(),
            palette_drag: None,
            pick_from: None,
            clipboard: None,
            prev_tool: ToolKind::Pencil,
            prefs: prefs_store.load(),
            prefs_store,
            level_palette: PaletteView::default(),
            map_palette: PaletteView::default(),
            panel: PANEL_MIN,
            world_tab: TAB_PICTURE,
            world_map: OnMap::World,
            point: 0,
            world_cam: Vec2::ZERO,
            route_draw: None,
            placing_point: false,
            drag_point: None,
            drag_from: None,
            level_cam: Vec2::ZERO,
            dialogs,
            text_files,
            pending_file: None,
            mario: None,
            placing: None,
            drag_start: false,
            drag_goal: None,
            look: None,
            naming: None,
            picker: None,
            menu,
            menu_enabled: [None; menu::FILE_MENU_LEN],
            mark: None,
            levels_open: false,
            side_tab: level_panel::SideTab::Palette,
            side_scroll: 0.0,
            side_content: 0.0,
            blank_choice: None,
            offered: HashMap::new(),
            captured: HashMap::new(),
            show_all_objects: false,
            points_scroll: 0.0,
            points_content: 0.0,
            tip: None,
            hold_click: false,
            music: MusicPlayer::default(),
            sound,
            look_loader,
            look_store,
            look_data: Vec::new(),
            looks_saved: 0,
        };
        for (l, k, look) in cached {
            ed.add_look(l, k, look);
        }
        ed.looks_saved = ed.look_data.len();
        ed.build_menu();
        ed
    }

    /// One frame: the menu, shortcuts, input, then drawing.
    pub fn frame(&mut self) {
        self.update_menu();
        if self.blank_choice.is_some() && !self.in_world {
            self.blank_frame();
            return;
        }
        if !self.update_naming() {
            for a in shortcuts::pressed(self.ctx()) {
                self.run(a);
            }
        }
        self.run_pending_file();
        self.panel = self.fitted_panel();
        if self.in_world {
            self.update_world();
            self.draw_world();
        } else {
            self.update_level();
            self.draw_level();
        }
        if self.in_world {
            self.music.stop();
        }
        self.music.pump(&*self.sound, self.prefs.music_volume as f32 / *crate::domain::prefs::MUSIC_VOLUMES.end() as f32);
    }

    fn music_playing(&self) -> bool {
        self.music.playing()
    }

    /// Listen to `music` with the game's own sound, or stop.
    fn toggle_music(&mut self, music: u8) {
        if self.music.playing() {
            self.music.stop();
            return;
        }
        if let Err(e) = self.music.play(&self.doc.rom, music, self.sound.sample_rate()) {
            self.status = format!("{} {e}", tr(self.es, "Could not play the music:", "No se pudo tocar la musica:"));
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
        (m - vec2(self.panel, 0.0)) / self.scale() + self.cam
    }

    fn ctx(&self) -> Ctx {
        match (self.in_world, self.world_tab == TAB_POINTS, self.mode) {
            (true, true, _) => Ctx::Points,
            (true, false, _) => Ctx::Picture,
            (false, _, Mode::Objects) => Ctx::Objects,
            (false, _, Mode::Blocks) => Ctx::Blocks,
        }
    }

    /// Do what a shortcut or a panel button asks.
    fn run(&mut self, a: Action) {
        match a {
            Action::Undo if self.in_world => self.world_undo(),
            Action::Undo => self.undo(),
            Action::Redo if self.in_world => self.world_redo(),
            Action::Redo => self.redo(),
            Action::Save => {
                self.save();
            }
            Action::Play if self.in_world => self.play_map(),
            Action::Play => self.play(),
            Action::Grid => self.grid = !self.grid,
            Action::Pencil => self.set_tool(ToolKind::Pencil),
            Action::BoxFill => self.set_tool(ToolKind::Box),
            Action::Select => self.set_tool(ToolKind::Select),
            Action::Overview => self.show_overview = !self.show_overview,
            Action::BlocksObjects => self.set_mode(if self.mode == Mode::Blocks { Mode::Objects } else { Mode::Blocks }),
            Action::PrevLevel => self.select_level((self.cur + self.level_count() - 1) % self.level_count()),
            Action::NextLevel => self.select_level((self.cur + 1) % self.level_count()),
            Action::NewLevel => self.new_level(),
            Action::NewBlankLevel if !self.in_world => self.open_blank_chooser(),
            Action::DeleteLevel => self.delete_level(),
            Action::WorldEditor => self.enter_world(!self.in_world),
            Action::PrevMap => self.step_map(-1),
            Action::NextMap => self.step_map(1),
            Action::SwitchTab => self.world_tab = if self.world_tab == TAB_PICTURE { TAB_POINTS } else { TAB_PICTURE },
            Action::Restore if self.in_world => self.restore_map(),
            Action::Restore => self.restore_original(),
            Action::ZoomOut => self.palette_zoom(-1),
            Action::ZoomIn => self.palette_zoom(1),
            Action::Copy => self.copy_selection(false),
            Action::Cut => self.copy_selection(true),
            Action::Paste => self.start_paste(),
            Action::ClearSelection => self.clear_selection_cells(),
            Action::Escape if self.placing.is_some() => self.placing = None,
            Action::Escape => self.escape_grid(),
            Action::PlaceStart => self.toggle_placing(start_view::Placing::Start),
            Action::PlaceFinish if !self.doc.ends_with_boss(self.cur) => self.toggle_placing(start_view::Placing::Finish),
            Action::PlaceSecret => self.toggle_placing(start_view::Placing::Secret),
            Action::PlaceCheckpoint => match self.doc.checkpoint_refused(self.cur) {
                Some(crate::domain::document::CheckpointRefused::NoRoom) => self.status = tr(self.es, "The game keeps no restart place for this level: it cannot have checkpoints.", "El juego no guarda punto de reaparicion para este nivel: no puede tener puntos de control.").into(),
                _ => self.toggle_placing(start_view::Placing::Checkpoint),
            },
            Action::PlayFromHere => self.toggle_placing(start_view::Placing::PlayHere),
            Action::SecretLeads => self.start_secret_picking(),
            Action::DeleteObject => self.delete_selected_object(),
            a if a.uses_files() => self.queue_file(a),
            _ => self.run_points(a),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.sel_obj = None;
        self.leave_grid();
    }

    fn palette_zoom_now(&self) -> u32 {
        if self.in_world {
            self.prefs.map_palette_zoom
        } else {
            self.prefs.level_palette_zoom
        }
    }

    /// One step bigger or smaller for the palette shown.
    fn palette_zoom(&mut self, d: i32) {
        let z = (self.palette_zoom_now() as i32 + d).clamp(*PALETTE_ZOOMS.start() as i32, *PALETTE_ZOOMS.end() as i32) as u32;
        if z == self.palette_zoom_now() {
            return;
        }
        if self.in_world {
            self.prefs.map_palette_zoom = z;
        } else {
            self.prefs.level_palette_zoom = z;
        }
        self.prefs_store.save(&self.prefs);
    }

    /// Pan with arrows / WASD, middle mouse or Space + left mouse; zoom at the mouse with
    /// the wheel; keep the camera inside a picture of `size` pixels. Returns whether Space
    /// is held, the visible size and the furthest camera position.
    fn navigate(&mut self, m: Vec2, ctrl: bool, size: Vec2) -> (bool, Vec2, Vec2) {
        let speed = 600.0 * get_frame_time() / self.scale();
        let mut d = Vec2::ZERO;
        // keys typed into a name do not move the view
        let down = |k: KeyCode| self.naming.is_none() && is_key_down(k);
        // Shift + arrows and Ctrl + arrows are shortcuts
        let arrows = !(ctrl || is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift));
        if (arrows && down(KeyCode::Left)) || (!ctrl && down(KeyCode::A)) {
            d.x -= speed;
        }
        if (arrows && down(KeyCode::Right)) || (!ctrl && down(KeyCode::D)) {
            d.x += speed;
        }
        if (arrows && down(KeyCode::Up)) || (!ctrl && down(KeyCode::W)) {
            d.y -= speed;
        }
        if (arrows && down(KeyCode::Down)) || (!ctrl && down(KeyCode::S)) {
            d.y += speed;
        }
        let space = down(KeyCode::Space);
        self.cam += d;
        if m.x >= self.panel && (is_mouse_button_pressed(MouseButton::Middle) || (space && is_mouse_button_pressed(MouseButton::Left))) {
            self.pan = Some((m, self.cam));
        }
        if let Some((m0, c0)) = self.pan {
            if is_mouse_button_down(MouseButton::Middle) || (space && is_mouse_button_down(MouseButton::Left)) {
                self.cam = c0 - (m - m0) / self.scale();
            } else {
                self.pan = None;
            }
        }
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && m.x >= self.panel {
            let before = self.world_pos(m);
            if wheel > 0.0 && self.zoom + 1 < ZOOMS.len() {
                self.zoom += 1;
            } else if wheel < 0.0 && self.zoom > 0 {
                self.zoom -= 1;
            }
            self.cam = before - (m - vec2(self.panel, 0.0)) / self.scale();
        }
        let view = vec2(screen_width() - self.panel, screen_height() - STATUS_H) / self.scale();
        let max = (size - view).max(Vec2::ZERO);
        // room over the map for the tabs while choosing a target
        let min = vec2(0.0, -self.picker_tabs_h() / self.scale());
        self.cam = self.cam.clamp(min, max);
        (space, view, max)
    }

    /// Let the game load the level (from the world map) and keep its graphics (a new
    /// level's are its base level's).
    fn ensure_art(&mut self, l: usize) {
        let l = self.doc.base_of(l);
        match self.art.ensure_level(&self.doc.rom, l) {
            Capture::Ready => return,
            Capture::Failed(e) => {
                self.status = e;
                return;
            }
            Capture::NotLoaded => {
                self.status = tr(self.es, "The game didn't load this level; its graphics may be wrong.", "El juego no cargo este nivel; sus graficos pueden no ser correctos.").into();
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
            OnMap::NewZone(k) => {
                let z = self.doc.new_zones[k].base;
                self.zone_atlases[z] = self.art.zone(z).map(make_world_atlas);
            }
        }
    }

    /// The tiles of the map picture shown.
    fn map_atlas(&self) -> Option<&Texture2D> {
        match self.world_map {
            OnMap::World => self.world_atlas.as_ref(),
            OnMap::Zone(z) => self.zone_atlases[z].as_ref(),
            OnMap::NewZone(k) => self.zone_atlases[self.doc.new_zones.get(k)?.base].as_ref(),
        }
    }

    /// Show another map picture (the world map or a zone map).
    pub fn show_map(&mut self, map: OnMap) {
        if map == self.world_map {
            return;
        }
        self.world_map = map;
        self.doc.show_added(map);
        self.cam = Vec2::ZERO;
        self.leave_grid();
        self.ensure_world_art();
    }

    /// Bring the view to the level pixel asked for (in `look`), a little left of and below its
    /// middle; unless it is already in view, when asked so.
    fn look_now(&mut self) {
        let Some((p, always)) = self.look.take() else { return };
        let view = vec2(screen_width() - self.panel, screen_height() - STATUS_H) / self.scale();
        if !always && Rect::new(self.cam.x, self.cam.y, view.x, view.y).contains(p) {
            return;
        }
        let at = vec2(p.x - 2.0 * BLOCK, p.y - view.y * 0.6);
        self.cam = at.clamp(Vec2::ZERO, (LEVEL_SIZE - view).max(Vec2::ZERO)).floor();
    }

    pub fn select_level(&mut self, l: usize) {
        if l != self.cur {
            self.music.stop();
        }
        self.cur = l;
        self.leave_grid();
        self.ensure_art(l);
        // Start the view where Mario begins the level.
        let s = self.doc.level_start(l);
        self.look = Some((vec2(s.mario_x as f32, s.mario_y as f32), true));
        self.overview_stale = true;
        let shared = self.doc.sharing(l);
        self.status = if shared.is_empty() {
            format!("{} {:02} {}", tr(self.es, "Level", "Nivel"), l, self.doc.level_name(l))
        } else {
            format!("{} {:02}: {} {:?}", tr(self.es, "Level", "Nivel"), l, tr(self.es, "its map is also used by level(s)", "su mapa tambien lo usa(n) el/los nivel(es)"), shared)
        };
    }

    pub fn enter_world(&mut self, on: bool) {
        if on == self.in_world {
            return;
        }
        self.leave_grid();
        if on {
            self.level_cam = self.cam;
            self.ensure_world_art();
            self.cam = self.world_cam;
            self.status = tr(self.es, "World map: paint its tiles, or choose which level each point opens.", "Mapa de mundos: pinta sus tiles o elige que nivel abre cada punto.").into();
        } else {
            self.world_cam = self.cam;
            self.cam = self.level_cam;
        }
        self.in_world = on;
    }

    pub fn level_count(&self) -> usize {
        self.doc.levels.len()
    }

    /// A new level, a copy of the one shown, and show it.
    fn new_level(&mut self) {
        match self.doc.add_level(self.cur) {
            Some(n) => {
                let base = self.doc.base_of(n);
                self.select_level(n);
                self.status = format!(
                    "{} {n:02}: {} {base:02} ({}). {}",
                    tr(self.es, "New level", "Nivel nuevo"),
                    tr(self.es, "plays as level", "se juega como el nivel"),
                    tr(self.es, "its graphics, music and time", "sus graficos, musica y tiempo"),
                    tr(self.es, "Give it to a map point to reach it in the game.", "Asignalo a un punto del mapa para llegar a el en el juego.")
                );
            }
            None => self.status = tr(self.es, "There can't be more levels.", "No caben mas niveles.").into(),
        }
    }

    /// Delete the new level shown (the 32 of the game stay).
    fn delete_level(&mut self) {
        let l = self.cur;
        if l < LEVELS {
            return;
        }
        if self.doc.remove_level(l) {
            self.select_level(l.min(self.doc.levels.len() - 1));
            self.status = format!("{} {l:02} {}", tr(self.es, "Level", "Nivel"), tr(self.es, "deleted (save to keep it); the ones after it moved down a number.", "borrado (guarda para conservarlo); los siguientes bajan un numero."));
        }
    }

    fn undo(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.doc.undo(self.cur) {
            self.level_changed(base);
        }
    }

    fn redo(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.doc.redo(self.cur) {
            self.level_changed(base);
        }
    }

    /// After the shown level changed under the view; `base` is what it played as before (a
    /// replaced new level can play as another and look different).
    fn level_changed(&mut self, base: usize) {
        self.sel_obj = None;
        self.overview_stale = true;
        if self.doc.base_of(self.cur) != base {
            let l = self.cur;
            self.looks.retain(|k, _| k.0 != l);
            self.ensure_art(l);
        }
    }

    fn restore_original(&mut self) {
        self.doc.restore_level(self.cur);
        self.sel_obj = None;
        self.overview_stale = true;
        let l = self.cur;
        self.status = if l >= LEVELS {
            format!("{} {l:02} {} {:02} {}", tr(self.es, "Level", "Nivel"), tr(self.es, "restored as its base level", "restaurado como su nivel base"), self.doc.base_of(l), tr(self.es, "is in the game (save to keep it; undo brings it back).", "esta en el juego (guarda para conservarlo; deshacer lo recupera)."))
        } else {
            format!("{} {l:02} {}", tr(self.es, "Level", "Nivel"), tr(self.es, "restored as it is in the game (save to keep it; undo brings it back).", "restaurado como esta en el juego (guarda para conservarlo; deshacer lo recupera)."))
        };
    }

    /// Save the edited copy; the status bar says what happened.
    fn save(&mut self) -> bool {
        self.music.stop();
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
                        Saved::Doors => tr(es, "doors", "puertas").to_string(),
                        Saved::Routes(n) => format!("{} ({n} bytes)", tr(es, "map paths", "caminos del mapa")),
                        Saved::NewLevels(n, used) => format!("{n} {} {used} bytes", tr(es, "new levels", "niveles nuevos")),
                        Saved::NewZones(n, used) => format!("{n} {} {used} bytes", tr(es, "new zones", "zonas nuevas")),
                        Saved::Starts(n) => format!("{n} {}", tr(es, "level starts and settings", "inicios y ajustes de nivel")),
                        Saved::SecretExits(n) => format!("{n} {}", tr(es, "secret exits", "salidas secretas")),
                    })
                    .collect();
                self.status = format!(
                    "{} {} {}",
                    tr(es, "Saved", "Guardado en"),
                    self.store.path().display(),
                    if notes.is_empty() { String::new() } else { format!("({})", notes.join(", ")) }
                );
                true
            }
            Err(SaveError::Layout(e)) => {
                self.status = format!("{}: {e}", tr(es, "Not saved", "No guardado"));
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
        self.play_level(None);
    }

    /// Play from here: Mario standing on the block at level pixel `w`, this time only.
    /// Refused where his body would be in solid blocks.
    fn play_from_here(&mut self, w: Vec2) {
        let start = Self::start_at(w);
        if start.blocked(&self.doc.levels[self.cur].content.map) {
            self.status = tr(self.es, "Mario cannot stand there (solid blocks or outside the level).", "Mario no cabe ahi (bloques solidos o fuera del nivel).").into();
            return;
        }
        self.placing = None;
        self.play_level(Some((start.mario_x, start.mario_y + cart_model::level_start::FEET)));
    }

    fn play_level(&mut self, start: Option<(u16, u16)>) {
        if !self.save() {
            return;
        }
        self.status = match self.launcher.launch(self.store.path(), Some(self.cur), start) {
            Ok(()) if self.cur >= LEVELS => format!("{} {:02} ({} {:02})", tr(self.es, "Playing level", "Jugando el nivel"), self.cur, tr(self.es, "as", "como"), self.doc.base_of(self.cur)),
            Ok(()) => format!("{} {:02}", tr(self.es, "Playing level", "Jugando el nivel"), self.cur),
            Err(e) => format!("{} {e}", tr(self.es, "Could not start", "No se pudo abrir")),
        };
    }

    /// Save, then start the game on the edited copy (its world map, to try the points).
    fn play_map(&mut self) {
        if !self.save() {
            return;
        }
        self.status = match self.launcher.launch(self.store.path(), None, None) {
            Ok(()) => tr(self.es, "Game started with the edited copy.", "Juego abierto con la copia editada.").into(),
            Err(e) => e,
        };
    }

    /// For the check pictures: capture this level's object pictures first.
    pub fn capture_level_looks(&mut self) {
        self.wait_for_level_looks();
        self.pump_mario();
    }

    /// The object palette with every type (for the command line).
    pub fn show_all_objects(&mut self) {
        self.show_all_objects = true;
    }

    /// Show the level inspector tab, and unfold the level grid (for the command line).
    pub fn show_inspector(&mut self, levels_open: bool) {
        self.side_tab = level_panel::SideTab::Level;
        self.levels_open = levels_open;
    }

    /// Choose "Place start" (for the command line).
    pub fn place_start(&mut self) {
        self.toggle_placing(start_view::Placing::Start);
    }

    /// Choose "Place finish" (for the command line).
    pub fn place_finish(&mut self) {
        self.toggle_placing(start_view::Placing::Finish);
    }

    /// Choose where the level's secret exit leads (for the command line).
    pub fn choose_secret(&mut self) {
        self.start_secret_picking();
    }

}

// The level's size in pixels.
const LEVEL_SIZE: Vec2 = Vec2::new(COLS as f32 * BLOCK, ROWS as f32 * BLOCK);
