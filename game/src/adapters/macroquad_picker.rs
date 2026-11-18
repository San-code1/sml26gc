//! The start screen when the game is opened without a cartridge on the command line. First the
//! base cartridge (`host::base_cartridge`): while there is no valid one, the screen only asks
//! for it. Then what to play, one at a time in a selector: VANILLA (the base cartridge) or a
//! project of the editor (`editor/projects/<name>/`, any that has its project file; its
//! cartridge is made from it over the base cartridge when missing or older,
//! `host::projects::ensure_rom`), each with its own saves, shown as a cartridge whose label
//! holds the cartridge's own title screen, running (`title_preview`). Choosing only selects:
//! Play starts the game, Edit opens the project in the editor.
use super::title_preview::TitlePreview;
use crate::domain::i18n::LANG_ES;
use crate::domain::settings::Settings;
use gb_core::palette::PALETTES;
use host::BaseProblem;
use macroquad::prelude::*;
use std::path::PathBuf;

/// Something to play: a cartridge and the folder its saves go to.
#[derive(Clone, Debug, PartialEq)]
pub struct Play {
    /// None: the original cartridge (VANILLA).
    pub project: Option<String>,
    pub rom: PathBuf,
}

impl Play {
    pub fn saves(&self) -> PathBuf {
        host::saves_dir(self.project.as_deref())
    }
}

pub enum Choice {
    /// Nothing yet.
    Wait,
    Play(Play),
    /// Open the system's dialog to choose the base cartridge.
    Browse,
    /// Turn the debug / cheats box on or off.
    Cheats,
    /// Open the editor on this project, or (None) on a new one.
    Edit(Option<String>),
    Quit,
}

/// What the selector says about a choice, read when the list is.
struct Card {
    kind: &'static str,
    name: String,
    about: String,
}

pub struct Picker {
    base: Result<PathBuf, BaseProblem>,
    plays: Vec<Play>,
    cards: Vec<Card>,
    pub selected: usize,
    /// Why the last choice didn't work.
    pub error: Option<String>,
    /// The "Debug / cheats" box.
    pub cheats: bool,
    es: bool,
    /// When the list was last read (projects made in the editor meanwhile show up).
    listed: f64,
    /// The base cartridge's title screen, and the palette it is shown with.
    preview: Option<TitlePreview>,
    palette: usize,
}

const BG: Color = Color::new(0.06, 0.08, 0.1, 1.0);
const PANEL: Color = Color::new(0.11, 0.14, 0.15, 1.0);
const RAISED: Color = Color::new(0.16, 0.2, 0.19, 1.0);
const HOVER: Color = Color::new(0.21, 0.27, 0.23, 1.0);
const LINE: Color = Color::new(0.25, 0.3, 0.28, 1.0);
const TEXT: Color = Color::new(0.93, 0.95, 0.9, 1.0);
const DIM: Color = Color::new(0.58, 0.64, 0.58, 1.0);
const ACCENT: Color = Color::new(0.55, 0.78, 0.36, 1.0);
const ACCENT_DARK: Color = Color::new(0.08, 0.12, 0.06, 1.0);
const RED: Color = Color::new(0.95, 0.45, 0.4, 1.0);
/// The cartridge: its grey plastic, the label paper and its ink.
const CART: Color = Color::new(0.62, 0.63, 0.61, 1.0);
const CART_LIGHT: Color = Color::new(0.75, 0.76, 0.74, 1.0);
const CART_DARK: Color = Color::new(0.49, 0.5, 0.49, 1.0);
const CART_EDGE: Color = Color::new(0.33, 0.34, 0.33, 1.0);
const PAPER: Color = Color::new(0.93, 0.92, 0.87, 1.0);
const INK: Color = Color::new(0.12, 0.12, 0.12, 1.0);
/// The label's stripe for the original game (a project's is the accent).
const STRIPE_ORIGINAL: Color = Color::new(0.82, 0.29, 0.27, 1.0);
/// The cartridge's parts, top to bottom: the grip, the label's margin round the title, the
/// name under it, the foot; its sides; the cut top right corner.
const GRIP: f32 = 30.0;
const FRAME: f32 = 8.0;
const NAME: f32 = 38.0;
const FOOT: f32 = 18.0;
const SIDE: f32 = 22.0;
const CUT: f32 = 16.0;
/// Room under the cartridge: its kind and about, the dots, the actions, the settings row and
/// the base cartridge's name.
const BELOW: f32 = 272.0;
const CART_TOP: f32 = 54.0;
/// The widest the content gets (centred in a larger window).
const COLUMN: f32 = 560.0;

/// A text's width at `size`.
fn width(t: &str, size: f32) -> f32 {
    measure_text(t, None, size as u16, 1.0).width
}

fn centred(t: &str, cx: f32, y: f32, size: f32, c: Color) {
    draw_text(t, cx - width(t, size) / 2.0, y, size, c);
}

/// A button: its label left, its key at the right in a small box; true when clicked.
fn button(r: Rect, label: &str, key: &str, primary: bool, on: bool) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = on && r.contains(m);
    let (fill, fg) = match (primary && on, hover) {
        (true, false) => (ACCENT, ACCENT_DARK),
        (true, true) => (Color::new(0.64, 0.86, 0.44, 1.0), ACCENT_DARK),
        (false, true) => (HOVER, TEXT),
        (false, false) => (RAISED, if on { TEXT } else { DIM }),
    };
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    if !primary {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if on { LINE } else { PANEL });
    }
    let size = if r.h >= 44.0 { 22.0 } else { 18.0 };
    let kw = width(key, 14.0) + 12.0;
    let room = r.w - kw - 28.0;
    let mut label = label.to_string();
    while width(&label, size) > room && label.pop().is_some() {}
    draw_text(&label, r.x + 16.0, r.y + r.h / 2.0 + size * 0.35, size, fg);
    if !key.is_empty() {
        let kx = r.x + r.w - kw - 10.0;
        let ky = r.y + (r.h - 20.0) / 2.0;
        draw_rectangle_lines(kx, ky, kw, 20.0, 1.0, if primary && on { ACCENT_DARK } else { DIM });
        draw_text(key, kx + 6.0, ky + 14.0, 14.0, if primary && on { ACCENT_DARK } else { DIM });
    }
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// The selector's round arrow buttons, and the room between them and the card.
const ARROW_R: f32 = 22.0;
const ARROW_GAP: f32 = 16.0;

/// A round arrow button pointing left (`-1`) or right (`1`); true when clicked.
fn arrow(cx: f32, cy: f32, dir: f32, on: bool) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = on && (m - vec2(cx, cy)).length() <= ARROW_R;
    draw_circle(cx, cy, ARROW_R, if hover { HOVER } else { RAISED });
    draw_circle_lines(cx, cy, ARROW_R, 1.0, LINE);
    let c = if on { TEXT } else { LINE };
    draw_triangle(vec2(cx + 7.0 * dir, cy), vec2(cx - 5.0 * dir, cy - 9.0), vec2(cx - 5.0 * dir, cy + 9.0), c);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

impl Picker {
    pub fn new(settings: &Settings) -> Picker {
        let mut p = Picker { base: Err(BaseProblem::NotChosen), plays: Vec::new(), cards: Vec::new(), selected: 0, error: None, cheats: settings.cheats, es: settings.lang == LANG_ES, listed: 0.0, preview: None, palette: settings.palette };
        p.reload(&settings.last_play);
        p
    }

    /// Check the base cartridge again and list what can be played, `last` chosen.
    pub fn reload(&mut self, last: &str) {
        self.listed = get_time();
        self.base = host::base_cartridge();
        self.plays.clear();
        if let Ok(base) = &self.base {
            self.plays.push(Play { project: None, rom: base.clone() });
            for name in host::projects().into_iter().filter(|n| host::projects::playable(n)) {
                let rom = host::project_rom_in(&name).unwrap_or_else(|| host::project_rom(&name, base));
                self.plays.push(Play { project: Some(name), rom });
            }
        }
        self.cards = self.plays.iter().map(|p| self.card(p)).collect();
        match &self.base {
            Ok(base) if self.preview.as_ref().is_none_or(|p| p.rom != *base) => {
                self.preview = TitlePreview::new(base, PALETTES[self.palette % PALETTES.len()]);
            }
            Ok(_) => {}
            Err(_) => self.preview = None,
        }
        self.selected = self.plays.iter().position(|p| p.project.as_deref().unwrap_or("") == last).unwrap_or(0);
    }

    fn card(&self, p: &Play) -> Card {
        let Some(name) = &p.project else {
            return Card { kind: self.tr("THE ORIGINAL GAME", "EL JUEGO ORIGINAL"), name: host::VANILLA.into(), about: self.tr("Super Mario Land 2 as it came, with its own saves", "Super Mario Land 2 tal cual, con sus propias partidas").into() };
        };
        let file = host::project_file(name);
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        // the project file's third line counts its levels (the game's 32 and the new ones)
        let new_levels = text.lines().nth(2).and_then(|l| l.strip_prefix("levels ")).and_then(|n| n.trim().parse::<usize>().ok()).map(|n| n.saturating_sub(32));
        let saved = std::fs::metadata(&file).and_then(|m| m.modified()).ok().map(|t| self.ago(t));
        let mut about = Vec::new();
        if let Some(n) = new_levels.filter(|&n| n > 0) {
            about.push(format!("{n} {}", if n == 1 { self.tr("new level", "nivel nuevo") } else { self.tr("new levels", "niveles nuevos") }));
        }
        if let Some(s) = saved {
            about.push(s);
        }
        Card { kind: self.tr("EDITOR PROJECT", "PROYECTO DEL EDITOR"), name: name.clone(), about: about.join("  -  ") }
    }

    fn ago(&self, t: std::time::SystemTime) -> String {
        let s = std::time::SystemTime::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
        let (n, en, es) = match s {
            0..=59 => return self.tr("saved just now", "guardado ahora mismo").into(),
            60..=3599 => (s / 60, "min", "min"),
            3600..=86399 => (s / 3600, "h", "h"),
            _ => (s / 86400, "days", "dias"),
        };
        if self.es {
            format!("guardado hace {n} {es}")
        } else {
            format!("saved {n} {en} ago")
        }
    }

    pub fn es(&self) -> bool {
        self.es
    }

    /// The cartridge to play: the base one, or the project's (made when needed).
    pub fn ready(&self, play: &Play) -> Result<Play, String> {
        let (Some(name), Ok(base)) = (&play.project, &self.base) else { return Ok(play.clone()) };
        host::projects::ensure_rom(base, name).map(|rom| Play { project: play.project.clone(), rom }).map_err(|e| e.text(self.es))
    }

    fn tr<'a>(&self, en: &'a str, es: &'a str) -> &'a str {
        if self.es {
            es
        } else {
            en
        }
    }

    /// Where the "Open" dialog starts: the folder of the cartridge chosen before.
    pub fn start_dir(&self) -> Option<PathBuf> {
        host::base_cartridge::chosen_before().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    pub fn dialog_texts(&self) -> (&'static str, &'static str) {
        if self.es {
            ("Elige el cartucho de Super Mario Land 2", "Cartuchos de Game Boy")
        } else {
            ("Choose the Super Mario Land 2 cartridge", "Game Boy cartridges")
        }
    }

    /// Draw the screen and read the keys and mouse.
    pub fn frame(&mut self) -> Choice {
        clear_background(BG);
        let (w, h) = (screen_width(), screen_height());
        let col_w = (w - 48.0).min(COLUMN);
        let x = (w - col_w) / 2.0;
        let cx = w / 2.0;
        draw_text("SUPER MARIO LAND 2", x, 40.0, 16.0, ACCENT);
        let choice = match self.base.clone() {
            Err(problem) => self.cartridge_screen(&problem, x, col_w),
            Ok(base) => self.play_screen(&base, x, col_w, cx, h),
        };
        if let Some(e) = &self.error {
            let lines: Vec<String> = e.lines().flat_map(|l| wrap(l, col_w, 16.0)).collect();
            let top = h - 44.0 - lines.len() as f32 * 19.0;
            for (i, line) in lines.iter().enumerate() {
                draw_text(line, x, top + i as f32 * 19.0, 16.0, RED);
            }
        }
        choice
    }

    /// No usable base cartridge: why, and choosing one.
    fn cartridge_screen(&mut self, problem: &BaseProblem, x: f32, col_w: f32) -> Choice {
        draw_text(self.tr("Base cartridge", "Cartucho base"), x, 82.0, 34.0, TEXT);
        let card = Rect::new(x, 108.0, col_w, 150.0);
        draw_rectangle(card.x, card.y, card.w, card.h, PANEL);
        draw_rectangle_lines(card.x, card.y, card.w, card.h, 1.0, LINE);
        let mut y = card.y + 34.0;
        let c = if *problem == BaseProblem::NotChosen { TEXT } else { RED };
        for line in problem.text(self.es).lines().flat_map(|l| wrap(l, col_w - 40.0, 18.0)) {
            draw_text(&line, x + 20.0, y, 18.0, c);
            y += 23.0;
        }
        for line in wrap(self.tr("The game and the editor share it; the file may have any name.", "El juego y el editor lo comparten; el fichero puede llamarse como sea."), col_w - 40.0, 15.0) {
            draw_text(&line, x + 20.0, card.y + card.h - 18.0, 15.0, DIM);
        }
        let mut choice = Choice::Wait;
        if button(Rect::new(x, card.y + card.h + 24.0, col_w, 48.0), self.tr("Choose the cartridge file...", "Elegir el fichero del cartucho..."), "O", true, true) || is_key_pressed(KeyCode::O) || is_key_pressed(KeyCode::Enter) {
            choice = Choice::Browse;
        }
        if button(Rect::new(x, card.y + card.h + 82.0, col_w, 38.0), self.tr("Quit", "Salir"), "Esc", false, true) || is_key_pressed(KeyCode::Escape) {
            choice = Choice::Quit;
        }
        choice
    }

    /// What to play: the selector, its actions, the settings row.
    fn play_screen(&mut self, base: &std::path::Path, x: f32, col_w: f32, cx: f32, h: f32) -> Choice {
        // every two seconds, to show projects made or renamed in the editor
        if get_time() - self.listed > 2.0 {
            let keep = self.plays.get(self.selected).and_then(|p| p.project.clone()).unwrap_or_default();
            self.reload(&keep);
        }
        let what = self.tr("What to play", "Que jugar");
        draw_text(what, x + col_w - width(what, 16.0), 40.0, 16.0, DIM);
        let n = self.plays.len();
        // the selector: the cartridge, arrows either side, its kind and a dot per choice below
        let scale = [2.0, 1.5, 1.0].into_iter().find(|&s| cartridge_size(s).1 <= h - BELOW - CART_TOP).unwrap_or(1.0);
        let (cart_w, cart_h) = cartridge_size(scale);
        let cart = Rect::new((cx - cart_w / 2.0).round(), CART_TOP, cart_w, cart_h);
        let wheel = mouse_wheel().1;
        let step = |d: i32, s: &mut usize| *s = (*s as i32 + d).rem_euclid(n as i32) as usize;
        let mid = cart.y + cart.h / 2.0;
        if arrow(cart.x - ARROW_GAP - ARROW_R, mid, -1.0, n > 1) || is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Up) || (wheel > 0.0) {
            step(-1, &mut self.selected);
        }
        if arrow(cart.x + cart.w + ARROW_GAP + ARROW_R, mid, 1.0, n > 1) || is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Down) || (wheel < 0.0) {
            step(1, &mut self.selected);
        }
        if let Some(p) = &mut self.preview {
            p.step();
        }
        let original = self.plays[self.selected].project.is_none();
        let c = &self.cards[self.selected];
        draw_cartridge(cart, scale, self.preview.as_ref().map(|p| p.texture()), &c.name, original);
        let below = cart.y + cart.h;
        centred(c.kind, cx, below + 24.0, 14.0, if original { STRIPE_ORIGINAL } else { ACCENT });
        if let Some(line) = wrap(&c.about, col_w, 15.0).first() {
            centred(line, cx, below + 44.0, 15.0, DIM);
        }
        if n == 1 {
            centred(self.tr("Projects made in the editor show here too.", "Aqui salen tambien los proyectos hechos en el editor."), cx, below + 64.0, 14.0, DIM);
        } else {
            let dots = 10.0 * n as f32;
            for i in 0..n {
                let dx = cx - dots / 2.0 + 5.0 + i as f32 * 10.0;
                if i == self.selected {
                    draw_circle(dx, below + 59.0, 3.5, ACCENT);
                } else {
                    draw_circle_lines(dx, below + 59.0, 3.0, 1.0, DIM);
                }
            }
            let place = format!("{} / {n}", self.selected + 1);
            draw_text(&place, x + col_w - width(&place, 14.0), below + 64.0, 14.0, DIM);
        }
        // actions on the choice
        let mut choice = Choice::Wait;
        let project = self.plays[self.selected].project.clone();
        let mut y = below + 74.0;
        if button(Rect::new(x, y, col_w, 50.0), self.tr("Play", "Jugar"), "Enter", true, true) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            choice = Choice::Play(self.plays[self.selected].clone());
        }
        y += 60.0;
        let half = (col_w - 10.0) / 2.0;
        let edit = if project.is_some() { self.tr("Edit in the editor", "Editar en el editor") } else { self.tr("The original is not edited", "El original no se edita") };
        if (button(Rect::new(x, y, half, 40.0), edit, "E", false, project.is_some()) || is_key_pressed(KeyCode::E)) && project.is_some() {
            choice = Choice::Edit(project.clone());
        }
        if button(Rect::new(x + half + 10.0, y, half, 40.0), self.tr("New project", "Proyecto nuevo"), "N", false, true) || is_key_pressed(KeyCode::N) {
            choice = Choice::Edit(None);
        }
        // the settings row, quieter
        y += 60.0;
        draw_line(x, y - 12.0, x + col_w, y - 12.0, 1.0, LINE);
        let third = (col_w - 20.0) / 3.0;
        let cheats = format!("{} {}", if self.cheats { "[x]" } else { "[ ]" }, self.tr("Cheats", "Trucos"));
        if button(Rect::new(x, y, third, 34.0), &cheats, "C", false, true) || is_key_pressed(KeyCode::C) {
            choice = Choice::Cheats;
        }
        if button(Rect::new(x + third + 10.0, y, third, 34.0), self.tr("Cartridge...", "Cartucho..."), "O", false, true) || is_key_pressed(KeyCode::O) {
            choice = Choice::Browse;
        }
        if button(Rect::new(x + 2.0 * (third + 10.0), y, third, 34.0), self.tr("Quit", "Salir"), "Esc", false, true) || is_key_pressed(KeyCode::Escape) {
            choice = Choice::Quit;
        }
        let note = format!("{} {}", self.tr("Base cartridge:", "Cartucho base:"), base.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
        let lines = wrap(&note, col_w, 14.0);
        for (i, line) in lines.iter().enumerate() {
            draw_text(line, x, h - 16.0 - (lines.len() - 1 - i) as f32 * 17.0, 14.0, DIM);
        }
        choice
    }
}

/// The cartridge's width and height with its title screen `scale` times its size.
fn cartridge_size(scale: f32) -> (f32, f32) {
    (160.0 * scale + 2.0 * SIDE, GRIP + 144.0 * scale + 2.0 * FRAME + NAME + FOOT)
}

/// The cartridge's outline filled with `c`: a rectangle with its top right corner cut.
fn cartridge_shape(r: Rect, c: Color) {
    draw_rectangle(r.x, r.y, r.w - CUT, r.h, c);
    draw_rectangle(r.x + r.w - CUT, r.y + CUT, CUT, r.h - CUT, c);
    draw_triangle(vec2(r.x + r.w - CUT, r.y), vec2(r.x + r.w, r.y + CUT), vec2(r.x + r.w - CUT, r.y + CUT), c);
}

/// A cartridge in `r`: grip ridges, a recessed label with `title` (the title screen, `scale`
/// times its size) and `name` under it, its stripe red for the original game, and the foot.
fn draw_cartridge(r: Rect, scale: f32, title: Option<&Texture2D>, name: &str, original: bool) {
    cartridge_shape(Rect::new(r.x + 6.0, r.y + 8.0, r.w, r.h), Color::new(0.0, 0.0, 0.0, 0.35));
    cartridge_shape(r, CART);
    let corners = [vec2(r.x, r.y), vec2(r.x + r.w - CUT, r.y), vec2(r.x + r.w, r.y + CUT), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % corners.len()];
        draw_line(a.x, a.y, b.x, b.y, 2.0, CART_EDGE);
    }
    draw_line(r.x + 2.0, r.y + 2.0, r.x + r.w - CUT - 1.0, r.y + 2.0, 1.0, CART_LIGHT);
    // the grip
    for i in 0..4 {
        let y = r.y + 8.0 + i as f32 * 5.0;
        draw_line(r.x + 18.0, y, r.x + r.w - CUT - 12.0, y, 1.0, CART_DARK);
        draw_line(r.x + 18.0, y + 1.0, r.x + r.w - CUT - 12.0, y + 1.0, 1.0, CART_LIGHT);
    }
    // the label in its recess
    let (tw, th) = (160.0 * scale, 144.0 * scale);
    let recess = Rect::new(r.x + 12.0, r.y + GRIP, r.w - 24.0, th + 2.0 * FRAME + NAME);
    draw_rectangle(recess.x, recess.y, recess.w, recess.h, CART_DARK);
    draw_line(recess.x, recess.y + recess.h, recess.x + recess.w, recess.y + recess.h, 1.0, CART_LIGHT);
    let paper = Rect::new(recess.x + 3.0, recess.y + 3.0, recess.w - 6.0, recess.h - 6.0);
    draw_rectangle(paper.x, paper.y, paper.w, paper.h, PAPER);
    draw_rectangle(paper.x, paper.y, paper.w, 3.0, if original { STRIPE_ORIGINAL } else { ACCENT });
    let (ix, iy) = ((r.x + (r.w - tw) / 2.0).round(), (recess.y + FRAME).round());
    draw_rectangle(ix - 2.0, iy - 2.0, tw + 4.0, th + 4.0, INK);
    match title {
        Some(t) => draw_texture_ex(t, ix, iy, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() }),
        None => draw_rectangle(ix, iy, tw, th, Color::new(0.2, 0.22, 0.2, 1.0)),
    }
    let mut size = 22.0;
    while width(name, size) > paper.w - 16.0 && size > 12.0 {
        size -= 1.0;
    }
    let mut shown = name.to_string();
    while width(&shown, size) > paper.w - 16.0 && shown.pop().is_some() {}
    centred(&shown, r.x + r.w / 2.0, (iy + th + 2.0 + NAME / 2.0 + size * 0.35).round(), size, INK);
    // the foot, an arrow the way it goes in
    let (fx, fy) = (r.x + r.w / 2.0, r.y + r.h - FOOT / 2.0);
    draw_triangle(vec2(fx - 6.0, fy - 3.0), vec2(fx + 6.0, fy - 3.0), vec2(fx, fy + 4.0), CART_DARK);
}

/// Split text into lines that fit `width` pixels at font size `size` (breaking long words
/// such as paths anywhere).
fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
    let fits = |s: &str| measure_text(s, None, size as u16, 1.0).width <= width;
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if fits(&cand) {
            cur = cand;
            continue;
        }
        if !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        // a word longer than the line: cut it
        for ch in word.chars() {
            cur.push(ch);
            if !fits(&cur) {
                let last = cur.pop().unwrap();
                lines.push(std::mem::take(&mut cur));
                cur.push(last);
            }
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}
