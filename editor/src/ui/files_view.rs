//! Level and project files from the panels: the file actions, run once the keys are let go,
//! and what the status bar says about each.
use super::shortcuts::Action;
use super::widgets::tr;
use super::Editor;
use crate::application::exchange::{self, ExchangeError};
use crate::domain::files::FileError;
use cart_model::levels::LEVELS;
use macroquad::prelude::*;

impl Editor {
    /// A dialog takes the keyboard while it is open, so the editor would never see Ctrl or
    /// Shift go up: the action waits until nothing is held.
    pub(super) fn queue_file(&mut self, a: Action) {
        if a == Action::ReplaceLevel && self.cur < LEVELS {
            self.status = tr(self.es, "Only new levels (32 and up) can be replaced from a file; the game's 32 are only edited.", "Solo los niveles nuevos (32 en adelante) se pueden reemplazar con un fichero; los 32 del juego solo se editan.").into();
            return;
        }
        self.pending_file = Some(a);
    }

    pub(super) fn run_pending_file(&mut self) {
        let held = [KeyCode::LeftControl, KeyCode::RightControl, KeyCode::LeftShift, KeyCode::RightShift, KeyCode::LeftAlt, KeyCode::RightAlt].iter().any(|&k| is_key_down(k))
            || [MouseButton::Left, MouseButton::Right, MouseButton::Middle].iter().any(|&b| is_mouse_button_down(b));
        if held {
            return;
        }
        if let Some(a) = self.pending_file.take() {
            self.run_file(a);
        }
    }

    /// The cartridge's name, to suggest file names.
    fn cartridge_name(&self) -> String {
        let stem = self.store.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        stem.trim_end_matches(" (edit)").to_string()
    }

    fn run_file(&mut self, a: Action) {
        let es = self.es;
        let (dialogs, files) = (&*self.dialogs, &*self.text_files);
        let l = self.cur;
        let status = match a {
            Action::ExportLevel => exchange::export_level(&self.doc, l, &format!("{} {} {l:02}", self.cartridge_name(), tr(es, "level", "nivel")), dialogs, files)
                .map(|p| p.map(|p| format!("{} {l:02} {} {}", tr(es, "Level", "Nivel"), tr(es, "exported to", "exportado a"), p.display()))),
            Action::ImportLevel => match exchange::import_level(&mut self.doc, dialogs, files) {
                Ok(Some(n)) => {
                    self.select_level(n);
                    Ok(Some(format!("{} {n:02} {} {:02}. {}", tr(es, "New level", "Nivel nuevo"), tr(es, "from the file; plays as level", "desde el fichero; se juega como el nivel"), self.doc.base_of(n), tr(es, "Save to keep it.", "Guarda para conservarlo."))))
                }
                other => other.map(|_| None),
            },
            Action::ReplaceLevel => {
                let base = self.doc.base_of(l);
                match exchange::replace_level(&mut self.doc, l, dialogs, files) {
                    Ok(Some(())) => {
                        self.level_changed(base);
                        Ok(Some(format!("{} {l:02} {} {:02}. {}", tr(es, "Level", "Nivel"), tr(es, "replaced from the file; plays as level", "reemplazado con el fichero; se juega como el nivel"), self.doc.base_of(l), tr(es, "Undo brings it back.", "Deshacer lo recupera."))))
                    }
                    other => other.map(|_| None),
                }
            }
            Action::ExportProject => exchange::export_project(&self.doc, &self.cartridge_name(), dialogs, files).map(|p| p.map(|p| format!("{} {}", tr(es, "Project exported to", "Proyecto exportado a"), p.display()))),
            Action::ImportProject => match exchange::import_project(&mut self.doc, dialogs, files) {
                Ok(Some(())) => {
                    self.project_replaced();
                    Ok(Some(tr(es, "Project imported. Nothing is written until you save; closing without saving keeps the edited copy as it was.", "Proyecto importado. No se escribe nada hasta que guardes; si cierras sin guardar, la copia editada queda como estaba.").to_string()))
                }
                other => other.map(|_| None),
            },
            _ => Ok(None),
        };
        match status {
            Ok(Some(s)) => self.status = s,
            Ok(None) => {}
            Err(e) => self.status = self.file_error(&e),
        }
    }

    /// Everything changed: back to the first level, nothing left from what was shown.
    fn project_replaced(&mut self) {
        self.clipboard = None;
        self.looks.clear();
        self.overview = None;
        self.enter_world(false);
        self.select_level(0);
    }

    fn file_error(&self, e: &ExchangeError) -> String {
        let es = self.es;
        match e {
            ExchangeError::Io(e) => format!("{}: {e}", tr(es, "The file could not be read or written", "No se pudo leer o escribir el fichero")),
            ExchangeError::NoRoom => tr(es, "There can't be more levels.", "No caben mas niveles.").into(),
            ExchangeError::Protected => tr(es, "Only new levels (32 and up) can be replaced from a file.", "Solo los niveles nuevos (32 en adelante) se pueden reemplazar con un fichero.").into(),
            ExchangeError::File(f) => match f {
                FileError::NotLevel => tr(es, "This is not a level file.", "Este no es un fichero de nivel.").into(),
                FileError::NotProject => tr(es, "This is not a project file.", "Este no es un fichero de proyecto.").into(),
                FileError::Newer => tr(es, "This file was made by a newer editor.", "Este fichero lo hizo un editor mas nuevo.").into(),
                FileError::Damaged(line) => format!("{} {line}).", tr(es, "The file is damaged or incomplete (line", "El fichero esta danado o incompleto (linea")),
                FileError::BaseOutside(b) => format!("{} {b}.", tr(es, "The file's level plays as a level that isn't one of the 32:", "El nivel del fichero se juega como un nivel que no es de los 32:")),
                FileError::OtherCartridge => tr(es, "This project belongs to another cartridge; nothing was changed.", "Este proyecto es de otro cartucho; no se ha cambiado nada.").into(),
            },
        }
    }
}
