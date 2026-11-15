//! The start screen when the game is opened without a cartridge on the command line: play
//! the cartridge used last time (or the one found where the game looks), or choose another
//! file with the system's "Open" dialog. The caller remembers the choice once the game
//! starts with it.
use crate::domain::i18n::LANG_ES;
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

pub enum Choice {
    /// Nothing yet.
    Wait,
    Play(PathBuf),
    /// Open the system's dialog to choose a file.
    Browse,
    Quit,
}

pub struct Picker {
    /// The cartridge offered: the one used last, or one found.
    pub offer: Option<PathBuf>,
    /// The offer is the one used last (not just found).
    pub remembered: bool,
    /// A remembered file that is no longer there.
    pub missing: Option<PathBuf>,
    /// Why the last choice didn't start.
    pub error: Option<String>,
    es: bool,
}

const BG: u32 = 0x101820;
const TEXT: Color = Color::new(0.92, 0.95, 0.88, 1.0);
const DIM: Color = Color::new(0.62, 0.68, 0.6, 1.0);
const ACCENT: Color = Color::new(0.55, 0.75, 0.35, 1.0);

impl Picker {
    pub fn new(lang: usize) -> Picker {
        let last = host::last_rom();
        let remembered = last.is_some();
        let missing = if remembered { None } else { host::last_rom_named() };
        let offer = last.or_else(|| host::find_rom(None).filter(|p| p.is_file()));
        Picker { offer, remembered, missing, error: None, es: lang == LANG_ES }
    }

    fn tr<'a>(&self, en: &'a str, es: &'a str) -> &'a str {
        if self.es {
            es
        } else {
            en
        }
    }

    /// Where the "Open" dialog starts: the offered file's folder, else the editor's copies.
    pub fn start_dir(&self) -> Option<PathBuf> {
        let edits = host::exe_dir().join("edits");
        self.offer.as_deref().and_then(Path::parent).map(Path::to_path_buf).or_else(|| edits.is_dir().then_some(edits))
    }

    pub fn dialog_texts(&self) -> (&'static str, &'static str) {
        if self.es {
            ("Elige un cartucho de Game Boy", "Cartuchos de Game Boy")
        } else {
            ("Choose a Game Boy cartridge", "Game Boy cartridges")
        }
    }

    /// Draw the screen and read the keys and mouse.
    pub fn frame(&self) -> Choice {
        clear_background(Color::from_hex(BG));
        let (w, h) = (screen_width(), screen_height());
        let x = 28.0;
        let mut y = 56.0;
        draw_text(self.tr("Cartridge", "Cartucho"), x, y, 40.0, TEXT);
        y += 44.0;
        let mut choice = Choice::Wait;
        let mouse: Vec2 = mouse_position().into();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let button = |label: &str, y: f32, on: bool| -> bool {
            let r = Rect::new(x, y, w - 2.0 * x, 40.0);
            let hover = on && r.contains(mouse);
            draw_rectangle(r.x, r.y, r.w, r.h, if hover { Color::new(0.25, 0.33, 0.22, 1.0) } else { Color::new(0.16, 0.2, 0.17, 1.0) });
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, if on { ACCENT } else { DIM });
            draw_text(label, r.x + 14.0, r.y + 27.0, 24.0, if on { TEXT } else { DIM });
            hover && clicked
        };
        match &self.offer {
            Some(p) => {
                let what = if self.remembered { self.tr("Used last time:", "Usado la ultima vez:") } else { self.tr("Found:", "Encontrado:") };
                draw_text(what, x, y, 22.0, DIM);
                y += 30.0;
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                for line in wrap(&name, w - 2.0 * x, 26.0) {
                    draw_text(&line, x, y, 26.0, TEXT);
                    y += 28.0;
                }
                let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
                for line in wrap(&dir, w - 2.0 * x, 18.0) {
                    draw_text(&line, x, y, 18.0, DIM);
                    y += 20.0;
                }
                y += 14.0;
                if button(self.tr("Enter: play this one", "Enter: jugar con este"), y, true) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    choice = Choice::Play(p.clone());
                }
                y += 50.0;
            }
            None => {
                draw_text(self.tr("No cartridge chosen yet.", "Aun no hay cartucho elegido."), x, y, 22.0, DIM);
                y += 36.0;
            }
        }
        if let Some(m) = &self.missing {
            for line in wrap(&format!("{} {}", self.tr("Not found any more:", "Ya no esta:"), m.display()), w - 2.0 * x, 18.0) {
                draw_text(&line, x, y, 18.0, DIM);
                y += 20.0;
            }
            y += 8.0;
        }
        if button(self.tr("O: choose another file...", "O: elegir otro archivo..."), y, true) || is_key_pressed(KeyCode::O) {
            choice = Choice::Browse;
        }
        y += 50.0;
        if button(self.tr("Esc: quit", "Esc: salir"), y, true) || is_key_pressed(KeyCode::Escape) {
            choice = Choice::Quit;
        }
        y += 60.0;
        if let Some(e) = &self.error {
            let red = Color::new(0.95, 0.45, 0.4, 1.0);
            draw_text(self.tr("That file did not start:", "Ese archivo no arranco:"), x, y, 22.0, red);
            y += 26.0;
            for line in e.lines().flat_map(|l| wrap(l, w - 2.0 * x, 18.0)) {
                draw_text(&line, x, y, 18.0, red);
                y += 20.0;
            }
        }
        let note = self.tr(
            "The editor's copies are in the 'edits' folder next to the game. The choice is remembered once the game starts with it.",
            "Las copias del editor estan en la carpeta 'edits' junto al juego. La eleccion se recuerda cuando el juego arranca con ella.",
        );
        let lines = wrap(note, w - 2.0 * x, 16.0);
        for (i, line) in lines.iter().enumerate() {
            draw_text(line, x, h - 16.0 - (lines.len() - 1 - i) as f32 * 18.0, 16.0, DIM);
        }
        choice
    }
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
