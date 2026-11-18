//! The screens before the editor: the base cartridge (`host::base_cartridge`), asked for until
//! a valid one is chosen, and the projects (`editor\projects\<name>\`): open, new, rename,
//! duplicate, delete, import. The editor comes back here from File, Projects.
use super::level_panel::fit;
use super::widgets::*;
use crate::adapters::fs_repository::{self, ProjectError};
use crate::application::save;
use crate::domain::files;
use crate::ports::{FileDialogs, FileKind};
use host::BaseProblem;
use macroquad::prelude::*;
use std::path::PathBuf;

const W: f32 = 640.0;
const ROW: f32 = 34.0;

/// The panel the screens draw in: centred, `h` high; its left, top and width.
fn panel(h: f32) -> Rect {
    clear_background(col(3));
    let r = Rect::new(((screen_width() - W) / 2.0).max(16.0), ((screen_height() - h) / 2.0).max(16.0), W.min(screen_width() - 32.0), h);
    draw_rectangle(r.x, r.y, r.w, r.h, col(0));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, col(3));
    r
}

fn lines(text: &str, x: f32, mut y: f32, w: f32, size: f32, c: Color) -> f32 {
    for l in text.lines().flat_map(|l| wrap_px(l, w, size)) {
        draw_text(&l, x, y, size, c);
        y += size + 4.0;
    }
    y
}

/// Asking for the base cartridge.
pub struct BaseScreen {
    es: bool,
    problem: BaseProblem,
}

impl BaseScreen {
    pub fn new(es: bool, problem: BaseProblem) -> BaseScreen {
        BaseScreen { es, problem }
    }

    /// One frame; the cartridge once a valid one is chosen.
    pub fn frame(&mut self, dialogs: &dyn FileDialogs) -> Option<PathBuf> {
        let es = self.es;
        let r = panel(240.0);
        let x = r.x + 24.0;
        draw_text(tr(es, "BASE CARTRIDGE", "CARTUCHO BASE"), x, r.y + 40.0, 24.0, col(3));
        let c = if self.problem == BaseProblem::NotChosen { col(3) } else { Severity::Error.colour() };
        lines(&self.problem.text(es), x, r.y + 76.0, r.w - 48.0, 17.0, c);
        let note = tr(es, "The editor and the game share it; the file may have any name.", "El editor y el juego lo comparten; el fichero puede llamarse como sea.");
        lines(note, x, r.y + r.h - 76.0, r.w - 48.0, 15.0, col(2));
        if button(Rect::new(x, r.y + r.h - 50.0, r.w - 48.0, ROW), tr(es, "Choose the cartridge file...", "Elegir el fichero del cartucho..."), true, 17.0) {
            let p = dialogs.pick_cartridge()?;
            match host::base_cartridge::choose(&p) {
                Ok(()) => return Some(p),
                Err(e) => self.problem = e,
            }
        }
        None
    }
}

/// What the project screen ends with.
pub enum ProjectsChoice {
    Open(String),
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Typing {
    New,
    Rename,
    Duplicate,
}

/// The project list and its actions.
pub struct ProjectsScreen {
    es: bool,
    base: PathBuf,
    names: Vec<String>,
    selected: usize,
    typing: Option<(Typing, String)>,
    deleting: bool,
    message: Option<(String, Severity)>,
    scroll: usize,
}

impl ProjectsScreen {
    pub fn new(es: bool, base: PathBuf, chosen: Option<&str>) -> ProjectsScreen {
        let mut s = ProjectsScreen { es, base, names: Vec::new(), selected: 0, typing: None, deleting: false, message: None, scroll: 0 };
        s.reload(chosen);
        s
    }

    /// Start typing a new project's name (the game's "new project in the editor").
    pub fn start_new(&mut self) {
        self.typing = Some((Typing::New, String::new()));
    }

    fn reload(&mut self, chosen: Option<&str>) {
        self.names = host::projects();
        self.selected = chosen.and_then(|c| self.names.iter().position(|n| n == c)).unwrap_or(0);
    }

    fn chosen(&self) -> Option<&String> {
        self.names.get(self.selected)
    }

    fn error_text(&self, e: &ProjectError) -> String {
        let es = self.es;
        match e {
            ProjectError::BadName => format!("{} {} {}", tr(es, "A project name has up to", "Un nombre de proyecto tiene hasta"), host::paths::PROJECT_NAME_MAX, tr(es, "characters (no accents, nor \\ / : * ? \" < > |), does not end in a dot and is not VANILLA.", "caracteres (sin acentos, ni \\ / : * ? \" < > |), no acaba en punto y no es VANILLA.")),
            ProjectError::Taken => tr(es, "Another project has that name.", "Otro proyecto tiene ese nombre.").into(),
            ProjectError::Missing => tr(es, "That project is not there any more.", "Ese proyecto ya no esta.").into(),
            ProjectError::Io(m) => format!("{} {m}", tr(es, "The files could not be written:", "No se pudieron escribir los ficheros:")),
        }
    }

    fn done(&mut self, result: Result<(), ProjectError>, name: &str, ok: String) {
        match result {
            Ok(()) => {
                self.reload(Some(name));
                self.message = Some((ok, Severity::Done));
            }
            Err(e) => self.message = Some((self.error_text(&e), Severity::Error)),
        }
    }

    /// A project file from any folder as a new project (named after the file).
    fn import(&mut self, dialogs: &dyn FileDialogs) {
        let es = self.es;
        let Some(path) = dialogs.open(FileKind::Project) else { return };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let file = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| files::read_project(&t).map_err(|e| format!("{e:?}")));
        let p = match file {
            Ok(p) => p,
            Err(e) => {
                self.message = Some((format!("{} {e}", tr(es, "Not a usable project file:", "No es un fichero de proyecto valido:")), Severity::Error));
                return;
            }
        };
        let name = fs_repository::free_name(&stem);
        let made = fs_repository::create_project(&self.base, &name).and_then(|_| fs_repository::open_project(&self.base, &name)).and_then(|mut o| {
            o.doc.replace_project(p).map_err(|e| ProjectError::Io(format!("{e:?}")))?;
            save::save(&mut o.doc, &o.store).map(|_| ()).map_err(|e| match e {
                save::SaveError::Layout(e) => ProjectError::Io(e.to_string()),
                save::SaveError::Write(e) => ProjectError::Io(e.to_string()),
            })
        });
        if made.is_err() {
            let _ = fs_repository::delete_project(&name);
        }
        let ok = format!("{} {name}", tr(es, "Imported as the project", "Importado como el proyecto"));
        self.done(made, &name, ok);
    }

    /// One frame; what to do once the person has chosen.
    pub fn frame(&mut self, dialogs: &dyn FileDialogs) -> Option<ProjectsChoice> {
        let es = self.es;
        let visible = 9;
        let r = panel(130.0 + visible as f32 * (ROW + 4.0) + 180.0);
        let (x, w) = (r.x + 24.0, r.w - 48.0);
        draw_text(tr(es, "PROJECTS", "PROYECTOS"), x, r.y + 40.0, 24.0, col(3));
        let base = self.base.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        draw_text(&fit(&format!("{} {base}", tr(es, "Base cartridge:", "Cartucho base:")), w, 15.0), x, r.y + 64.0, 15.0, col(2));
        let mut y = r.y + 82.0;
        // the list
        if self.names.is_empty() {
            lines(tr(es, "No projects yet: make a new one, or import a project file.", "Aun no hay proyectos: crea uno nuevo o importa un fichero de proyecto."), x, y + 20.0, w, 17.0, col(3));
        }
        if self.typing.is_none() && !self.deleting && !self.names.is_empty() {
            if is_key_pressed(KeyCode::Down) {
                self.selected = (self.selected + 1).min(self.names.len() - 1);
            }
            if is_key_pressed(KeyCode::Up) {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        self.scroll = self.scroll.min(self.selected).max((self.selected + 1).saturating_sub(visible));
        let mut open = None;
        for (i, name) in self.names.iter().enumerate().skip(self.scroll).take(visible) {
            let row = Rect::new(x, y, w, ROW);
            let rom = host::project_rom_in(name).and_then(|p| std::fs::metadata(p).ok()).and_then(|m| m.modified().ok());
            if button(row, "", i == self.selected, 16.0) {
                if i == self.selected && self.typing.is_none() {
                    open = Some(name.clone());
                }
                self.selected = i;
            }
            let (fg, dim) = if i == self.selected { (col(0), col(1)) } else { (col(3), col(2)) };
            draw_text(&fit(name, w * 0.6, 18.0), row.x + 12.0, row.y + 23.0, 18.0, fg);
            if let Some(t) = rom {
                let when = saved_when(es, t);
                let tw = measure_text(&when, None, 14, 1.0).width;
                draw_text(&when, row.x + row.w - tw - 12.0, row.y + 22.0, 14.0, dim);
            }
            y += ROW + 4.0;
        }
        let y0 = r.y + 82.0 + visible as f32 * (ROW + 4.0) + 8.0;
        // a name being typed, or the delete question
        if let Some((what, text)) = &mut self.typing {
            let label = match what {
                Typing::New => tr(es, "New project's name:", "Nombre del proyecto nuevo:"),
                Typing::Rename => tr(es, "New name:", "Nombre nuevo:"),
                Typing::Duplicate => tr(es, "Name of the copy:", "Nombre de la copia:"),
            };
            draw_text(label, x, y0 + 16.0, 17.0, col(3));
            while let Some(c) = get_char_pressed() {
                if c.is_ascii() && !c.is_ascii_control() && text.chars().count() < host::paths::PROJECT_NAME_MAX {
                    text.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                text.pop();
            }
            let field = Rect::new(x, y0 + 26.0, w, ROW);
            draw_rectangle(field.x, field.y, field.w, field.h, col(3));
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { "" };
            draw_text(&format!("{text}{caret}"), field.x + 8.0, field.y + 23.0, 18.0, col(0));
            let (what, text) = (*what, text.trim().to_string());
            let bw = (w - 12.0) / 2.0;
            if button(Rect::new(x, y0 + 70.0, bw, ROW), tr(es, "OK (Enter)", "Aceptar (Enter)"), true, 16.0) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                self.typing = None;
                let old = self.chosen().cloned().unwrap_or_default();
                let (result, ok) = match what {
                    Typing::New => (fs_repository::create_project(&self.base, &text), format!("{} {text}", tr(es, "New project", "Proyecto nuevo"))),
                    Typing::Rename => (fs_repository::rename_project(&self.base, &old, &text), format!("{old} {} {text}", tr(es, "is now", "ahora es"))),
                    Typing::Duplicate => (fs_repository::duplicate_project(&self.base, &old, &text), format!("{old} {} {text}", tr(es, "copied as", "copiado como"))),
                };
                self.done(result, &text, ok);
            } else if button(Rect::new(x + bw + 12.0, y0 + 70.0, bw, ROW), tr(es, "Cancel (Esc)", "Cancelar (Esc)"), false, 16.0) || is_key_pressed(KeyCode::Escape) {
                self.typing = None;
            }
        } else if self.deleting {
            let name = self.chosen().cloned().unwrap_or_default();
            lines(&format!("{} {name}? {}", tr(es, "Delete", "Borrar"), tr(es, "Its cartridge and its saves are deleted for good.", "Su cartucho y sus partidas se borran para siempre.")), x, y0 + 16.0, w, 17.0, Severity::Warning.colour());
            let bw = (w - 12.0) / 2.0;
            if button(Rect::new(x, y0 + 70.0, bw, ROW), tr(es, "Delete", "Borrar"), true, 16.0) {
                self.deleting = false;
                let ok = format!("{} {name}", tr(es, "Deleted", "Borrado"));
                let result = fs_repository::delete_project(&name);
                self.done(result, "", ok);
            } else if button(Rect::new(x + bw + 12.0, y0 + 70.0, bw, ROW), tr(es, "Keep it (Esc)", "Conservarlo (Esc)"), false, 16.0) || is_key_pressed(KeyCode::Escape) {
                self.deleting = false;
            }
        } else {
            // the actions: open first, then what makes, changes or removes a project
            let bw = (w - 3.0 * 8.0) / 4.0;
            let at = |i: usize, row: usize| Rect::new(x + i as f32 * (bw + 8.0), y0 + 8.0 + row as f32 * (ROW + 8.0), bw, ROW);
            let some = self.chosen().is_some();
            if (button(at(0, 0), tr(es, "Open (Enter)", "Abrir (Enter)"), some, 16.0) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter)) && some {
                open = self.chosen().cloned();
            }
            if button(at(1, 0), tr(es, "New", "Nuevo"), false, 16.0) {
                while get_char_pressed().is_some() {}
                self.typing = Some((Typing::New, String::new()));
            }
            if button(at(2, 0), tr(es, "Import...", "Importar..."), false, 16.0) {
                self.import(dialogs);
            }
            if button(at(3, 0), tr(es, "Quit", "Salir"), false, 16.0) {
                return Some(ProjectsChoice::Quit);
            }
            if some {
                let name = self.chosen().cloned().unwrap_or_default();
                if button(at(0, 1), tr(es, "Rename", "Renombrar"), false, 16.0) {
                    while get_char_pressed().is_some() {}
                    self.typing = Some((Typing::Rename, name.clone()));
                }
                if button(at(1, 1), tr(es, "Duplicate", "Duplicar"), false, 16.0) {
                    while get_char_pressed().is_some() {}
                    self.typing = Some((Typing::Duplicate, fs_repository::free_name(&format!("{name} copy"))));
                }
                if button(at(2, 1), tr(es, "Delete", "Borrar"), false, 16.0) {
                    self.deleting = true;
                }
            }
        }
        if let Some((m, kind)) = &self.message {
            message_lines(m, x, r.y + r.h - 20.0, w, 15.0, *kind);
        }
        open.map(ProjectsChoice::Open)
    }
}

/// When a project was last saved, as the screen says it.
fn saved_when(es: bool, t: std::time::SystemTime) -> String {
    let ago = std::time::SystemTime::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    let (n, en, sp) = match ago {
        0..=59 => return tr(es, "saved just now", "guardado ahora mismo").into(),
        60..=3599 => (ago / 60, "min ago", "min"),
        3600..=86399 => (ago / 3600, "h ago", "h"),
        _ => (ago / 86400, "days ago", "dias"),
    };
    if es {
        format!("guardado hace {n} {sp}")
    } else {
        format!("saved {n} {en}")
    }
}
