//! Level editor: opens the person's cartridge (or their edited copy), shows each of the
//! 32 levels with the game's own blocks and graphics, lets them paint blocks and saves
//! the result into an edited copy of the cartridge. The original file is never written.
use cart_model::custom_map::{CustomMap, Point};
use cart_model::levels::{self, LevelArt, COLS, LEVELS, ROWS};
use cart_model::objects::{self, Obj};
use cart_model::world::{self, WorldArt};
use cart_model::GameDriver;
use gb_core::Gb;
use host::find_rom;
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

const PAL: [u32; 4] = [0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1F];
const PANEL: f32 = 300.0;
const STATUS_H: f32 = 26.0;
const BLOCK: f32 = 16.0;
const ZOOMS: [f32; 7] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

fn col(i: usize) -> Color {
    let c = PAL[i];
    Color::from_rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255)
}

#[derive(PartialEq, Clone, Copy)]
enum Tool {
    Pencil,
    Rect,
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Blocks,
    Objects,
}

type Snap = (Vec<u8>, Vec<Obj>);

struct Level {
    map: Vec<u8>,
    objs: Vec<Obj>,
    art: Option<LevelArt>,
    atlas: Option<Texture2D>,
    dirty: bool,
    objs_dirty: bool,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
}

struct Editor {
    es: bool,
    original: Vec<u8>,
    rom: Vec<u8>,
    edit_path: PathBuf,
    boot: Option<Gb>,
    levels: Vec<Level>,
    cur: usize,
    block: u8,
    cam: Vec2,
    zoom: usize,
    tool: Tool,
    rect_start: Option<(i32, i32)>,
    grid: bool,
    status: String,
    pan: Option<(Vec2, Vec2)>,
    space_cache: (usize, usize),
    space_at: f64,
    overview: Option<Texture2D>,
    overview_stale: bool,
    show_overview: bool,
    mode: Mode,
    kind: u8,
    sel_obj: Option<usize>,
    drag_obj: Option<(usize, Vec2)>,
    obj_space: (usize, usize),
    // world map
    in_world: bool,
    world: Vec<u8>,
    world_dirty: bool,
    world_undo: Vec<Vec<u8>>,
    world_redo: Vec<Vec<u8>>,
    points: Vec<u8>,
    points_dirty: bool,
    world_atlas: Option<Texture2D>,
    tile: u8,
    world_tab: usize,
    point: usize,
    world_cam: Vec2,
    level_cam: Vec2,
    /// What each object type looks like in each level, captured from the game: texture and
    /// where its top-left sits relative to the object's position. None = draws nothing.
    looks: std::collections::HashMap<(usize, u8), Option<(Texture2D, f32, f32)>>,
    // the port's own world map (custom_map.rs)
    custom: CustomMap,
    custom_dirty: bool,
    sel_point: Option<usize>,
    link_from: Option<usize>,
    drag_point: bool,
}

fn look_texture(l: &objects::Look) -> Texture2D {
    let mut img = Image::gen_image_color(l.w as u16, l.h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..l.h {
        for x in 0..l.w {
            if let Some(s) = l.px[y * l.w + x] {
                img.set_pixel(x as u32, y as u32, col(s as usize));
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

/// One pixel per block: the block's average shade, for the whole-level overview.
fn make_overview(art: &LevelArt, map: &[u8]) -> Texture2D {
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
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

fn tr(es: bool, en: &'static str, es_text: &'static str) -> &'static str {
    if es {
        es_text
    } else {
        en
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// Language from the game's settings file (English unless Spanish was chosen).
fn spanish() -> bool {
    std::fs::read_to_string(exe_dir().join("settings.cfg"))
        .map(|t| t.lines().any(|l| l.trim() == "language=es"))
        .unwrap_or(false)
}

fn make_atlas(art: &LevelArt) -> Texture2D {
    let mut img = Image::gen_image_color(256, 128, BLACK);
    for id in 0..128u8 {
        for y in 0..16 {
            for x in 0..16 {
                let s = art.block_pixel(id, x, y) as usize;
                img.set_pixel((id as u32 % 16) * 16 + x as u32, (id as u32 / 16) * 16 + y as u32, col(s));
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

impl Editor {
    fn new(original_path: &Path) -> Result<Editor, String> {
        let original = std::fs::read(original_path).map_err(|e| e.to_string())?;
        let stem = original_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into());
        let edit_path = exe_dir().join("edits").join(format!("{stem} (edit).gb"));
        // Work on the edited copy if there is one; otherwise start a new copy, made 1 MB so
        // every zone has room for bigger levels (levels::expand).
        let mut rom = std::fs::read(&edit_path)
            .ok()
            .filter(|r| r.len() == original.len() || levels::is_expanded(r))
            .unwrap_or_else(|| levels::expand(&original).unwrap_or_else(|_| original.clone()));
        levels::upgrade(&mut rom); // copies made before the object patch get it now
        let levels = (0..LEVELS)
            .map(|l| {
                let (_, start) = levels::map_location(&rom, levels::header(&rom, l));
                Level {
                    map: levels::decode(&rom, start).0,
                    objs: objects::read(&rom, l).0,
                    art: None,
                    atlas: None,
                    dirty: false,
                    objs_dirty: false,
                    undo: vec![],
                    redo: vec![],
                }
            })
            .collect();
        let es = spanish();
        let rom_for_world = rom.clone();
        let edit_path_for_map = edit_path.clone();
        let status = if !std::path::Path::new(&edit_path).exists() {
            tr(es, "Original cartridge opened. Saving makes an edited copy; the original is never changed.", "Cartucho original abierto. Al guardar se crea una copia editada; el original no se toca.").to_string()
        } else {
            format!("{} {}", tr(es, "Edited copy opened:", "Copia editada abierta:"), edit_path.display())
        };
        Ok(Editor {
            es,
            original,
            rom,
            edit_path,
            boot: None,
            levels,
            cur: 0,
            block: 0,
            cam: Vec2::ZERO,
            zoom: 3,
            tool: Tool::Pencil,
            rect_start: None,
            grid: true,
            status,
            pan: None,
            space_cache: (0, 1),
            space_at: -1.0,
            overview: None,
            overview_stale: true,
            show_overview: true,
            mode: Mode::Blocks,
            kind: 0,
            sel_obj: None,
            drag_obj: None,
            obj_space: (0, 1),
            in_world: false,
            world: world::read(&rom_for_world),
            world_dirty: false,
            world_undo: vec![],
            world_redo: vec![],
            points: world::point_levels(&rom_for_world),
            points_dirty: false,
            world_atlas: None,
            tile: 0,
            world_tab: 0,
            point: 0,
            world_cam: Vec2::ZERO,
            level_cam: Vec2::ZERO,
            looks: Default::default(),
            custom: CustomMap::load(&edit_path_for_map).unwrap_or_default(),
            custom_dirty: false,
            sel_point: None,
            link_from: None,
            drag_point: false,
        })
    }

    fn scale(&self) -> f32 {
        ZOOMS[self.zoom]
    }

    /// The world map's tiles, as the game shows them (captured on the map screen).
    fn ensure_world_art(&mut self) {
        if self.world_atlas.is_some() {
            return;
        }
        if self.boot.is_none() {
            self.ensure_art(self.cur);
        }
        let Some(g) = &self.boot else { return };
        let art = WorldArt::capture(g);
        let mut img = Image::gen_image_color(128, 128, BLACK);
        for t in 0..=255u8 {
            for y in 0..8 {
                for x in 0..8 {
                    img.set_pixel((t as u32 % 16) * 8 + x as u32, (t as u32 / 16) * 8 + y as u32, col(art.pixel(t, x, y) as usize));
                }
            }
        }
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        self.world_atlas = Some(tex);
    }

    fn enter_world(&mut self, on: bool) {
        if on == self.in_world {
            return;
        }
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

    fn world_snapshot(&mut self) {
        self.world_undo.push(self.world.clone());
        if self.world_undo.len() > 100 {
            self.world_undo.remove(0);
        }
        self.world_redo.clear();
    }

    /// Tile under the mouse on the world map.
    fn world_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        if m.x < PANEL || m.y > screen_height() - STATUS_H {
            return None;
        }
        let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
        let (x, y) = ((w.x / 8.0).floor() as i32, (w.y / 8.0).floor() as i32);
        if x >= 0 && y >= 0 && (x as usize) < world::W && (y as usize) < world::H {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    fn update_world(&mut self) {
        let m: Vec2 = mouse_position().into();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::Z) {
            if let Some(w) = self.world_undo.pop() {
                self.world_redo.push(std::mem::replace(&mut self.world, w));
                self.world_dirty = true;
            }
        }
        if ctrl && is_key_pressed(KeyCode::Y) {
            if let Some(w) = self.world_redo.pop() {
                self.world_undo.push(std::mem::replace(&mut self.world, w));
                self.world_dirty = true;
            }
        }
        if ctrl && is_key_pressed(KeyCode::S) {
            self.save();
        }
        if !ctrl {
            if is_key_pressed(KeyCode::G) {
                self.grid = !self.grid;
            }
            if is_key_pressed(KeyCode::B) {
                self.tool = Tool::Pencil;
            }
            if is_key_pressed(KeyCode::R) {
                self.tool = Tool::Rect;
            }
        }
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
        if m.x >= PANEL && (is_mouse_button_pressed(MouseButton::Middle) || (space && is_mouse_button_pressed(MouseButton::Left))) {
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
        if wheel != 0.0 && m.x >= PANEL {
            let before = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
            if wheel > 0.0 && self.zoom + 1 < ZOOMS.len() {
                self.zoom += 1;
            } else if wheel < 0.0 && self.zoom > 0 {
                self.zoom -= 1;
            }
            self.cam = before - (m - vec2(PANEL, 0.0)) / self.scale();
        }
        let view = vec2(screen_width() - PANEL, screen_height() - STATUS_H) / self.scale();
        let max = vec2(world::W as f32 * 8.0 - view.x, world::H as f32 * 8.0 - view.y).max(Vec2::ZERO);
        self.cam = self.cam.clamp(Vec2::ZERO, max);
        if self.pan.is_some() || space {
            return;
        }
        if self.world_tab == 2 {
            self.update_custom(m);
            return;
        }
        let cell = self.world_cell(m);
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some((x, y)) = cell {
                self.tile = self.world[y * world::W + x];
            }
        }
        let put = |ed: &mut Editor, x: usize, y: usize| {
            let i = y * world::W + x;
            if ed.world[i] != ed.tile {
                ed.world[i] = ed.tile;
                ed.world_dirty = true;
            }
        };
        match self.tool {
            Tool::Pencil => {
                if is_mouse_button_pressed(MouseButton::Left) && cell.is_some() {
                    self.world_snapshot();
                }
                if is_mouse_button_down(MouseButton::Left) {
                    if let Some((x, y)) = cell {
                        put(self, x, y);
                    }
                }
            }
            Tool::Rect => {
                if is_mouse_button_pressed(MouseButton::Left) && cell.is_some() {
                    self.rect_start = cell.map(|(x, y)| (x as i32, y as i32));
                }
                if is_mouse_button_released(MouseButton::Left) {
                    if let (Some((x0, y0)), Some((x1, y1))) = (self.rect_start.take(), cell) {
                        self.world_snapshot();
                        let (x1, y1) = (x1 as i32, y1 as i32);
                        for y in y0.min(y1)..=y0.max(y1) {
                            for x in x0.min(x1)..=x0.max(x1) {
                                put(self, x as usize, y as usize);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The custom map point under a screen position (within 7 world pixels).
    fn point_at(&self, m: Vec2) -> Option<usize> {
        let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
        self.custom
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| (i, (vec2(p.x as f32, p.y as f32) - w).length()))
            .filter(|(_, d)| *d <= 7.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| i)
    }

    fn update_custom(&mut self, m: Vec2) {
        if self.world_cell(m).is_none() {
            return;
        }
        let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
        let snap = |v: f32| ((v / 4.0).round() * 4.0).clamp(4.0, 508.0) as i32;
        if is_key_pressed(KeyCode::L) {
            self.link_from = self.sel_point;
        }
        if (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace)) && self.sel_point.is_some() {
            self.custom.remove_point(self.sel_point.unwrap());
            self.sel_point = None;
            self.custom_dirty = true;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            match (self.point_at(m), self.link_from) {
                (Some(b), Some(a)) => {
                    // joining mode: the second point toggles a path
                    self.custom.toggle_path(a, b);
                    self.link_from = None;
                    self.sel_point = Some(b);
                    self.custom_dirty = true;
                }
                (Some(i), None) => {
                    self.sel_point = Some(i);
                    self.drag_point = true;
                }
                (None, _) => {
                    // a new point, joined to the selected one so a route is drawn click by click
                    self.custom.points.push(Point { x: snap(w.x), y: snap(w.y), level: None });
                    let n = self.custom.points.len() - 1;
                    if let Some(s) = self.sel_point {
                        self.custom.toggle_path(s, n);
                    }
                    self.sel_point = Some(n);
                    self.link_from = None;
                    self.drag_point = true;
                    self.custom_dirty = true;
                }
            }
        }
        if self.drag_point {
            if is_mouse_button_down(MouseButton::Left) {
                if let Some(i) = self.sel_point {
                    let (x, y) = (snap(w.x), snap(w.y));
                    let p = &mut self.custom.points[i];
                    if (p.x, p.y) != (x, y) {
                        p.x = x;
                        p.y = y;
                        self.custom_dirty = true;
                    }
                }
            } else {
                self.drag_point = false;
            }
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some(i) = self.point_at(m) {
                self.custom.remove_point(i);
                self.sel_point = None;
                self.link_from = None;
                self.custom_dirty = true;
            } else {
                self.sel_point = None;
                self.link_from = None;
            }
        }
    }

    fn draw_custom(&self) {
        let to = |x: i32, y: i32| vec2(PANEL + (x as f32 - self.cam.x) * self.scale(), (y as f32 - self.cam.y) * self.scale());
        for &(a, b) in &self.custom.paths {
            let (pa, pb) = (self.custom.points[a], self.custom.points[b]);
            let (s, e) = (to(pa.x, pa.y), to(pb.x, pb.y));
            draw_line(s.x, s.y, e.x, e.y, 3.0 * self.scale().max(1.0), Color::new(0.12, 0.12, 0.12, 0.85));
        }
        if let (Some(a), true) = (self.link_from, true) {
            let p = self.custom.points[a];
            let s = to(p.x, p.y);
            let m: Vec2 = mouse_position().into();
            draw_line(s.x, s.y, m.x, m.y, 2.0, WHITE);
        }
        for (i, p) in self.custom.points.iter().enumerate() {
            let c = to(p.x, p.y);
            let r = 5.0 * self.scale();
            let sel = self.sel_point == Some(i);
            match p.level {
                Some(l) => {
                    draw_rectangle(c.x - r, c.y - r, r * 2.0, r * 2.0, col(3));
                    draw_rectangle(c.x - r + 2.0, c.y - r + 2.0, r * 2.0 - 4.0, r * 2.0 - 4.0, col(0));
                    let t = format!("{l:02}");
                    let fs = (r * 1.2).clamp(12.0, 22.0);
                    let d = measure_text(&t, None, fs as u16, 1.0);
                    draw_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, col(3));
                }
                None => draw_circle(c.x, c.y, r * 0.5, col(3)),
            }
            if i == self.custom.start {
                draw_text("S", c.x + r + 2.0, c.y - r, 18.0, WHITE);
            }
            if sel {
                draw_rectangle_lines(c.x - r - 3.0, c.y - r - 3.0, r * 2.0 + 6.0, r * 2.0 + 6.0, 2.0, WHITE);
            }
        }
    }

    fn draw_world(&mut self) {
        clear_background(col(3));
        let s = self.scale() * 8.0;
        let (cw, ch) = (screen_width() - PANEL, screen_height() - STATUS_H);
        if let Some(atlas) = &self.world_atlas {
            let x0 = (self.cam.x / 8.0).floor() as i32;
            let y0 = (self.cam.y / 8.0).floor() as i32;
            let nx = (cw / s).ceil() as i32 + 1;
            let ny = (ch / s).ceil() as i32 + 1;
            for y in y0..(y0 + ny).min(world::H as i32) {
                for x in x0..(x0 + nx).min(world::W as i32) {
                    let t = self.world[y as usize * world::W + x as usize];
                    let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                    let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                    draw_texture_ex(
                        atlas,
                        px.floor(),
                        py.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            if self.grid && s >= 12.0 {
                let gc = Color::new(0.0, 0.0, 0.0, 0.12);
                for x in x0..=x0 + nx {
                    let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                    draw_line(px, 0.0, px, ch, 1.0, gc);
                }
                for y in y0..=y0 + ny {
                    let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                    draw_line(PANEL, py, screen_width(), py, 1.0, gc);
                }
            }
            let m: Vec2 = mouse_position().into();
            if self.world_tab == 2 {
                self.draw_custom();
            } else if let Some((x, y)) = self.world_cell(m) {
                let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                draw_texture_ex(
                    atlas,
                    px,
                    py,
                    Color::new(1.0, 1.0, 1.0, 0.6),
                    DrawTextureParams {
                        source: Some(Rect::new((self.tile % 16) as f32 * 8.0, (self.tile / 16) as f32 * 8.0, 8.0, 8.0)),
                        dest_size: Some(vec2(s, s)),
                        ..Default::default()
                    },
                );
                draw_rectangle_lines(px, py, s, s, 2.0, WHITE);
                let t = self.world[y * world::W + x];
                draw_text(&format!("{x},{y}  #{t:02X}"), PANEL + 8.0, ch - 8.0, 20.0, WHITE);
            }
        }
        // --- panel
        draw_rectangle(0.0, 0.0, PANEL, screen_height(), col(0));
        draw_rectangle(PANEL - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "WORLD MAP EDITOR", "EDITOR DEL MAPA"), 10.0, 22.0, 24.0, col(3));
        let mut y = 34.0;
        let mut act = 0;
        if button(Rect::new(10.0, y, 276.0, 26.0), tr(self.es, "Back to the levels", "Volver a los niveles"), false, 16.0) {
            act = 9;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 88.0, 26.0), tr(self.es, "Picture", "Dibujo"), self.world_tab == 0, 15.0) {
            self.world_tab = 0;
        }
        if button(Rect::new(102.0, y, 88.0, 26.0), tr(self.es, "Orig. points", "Puntos orig."), self.world_tab == 1, 15.0) {
            self.world_tab = 1;
        }
        if button(Rect::new(194.0, y, 92.0, 26.0), tr(self.es, "Your map", "Mapa propio"), self.world_tab == 2, 15.0) {
            self.world_tab = 2;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 92.0, 26.0), tr(self.es, "Pencil (B)", "Lapiz (B)"), self.tool == Tool::Pencil, 16.0) {
            self.tool = Tool::Pencil;
        }
        if button(Rect::new(106.0, y, 92.0, 26.0), tr(self.es, "Box (R)", "Caja (R)"), self.tool == Tool::Rect, 16.0) {
            self.tool = Tool::Rect;
        }
        if button(Rect::new(202.0, y, 84.0, 26.0), tr(self.es, "Grid (G)", "Rejilla (G)"), self.grid, 16.0) {
            self.grid = !self.grid;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 92.0, 26.0), tr(self.es, "Undo", "Deshacer"), false, 16.0) {
            act = 1;
        }
        if button(Rect::new(106.0, y, 92.0, 26.0), tr(self.es, "Redo", "Rehacer"), false, 16.0) {
            act = 2;
        }
        if button(Rect::new(202.0, y, 84.0, 26.0), tr(self.es, "Original", "Original"), false, 16.0) {
            act = 3;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 276.0, 28.0), tr(self.es, "Save (Ctrl+S)", "Guardar (Ctrl+S)"), false, 17.0) {
            act = 4;
        }
        y += 38.0;
        let mp: Vec2 = mouse_position().into();
        if self.world_tab == 0 {
            draw_text(&format!("{} #{:02X}", tr(self.es, "TILES  selected", "TILES  elegido"), self.tile), 10.0, y + 12.0, 18.0, col(3));
            y += 18.0;
            if let Some(atlas) = &self.world_atlas {
                let cell = (17.0f32).min((screen_height() - y - 30.0) / 16.0);
                for t in 0..=255u8 {
                    let x = 10.0 + (t % 16) as f32 * cell;
                    let yy = y + (t / 16) as f32 * cell;
                    draw_texture_ex(atlas, x, yy, WHITE, DrawTextureParams {
                        source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
                        dest_size: Some(vec2(cell - 1.0, cell - 1.0)),
                        ..Default::default()
                    });
                    if t == self.tile {
                        draw_rectangle_lines(x - 1.0, yy - 1.0, cell + 1.0, cell + 1.0, 2.0, col(3));
                    }
                    if Rect::new(x, yy, cell - 1.0, cell - 1.0).contains(mp) && is_mouse_button_pressed(MouseButton::Left) {
                        self.tile = t;
                    }
                }
                draw_text(tr(self.es, "Right click on the map: pick a tile", "Clic derecho en el mapa: coger tile"), 10.0, (y + 16.0 * cell + 14.0).min(screen_height() - 4.0), 15.0, col(2));
            }
        } else if self.world_tab == 2 {
            // the port's own map: points anywhere, paths, a level per point
            draw_text(&format!("{} {}   {} {}", tr(self.es, "points:", "puntos:"), self.custom.points.len(), tr(self.es, "paths:", "caminos:"), self.custom.paths.len()), 10.0, y + 12.0, 17.0, col(3));
            y += 22.0;
            if let Some(i) = self.sel_point.filter(|&i| i < self.custom.points.len()) {
                let cur = self.custom.points[i].level;
                draw_text(&format!("{} {i}", tr(self.es, "Point", "Punto")), 10.0, y + 18.0, 18.0, col(3));
                y += 28.0;
                draw_text(tr(self.es, "Opens", "Abre"), 10.0, y + 18.0, 18.0, col(3));
                let mut change = 0i32;
                if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
                    change = -1;
                }
                let label = match cur {
                    Some(l) => format!("{} {:02}", tr(self.es, "level", "nivel"), l),
                    None => tr(self.es, "nothing (junction)", "nada (cruce)").to_string(),
                };
                draw_text(&label, 104.0, y + 18.0, 17.0, col(3));
                if button(Rect::new(260.0, y, 26.0, 26.0), "+", false, 18.0) {
                    change = 1;
                }
                if change != 0 {
                    let idx = cur.map(|l| l as i32).unwrap_or(32);
                    let n = (idx + change).rem_euclid(33);
                    self.custom.points[i].level = if n == 32 { None } else { Some(n as u8) };
                    self.custom_dirty = true;
                }
                y += 32.0;
                if button(Rect::new(10.0, y, 136.0, 26.0), tr(self.es, "Mario starts here", "Mario empieza aqui"), self.custom.start == i, 14.0) {
                    self.custom.start = i;
                    self.custom_dirty = true;
                }
                if button(Rect::new(150.0, y, 136.0, 26.0), tr(self.es, "Join to... (L)", "Unir con... (L)"), self.link_from == Some(i), 14.0) {
                    self.link_from = if self.link_from == Some(i) { None } else { Some(i) };
                }
                y += 30.0;
                if button(Rect::new(10.0, y, 136.0, 26.0), tr(self.es, "Delete point", "Borrar punto"), false, 14.0) {
                    self.custom.remove_point(i);
                    self.sel_point = None;
                    self.custom_dirty = true;
                }
                y += 34.0;
            } else {
                y += 4.0;
            }
            if button(Rect::new(10.0, y, 276.0, 28.0), tr(self.es, "Try it in the game", "Probarlo en el juego"), false, 16.0) {
                act = 6;
            }
            y += 40.0;
            let help = tr(
                self.es,
                "Click empty map: new point, joined to the selected one. Drag: move. Right click: delete. Join to... then click another point: add or remove a path. In the game, the d-pad walks the paths and A or Start enters the point's level. The pause menu switches between this map and the original.",
                "Clic en el mapa vacio: punto nuevo, unido al elegido. Arrastrar: mover. Clic derecho: borrar. Unir con... y clic en otro punto: pone o quita un camino. En el juego, la cruceta recorre los caminos y A o Start entra en el nivel del punto. El menu de pausa cambia entre este mapa y el original.",
            );
            for (k, line) in wrap(help, 38).iter().enumerate() {
                draw_text(line, 10.0, y + k as f32 * 16.0, 15.0, col(2));
            }
        } else {
            // which level each point of the map opens
            draw_text(tr(self.es, "Point", "Punto"), 10.0, y + 18.0, 18.0, col(3));
            if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
                self.point = (self.point + world::POINT_COUNT - 1) % world::POINT_COUNT;
            }
            draw_text(&format!("{:02X}", self.point), 104.0, y + 18.0, 20.0, col(3));
            if button(Rect::new(130.0, y, 26.0, 26.0), "+", false, 18.0) {
                self.point = (self.point + 1) % world::POINT_COUNT;
            }
            y += 30.0;
            let cur = self.points[self.point];
            let label = if cur == world::NO_LEVEL { tr(self.es, "no level", "sin nivel").to_string() } else { format!("{} {:02}", tr(self.es, "level", "nivel"), cur) };
            draw_text(tr(self.es, "Opens", "Abre"), 10.0, y + 18.0, 18.0, col(3));
            let mut change: i32 = 0;
            if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
                change = -1;
            }
            draw_text(&label, 104.0, y + 18.0, 18.0, col(3));
            if button(Rect::new(210.0, y, 26.0, 26.0), "+", false, 18.0) {
                change = 1;
            }
            if change != 0 && (cur < 32 || cur == world::NO_LEVEL) {
                // cycle: no level, 0, 1, ... 31, no level
                let i = if cur == world::NO_LEVEL { 32 } else { cur as i32 };
                let n = (i + change).rem_euclid(33);
                self.points[self.point] = if n == 32 { world::NO_LEVEL } else { n as u8 };
                self.points_dirty = true;
            }
            y += 34.0;
            draw_text(tr(self.es, "Points that open a level (click to choose):", "Puntos que abren un nivel (clic para elegir):"), 10.0, y + 12.0, 15.0, col(2));
            y += 18.0;
            let list: Vec<usize> = (0..world::POINT_COUNT).filter(|&p| self.points[p] < 32).collect();
            for (k, &p) in list.iter().enumerate() {
                let r = Rect::new(10.0 + (k % 3) as f32 * 92.0, y + (k / 3) as f32 * 24.0, 88.0, 22.0);
                if button(r, &format!("{:02X}>{:02}", p, self.points[p]), p == self.point, 15.0) {
                    self.point = p;
                }
            }
            let yy = y + ((list.len() + 2) / 3) as f32 * 24.0 + 16.0;
            let note = tr(self.es, "Where each point sits and the paths between them are fixed in the game's code.", "La posicion de cada punto y los caminos estan fijos en el codigo del juego.");
            for (i, line) in wrap(note, 36).iter().enumerate() {
                draw_text(line, 10.0, yy + i as f32 * 16.0, 15.0, col(2));
            }
        }
        draw_rectangle(PANEL, ch, cw, STATUS_H, col(1));
        draw_text(&self.status, PANEL + 8.0, ch + 18.0, 18.0, col(3));
        match act {
            1 => {
                if let Some(w) = self.world_undo.pop() {
                    self.world_redo.push(std::mem::replace(&mut self.world, w));
                    self.world_dirty = true;
                }
            }
            2 => {
                if let Some(w) = self.world_redo.pop() {
                    self.world_undo.push(std::mem::replace(&mut self.world, w));
                    self.world_dirty = true;
                }
            }
            3 => {
                self.world_snapshot();
                self.world = world::read(&self.original);
                self.points = world::point_levels(&self.original);
                self.world_dirty = true;
                self.points_dirty = true;
            }
            4 => {
                self.save();
            }
            6 => {
                if self.save() {
                    let game = exe_dir().join(if cfg!(windows) { "game.exe" } else { "game" });
                    match std::process::Command::new(&game).arg(&self.edit_path).spawn() {
                        Ok(_) => self.status = tr(self.es, "Game started with your map.", "Juego abierto con tu mapa.").into(),
                        Err(e) => self.status = format!("{}: {e}", game.display()),
                    }
                }
            }
            9 => self.enter_world(false),
            _ => {}
        }
    }

    /// Let the game load the level (from the world map) and keep its graphics.
    fn ensure_art(&mut self, l: usize) {
        if self.levels[l].art.is_some() {
            return;
        }
        if self.boot.is_none() {
            let mut g = match Gb::new(self.rom.clone(), 44100) {
                Ok(g) => g,
                Err(e) => {
                    self.status = e;
                    return;
                }
            };
            g.boot_to_map();
            self.boot = Some(g);
        }
        let mut g = self.boot.clone().unwrap();
        if !g.enter_level(l as u8) {
            self.status = tr(self.es, "The game didn't load this level; its graphics may be wrong.", "El juego no cargo este nivel; sus graficos pueden no ser correctos.").into();
        }
        let art = LevelArt::capture(&g);
        self.levels[l].atlas = Some(make_atlas(&art));
        self.levels[l].art = Some(art);
    }

    fn select_level(&mut self, l: usize) {
        self.cur = l;
        self.ensure_art(l);
        // Start the view on the part of the map that isn't the background filler.
        let map = &self.levels[l].map;
        let mut count = [0usize; 128];
        for &b in map {
            count[b as usize & 0x7F] += 1;
        }
        let bg = (0..128).max_by_key(|&i| count[i]).unwrap_or(0) as u8;
        // first column with something in it, then the rows used near it
        let c0 = (0..COLS).find(|&c| (0..ROWS).any(|r| map[r * COLS + c] != bg)).unwrap_or(0);
        let near: Vec<usize> = (0..ROWS)
            .filter(|&r| (c0..(c0 + 30).min(COLS)).any(|c| map[r * COLS + c] != bg))
            .collect();
        let (r0, r1) = (near.first().copied().unwrap_or(0), near.last().copied().unwrap_or(12));
        let view_h = (screen_height() - STATUS_H) / self.scale();
        let content_h = (r1 + 1 - r0) as f32 * BLOCK;
        let y = if content_h <= view_h { (r1 + 1) as f32 * BLOCK - view_h } else { r0 as f32 * BLOCK };
        self.cam = vec2(c0 as f32 * BLOCK, y.max(0.0));
        self.overview_stale = true;
        let shared = levels::sharing(&self.rom, l);
        self.status = if shared.is_empty() {
            format!("{} {:02}", tr(self.es, "Level", "Nivel"), l)
        } else {
            format!(
                "{} {:02}: {} {:?}",
                tr(self.es, "Level", "Nivel"),
                l,
                tr(self.es, "its map is also used by level(s)", "su mapa tambien lo usa(n) el/los nivel(es)"),
                shared
            )
        };
    }

    fn snapshot(&mut self) {
        let lv = &mut self.levels[self.cur];
        lv.undo.push((lv.map.clone(), lv.objs.clone()));
        if lv.undo.len() > 100 {
            lv.undo.remove(0);
        }
        lv.redo.clear();
    }

    fn undo(&mut self) {
        let lv = &mut self.levels[self.cur];
        if let Some((m, o)) = lv.undo.pop() {
            let now = (std::mem::replace(&mut lv.map, m), std::mem::replace(&mut lv.objs, o));
            lv.redo.push(now);
            lv.dirty = true;
            lv.objs_dirty = true;
            self.sel_obj = None;
            self.overview_stale = true;
        }
    }

    fn redo(&mut self) {
        let lv = &mut self.levels[self.cur];
        if let Some((m, o)) = lv.redo.pop() {
            let now = (std::mem::replace(&mut lv.map, m), std::mem::replace(&mut lv.objs, o));
            lv.undo.push(now);
            lv.dirty = true;
            lv.objs_dirty = true;
            self.sel_obj = None;
            self.overview_stale = true;
        }
    }

    fn restore_original(&mut self) {
        self.snapshot();
        let (_, start) = levels::map_location(&self.original, levels::header(&self.original, self.cur));
        self.levels[self.cur].map = levels::decode(&self.original, start).0;
        self.levels[self.cur].objs = objects::read(&self.original, self.cur).0;
        self.levels[self.cur].objs_dirty = true;
        self.sel_obj = None;
        self.levels[self.cur].dirty = true;
        self.overview_stale = true;
        self.status = tr(self.es, "Level restored to the original (save to keep it).", "Nivel restaurado al original (guarda para conservarlo).").into();
    }

    /// The edited maps of one bank: (map number, blocks), one per map.
    fn bank_edits(&self, bank: u8) -> Vec<(u8, &[u8])> {
        let mut v: Vec<(u8, &[u8])> = Vec::new();
        for l in 0..LEVELS {
            let h = levels::header(&self.rom, l);
            if h.bank == bank && self.levels[l].dirty && !v.iter().any(|e| e.0 == h.map) {
                v.push((h.map, self.levels[l].map.as_slice()));
            }
        }
        v
    }

    /// Bytes this level's zone needs and has.
    fn space(&self) -> (usize, usize) {
        let bank = levels::header(&self.rom, self.cur).bank;
        levels::bank_space(&self.rom, &self.original, bank, &self.bank_edits(bank))
    }

    fn save(&mut self) -> bool {
        let mut notes = Vec::new();
        let mut banks: Vec<u8> = (0..LEVELS).filter(|&l| self.levels[l].dirty).map(|l| levels::header(&self.rom, l).bank).collect();
        banks.sort();
        banks.dedup();
        for bank in banks {
            let edits: Vec<(u8, Vec<u8>)> = self.bank_edits(bank).into_iter().map(|(m, d)| (m, d.to_vec())).collect();
            let refs: Vec<(u8, &[u8])> = edits.iter().map(|(m, d)| (*m, d.as_slice())).collect();
            let mut rom = self.rom.clone();
            match levels::store_bank(&mut rom, &self.original, bank, &refs) {
                Ok(n) => {
                    notes.push(n);
                    self.rom = rom;
                }
                Err(e) => {
                    self.status = format!("{}: {e}", tr(self.es, "Not saved", "No guardado"));
                    return false;
                }
            }
            for l in 0..LEVELS {
                let h = levels::header(&self.rom, l);
                if h.bank != bank {
                    continue;
                }
                // Levels that share an edited map see the change too.
                if let Some((_, d)) = edits.iter().find(|e| e.0 == h.map) {
                    self.levels[l].map = d.clone();
                }
                self.levels[l].dirty = false;
            }
        }
        if self.world_dirty {
            world::write(&mut self.rom, &self.world);
            self.world_dirty = false;
            notes.push(tr(self.es, "world map", "mapa de mundos").to_string());
        }
        if self.points_dirty {
            world::write_point_levels(&mut self.rom, &self.points);
            self.points_dirty = false;
            notes.push(tr(self.es, "level points", "puntos de nivel").to_string());
        }
        if self.levels.iter().any(|l| l.objs_dirty) {
            let lists: Vec<Vec<Obj>> = self.levels.iter().map(|l| l.objs.clone()).collect();
            let mut rom = self.rom.clone();
            match objects::store(&mut rom, &self.original, &lists) {
                Ok(n) => {
                    notes.push(n);
                    self.rom = rom;
                    for l in 0..LEVELS {
                        // what the game will read: page by page, as stored
                        self.levels[l].objs = objects::read(&self.rom, l).0;
                        self.levels[l].objs_dirty = false;
                    }
                    self.sel_obj = None;
                }
                Err(e) => {
                    self.status = format!("{}: {e}", tr(self.es, "Not saved", "No guardado"));
                    return false;
                }
            }
        }
        if let Some(d) = self.edit_path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if self.custom_dirty {
            let res = if self.custom.points.is_empty() {
                // no points: no custom map, the game uses its own
                std::fs::remove_file(CustomMap::file_for(&self.edit_path)).or(Ok(()))
            } else {
                self.custom.save(&self.edit_path)
            };
            match res {
                Ok(_) => {
                    self.custom_dirty = false;
                    notes.push(tr(self.es, "your world map", "tu mapa de mundos").to_string());
                }
                Err(e) => {
                    self.status = format!("{}: {e}", tr(self.es, "Could not save the map", "No se pudo guardar el mapa"));
                    return false;
                }
            }
        }
        match std::fs::write(&self.edit_path, &self.rom) {
            Ok(_) => {
                self.status = format!(
                    "{} {} {}",
                    tr(self.es, "Saved", "Guardado en"),
                    self.edit_path.display(),
                    if notes.is_empty() { String::new() } else { format!("({})", notes.join(", ")) }
                );
                true
            }
            Err(e) => {
                self.status = format!("{}: {e}", tr(self.es, "Could not save", "No se pudo guardar"));
                false
            }
        }
    }

    fn play(&mut self) {
        if !self.save() {
            return;
        }
        let game = exe_dir().join(if cfg!(windows) { "game.exe" } else { "game" });
        match std::process::Command::new(&game)
            .arg(&self.edit_path)
            .arg("--level")
            .arg(self.cur.to_string())
            .spawn()
        {
            Ok(_) => self.status = format!("{} {:02}", tr(self.es, "Playing level", "Jugando el nivel"), self.cur),
            Err(e) => self.status = format!("{} {}: {e}", tr(self.es, "Could not start", "No se pudo abrir"), game.display()),
        }
    }

    /// Where the whole-level overview sits (top right of the level view).
    fn overview_rect(&self) -> Rect {
        let k = ((screen_width() - PANEL - 20.0) / COLS as f32).min(3.0);
        let (w, h) = (COLS as f32 * k, ROWS as f32 * k);
        Rect::new(screen_width() - w - 10.0, 10.0, w, h)
    }

    fn cell_at(&self, m: Vec2) -> Option<(i32, i32)> {
        if m.x < PANEL || m.y > screen_height() - STATUS_H {
            return None;
        }
        if self.show_overview && self.overview_rect().contains(m) {
            return None;
        }
        let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
        let (c, r) = ((w.x / BLOCK).floor() as i32, (w.y / BLOCK).floor() as i32);
        if c >= 0 && r >= 0 && (c as usize) < COLS && (r as usize) < ROWS {
            Some((c, r))
        } else {
            None
        }
    }

    fn set(&mut self, c: i32, r: i32, b: u8) {
        let i = r as usize * COLS + c as usize;
        let lv = &mut self.levels[self.cur];
        if lv.map[i] != b {
            lv.map[i] = b;
            lv.dirty = true;
            self.overview_stale = true;
        }
    }

    fn update(&mut self) {
        if self.in_world {
            self.update_world();
            return;
        }
        let m: Vec2 = mouse_position().into();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::Z) {
            self.undo();
        }
        if ctrl && is_key_pressed(KeyCode::Y) {
            self.redo();
        }
        if ctrl && is_key_pressed(KeyCode::S) {
            self.save();
        }
        if is_key_pressed(KeyCode::F5) {
            self.play();
        }
        if !ctrl {
            if is_key_pressed(KeyCode::G) {
                self.grid = !self.grid;
            }
            if is_key_pressed(KeyCode::B) {
                self.tool = Tool::Pencil;
            }
            if is_key_pressed(KeyCode::R) {
                self.tool = Tool::Rect;
            }
        }
        // Pan with arrows / WASD, middle mouse or Space + left mouse.
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
        if m.x >= PANEL && (is_mouse_button_pressed(MouseButton::Middle) || (space && is_mouse_button_pressed(MouseButton::Left))) {
            self.pan = Some((m, self.cam));
        }
        if let Some((m0, c0)) = self.pan {
            if is_mouse_button_down(MouseButton::Middle) || (space && is_mouse_button_down(MouseButton::Left)) {
                self.cam = c0 - (m - m0) / self.scale();
            } else {
                self.pan = None;
            }
        }
        // Zoom at the mouse.
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && m.x >= PANEL {
            let before = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
            if wheel > 0.0 && self.zoom + 1 < ZOOMS.len() {
                self.zoom += 1;
            } else if wheel < 0.0 && self.zoom > 0 {
                self.zoom -= 1;
            }
            self.cam = before - (m - vec2(PANEL, 0.0)) / self.scale();
        }
        let view = vec2(screen_width() - PANEL, screen_height() - STATUS_H) / self.scale();
        let max = vec2(COLS as f32 * BLOCK - view.x, ROWS as f32 * BLOCK - view.y).max(Vec2::ZERO);
        self.cam = self.cam.clamp(Vec2::ZERO, max);

        if !ctrl && is_key_pressed(KeyCode::M) {
            self.show_overview = !self.show_overview;
        }
        // Click or drag on the overview: centre the view there.
        let ov = self.overview_rect();
        if self.show_overview && ov.contains(m) && is_mouse_button_down(MouseButton::Left) {
            let k = ov.w / COLS as f32;
            let target = vec2((m.x - ov.x) / k, (m.y - ov.y) / k) * BLOCK;
            self.cam = (target - view * 0.5).clamp(Vec2::ZERO, max);
            return;
        }
        if !ctrl && (is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::O)) {
            self.mode = if self.mode == Mode::Blocks { Mode::Objects } else { Mode::Blocks };
            self.sel_obj = None;
        }
        if self.pan.is_some() || space {
            return;
        }
        if self.mode == Mode::Objects {
            self.pump_looks();
            self.update_objects(m);
            return;
        }
        let cell = self.cell_at(m);
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some((c, r)) = cell {
                self.block = self.levels[self.cur].map[r as usize * COLS + c as usize] & 0x7F;
            }
        }
        match self.tool {
            Tool::Pencil => {
                if is_mouse_button_pressed(MouseButton::Left) && cell.is_some() {
                    self.snapshot();
                }
                if is_mouse_button_down(MouseButton::Left) {
                    if let Some((c, r)) = cell {
                        self.set(c, r, self.block);
                    }
                }
            }
            Tool::Rect => {
                if is_mouse_button_pressed(MouseButton::Left) && cell.is_some() {
                    self.rect_start = cell;
                }
                if is_mouse_button_released(MouseButton::Left) {
                    if let (Some((c0, r0)), Some((c1, r1))) = (self.rect_start.take(), cell) {
                        self.snapshot();
                        for r in r0.min(r1)..=r0.max(r1) {
                            for c in c0.min(c1)..=c0.max(c1) {
                                self.set(c, r, self.block);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Capture one more object picture (types used in this level first, then the chosen
    /// type, then the rest), one per frame so the window stays responsive.
    fn pump_looks(&mut self) {
        if self.mode != Mode::Objects || self.in_world {
            return;
        }
        let Some(boot) = &self.boot else { return };
        let l = self.cur;
        let mut want: Vec<u8> = self.levels[l].objs.iter().map(|o| o.kind).collect();
        want.push(self.kind);
        want.extend(0..128u8);
        let Some(k) = want.into_iter().find(|k| !self.looks.contains_key(&(l, *k))) else { return };
        let look = objects::capture_look(boot, &self.rom, &self.original, l, k);
        let v = look.map(|lk| (look_texture(&lk), lk.dx as f32, lk.dy as f32));
        self.looks.insert((l, k), v);
    }

    fn look(&self, k: u8) -> Option<&(Texture2D, f32, f32)> {
        self.looks.get(&(self.cur, k)).and_then(|v| v.as_ref())
    }

    /// The object under a point of the level (world pixels), topmost first.
    fn object_at(&self, w: Vec2) -> Option<usize> {
        self.levels[self.cur].objs.iter().rposition(|o| Rect::new(o.x as f32, o.y as f32, 16.0, 16.0).contains(w))
    }

    fn update_objects(&mut self, m: Vec2) {
        let in_view = self.cell_at(m).is_some();
        let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
        let snap = |v: f32, max: f32| ((v / 8.0).floor() * 8.0).clamp(0.0, max) as u16;
        if is_mouse_button_pressed(MouseButton::Left) && in_view {
            self.snapshot();
            if let Some(i) = self.object_at(w) {
                let o = self.levels[self.cur].objs[i];
                self.sel_obj = Some(i);
                self.kind = o.kind;
                self.drag_obj = Some((i, w - vec2(o.x as f32, o.y as f32)));
            } else {
                let o = Obj { x: snap(w.x, 4088.0), y: snap(w.y, 760.0), kind: self.kind, flag: false };
                let lv = &mut self.levels[self.cur];
                lv.objs.push(o);
                lv.objs_dirty = true;
                let i = lv.objs.len() - 1;
                self.sel_obj = Some(i);
                self.drag_obj = Some((i, vec2(4.0, 4.0)));
            }
        }
        if let Some((i, off)) = self.drag_obj {
            if is_mouse_button_down(MouseButton::Left) {
                let p = w - off;
                let lv = &mut self.levels[self.cur];
                if let Some(o) = lv.objs.get_mut(i) {
                    let (nx, ny) = (snap(p.x, 4088.0), snap(p.y, 760.0));
                    if (nx, ny) != (o.x, o.y) {
                        o.x = nx;
                        o.y = ny;
                        lv.objs_dirty = true;
                    }
                }
            } else {
                self.drag_obj = None;
            }
        }
        let delete_here = is_mouse_button_pressed(MouseButton::Right) && in_view;
        let target = if delete_here { self.object_at(w) } else if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) { self.sel_obj } else { None };
        if let Some(i) = target {
            self.snapshot();
            let lv = &mut self.levels[self.cur];
            if i < lv.objs.len() {
                lv.objs.remove(i);
                lv.objs_dirty = true;
            }
            self.sel_obj = None;
            self.drag_obj = None;
        }
    }

    /// Pick an object type in the palette: the selected object takes it too.
    fn choose_kind(&mut self, k: u8) {
        self.kind = k;
        if let Some(i) = self.sel_obj {
            if self.levels[self.cur].objs.get(i).map(|o| o.kind != k).unwrap_or(false) {
                self.snapshot();
                let lv = &mut self.levels[self.cur];
                lv.objs[i].kind = k;
                lv.objs_dirty = true;
            }
        }
    }

    fn draw_objects(&self) {
        let objs_mode = self.mode == Mode::Objects;
        for (i, o) in self.levels[self.cur].objs.iter().enumerate() {
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
                let tint = if objs_mode { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.7) };
                draw_texture_ex(tex, x + dx * self.scale(), y + dy * self.scale(), tint, DrawTextureParams {
                    dest_size: Some(vec2(tex.width() * self.scale(), tex.height() * self.scale())),
                    ..Default::default()
                });
                draw_rectangle_lines(x, y, s, s, if sel { 3.0 } else { 1.0 }, if sel { WHITE } else { Color::new(1.0, 1.0, 1.0, if objs_mode { 0.35 } else { 0.0 }) });
                if objs_mode {
                    draw_text(&format!("{:02X}", o.kind), x + 2.0, y + s - 2.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.8));
                }
                continue;
            }
            draw_rectangle(x, y, s, s, Color::new(0.12, 0.12, 0.12, a));
            draw_rectangle_lines(x, y, s, s, if sel { 3.0 } else { 1.5 }, if sel { WHITE } else { Color::new(0.77, 0.81, 0.63, a + 0.2) });
            let t = format!("{:02X}", o.kind);
            let fs = (s * 0.55).clamp(10.0, 26.0);
            let dim = measure_text(&t, None, fs as u16, 1.0);
            draw_text(&t, x + (s - dim.width) / 2.0, y + (s + dim.offset_y) / 2.0, fs, Color::new(0.9, 0.95, 0.8, a + 0.25));
        }
    }

    fn draw(&mut self) {
        if self.in_world {
            self.draw_world();
            return;
        }
        clear_background(col(3));
        let s = self.scale() * BLOCK;
        let (cw, ch) = (screen_width() - PANEL, screen_height() - STATUS_H);
        // --- the level
        if let Some(atlas) = &self.levels[self.cur].atlas {
            let c0 = (self.cam.x / BLOCK).floor() as i32;
            let r0 = (self.cam.y / BLOCK).floor() as i32;
            let ncol = (cw / s).ceil() as i32 + 1;
            let nrow = (ch / s).ceil() as i32 + 1;
            let map = &self.levels[self.cur].map;
            for r in r0..(r0 + nrow).min(ROWS as i32) {
                for c in c0..(c0 + ncol).min(COLS as i32) {
                    let id = map[r as usize * COLS + c as usize] & 0x7F;
                    let x = PANEL + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_texture_ex(
                        atlas,
                        x.floor(),
                        y.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((id % 16) as f32 * 16.0, (id / 16) as f32 * 16.0, 16.0, 16.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            if self.grid && s >= 16.0 {
                let gc = Color::new(0.0, 0.0, 0.0, 0.15);
                for c in c0..=c0 + ncol {
                    let x = PANEL + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    draw_line(x, 0.0, x, ch, 1.0, gc);
                }
                for r in r0..=r0 + nrow {
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_line(PANEL, y, screen_width(), y, 1.0, gc);
                }
            }
            // where the level ends
            let end_x = PANEL + (COLS as f32 * BLOCK - self.cam.x) * self.scale();
            let end_y = (ROWS as f32 * BLOCK - self.cam.y) * self.scale();
            draw_rectangle_lines(PANEL - self.cam.x * self.scale(), -self.cam.y * self.scale(), end_x - (PANEL - self.cam.x * self.scale()), end_y + self.cam.y * self.scale(), 3.0, col(3));
            // hover / rectangle preview
            let m: Vec2 = mouse_position().into();
            if let (Mode::Blocks, Some((c, r))) = (self.mode, self.cell_at(m)) {
                let (a, b) = match (self.tool, self.rect_start) {
                    (Tool::Rect, Some((c0, r0))) => ((c0.min(c), r0.min(r)), (c0.max(c), r0.max(r))),
                    _ => ((c, r), (c, r)),
                };
                let x = PANEL + (a.0 as f32 * BLOCK - self.cam.x) * self.scale();
                let y = (a.1 as f32 * BLOCK - self.cam.y) * self.scale();
                let w = (b.0 - a.0 + 1) as f32 * s;
                let h = (b.1 - a.1 + 1) as f32 * s;
                if self.tool == Tool::Pencil || self.rect_start.is_none() {
                    draw_texture_ex(
                        atlas,
                        x,
                        y,
                        Color::new(1.0, 1.0, 1.0, 0.6),
                        DrawTextureParams {
                            source: Some(Rect::new((self.block % 16) as f32 * 16.0, (self.block / 16) as f32 * 16.0, 16.0, 16.0)),
                            dest_size: Some(vec2(s, s)),
                            ..Default::default()
                        },
                    );
                }
                draw_rectangle_lines(x, y, w, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.9));
                let id = self.levels[self.cur].map[r as usize * COLS + c as usize];
                draw_text(&format!("{c},{r}  #{id:02X}"), PANEL + 8.0, ch - 8.0, 20.0, WHITE);
            }
        } else {
            draw_text(tr(self.es, "Loading the level from the game...", "Cargando el nivel desde el juego..."), PANEL + 30.0, 60.0, 26.0, col(0));
        }
        self.draw_objects();
        if self.mode == Mode::Objects {
            let m: Vec2 = mouse_position().into();
            if self.cell_at(m).is_some() {
                let w = (m - vec2(PANEL, 0.0)) / self.scale() + self.cam;
                let tip = match self.object_at(w) {
                    Some(i) => {
                        let o = self.levels[self.cur].objs[i];
                        format!("#{:02X}  x {} y {}  {}", o.kind, o.x, o.y, tr(self.es, "drag: move   right click: delete", "arrastra: mover   clic dcho: borrar"))
                    }
                    None => format!("{} #{:02X}", tr(self.es, "click: place object", "clic: poner objeto"), self.kind),
                };
                draw_text(&tip, PANEL + 8.0, ch - 8.0, 20.0, WHITE);
            }
        }
        // --- whole-level overview (M), with the part on screen outlined
        if self.show_overview {
            if self.overview_stale || self.overview.is_none() {
                if let Some(art) = &self.levels[self.cur].art {
                    self.overview = Some(make_overview(art, &self.levels[self.cur].map));
                    self.overview_stale = false;
                }
            }
            if let Some(t) = &self.overview {
                let r = self.overview_rect();
                let k = r.w / COLS as f32;
                draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, col(3));
                draw_texture_ex(t, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
                let v = vec2(cw, ch) / self.scale() / BLOCK * k;
                let p = self.cam / BLOCK * k;
                draw_rectangle_lines(r.x + p.x, r.y + p.y, v.x.min(r.w), v.y.min(r.h), 2.0, Color::from_rgba(255, 255, 255, 230));
                draw_text(tr(self.es, "MAP (M)", "MAPA (M)"), r.x + 4.0, r.y + r.h + 16.0, 16.0, col(0));
            }
        }

        // --- side panel
        draw_rectangle(0.0, 0.0, PANEL, screen_height(), col(0));
        draw_rectangle(PANEL - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "LEVEL EDITOR", "EDITOR DE NIVELES"), 10.0, 22.0, 24.0, col(3));
        let mut y = 34.0;
        let mut pick = None;
        for l in 0..LEVELS {
            let (cx, cy) = (10.0 + (l % 8) as f32 * 35.0, y + (l / 8) as f32 * 25.0);
            let label = format!("{:02}{}", l, if self.levels[l].dirty || self.levels[l].objs_dirty { "*" } else { "" });
            if button(Rect::new(cx, cy, 33.0, 23.0), &label, l == self.cur, 16.0) {
                pick = Some(l);
            }
        }
        y += 4.0 * 25.0 + 4.0;
        let bw = 92.0;
        let mut act = 0;
        if button(Rect::new(10.0, y, 276.0, 24.0), tr(self.es, "World map editor", "Editor del mapa de mundos"), false, 16.0) {
            act = 9;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 136.0, 26.0), tr(self.es, "Blocks (Tab)", "Bloques (Tab)"), self.mode == Mode::Blocks, 16.0) {
            self.mode = Mode::Blocks;
            self.sel_obj = None;
        }
        if button(Rect::new(150.0, y, 136.0, 26.0), tr(self.es, "Objects (Tab)", "Objetos (Tab)"), self.mode == Mode::Objects, 16.0) {
            self.mode = Mode::Objects;
        }
        y += 30.0;
        if self.mode == Mode::Blocks {
            if button(Rect::new(10.0, y, bw, 26.0), tr(self.es, "Pencil (B)", "Lapiz (B)"), self.tool == Tool::Pencil, 16.0) {
                self.tool = Tool::Pencil;
            }
            if button(Rect::new(10.0 + bw + 4.0, y, bw, 26.0), tr(self.es, "Box (R)", "Caja (R)"), self.tool == Tool::Rect, 16.0) {
                self.tool = Tool::Rect;
            }
        } else {
            let n = self.levels[self.cur].objs.len();
            draw_text(&format!("{} {n}", tr(self.es, "objects:", "objetos:")), 14.0, y + 18.0, 18.0, col(3));
        }
        if button(Rect::new(10.0 + 2.0 * (bw + 4.0), y, bw - 8.0, 26.0), tr(self.es, "Grid (G)", "Rejilla (G)"), self.grid, 16.0) {
            self.grid = !self.grid;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, bw, 26.0), tr(self.es, "Undo", "Deshacer"), false, 16.0) {
            act = 1;
        }
        if button(Rect::new(10.0 + bw + 4.0, y, bw, 26.0), tr(self.es, "Redo", "Rehacer"), false, 16.0) {
            act = 2;
        }
        if button(Rect::new(10.0 + 2.0 * (bw + 4.0), y, bw - 8.0, 26.0), tr(self.es, "Original", "Original"), false, 16.0) {
            act = 3;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 136.0, 28.0), tr(self.es, "Save (Ctrl+S)", "Guardar (Ctrl+S)"), false, 17.0) {
            act = 4;
        }
        if button(Rect::new(150.0, y, 136.0, 28.0), tr(self.es, "Play (F5)", "Jugar (F5)"), false, 17.0) {
            act = 5;
        }
        y += 36.0;
        if self.mode == Mode::Objects {
            // object types 00-7F; the ones this level uses are dark
            draw_text(&format!("{} #{:02X}", tr(self.es, "OBJECT TYPES  chosen", "TIPOS DE OBJETO  elegido"), self.kind), 10.0, y + 12.0, 18.0, col(3));
            y += 18.0;
            let used: Vec<u8> = self.levels[self.cur].objs.iter().map(|o| o.kind).collect();
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
        } else {
        draw_text(&format!("{} #{:02X}", tr(self.es, "BLOCKS  selected", "BLOQUES  elegido"), self.block), 10.0, y + 12.0, 18.0, col(3));
        y += 18.0;
        if let Some(atlas) = &self.levels[self.cur].atlas {
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
        // --- status bar, with how full this level's zone is
        draw_rectangle(PANEL, ch, cw, STATUS_H, col(1));
        draw_text(&self.status, PANEL + 8.0, ch + 18.0, 18.0, col(3));
        if get_time() - self.space_at > 0.25 {
            self.space_cache = self.space();
            let lists: Vec<Vec<Obj>> = self.levels.iter().map(|l| l.objs.clone()).collect();
            self.obj_space = objects::space(&self.rom, &self.original, &lists);
            self.space_at = get_time();
        }
        let (need, cap) = if self.mode == Mode::Objects { self.obj_space } else { self.space_cache };
        let bw = 220.0;
        let bx = screen_width() - bw - 10.0;
        let frac = (need as f32 / cap.max(1) as f32).min(1.0);
        draw_rectangle(bx, ch + 6.0, bw, 14.0, col(0));
        draw_rectangle(bx, ch + 6.0, bw * frac, 14.0, if need > cap { Color::from_rgba(170, 40, 30, 255) } else { col(2) });
        draw_rectangle_lines(bx, ch + 6.0, bw, 14.0, 2.0, col(3));
        let what = if self.mode == Mode::Objects { tr(self.es, "object space", "espacio objetos") } else { tr(self.es, "zone space", "espacio zona") };
        let label = format!("{what} {need}/{cap}");
        let dim = measure_text(&label, None, 16, 1.0);
        draw_text(&label, bx - dim.width - 8.0, ch + 18.0, 16.0, col(3));

        match act {
            1 => self.undo(),
            2 => self.redo(),
            3 => self.restore_original(),
            4 => {
                self.save();
            }
            5 => self.play(),
            9 => self.enter_world(true),
            _ => {}
        }
        if let Some(l) = pick {
            self.select_level(l);
        }
    }
}

/// Split text into lines of at most `n` characters at spaces.
fn wrap(t: &str, n: usize) -> Vec<String> {
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

fn button(r: Rect, label: &str, active: bool, size: f32) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = r.contains(m);
    let (bg, fg) = if active { (col(3), col(0)) } else if hover { (col(1), col(3)) } else { (col(0), col(3)) };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(3));
    let dim = measure_text(label, None, size as u16, 1.0);
    draw_text(label, r.x + (r.w - dim.width) / 2.0, r.y + (r.h + dim.offset_y) / 2.0 - 1.0, size, fg);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

fn conf() -> Conf {
    Conf {
        window_title: "Level Editor (Rust)".into(),
        window_width: 1280,
        window_height: 820,
        window_resizable: true,
        high_dpi: false,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let shot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let level = args.iter().position(|a| a == "--level").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok());
    let rom_arg = args.iter().find(|a| !a.starts_with("--") && a.to_lowercase().ends_with(".gb")).map(PathBuf::from);

    clear_background(col(3));
    draw_text("Loading...", 40.0, 60.0, 30.0, col(0));
    next_frame().await;

    let Some(rom_path) = find_rom(rom_arg) else {
        loop {
            clear_background(col(3));
            draw_text("No Game Boy cartridge (.gb) was found.", 40.0, 60.0, 30.0, col(0));
            next_frame().await;
        }
    };
    let mut ed = match Editor::new(&rom_path) {
        Ok(e) => e,
        Err(e) => loop {
            clear_background(col(3));
            draw_text(&e, 40.0, 60.0, 26.0, col(0));
            next_frame().await;
        },
    };
    ed.select_level(level.unwrap_or(0).min(LEVELS - 1));
    if args.iter().any(|a| a == "--objects") {
        ed.mode = Mode::Objects;
    }
    if args.iter().any(|a| a == "--world") {
        ed.enter_world(true);
        if args.iter().any(|a| a == "--points") {
            ed.world_tab = 1;
        }
        if args.iter().any(|a| a == "--custom") {
            ed.world_tab = 2;
            if ed.custom.points.is_empty() {
                // a sample route for the check picture (not saved)
                for (i, (x, y, l)) in [(120, 120, Some(0u8)), (180, 140, None), (240, 150, Some(1)), (300, 200, Some(2))].iter().enumerate() {
                    ed.custom.points.push(Point { x: *x, y: *y, level: *l });
                    if i > 0 {
                        ed.custom.paths.push((i - 1, i));
                    }
                }
                ed.sel_point = Some(2);
            }
        }
    }
    let mut frames = 0;
    if shot.is_some() && ed.mode == Mode::Objects {
        // for the check picture: capture this level's object pictures first
        for _ in 0..ed.levels[ed.cur].objs.len() + 1 {
            ed.pump_looks();
        }
    }
    loop {
        ed.update();
        ed.draw();
        frames += 1;
        if let Some(p) = &shot {
            if frames >= 20 {
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                get_screen_data().export_png(&p.to_string_lossy());
                return;
            }
        }
        next_frame().await;
    }
}
